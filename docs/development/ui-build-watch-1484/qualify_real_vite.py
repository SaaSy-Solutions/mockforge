#!/usr/bin/env python3
"""Run actual pinned Vite and the copied production asset build script in an owned fixture."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib


def run(command, cwd, log, environment=None):
    with log.open('w') as output:
        subprocess.run(command, cwd=cwd, env=environment, stdout=output,
                       stderr=subprocess.STDOUT, check=True, timeout=900)


def observe(fixture, evidence, label):
    command = [str(fixture / 'scripts/dev/cargo'), 'test', '--locked', '--lib', '--message-format=json']
    with (evidence / f'{label}.jsonl').open('w') as stdout, (evidence / f'{label}.stderr').open('w') as stderr:
        completed = subprocess.run(command, cwd=fixture, stdout=stdout, stderr=stderr, check=False, timeout=900)
    captured = [json.loads(line) for line in (evidence / f'{label}.jsonl').read_text().splitlines() if line.startswith('{')]
    package_ids = {m['package_id'] for m in captured if m.get('reason') == 'compiler-artifact'
                   and m.get('manifest_path') == str(fixture / 'Cargo.toml')}
    for message in captured:
        if message.get('reason') == 'build-script-executed' and message['package_id'] in package_ids:
            directory = Path(message['out_dir']).parent
            for filename in ['output', 'stderr']:
                source = directory / filename
                if source.is_file():
                    (evidence / f'{label}-producer-{filename}.log').write_bytes(source.read_bytes())
    if completed.returncode:
        raise subprocess.CalledProcessError(completed.returncode, completed.args)
    lines = (evidence / f'{label}.jsonl').read_text().splitlines()
    assert any('test result: ok. 1 passed; 0 failed' in line for line in lines)
    messages = [json.loads(line) for line in lines if line.startswith('{')]
    artifacts = [m for m in messages if m.get('reason') == 'compiler-artifact']
    root = [m for m in artifacts if m['target']['name'] == 'mockforge_ui_asset_probe']
    assert len(root) == 1
    scripts = [m for m in messages if m.get('reason') == 'build-script-executed' and m['package_id'] == root[0]['package_id']]
    assert len(scripts) == 1
    output_dir = Path(scripts[0]['out_dir'])
    assert output_dir.is_relative_to(fixture / 'target')
    generated = output_dir / 'ui-dist'
    assert (generated / 'index.html').is_file() and (generated / 'stats.html').is_file()
    assert not (fixture / 'ui/dist').exists(), 'producer wrote into prebuilt fallback'
    output = output_dir.parent / 'output'
    assert 'React UI build completed' in output.read_text()
    receipts = [json.loads(line) for line in (evidence / f'{label}.stderr').read_text().splitlines()
                if line.startswith('{') and '"policy_sha256"' in line]
    assert len(receipts) == 1 and receipts[0]['exit_status'] == 0
    return {'artifacts': len(artifacts), 'fresh': sum(m['fresh'] for m in artifacts),
            'root_fresh': root[0]['fresh'], 'output_mtime_ns': output.stat().st_mtime_ns,
            'admission': receipts[0], 'actual_vite_success': True,
            'owned_index_and_stats': True, 'prebuilt_fallback_untouched': True}


def prepare(repo, fixture, evidence):
    client = ['cargo', 'rust_build_policy.py', 'rust_policy_config.py', 'rust_policy_runtime.py', 'rust_policy_warmth.py']
    files = [('crates/mockforge-ui/' + name, name) for name in ['build.rs', 'build_support.rs', 'build_ui.sh']]
    files += [('scripts/dev/' + name, 'scripts/dev/' + name) for name in client]
    files += [(name, name) for name in ['rust-toolchain.toml', '.cargo/config.toml']]
    configs = ['package.json', 'pnpm-lock.yaml', 'pnpm-workspace.yaml', 'vite.config.ts', 'postcss.config.js', 'tailwind.config.js',
               'tsconfig.json', 'tsconfig.app.json', 'tsconfig.node.json', 'tsconfig.test.json']
    files += [('crates/mockforge-ui/ui/' + name, 'ui/' + name) for name in configs]
    public = ['manifest.json', 'sw.js', 'mockforge-icon.png', 'mockforge-icon-32.png',
              'mockforge-icon-48.png', 'mockforge-logo-40.png', 'mockforge-logo-80.png']
    files += [('crates/mockforge-ui/ui/public/' + name, 'ui/public/' + name) for name in public]
    identities = {}
    for source_name, destination_name in files:
        source, destination = repo / source_name, fixture / destination_name
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
        destination.chmod(source.stat().st_mode & 0o777)
        identities[source_name] = hashlib.sha256(source.read_bytes()).hexdigest()
    (fixture / 'ui/src').mkdir()
    (fixture / 'ui/index.html').write_text('<!doctype html><title>ui-owned-output-proof</title><script type="module" src="/src/main.js"></script>')
    (fixture / 'ui/src/main.js').write_text('document.body.dataset.proof = "ui-owned-output-proof-js";\n')
    run(['corepack', 'pnpm', 'install', '--frozen-lockfile'], fixture / 'ui', evidence / 'pnpm-install.log')
    lock = tomllib.loads((repo / 'Cargo.lock').read_text())
    versions = {name: next(p['version'] for p in lock['package'] if p['name'] == name)
                for name in ['serde', 'serde_json', 'vergen']}
    manifest = '[package]\nname="mockforge-ui-asset-probe"\nversion="0.0.0"\nedition="2021"\n[workspace]\n[lib]\npath="lib.rs"\n[build-dependencies]\n'
    manifest += 'serde={version="=' + versions['serde'] + '",features=["derive"]}\n'
    manifest += 'serde_json="=' + versions['serde_json'] + '"\n'
    manifest += 'vergen={version="=' + versions['vergen'] + '",features=["build","git","gitcl"]}\n'
    (fixture / 'Cargo.toml').write_text(manifest)
    shutil.copyfile(repo / 'Cargo.lock', fixture / 'Cargo.lock')
    (fixture / 'lib.rs').write_text('''pub mod content { include!(concat!(env!("OUT_DIR"), "/ui_content.rs")); }
pub mod paths { include!(concat!(env!("OUT_DIR"), "/asset_paths.rs")); }
pub mod icons { include!(concat!(env!("OUT_DIR"), "/icon_assets.rs")); }
#[test]
fn embeds_actual_vite_output() {
    assert!(content::get_admin_html().contains("ui-owned-output-proof"));
    assert!(content::get_admin_js().contains("ui-owned-output-proof-js"));
    assert!(paths::get_asset_map().contains_key("index.js"));
}
''')
    run([str(fixture / 'scripts/dev/cargo'), 'metadata', '--offline', '--format-version=1'], fixture, evidence / 'metadata.json')
    resolved = tomllib.loads((fixture / 'Cargo.lock').read_text())
    known = {(p['name'], p['version'], p.get('source')) for p in lock['package']}
    added = {(p['name'], p['version'], p.get('source')) for p in resolved['package']} - known
    assert added == {('mockforge-ui-asset-probe', '0.0.0', None)}, added
    (fixture / '.gitignore').write_text('/target/\n/ui/node_modules/\n/ui/dist/\n')
    run(['git', 'init', '-q'], fixture, evidence / 'git-init.log')
    run(['git', 'add', '.'], fixture, evidence / 'git-add.log')
    run(['git', '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
         'commit', '-qm', 'owned synthetic asset fixture'], fixture, evidence / 'git-commit.log')
    (evidence / 'identities.json').write_text(json.dumps({'source_sha256': identities,
        'rustc': subprocess.check_output(['rustc', '-vV'], cwd=fixture, text=True),
        'package_manager': json.loads((fixture / 'ui/package.json').read_text())['packageManager'],
        'unchanged_dependency_versions': True}, indent=2) + '\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    evidence = args.output.resolve()
    assert Path('/tmp') in evidence.parents
    evidence.mkdir(exist_ok=False)
    repo = args.repo.resolve()
    with tempfile.TemporaryDirectory(prefix='ui asset probe ', dir=repo / 'target') as folder:
        fixture = Path(folder)
        prepare(repo, fixture, evidence)
        first = observe(fixture, evidence, 'first')
        second = observe(fixture, evidence, 'second')
        assert second['fresh'] == second['artifacts'] and second['root_fresh']
        assert first['output_mtime_ns'] == second['output_mtime_ns']
        generated = next(Path(fixture / 'target/debug/build').glob('mockforge-ui-asset-probe-*/out/ui-dist'))
        owned_bytes = {name: (generated / name).read_bytes() for name in ['index.html', 'assets/index.js']}
        run(['bash', str(fixture / 'build_ui.sh')], fixture, evidence / 'standalone-build.log')
        assert all((fixture / 'ui/dist' / name).read_bytes() == data for name, data in owned_bytes.items())
        # Restore the absence state before the third capture's asserted absence check.
        # These are exclusively this fixture's generated output files.
        shutil.rmtree(fixture / 'ui/dist')
        third = observe(fixture, evidence, 'after-standalone')
        assert third['fresh'] == third['artifacts'] and third['root_fresh']
        assert third['output_mtime_ns'] == first['output_mtime_ns']
        (evidence / 'results.json').write_text(json.dumps({'first': first, 'second': second,
            'after_standalone': third, 'standalone_default_html_js_byte_identical': True}, indent=2) + '\n')
        print('Authentic pinned Vite + copied production build-script native embedding and unchanged repeat PASS')


if __name__ == '__main__':
    main()
