"""Run comparable server startup, load, memory, and fidelity measurements."""
import argparse
import datetime
import json
import os
import pathlib
import platform
import re
import signal
import socket
import subprocess
import sys
import tempfile
import threading
import time
import urllib.request

BASE = pathlib.Path(__file__).resolve().parent


def machine():
    cpu = 'unknown'; ram = None
    try:
        with open('/proc/cpuinfo') as source:
            for line in source:
                if line.lower().startswith('model name'):
                    cpu = line.split(':', 1)[1].strip(); break
        with open('/proc/meminfo') as source:
            for line in source:
                if line.startswith('MemTotal:'):
                    ram = int(line.split()[1]) * 1024; break
    except OSError:
        pass
    return {'cpu_model': cpu, 'cores': os.cpu_count(), 'ram_bytes': ram,
            'os': platform.platform(), 'kernel': platform.release()}


def ready(url, timeout=60):
    until = time.monotonic() + timeout
    while time.monotonic() < until:
        try:
            with urllib.request.urlopen(url, timeout=.3) as response:
                if response.status == 200:
                    return
        except urllib.error.HTTPError:
            return
        except Exception:
            time.sleep(.05)
    raise RuntimeError('server did not return HTTP 200: ' + url)


def children(pid):
    """Read all task child lists, recursively, to include forked workers."""
    found = set(); pending = [int(pid)]
    while pending:
        current = pending.pop()
        if current in found:
            continue
        found.add(current)
        try:
            tasks = pathlib.Path(f'/proc/{current}/task').iterdir()
            for task in tasks:
                child_file = pathlib.Path(task, 'children')
                pending.extend(int(value) for value in child_file.read_text().split())
        except (OSError, ValueError):
            continue
    return found


def process_rss(pid):
    total = 0
    for child in children(pid):
        try:
            for line in pathlib.Path(f'/proc/{child}/status').read_text().splitlines():
                if line.startswith('VmRSS:'):
                    total += int(line.split()[1]) * 1024
                    break
        except (OSError, ValueError):
            pass
    return total


def rss_sampler(pid, stop, output, container=None):
    peak = 0
    while not stop.is_set():
        try:
            if container:
                raw = subprocess.run(['docker', 'stats', '--no-stream', '--format', '{{.MemUsage}}', container],
                                     text=True, capture_output=True, check=True).stdout.strip()
                used = raw.split('/')[0].strip()
                match = re.fullmatch(r'([0-9.]+)(B|kB|MB|GB|TB|KiB|MiB|GiB|TiB)', used)
                if match:
                    unit = match.group(2)
                    scale = {'B': 1, 'kB': 1000, 'MB': 1000**2, 'GB': 1000**3, 'TB': 1000**4,
                             'KiB': 1024, 'MiB': 1024**2, 'GiB': 1024**3, 'TiB': 1024**4}[unit]
                    peak = max(peak, int(float(match.group(1)) * scale))
            else:
                peak = max(peak, process_rss(pid))
        except (OSError, subprocess.SubprocessError, ValueError, KeyError, IndexError):
            pass
        stop.wait(.1)
    output.append(peak)


def client_command(client, url, concurrency, duration, method='GET'):
    if client[0] == 'oha':
        return [*client, '-c', str(concurrency), '-z', duration, '-m', method, '--output-format', 'json', url]
    if client[0] == 'wrk':
        return [*client, '-t', str(concurrency), '-c', str(concurrency), '-d', duration, url]
    return [*client, url, '--method', method, '--concurrency', str(concurrency), '--duration', str(duration_seconds(duration))]


def duration_seconds(value):
    return float(value[:-1]) if value.endswith('s') else float(value)


