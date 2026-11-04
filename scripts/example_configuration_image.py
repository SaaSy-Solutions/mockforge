#!/usr/bin/env python3
"""Qualify a configuration-only image; never compile Rust or pull a base image."""
import argparse
import gzip
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[1]
RUNTIME_SOURCE = '3871677f241ee8e9726ad7646fa356c0a2df04ca'
BASE_MANIFEST = 'sha256:d56a4da07d89f909d9d50ba1f974291f107955af253527926f7bcafb37cbba4d'
BASE_CONFIG = 'sha256:10505b12bdd913abedf557e656dbd0ff961c57f777c4fed809d07f04e235b96f'
BASE_REF = 'ghcr.io/saasy-solutions/mockforge@' + BASE_MANIFEST
BINARY = '2e1d3abb38ac81d6597d1f8fb303984852db44a6c3bd779540d4f4762f87cd17'
SPECS = tuple('examples/scenarios/' + s + '/openapi.json' for s in ('ecommerce-store', 'weather-geo', 'chat-api'))
# Closed input set derived from the unchanged root Dockerfile. Compare whole
# copied trees, including files ignored by Docker, to fail conservatively.
INPUTS = ('Dockerfile', '.dockerignore', 'Cargo.toml', 'Cargo.lock', 'vendor/aws-sigv4',
          'crates', 'examples', 'proto', 'config.example.yaml', 'docker/entrypoint.sh')
DOCKER = ['/usr/bin/docker', '--host', 'unix:///var/run/docker.sock']


def require(value, message):
    if not value:
        raise ValueError(message)


def command(argv, **kwargs):
    return subprocess.check_output(argv, cwd=ROOT, timeout=kwargs.pop('timeout', 30), **kwargs)


def source_guard(source):
    require(re.fullmatch('[0-9a-f]{40}', source), 'Exact config source required')
    require(command(['git', 'rev-parse', 'HEAD']).decode().strip() == source, 'Config source is not actual HEAD')
    require(not command(['git', 'status', '--porcelain', '--untracked-files=all']).strip(), 'Checkout must be completely clean')
    changed = command(['git', 'diff', '--name-only', '-z', RUNTIME_SOURCE, source, '--', *INPUTS]).decode().split('\0')
    require(set(filter(None, changed)) <= set(SPECS), 'Native/runtime build input changed; full rebuild required')
    for path in SPECS:
        require((ROOT / path).is_file() and not (ROOT / path).is_symlink(), 'Regular committed spec required')
        json.loads((ROOT / path).read_text())
    return {'runtime_source': RUNTIME_SOURCE, 'config_source': source, 'runtime_inputs': INPUTS,
            'allowed_configuration_files': SPECS, 'changed_inputs': sorted(filter(None, changed))}



def refresh_dockerfile_guard(text):
    require(not re.search(r'^#\s*(syntax|escape)\s*=', text, re.MULTILINE | re.IGNORECASE), 'No external Dockerfile frontend/directives')
    instructions = []
    pending = ''
    for line in text.splitlines():
        if not line.strip() or line.lstrip().startswith('#'):
            continue
        pending += line.strip().removesuffix('\\').strip() + ' '
        if not line.rstrip().endswith('\\'):
            instructions.append(pending.strip())
            pending = ''
    require(not pending and len(instructions) == 5
            and instructions[:3] == ['ARG RUNTIME_IMAGE', 'FROM ${RUNTIME_IMAGE}', 'ARG CONFIG_SOURCE']
            and instructions[3].startswith('LABEL ')
            and instructions[4] == 'COPY --chown=mockforge:mockforge examples/ /app/examples/',
            'Refresh Dockerfile may only label and copy the bounded context; no RUN or additional stage')


def digest_stream(stream):
    h = hashlib.sha256()
    for part in iter(lambda: stream.read(1024 * 1024), b''):
        h.update(part)
    return 'sha256:' + h.hexdigest()



def base_identity_guard(base, server):
    require(int(server['Version'].split('.')[0]) >= 28
            and any(c.get('Name') == 'Engine' for c in server.get('Components', [])),
            'Genuine local Docker Engine 28 or newer required')
    require(base['Id'] in (BASE_CONFIG, BASE_MANIFEST) and BASE_REF in base.get('RepoDigests', [])
            and base['Os'] == 'linux' and base['Architecture'] == 'amd64',
            'Qualified immutable Linux amd64 base is not cached')


def image_identity_guard(image, graph):
    require(image['Id'] in (graph['config_digest'], graph['manifest_digest'])
            and image['RootFS']['Layers'] == graph['rootfs_diff_ids']
            and image['Os'] == 'linux' and image['Architecture'] == 'amd64',
            'Actual loaded image differs from verified archive')


