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
    """Wait for a fixture operation to return a successful response."""
    started = time.perf_counter()
    until = time.monotonic() + timeout
    while time.monotonic() < until:
        try:
            with urllib.request.urlopen(url, timeout=.3) as response:
                if 200 <= response.status < 300:
                    return time.perf_counter() - started
        except urllib.error.HTTPError as error:
            if 200 <= error.code < 300:
                return time.perf_counter() - started
        except Exception:
            time.sleep(.05)
    raise RuntimeError('server did not return HTTP 2xx: ' + url)


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
        if not peak:
            stop.wait(.02)
        else:
            stop.wait(.1)
    output.append(peak)


def client_command(client, url, concurrency, duration, method='GET'):
    if pathlib.Path(client[0]).name == 'oha':
        return [*client, '-c', str(concurrency), '-z', duration, '-m', method, '--no-tui', '--output-format', 'json', url]
    if client[0] == 'wrk':
        return [*client, '-t', str(concurrency), '-c', str(concurrency), '-d', duration, url]
    return [*client, url, '--method', method, '--concurrency', str(concurrency), '--duration', str(duration_seconds(duration))]


def duration_seconds(value):
    return float(value[:-1]) if value.endswith('s') else float(value)


def stop_server(proc, port, container=None):
    if container:
        subprocess.run(['docker', 'rm', '-f', container], check=False,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        return
    if proc.poll() is None:
        try:
            os.killpg(proc.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
    try:
        proc.wait(timeout=30)
    except subprocess.TimeoutExpired:
        pass

    if wait_for_port_free(port, timeout=30):
        return

    try:
        os.killpg(proc.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        pass
    if wait_for_port_free(port, timeout=30):
        return

    holders = port_listener_pids(port)
    holder_text = ', '.join(str(pid) for pid in holders) if holders else 'unknown'
    raise RuntimeError(f'port {port} remained occupied after stopping process group {proc.pid}; '
                       f'listener PID(s): {holder_text}')


def port_is_free(port):
    with socket.socket() as probe:
        probe.settimeout(.25)
        return probe.connect_ex(('127.0.0.1', port)) != 0


def wait_for_port_free(port, timeout):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if port_is_free(port):
            return True
        time.sleep(.1)
    return port_is_free(port)


def port_listener_pids(port):
    """Find Linux process IDs with a listening socket on the given port."""
    inodes = set()
    port_hex = f'{port:04X}'
    for table in ('/proc/net/tcp', '/proc/net/tcp6'):
        try:
            lines = pathlib.Path(table).read_text().splitlines()[1:]
        except OSError:
            continue
        for line in lines:
            fields = line.split()
            if len(fields) > 9 and fields[1].rsplit(':', 1)[-1].upper() == port_hex and fields[3] == '0A':
                inodes.add(fields[9])
    holders = set()
    for process in pathlib.Path('/proc').iterdir():
        if not process.name.isdigit():
            continue
        try:
            descriptors = (process / 'fd').iterdir()
            for descriptor in descriptors:
                try:
                    target = os.readlink(descriptor)
                except OSError:
                    continue
                if target.startswith('socket:[') and target[8:-1] in inodes:
                    holders.add(int(process.name))
                    break
        except OSError:
            continue
    return sorted(holders)


def assert_port_free(port):
    with socket.socket() as listener:
        listener.settimeout(.25)
        if listener.connect_ex(('127.0.0.1', port)) == 0:
            raise RuntimeError(f'port {port} is already listening; refusing to start benchmark server')


def start_server(cmd, port):
    assert_port_free(port)
    env = os.environ.copy()
    if pathlib.Path(cmd[0]).name == 'mockforge':
        env['MOCKFORGE_RATE_LIMIT_ENABLED'] = 'false'
    return subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                            start_new_session=True, env=env)


def fixture_operations(fixture):
    document = json.loads(fixture.read_text())
    return [{'operation': op.get('operationId', f'{method.upper()} {path}'),
             'method': method.upper(), 'path': re.sub(r'\{[^}]+\}', '1', path)}
            for path, item in document.get('paths', {}).items()
            for method, op in item.items()
            if method.lower() in {'get', 'head', 'options', 'post', 'put', 'patch', 'delete'}]


def client_identity(client):
    if pathlib.Path(client[0]).name == 'oha':
        version = subprocess.run([*client, '--version'], text=True, capture_output=True, check=True)
        output = (version.stdout or version.stderr).strip()
        return {'name': 'oha', 'version': output}
    return {'name': 'load.py', 'version': 'fallback (client-bound)'}


def load_result(client, url, concurrency, duration, method='GET'):
    command = client_command(client, url, concurrency, duration, method)
    completed = subprocess.run(command, text=True, capture_output=True)
    if completed.returncode:
        return {'client_name': client_identity(client), 'client': command, 'exit_code': completed.returncode,
                'error': completed.stderr.strip() or 'load client failed', 'raw_output': completed.stdout.strip(),
                'requests': None, 'errors': None, 'non_2xx': None, 'p50_ms': None, 'p95_ms': None,
                'p99_ms': None, 'failed': True}
    if client[-1].endswith('load.py'):
        data = json.loads(completed.stdout)
        return {'client_name': client_identity(client), 'client': command, 'exit_code': 0, **data, 'error': None, 'raw_output': completed.stdout.strip()}
    data = json.loads(completed.stdout)
    status_counts = {int(code): count for code, count in data['statusCodeDistribution'].items()}
    error_count = sum(data['errorDistribution'].values())
    metrics = data['metrics']; latency = data['latencyPercentiles']
    return {'client_name': client_identity(client), 'client': command, 'exit_code': 0, 'requests': sum(status_counts.values()) + error_count,
            'throughput_rps': metrics['requests_per_sec'], 'errors': error_count,
            'non_2xx': sum(count for code, count in status_counts.items() if not 200 <= code < 300),
            'status_counts': status_counts, 'p50_ms': latency['p50'] * 1000, 'p95_ms': latency['p95'] * 1000,
            'p99_ms': latency['p99'] * 1000, 'failed': metrics['success_rate'] < 1,
            'error': None, 'raw_output': completed.stdout.strip()}


def runtime_version(name, cmd, container=False, base_url='http://127.0.0.1:4010'):
    if name == 'WireMock' and container:
        return os.environ.get('WIREMOCK_VERSION', '3.13.2') + ' (image tag)'
    if name == 'WireMock':
        with urllib.request.urlopen(base_url + '/__admin/version', timeout=2) as response:
            return response.read().decode().strip()
    if name == 'MockForge':
        return subprocess.run([cmd[0], '--version'], text=True, capture_output=True, check=True).stdout.strip()
    package_prefix = '@stoplight/prism-cli@' if name == 'Prism' else '@mockoon/cli@'
    package_spec = next((part for part in cmd if part.startswith(package_prefix)), None)
    if package_spec is None:
        raise RuntimeError(f'{name} command is missing package spec {package_prefix!r}')
    executable = 'prism' if name == 'Prism' else 'mockoon-cli'
    output = subprocess.run(['npx', '--yes', f'--package={package_spec}', executable, '--version'],
                            text=True, capture_output=True, check=True).stdout.strip()
    expected = package_spec.rsplit('@', 1)[-1]
    match = re.search(r'\d+\.\d+\.\d+(?:[-+][\w.-]+)?', output)
    actual = match.group(0) if match else output
    if actual != expected:
        raise RuntimeError(f'{name} version mismatch: expected {expected}, got {output!r}')
    return actual


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', required=True); parser.add_argument('--runs', type=int, default=5)
    parser.add_argument('--warmup', type=int, default=1); parser.add_argument('--warmup-duration', default='2s')
    parser.add_argument('--concurrency', type=int, default=16); parser.add_argument('--duration', default='15s')
    parser.add_argument('--port', type=int, default=4010); parser.add_argument('--load-client', nargs='+', required=True)
    parser.add_argument('--wiremock-version', default='3.13.2'); parser.add_argument('--prism-version', default='5.16.0')
    parser.add_argument('--mockoon-version', default='9.10.0')
    parser.add_argument('--tools', nargs='+', choices=['MockForge', 'WireMock', 'Prism', 'Mockoon'])
    parser.add_argument('--fixtures', nargs='+')
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
            ('MockForge', [args.binary, 'serve', '--spec', None, '--http-port', str(args.port), '--no-rate-limit'], 'process'),
            ('WireMock', wire_command, 'container' if docker else 'process'),
            ('Prism', ['npx', '--yes', f'@stoplight/prism-cli@{args.prism_version}', 'mock', '-h', '0.0.0.0', '-p', str(args.port), None], 'process'),
            ('Mockoon', ['npx', '--yes', f'@mockoon/cli@{args.mockoon_version}', 'start', '--data', None, '--port', str(args.port)], 'process'),
        ]
        all_tools = []
        for tool_name, template, mode in tool_defs:
            if args.tools and tool_name not in args.tools:
                continue
            fixture_results = []
            for fixture in sorted((BASE / 'specs').glob('*.json')):
                if args.fixtures and fixture.stem not in args.fixtures:
                    continue
                operations = fixture_operations(fixture)
                probe = next((op for op in operations if op['method'] == 'GET'), operations[0])
                probe_url = root_url + probe['path']
                cmd = [str(fixture) if value is None else value for value in template]
                container = docker_name if tool_name == 'WireMock' and docker else None
                def discard_warmups():
                    for _ in range(args.warmup):
                        if container:
                            assert_port_free(args.port)
                        proc = start_server(cmd, args.port)
                        ready(probe_url)
                        for operation in operations:
                            load_result(args.load_client, root_url + operation['path'], args.concurrency, args.warmup_duration, operation['method'])
                        stop_server(proc, args.port, container)

                starts = []
                for _ in range(args.runs):
                    discard_warmups()
                    if container:
                        assert_port_free(args.port)
                    started = time.perf_counter(); proc = start_server(cmd, args.port)
                    ready(probe_url); starts.append(time.perf_counter() - started)
                    stop_server(proc, args.port, container)
                discard_warmups()
                if container:
                    assert_port_free(args.port)
                proc = start_server(cmd, args.port)
                ready(probe_url)
                version = runtime_version(tool_name, cmd, container=container is not None, base_url=root_url)
                stop = threading.Event(); peaks = []
                sampler = threading.Thread(target=rss_sampler, args=(proc.pid, stop, peaks, container))
                sampler.start()
                # Ensure a sample is taken after the server starts and before traffic begins.
                time.sleep(.12)
                per_operation_load = []
                load_failed = False
                for operation in operations:
                    # Capture the live server before and throughout the load phase.
                    if not per_operation_load:
                        time.sleep(.12)
                    measured = load_result(args.load_client, root_url + operation['path'], args.concurrency, args.duration, operation['method'])
                    per_operation_load.append({**operation, 'load': measured})
                    if measured.get('exit_code') != 0:
                        load_failed = True
                        break
                stop.set(); sampler.join()
                if load_failed:
                    fidelity_result = {'status': 'not-run', 'reason': 'load client failed; fixture run is invalid'}
                elif tool_name == 'WireMock':
                    fidelity_result = {'status': 'n/a', 'reason': 'no native OpenAPI import'}
                else:
                    fidelity = subprocess.run([sys.executable, str(BASE / 'fidelity.py'), str(fixture), root_url],
                                              text=True, capture_output=True)
                    if fidelity.returncode:
                        fidelity_result = {'status': 'failed', 'reason': fidelity.stderr.strip()}
                    else:
                        fidelity_result = json.loads(fidelity.stdout)
                stop_server(proc, args.port, container)
                fixture_failed = load_failed or fidelity_result.get('status') == 'failed'
                successful_operations = [item['operation'] for item in per_operation_load
                                         if not item['load'].get('failed') and item['load'].get('errors') == 0
                                         and any(200 <= int(code) < 300 and count > 0
                                                 for code, count in item['load'].get('status_counts', {}).items())]
                fixture_results.append({'fixture': fixture.stem, 'version': version,
                    'metrics': {'cold_start_seconds_median': sorted(starts)[len(starts)//2],
                                'cold_start_run_count': len(starts),
                                'warmup_runs_discarded': args.warmup * (args.runs + 1),
                                'load': {'client': client_identity(args.load_client), 'per_operation': per_operation_load,
                                         'failed': fixture_failed,
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