def stop_server(proc, container=None):
    if container:
        subprocess.run(['docker', 'stop', '--time', '1', container], check=False,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    elif proc.poll() is None:
        try:
            os.killpg(proc.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
    try:
        proc.wait(timeout=15)
    except subprocess.TimeoutExpired:
        if proc.poll() is None:
            try:
                os.killpg(proc.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        proc.wait(timeout=5)


def fixture_operations(fixture):
    document = json.loads(fixture.read_text())
    return [{'operation': op.get('operationId', f'{method.upper()} {path}'),
             'method': method.upper(), 'path': re.sub(r'\{[^}]+\}', '1', path)}
            for path, item in document.get('paths', {}).items()
            for method, op in item.items()
            if method.lower() in {'get', 'head', 'options', 'post', 'put', 'patch', 'delete'}]


def client_identity(client):
    if client[0] == 'oha':
        version = subprocess.run([*client, '--version'], text=True, capture_output=True, check=True).stdout.strip()
        return {'name': 'oha', 'version': version.split()[-1]}
    return {'name': 'load.py', 'version': 'fallback (client-bound)'}


def load_result(client, url, concurrency, duration, method='GET'):
    command = client_command(client, url, concurrency, duration, method)
    completed = subprocess.run(command, text=True, capture_output=True)
    if completed.returncode:
        return {'client': command, 'exit_code': completed.returncode,
                'error': completed.stderr.strip() or 'load client failed', 'raw_output': completed.stdout.strip(),
                'requests': None, 'errors': None, 'non_2xx': None, 'p50_ms': None, 'p95_ms': None,
                'p99_ms': None, 'failed': True}
    if client[-1].endswith('load.py'):
        data = json.loads(completed.stdout)
        return {'client': command, 'exit_code': 0, **data, 'error': None, 'raw_output': completed.stdout.strip()}
    data = json.loads(completed.stdout)
    status_counts = {int(code): count for code, count in data['statusCodeDistribution'].items()}
    error_count = sum(data['errorDistribution'].values())
    metrics = data['metrics']; latency = metrics['latency_ms']
    return {'client': command, 'exit_code': 0, 'requests': sum(status_counts.values()) + error_count,
            'throughput_rps': metrics['requests_per_sec'], 'errors': error_count,
            'non_2xx': sum(count for code, count in status_counts.items() if not 200 <= code < 300),
            'status_counts': status_counts, 'p50_ms': latency['p50'], 'p95_ms': latency['p95'],
            'p99_ms': latency['p99'], 'failed': metrics['success_rate'] < 1,
            'error': None, 'raw_output': completed.stdout.strip()}


def runtime_version(name, cmd, container=False, base_url='http://127.0.0.1:4010'):
    if name == 'WireMock' and container:
        return os.environ.get('WIREMOCK_VERSION', '3.13.2') + ' (image tag)'
    if name == 'WireMock':
        with urllib.request.urlopen(base_url + '/__admin/version', timeout=2) as response:
            return response.read().decode().strip()
    if name == 'MockForge':
        return subprocess.run([cmd[0], '--version'], text=True, capture_output=True, check=True).stdout.strip()
    if name == 'Prism':
        return subprocess.run(['npx', '--yes', cmd[1], '--version'], text=True,
                              capture_output=True, check=True).stdout.strip()
    return subprocess.run(['npx', '--yes', cmd[1], '--version'], text=True,
                          capture_output=True, check=True).stdout.strip()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', required=True); parser.add_argument('--runs', type=int, default=5)
    parser.add_argument('--warmup', type=int, default=1); parser.add_argument('--warmup-duration', default='2s')
    parser.add_argument('--concurrency', type=int, default=16); parser.add_argument('--duration', default='15s')
    parser.add_argument('--port', type=int, default=4010); parser.add_argument('--load-client', nargs='+', required=True)
    parser.add_argument('--wiremock-version', default='3.13.2'); parser.add_argument('--prism-version', default='5.16.0')
    parser.add_argument('--mockoon-version', default='9.10.0')
    args = parser.parse_args()
    root_url = f'http://127.0.0.1:{args.port}'
    with tempfile.TemporaryDirectory(prefix='mockforge-compare-') as temp:
        adapters = pathlib.Path(temp) / 'adapters'
        subprocess.run([sys.executable, str(BASE / 'adapters.py'), str(BASE / 'specs'), str(adapters)], check=True)
        docker = not bool(os.environ.get('WIREMOCK_JAR'))
        docker_name = f'mockforge-compare-{os.getpid()}'
        wire_command = (['docker', 'run', '--rm', '--name', docker_name, '-p', f'{args.port}:8080',
                         '-v', f'{adapters}:/home/wiremock/mappings:ro',
                         f'wiremock/wiremock:{args.wiremock_version}'] if docker else
                        ['java', '-jar', os.environ['WIREMOCK_JAR'], '--port', str(args.port), '--root-dir', str(adapters)])
        tool_defs = [
            ('MockForge', [args.binary, 'serve', '--spec', None, '--http-port', str(args.port)], 'process'),
            ('WireMock', wire_command, 'container' if docker else 'process'),
            ('Prism', ['npx', '--yes', f'@stoplight/prism-cli@{args.prism_version}', 'mock', '-h', '0.0.0.0', '-p', str(args.port), None], 'process'),
            ('Mockoon', ['npx', '--yes', f'@mockoon/cli@{args.mockoon_version}', 'start', '--data', None, '--port', str(args.port)], 'process'),
        ]
        all_tools = []
        for tool_name, template, mode in tool_defs:
            fixture_results = []
            for fixture in sorted((BASE / 'specs').glob('*.json')):
                operations = fixture_operations(fixture)
                probe_url = root_url + operations[0]['path']
                cmd = [str(fixture) if value is None else value for value in template]
                container = docker_name if tool_name == 'WireMock' and docker else None
                def discard_warmups():
                    for _ in range(args.warmup):
                        proc = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                                start_new_session=True)
                        ready(probe_url)
                        for operation in operations:
                            load_result(args.load_client, root_url + operation['path'], args.concurrency, args.warmup_duration, operation['method'])
                        stop_server(proc, container)

                starts = []
                for _ in range(args.runs):
                    discard_warmups()
                    started = time.perf_counter(); proc = subprocess.Popen(cmd, stdout=subprocess.DEVNULL,
                        stderr=subprocess.DEVNULL, start_new_session=True)
                    ready(probe_url); starts.append(time.perf_counter() - started)
                    stop_server(proc, container)
                discard_warmups()
                proc = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                        start_new_session=True)
                ready(probe_url)
                version = runtime_version(tool_name, cmd, container=container is not None, base_url=root_url)
                stop = threading.Event(); peaks = []
                sampler = threading.Thread(target=rss_sampler, args=(proc.pid, stop, peaks, container))
                sampler.start()
                per_operation_load = []
                for operation in operations:
                    measured = load_result(args.load_client, root_url + operation['path'], args.concurrency, args.duration, operation['method'])
                    per_operation_load.append({**operation, 'load': measured})
                stop.set(); sampler.join()
                if tool_name == 'WireMock':
                    fidelity_result = {'status': 'n/a', 'reason': 'no native OpenAPI import'}
                else:
                    fidelity = subprocess.run([sys.executable, str(BASE / 'fidelity.py'), str(fixture), root_url],
                                              text=True, capture_output=True, check=True)
                    fidelity_result = json.loads(fidelity.stdout)
                stop_server(proc, container)
                successful_operations = [item['operation'] for item in per_operation_load
                                         if not item['load'].get('failed') and item['load'].get('non_2xx') == 0
                                         and item['load'].get('errors') == 0]
                fixture_results.append({'fixture': fixture.stem, 'version': version,
                    'metrics': {'cold_start_seconds_median': sorted(starts)[len(starts)//2],
                                'cold_start_run_count': len(starts),
                                'warmup_runs_discarded': args.warmup * (args.runs + 1),
                                'load': {'client': client_identity(args.load_client), 'per_operation': per_operation_load,
                                         'operations_returning_2xx': successful_operations},
                                'fidelity': fidelity_result,
                                'peak_rss_bytes': max(peaks, default=None), 'peak_rss_mode': mode}})
            all_tools.append({'tool': tool_name, 'fixtures': fixture_results})
        result = {'schema_version': 3, 'date': datetime.date.today().isoformat(), 'host': socket.gethostname(),
                  'load_client': client_identity(args.load_client),
                  'machine': machine(), 'parameters': {'runs': args.runs, 'warmup_runs': args.warmup,
                  'warmup_duration': args.warmup_duration, 'concurrency': args.concurrency,
                  'duration': args.duration}, 'tools': all_tools}
        out = BASE / 'results' / f"{result['date']}-{result['host']}.json"
        out.parent.mkdir(exist_ok=True); out.write_text(json.dumps(result, indent=2) + '\n'); print(out)


if __name__ == '__main__':
    main()
