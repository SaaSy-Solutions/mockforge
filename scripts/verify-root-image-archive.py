#!/usr/bin/env python3
"""Import a supplied root-image archive after publisher and complete OCI checks.

This never pulls images or contacts a deployment host. The caller supplies the
archive, measured source metadata, the pinned shared OCI verifier, and the demo
HTTP verifier. All Docker operations use the local Engine socket.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys

VERIFIER_SHA = '36d648d37f87535a9bdbe3c2a7d2636b92952bff7a735ee4fc8dae89335dec54'


def require(value, message):
    if not value:
        raise RuntimeError(message)


def file_sha(path):
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for part in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(part)
    return digest.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', type=Path, required=True)
    parser.add_argument('--metadata', type=Path, required=True)
    parser.add_argument('--oci-verifier', type=Path, required=True)
    parser.add_argument('--demo-verifier', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    require(not sys.flags.optimize, 'Optimized Python is not allowed')
    metadata = json.loads(args.metadata.read_text())
    source = metadata['source']
    require(re.fullmatch('[0-9a-f]{40}', source), 'Exact source commit required')
    tag = 'ghcr.io/saasy-solutions/mockforge:' + source
    manifest = metadata['manifest_digest']
    require(metadata['ref'] == tag + '@' + manifest, 'Image source/ref differs')
    require(type(metadata['publisher_run_id']) is int and metadata['publisher_run_id'] > 0, 'Publisher run required')
    run = subprocess.run(['gh', 'api', 'repos/SaaSy-Solutions/mockforge/actions/runs/' + str(metadata['publisher_run_id'])],
                         capture_output=True, text=True, timeout=30)
    require(run.returncode == 0, 'Publisher metadata unavailable')
    publisher = json.loads(run.stdout)
    require(publisher['head_sha'] == source and publisher['head_branch'] == 'main'
            and publisher['path'] == '.github/workflows/docker-build.yml'
            and publisher['event'] in ('push', 'workflow_dispatch')
            and publisher['status'] == 'completed' and publisher['conclusion'] == 'success',
            'Successful exact-main publisher required')
    verifier = args.oci_verifier.read_bytes()
    require(hashlib.sha256(verifier).hexdigest() == VERIFIER_SHA, 'Pinned shared OCI verifier changed')
    require(file_sha(args.archive) == metadata['archive_sha256'], 'Archive transfer hash differs')
    os.environ.pop('DOCKER_CONTEXT', None)
    os.environ.pop('DOCKER_HOST', None)
    spec = importlib.util.spec_from_file_location('reviewed_oci_verifier', args.oci_verifier)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    # Execute the exact bytes hashed above, without writing a foreign pycache.
    exec(compile(verifier, str(args.oci_verifier), 'exec'), module.__dict__)
    engine = module.require_local_engine()
    graph = module.archive_graph(args.archive, manifest, metadata['config_digest'], tag, metadata['rootfs_diff_ids'])
    existing = subprocess.run(['/usr/bin/docker', '--host', 'unix:///var/run/docker.sock', 'image', 'inspect', tag], capture_output=True, timeout=30)
    if existing.returncode == 0:
        require(json.loads(existing.stdout)[0]['Id'] in (manifest, metadata['config_digest']),
                'Existing local source tag differs; refuse to overwrite it')
    args.output.mkdir(mode=0o700, exist_ok=False)
    receipt = {'source': source, 'ref': metadata['ref'], 'publisher_run_id': metadata['publisher_run_id'],
               'publisher_url': publisher['html_url'], 'archive_sha256': metadata['archive_sha256'],
               'oci_verifier_sha256': VERIFIER_SHA, 'graph': graph, 'local_engine_id': engine,
               'production_mutations': False, 'remote_pulls': False, 'status': 'verified_archive'}
    path = args.output / 'receipt.json'
    try:
        load = subprocess.run(['/usr/bin/docker', '--host', 'unix:///var/run/docker.sock', 'load', '--input', str(args.archive)],
                              capture_output=True, timeout=300)
        (args.output / 'load.stdout').write_bytes(load.stdout)
        (args.output / 'load.stderr').write_bytes(load.stderr)
        require(load.returncode == 0, 'Verified archive load failed')
        identity = module.resolve_local_image(manifest, metadata['config_digest'], metadata['rootfs_diff_ids'])
        receipt['local_image_id'] = identity
        receipt['binary_sha256'] = module.measure_binary(identity, '/usr/local/bin/mockforge', 'mockforge', args.output / 'binary-probes')
        result = subprocess.run([sys.executable, str(args.demo_verifier), '--image', identity,
                                 '--receipt', str(args.output / 'bundled-http.json')], capture_output=True, timeout=180)
        (args.output / 'http.stdout').write_bytes(result.stdout)
        (args.output / 'http.stderr').write_bytes(result.stderr)
        require(result.returncode == 0, 'Actual bundled HTTP verification failed')
        http = json.loads((args.output / 'bundled-http.json').read_text())
        require(not http['mounted_checkout_specs'] and http['cleanup_verified']
                and len(http['cases']) == 6 and all(c['passed'] for c in http['cases'])
                and http['binary_sha256'] == receipt['binary_sha256'], 'Published bundle/runtime proof differs')
        receipt['demo_verifier_sha256'] = file_sha(args.demo_verifier)
        receipt['status'] = 'passed'
    except BaseException as error:
        receipt['failure_type'] = type(error).__name__
        raise
    finally:
        path.write_text(json.dumps(receipt, indent=2) + '\n')
        print(json.dumps(receipt, indent=2))


if __name__ == '__main__':
    main()
