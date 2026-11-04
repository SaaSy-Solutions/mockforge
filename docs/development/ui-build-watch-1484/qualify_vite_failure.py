#!/usr/bin/env python3
"""Check actual Vite failure and fallback freshness with the production build script."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import runpy
import subprocess
import tempfile


def observe(fixture, evidence, label, expected):
    environment = dict(os.environ, UI_ASSET_PROBE_EXPECTED=expected,
                       CARGO_LOG='cargo::core::compiler::fingerprint=info')
    with (evidence / f'{label}.jsonl').open('w') as stdout, (evidence / f'{label}.stderr').open('w') as stderr:
        completed = subprocess.run([str(fixture / 'scripts/dev/cargo'), 'test', '--locked', '--lib', '--message-format=json'],
                       cwd=fixture, env=environment, stdout=stdout, stderr=stderr, check=False, timeout=1200)
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
    root = next(m for m in artifacts if m['target']['name'] == 'mockforge_ui_asset_probe')
    script = next(m for m in messages if m.get('reason') == 'build-script-executed' and m['package_id'] == root['package_id'])
    out = Path(script['out_dir'])
    text = (out.parent / 'output').read_text()
    receipts = [json.loads(line) for line in (evidence / f'{label}.stderr').read_text().splitlines()
                if line.startswith('{') and '"policy_sha256"' in line]
    assert len(receipts) == 1 and receipts[0]['exit_status'] == 0
    return {'stage': label, 'expected': expected, 'artifact_count': len(artifacts),
            'fresh_count': sum(m['fresh'] for m in artifacts),
            'owned_index_still_exists': (out / 'ui-dist/index.html').is_file(),
            'script_output_mtime_ns': (out.parent / 'output').stat().st_mtime_ns,
            'frontend_success': 'React UI build completed' in text,
            'frontend_failure_warning': 'UI build script failed' in text, 'admission': receipts[0]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--positive-probe', type=Path, required=True)
    args = parser.parse_args()
    repo, evidence = args.repo.resolve(), args.output.resolve()
    assert Path('/tmp') in evidence.parents
    evidence.mkdir(exist_ok=False)
    helper = runpy.run_path(str(args.positive_probe))
    results = []
    try:
        with tempfile.TemporaryDirectory(prefix='ui failure probe ', dir=repo / 'target') as folder:
            fixture = Path(folder)
            helper['prepare'](repo, fixture, evidence)
            (fixture / 'lib.rs').write_text('''pub mod content { include!(concat!(env!("OUT_DIR"), "/ui_content.rs")); }
#[test]
fn embeds_expected_assets() {
    let expected = std::env::var("UI_ASSET_PROBE_EXPECTED").expect("expected asset mode");
    let html = content::get_admin_html();
    match expected.as_str() {
        "owned" => assert!(html.contains("ui-owned-output-proof")),
        "placeholder" => {
            assert!(html.contains("Admin UI not bundled"));
            assert!(!html.contains("ui-owned-output-proof"));
        },
        "prebuilt" => {
            assert!(html.contains("prebuilt fallback marker"));
            assert!(!html.contains("ui-owned-output-proof"));
        },
        _ => panic!("unknown expected mode"),
    }
}
''')
            helper['run'](['git', 'add', 'lib.rs'], fixture, evidence / 'expectation-add.log')
            helper['run'](['git', '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
                           'commit', '-qm', 'fixture runtime expectations'], fixture, evidence / 'expectation-commit.log')
            results.append(observe(fixture, evidence, 'successful-seed', 'owned'))
            (fixture / 'ui/src/main.js').write_text('this is invalid JavaScript :::\n')
            results.append(observe(fixture, evidence, 'failed-producer', 'placeholder'))
            results.append(observe(fixture, evidence, 'failed-producer-repeat', 'placeholder'))
            assert results[1]['frontend_failure_warning'] and not results[1]['frontend_success']
            assert results[1]['owned_index_still_exists'], 'failure did not retain the stale-index scenario'
            assert results[2]['fresh_count'] == results[2]['artifact_count'], results[2]
            assert results[1]['script_output_mtime_ns'] == results[2]['script_output_mtime_ns']
            prebuilt = fixture / 'ui/dist'
            prebuilt.mkdir()
            (prebuilt / 'index.html').write_text('<title>prebuilt fallback marker</title>')
            results.append(observe(fixture, evidence, 'failed-prebuilt', 'prebuilt'))
            results.append(observe(fixture, evidence, 'failed-prebuilt-repeat', 'prebuilt'))
            assert results[-1]['fresh_count'] == results[-1]['artifact_count']
            assert results[-2]['script_output_mtime_ns'] == results[-1]['script_output_mtime_ns']
            (fixture / 'ui/src/main.js').write_text('document.body.dataset.proof = "ui-owned-output-proof-js";\n')
            results.append(observe(fixture, evidence, 'recovered-producer', 'owned'))
            results.append(observe(fixture, evidence, 'recovered-producer-repeat', 'owned'))
            assert results[-1]['fresh_count'] == results[-1]['artifact_count']
            assert results[-2]['script_output_mtime_ns'] == results[-1]['script_output_mtime_ns']
            print('Actual Vite success/failure/recovery, stale-output rejection, placeholder/prebuilt selection and fresh repeats PASS')
    finally:
        (evidence / 'results.json').write_text(json.dumps(results, indent=2) + '\n')


if __name__ == '__main__':
    main()