def verify_archive(path, base, source):
    """Check every OCI blob and diff ID, then allow only three overlay files."""
    with tarfile.open(path) as archive:
        entries = archive.getmembers()
        require(len(entries) < 100000 and len({m.name for m in entries}) == len(entries), 'Duplicate/excessive archive members')
        for m in entries:
            require(not m.name.startswith('/') and '..' not in Path(m.name).parts
                    and (m.isfile() or m.isdir()), 'Unsafe outer archive member')

        def read(name, limit=1024 * 1024):
            m = archive.getmember(name)
            require(m.isfile() and m.size <= limit, 'Invalid OCI metadata')
            return archive.extractfile(m).read()

        def blob(descriptor, metadata=False):
            digest = descriptor['digest']
            require(re.fullmatch('sha256:[0-9a-f]{64}', digest), 'Invalid OCI digest')
            name = 'blobs/sha256/' + digest[7:]
            m = archive.getmember(name)
            require(m.isfile() and m.size == descriptor['size'], 'OCI blob size differs')
            require(digest_stream(archive.extractfile(m)) == digest, 'OCI blob hash differs')
            return json.loads(read(name)) if metadata else name

        index = json.loads(read('index.json'))
        require(index['schemaVersion'] == 2 and len(index['manifests']) == 1, 'One OCI image required')
        manifest = blob(index['manifests'][0], True)
        config = blob(manifest['config'], True)
        layers = manifest['layers']
        compat = json.loads(read('manifest.json'))
        require(len(compat) == 1 and compat[0]['Config'] == 'blobs/sha256/' + manifest['config']['digest'][7:]
                and compat[0]['Layers'] == ['blobs/sha256/' + layer['digest'][7:] for layer in layers],
                'Docker compatibility manifest differs from OCI graph')
        # This driver saves by immutable image ID, never by a mutable tag.
        require(not compat[0].get('RepoTags')
                and not index['manifests'][0].get('annotations', {}).get('org.opencontainers.image.ref.name'),
                'Unexpected tagged archive; image-ID export required')
        expected = base['RootFS']['Layers']
        require(len(expected) == 12 and config['rootfs']['diff_ids'][:-1] == expected
                and len(layers) == 13, 'Exactly qualified base layers plus one configuration layer required')
        for layer, diff_id in zip(layers, config['rootfs']['diff_ids'], strict=True):
            name = blob(layer)
            stream = archive.extractfile(name)
            if layer['mediaType'].endswith('+gzip') or layer['mediaType'].endswith('.gzip'):
                stream = gzip.GzipFile(fileobj=stream)
            else:
                require(layer['mediaType'].endswith('.tar'), 'Unsupported OCI layer encoding')
            require(digest_stream(stream) == diff_id, 'Uncompressed layer differs')
        # Runtime configuration must be identical except explicit provenance labels.
        runtime = dict(config['config'])
        labels = runtime.pop('Labels', {})
        original = dict(base['Config'])
        original.pop('Labels', None)
        # Docker classic archive serialization may omit empty legacy fields.
        # Classic Docker adds parent Image bookkeeping and explicit false
        # attach/TTY defaults that the daemon's image Config view omits.
        legacy_defaults = {'AttachStdin', 'AttachStdout', 'AttachStderr', 'OpenStdin', 'StdinOnce', 'Tty'}
        normalized = lambda d: {k: v for k, v in d.items()
                                if k != 'Image' and v not in (None, {}, [], '')
                                and not (k in legacy_defaults and v is False)}
        require(normalized(runtime) == normalized(original), 'Runtime settings changed')
        required_labels = {'org.opencontainers.image.revision': RUNTIME_SOURCE,
                           'io.mockforge.image.kind': 'example-configuration-refresh',
                           'io.mockforge.runtime.source': RUNTIME_SOURCE,
                           'io.mockforge.config.source': source,
                           'io.mockforge.runtime.base.digest': BASE_MANIFEST,
                           'io.mockforge.runtime.binary.sha256': BINARY}
        require(labels == {**base['Config'].get('Labels', {}), **required_labels}, 'Provenance labels differ')
        overlay_name = 'blobs/sha256/' + layers[-1]['digest'][7:]
        files = {}
        allowed = {'app/' + p for p in SPECS}
        directories = {str(parent) for p in allowed for parent in Path(p).parents if str(parent) != '.'}
        with tarfile.open(fileobj=archive.extractfile(overlay_name), mode='r|*') as overlay:
            for member in overlay:
                name = member.name.removeprefix('./').rstrip('/')
                require(name not in files and not name.startswith('/') and '..' not in Path(name).parts, 'Duplicate/unsafe overlay path')
                if member.isdir():
                    require(name in directories, 'Unexpected overlay directory')
                else:
                    require(member.isfile() and name in allowed and member.size < 1024 * 1024, 'Unexpected overlay file/type')
                    content = overlay.extractfile(member).read()
                    require(content == (ROOT / name.removeprefix('app/')).read_bytes(), 'Overlay spec bytes differ')
                    files[name] = hashlib.sha256(content).hexdigest()
        require(set(files) == allowed, 'Missing configuration file')
        return {'manifest_digest': index['manifests'][0]['digest'], 'config_digest': manifest['config']['digest'],
                'rootfs_diff_ids': config['rootfs']['diff_ids'], 'verified_blob_count': 15,
                'overlay_files': files, 'runtime_settings_unchanged': True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--check-only', action='store_true')
    args = parser.parse_args()
    receipt = source_guard(args.source)
    require(not args.output.exists(), 'Preserve prior output')
    args.output.mkdir(mode=0o700, parents=True)
    receipt.update(status='source_checked', published=False)
    try:
        if args.check_only:
            return
        # No daemon context override, remote build daemon, pull, or Rust build.
        env = dict(os.environ)
        env.pop('DOCKER_CONTEXT', None)
        env.pop('DOCKER_HOST', None)
        env['DOCKER_BUILDKIT'] = '0'
        base = json.loads(command(DOCKER + ['image', 'inspect', BASE_REF], env=env))[0]
        server = json.loads(command(DOCKER + ['version', '--format', '{{json .Server}}'], env=env))
        base_identity_guard(base, server)
        size = base['Size']
        docker_root = command(DOCKER + ['info', '--format', '{{.DockerRootDir}}'], env=env).decode().strip()
        require(Path(docker_root).is_dir(), 'Local engine filesystem required')
        free = shutil.disk_usage(docker_root).free
        budget = max(2 * 1024**3, 8 * size)
        require(free >= budget and shutil.disk_usage(args.output).free >= budget, 'Insufficient bounded archive/build disk budget')
        receipt.update(base_image_size=size, disk_budget_bytes=budget, daemon_available_before=free,
                       base_manifest=BASE_MANIFEST, base_config=BASE_CONFIG, binary_sha256=BINARY)
        tag = 'mockforge-examples-local:' + args.source
        require(subprocess.run(DOCKER + ['image', 'inspect', tag], capture_output=True, env=env).returncode != 0, 'Preserve existing source tag')
        with tempfile.TemporaryDirectory(prefix='mockforge-examples-context-') as directory:
            context = Path(directory)
            refresh_dockerfile_guard((ROOT / 'docker/examples-refresh.Dockerfile').read_text())
            shutil.copyfile(ROOT / 'docker/examples-refresh.Dockerfile', context / 'Dockerfile')
            for path in SPECS:
                dest = context / path
                dest.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(ROOT / path, dest)
            with (args.output / 'build.log').open('wb') as log:
                subprocess.run(DOCKER + ['build', '--pull=false', '--network=none', '--rm',
                               '--build-arg', 'RUNTIME_IMAGE=' + BASE_REF, '--build-arg', 'CONFIG_SOURCE=' + args.source,
                               '--tag', tag, str(context)], env=env, stdout=log, stderr=subprocess.STDOUT, timeout=120, check=True)
        receipt['local_tag'] = tag
        image = json.loads(command(DOCKER + ['image', 'inspect', tag], env=env))[0]
        archive = args.output / 'image.tar'
        command(DOCKER + ['image', 'save', '--output', str(archive), image['Id']], env=env, timeout=120)
        receipt['oci'] = verify_archive(archive, base, args.source)
        image_identity_guard(image, receipt['oci'])
        receipt['archive_sha256'] = digest_stream(archive.open('rb'))[7:]
        spec = importlib.util.spec_from_file_location('owned_lifecycle', ROOT / 'scripts/verify-root-image-archive.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        result = module.run_owned_verifier([sys.executable, str(ROOT / 'scripts/verify-bundled-demo.py'), '--image', image['Id'],
                                           '--receipt', str(args.output / 'http.json')], args.output, receipt)
        require(result == 0, 'Bundled HTTP verification failed')
        http = json.loads((args.output / 'http.json').read_text())
        require(not http['mounted_checkout_specs'] and http['cleanup_verified'] and len(http['cases']) == 6
                and all(c['passed'] for c in http['cases']) and http['binary_sha256'] == BINARY, 'No-mount original-binary HTTP proof required')
        source_guard(args.source)
        receipt.update(status='qualified', image_id=image['Id'], six_bundled_http_cases_passed=True)
    except BaseException as error:
        receipt.update(status='failed', failure_type=type(error).__name__)
        raise
    finally:
        (args.output / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
        print(json.dumps(receipt, indent=2))


if __name__ == '__main__':
    main()
