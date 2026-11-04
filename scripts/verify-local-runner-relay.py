#!/usr/bin/env python3
"""Verify real runner jobs and local relay forwarding in owned isolated containers.

Requires already cached images. The helper image needs Node; it runs only this
checkout's synthetic collector. No production registry, providers, billing, public
DNS or successful public-client tunnel is exercised.
"""
import argparse
import hashlib
import json
from pathlib import Path
import secrets
import signal
import subprocess
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]


def require(value, message):
    if not value:
        raise AssertionError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ('runner-image', 'relay-image', 'redis-image', 'helper-image', 'runner-binary-sha256', 'relay-binary-sha256'):
        parser.add_argument('--' + key, required=True)
    parser.add_argument('--receipt', type=Path, required=True)
    args = parser.parse_args()
    require(not args.receipt.exists(), 'Preserve previous receipts')
    docker = ['/usr/bin/docker', '--host', 'unix:///var/run/docker.sock']

    def run(*arguments, timeout=30):
        result = subprocess.run(docker + list(arguments), capture_output=True, timeout=timeout)
        require(result.returncode == 0, 'Docker operation failed: ' + result.stderr.decode()[-1000:])
        return result.stdout

    def inspect(identity):
        result = json.loads(run('inspect', identity))
        require(len(result) == 1, 'Ambiguous identity')
        return result[0]

    images = {}
    for key in ('runner', 'relay', 'redis', 'helper'):
        image = json.loads(run('image', 'inspect', getattr(args, key + '_image')))[0]
        require(image['Os'] == 'linux' and image['Architecture'] == 'amd64', 'Expected Linux amd64 image')
        images[key] = image['Id']
    owner = 'mockforge-local-proof-' + secrets.token_hex(8)
    network = None
    planned = {}
    containers = {}
    receipt = {'image_ids': images, 'local_only': True, 'production_mutations': False,
               'public_tunnel_qualified': False, 'billing_or_provider_qualified': False,
               'cleanup_verified': False}
    nonce, token = secrets.token_hex(16), secrets.token_urlsafe(32)
    positive, negative, reservation = [str(uuid.uuid4()) for _ in range(3)]
    helper_source = ROOT / 'scripts/fixtures/runner-relay-server.cjs'
    receipt['collector_sha256'] = hashlib.sha256(helper_source.read_bytes()).hexdigest()

    def create(role, image, memory, command, env=None, extra=None, share_helper_network=False):
        name = owner + '-' + role
        planned[name] = image
        mode = 'container:' + containers['fixture'] if share_helper_network else network
        argv = ['run', '-d', '--name', name, '--label', 'saasy.fixture.owner=' + owner,
                '--network', mode, '--read-only', '--cap-drop', 'ALL',
                '--security-opt', 'no-new-privileges', '--memory', str(memory), '--memory-swap', str(memory),
                '--cpus', '1', '--pids-limit', '128', '--tmpfs', '/tmp:rw,nosuid,nodev,size=32m,mode=1777']
        if not share_helper_network:
            argv += ['--network-alias', role]
        for key, value in (env or {}).items():
            argv += ['--env', key + '=' + value]
        argv += (extra or []) + [image] + command
        identity = run(*argv).decode().strip()
        item = inspect(identity)
        require(item['Config'].get('Labels', {}).get('saasy.fixture.owner') == owner and item['Image'] == image, 'Unexpected created generation')
        host = item['HostConfig']
        require(host['Memory'] == host['MemorySwap'] == memory and host['NanoCpus'] == 1_000_000_000
                and host['PidsLimit'] == 128 and host['ReadonlyRootfs'] and host['CapDrop'] == ['ALL']
                and 'no-new-privileges' in host['SecurityOpt'] and not host['PortBindings'], 'Fixture controls differ')
        if share_helper_network:
            require(host['NetworkMode'] == mode and not item['NetworkSettings']['Networks'], 'Unexpected shared network namespace')
            require({n['NetworkID'] for n in inspect(containers['fixture'])['NetworkSettings']['Networks'].values()} == {network}, 'Helper left its owned network')
        else:
            require({n['NetworkID'] for n in item['NetworkSettings']['Networks'].values()} == {network}, 'Unexpected fixture network')
        containers[role] = identity
        return identity

    def request(url, method='GET', headers=None, body=None):
        options = {'method': method, 'headers': headers or {}}
        if body is not None:
            options['headers'] = {'content-type': 'application/json', **options['headers']}
            options['body'] = json.dumps(body)
        client = """const http = require('node:http');
const options = JSON.parse(process.argv[2]);
const req = http.request(process.argv[1], options, res => {
  let body = '';
  res.on('data', data => { body += data; });
  res.on('end', () => { console.log(JSON.stringify({status:res.statusCode,headers:res.headers,body})); process.exit(0); });
});
req.setTimeout(10000, () => req.destroy(new Error('fixture request timed out')));
req.on('error', error => { console.error(error.message); process.exit(1); });
if (options.body) req.write(options.body);
req.end();"""
        return json.loads(run('exec', containers['fixture'], 'node', '-e', client, url, json.dumps(options)))

    def wait_http(url):
        for _ in range(60):
            try:
                result = request(url)
                if result['status'] == 200:
                    return result
            except AssertionError:
                pass
            time.sleep(0.25)
        raise AssertionError('Owned HTTP service not ready: ' + url)

    def state():
        return json.loads(request('http://fixture:8080/state')['body'])

    def job(run_id, fail=False):
        steps = [{'id': 'first', 'request': {'method': 'GET', 'path': '/target/runner-first',
                  'headers': {'x-fixture-proof': nonce}, 'query_params': {'case': run_id}},
                  'validation': {'status_code': 418 if fail else 200,
                  'body_assertions': [{'path': 'nonce', 'operator': 'equals', 'value': nonce}]}}]
        if not fail:
            steps.append({'id': 'second', 'request': {'method': 'POST', 'path': '/target/runner-second',
                          'headers': {'x-fixture-proof': nonce}, 'body': {'message': nonce}},
                          'validation': {'status_code': 201, 'body_assertions': [
                              {'path': 'body.message', 'operator': 'equals', 'value': nonce}]}})
        return {'run_id': run_id, 'org_id': str(uuid.uuid4()), 'source_id': str(uuid.uuid4()),
                'kind': 'integration', 'payload': {'setup': {'base_url': 'http://127.0.0.1:8080'}, 'steps': steps}}

    def interrupted(*_):
        raise InterruptedError('Local verification interrupted; cleaning owned fixtures')
    for sig in (signal.SIGINT, signal.SIGTERM):
        signal.signal(sig, interrupted)
    try:
        network = run('network', 'create', '--internal', '--label', 'saasy.fixture.owner=' + owner, owner).decode().strip()
        net = json.loads(run('network', 'inspect', network))[0]
        require(net['Internal'] and net['Labels'].get('saasy.fixture.owner') == owner, 'Not an owned internal network')
        create('fixture', images['helper'], 256 * 1024**2, ['/fixtures/server.cjs'],
               {'FIXTURE_NONCE': nonce, 'FIXTURE_TOKEN': token,
                'FIXTURE_RUN_IDS': json.dumps([positive, negative]), 'FIXTURE_RESERVATION_ID': reservation},
               ['--entrypoint', 'node', '--mount', f'type=bind,src={helper_source},dst=/fixtures/server.cjs,readonly'])
        wait_http('http://fixture:8080/health')
        require(run('exec', containers['fixture'], 'sha256sum', '/fixtures/server.cjs').decode().split()[0] == receipt['collector_sha256'], 'Collector mount differs')
        create('redis', images['redis'], 64 * 1024**2,
               ['redis-server', '--save', '', '--appendonly', 'no', '--maxmemory', '32mb'],
               extra=['--user', 'redis', '--tmpfs', '/data:rw,nosuid,nodev,size=32m,mode=1777'])
        for _ in range(30):
            try:
                if run('exec', containers['redis'], 'redis-cli', 'PING').strip() == b'PONG':
                    break
            except AssertionError:
                pass
            time.sleep(0.25)
        else:
            raise AssertionError('Owned Redis not ready')
        create('runner', images['runner'], 512 * 1024**2, [], {
            'MOCKFORGE_RUNNER_REDIS_URL': 'redis://redis:6379',
            'MOCKFORGE_RUNNER_REGISTRY_INTERNAL_BASE_URL': 'http://127.0.0.1:8080',
            'MOCKFORGE_RUNNER_REGISTRY_INTERNAL_TOKEN': token,
            'MOCKFORGE_RUNNER_MAX_CONCURRENT_JOBS': '1', 'MOCKFORGE_RUNNER_POLL_TIMEOUT_SECS': '1',
            'MOCKFORGE_SSRF_ALLOW_LOOPBACK': '1'}, share_helper_network=True)
        receipt['runner_binary_sha256'] = run('exec', containers['runner'], 'sha256sum', '/proc/1/exe').decode().split()[0]
        require(receipt['runner_binary_sha256'] == args.runner_binary_sha256, 'Runner binary differs')
        wrong = request(f'http://fixture:8080/api/v1/internal/test-runs/{positive}/start', 'POST', body={})
        require(wrong['status'] == 403, 'Collector accepted a callback without its synthetic token')
        for run_id, fail in [(positive, False), (negative, True)]:
            run('exec', containers['redis'], 'redis-cli', 'LPUSH', 'test_runs:queued', json.dumps(job(run_id, fail)))
        for _ in range(120):
            observed = state()
            if all(observed['jobs'][run_id]['finish'] for run_id in (positive, negative)):
                break
            time.sleep(0.25)
        else:
            raise AssertionError('Runner did not finish both synthetic jobs')
        receipt['runner_observed_state'] = observed
        for run_id, passed, failed in [(positive, 2, 0), (negative, 0, 1)]:
            result = observed['jobs'][run_id]
            # TestExecutor and integration executor currently both mark start;
            # the real registry explicitly treats the second call as 200/noop.
            require(1 <= len(result['start']) <= 2 and len(result['finish']) == 1, 'Missing start or duplicated finish callback')
            outcome = result['finish'][0]
            require(outcome['status'] == ('passed' if passed else 'failed') and outcome['runner_seconds'] >= 1, 'Wrong terminal job outcome')
            summary = outcome['summary']
            require(summary['executor_phase'] == 'real' and summary['steps_passed'] == passed
                    and summary['steps_failed'] == failed and summary['steps_errored'] == 0, 'Runner did not execute/assert the expected steps')
            kinds = [event['event_type'] for event in result['events']]
            require(kinds.count('step_passed') == passed and kinds.count('step_failed') == failed, 'Step event evidence differs')
        hits = [hit for hit in observed['targets'] if hit['path'].startswith('/target/runner-')]
        require(len(hits) == 3 and all(hit['proof'] == nonce for hit in hits), 'Backend did not observe real worker requests')
        require(any(hit['method'] == 'POST' and hit['body'] == {'message': nonce} for hit in hits), 'Worker did not transmit the expected POST body')
        receipt['runner'] = {'positive': observed['jobs'][positive], 'negative': observed['jobs'][negative],
                             'real_backend_requests': len(hits), 'ssrf_opt_in_only_owned_local_fixture': True,
                             'collector_refused_missing_token': True, 'owned_helper_network_namespace_shared': True}
        receipt.pop('runner_observed_state', None)
        create('relay', images['relay'], 192 * 1024**2, [], {'TUNNEL_RATE_LIMIT_ENABLED': 'false'})
        wait_http('http://relay:8080/health')
        receipt['relay_binary_sha256'] = run('exec', containers['relay'], 'sha256sum', '/proc/1/exe').decode().split()[0]
        require(receipt['relay_binary_sha256'] == args.relay_binary_sha256, 'Relay binary differs')
        registration = request('http://relay:8080/api/tunnels', 'POST', body={'subdomain': 'local-fixture', 'local_url': 'http://fixture:8080'})
        require(registration['status'] == 200, 'Local tunnel registration failed')
        for method in ('GET', 'POST'):
            result = request('http://relay:8080/target/relay?case=' + nonce, method,
                             {'Host': 'local-fixture.fixture.invalid', 'x-fixture-proof': nonce},
                             {'message': nonce} if method == 'POST' else None)
            body = json.loads(result['body'])
            require(result['status'] == (201 if method == 'POST' else 200)
                    and result['headers'].get('x-fixture-proof') == nonce and body['nonce'] == nonce
                    and body['method'] == method and body['path'] == '/target/relay'
                    and body['query'] == {'case': nonce} and body['proof'] == nonce
                    and body['body'] == ({'message': nonce} if method == 'POST' else None), 'Local relay roundtrip lost status/headers/query/body')
        receipt['local_relay'] = {'get_roundtrip': True, 'post_roundtrip': True, 'local_registration_only': True}
        create('cloudrelay', images['relay'], 192 * 1024**2, [], {
            'REGISTRY_URL': 'http://fixture:8080', 'MOCKFORGE_INTERNAL_API_TOKEN': token,
            'TUNNEL_RATE_LIMIT_ENABLED': 'false'})
        wait_http('http://cloudrelay:8080/health')
        result = request('http://cloudrelay:8080/target/reservation', headers={'Host': 'reserved-fixture.fixture.invalid'})
        observed = state()
        require(400 <= result['status'] <= 599 and observed['lookups'] == ['reserved-fixture'], 'Reservation without a connected destination unexpectedly forwarded')
        require(not any(hit['path'] == '/target/reservation' for hit in observed['targets']), 'Reservation-only request unexpectedly reached a destination')
        receipt['cloud_reservation_limit'] = {'authorized_lookup_seen': True, 'destination_missing_http_status': result['status'], 'backend_not_reached': True}
        receipt['bounded_owned_container_count'] = len(containers)
        receipt['passed_with_recorded_cloud_limit'] = True
    finally:
        for sig in (signal.SIGINT, signal.SIGTERM):
            signal.signal(sig, signal.SIG_IGN)
        for name, expected_image in reversed(list(planned.items())):
            result = subprocess.run(docker + ['inspect', name], capture_output=True, timeout=30)
            if result.returncode:
                continue
            item = json.loads(result.stdout)[0]
            require(item['Config'].get('Labels', {}).get('saasy.fixture.owner') == owner and item['Image'] == expected_image, 'Unknown fixture generation; cleanup refused')
            run('rm', '--force', item['Id'])
        # Creation may succeed before its CLI times out or is interrupted.
        # Reconcile only our exact preselected name even without a returned ID.
        network_matches = run('network', 'ls', '--filter', 'name=^' + owner + '$', '--format', '{{.ID}}').decode().split()
        require(len(network_matches) <= 1, 'Ambiguous owned network; cleanup refused')
        if network_matches:
            net = json.loads(run('network', 'inspect', network_matches[0]))[0]
            require(net['Name'] == owner and (network is None or net['Id'] == network)
                    and net['Internal'] and net['Labels'].get('saasy.fixture.owner') == owner
                    and not net.get('Containers'), 'Network still has unknown users or generation; cleanup refused')
            run('network', 'rm', net['Id'])
        require(not run('network', 'ls', '--filter', 'name=^' + owner + '$', '--format', '{{.ID}}').strip(),
                'Owned network remains after cleanup')
        remaining = run('ps', '-a', '--filter', 'label=saasy.fixture.owner=' + owner, '--format', '{{.ID}}')
        receipt['cleanup_verified'] = not bool(remaining.strip())
        args.receipt.write_text(json.dumps(receipt, indent=2) + '\n')
        print(json.dumps(receipt, indent=2))


if __name__ == '__main__':
    main()
