"""Static routing guards; never dispatch jobs or contact a runner."""

from pathlib import Path
import json
import re
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / '.github/workflows'
LABELS = ['self-hosted', 'linux', 'x64', 'rust']
ROUTE = '${{ (github.event_name != \'pull_request\' || github.event.pull_request.head.repo.full_name == github.repository) && fromJSON(\'["self-hosted","linux","x64","rust"]\') || \'ubuntu-latest\' }}'
HETZNER = '[self-hosted, linux, x64, rust]'
PLUGIN_MATRIX_ROUTE = '${{ matrix.os == \'ubuntu-latest\' && fromJSON(\'["self-hosted","linux","x64","rust"]\') || matrix.os }}'
TARGETS = {
    'ci.yml': ['changes', 'test', 'ui-test', 'warning-gate',
               'warning-ratchet-preview', 'security-audit', 'msrv'],
    'registry-e2e.yml': ['changes', 'registry-e2e', 'registry-e2e-rls'],
}


def jobs(filename):
    text = (WORKFLOWS / filename).read_text().split('\njobs:\n', 1)[1]
    matches = list(re.finditer(r'^  ([\w-]+):\n', text, flags=re.M))
    return {match.group(1): text[match.end():matches[i+1].start() if i+1 < len(matches) else len(text)]
            for i, match in enumerate(matches)}


def route(block):
    return re.search(r'^    runs-on: (.+)$', block, flags=re.M).group(1)


def select_runner(expression, event_name, head_repository, repository):
    # Evaluate only the deliberately narrow, audited expression grammar.
    # Reject any grammar expansion rather than treating an unknown expression
    # as a trusted route. Metadata comes from the event, never the PR text.
    pattern = (r"\$\{\{ \(github.event_name != '(?P<event>[^']+)' \|\| "
               r"github.event.pull_request.head.repo.full_name == github.repository\) "
               r"&& fromJSON\('(?P<labels>[^']+)'\) \|\| '(?P<fallback>[^']+)' \}\}")
    match = re.fullmatch(pattern, expression)
    if not match:
        raise ValueError('unrecognized routing expression')
    trusted = event_name != match['event'] or head_repository == repository
    return json.loads(match['labels']) if trusted else match['fallback']



# The supported run grammar is pinned to rust-lang/rustup
# 61271d00a979e8fcc13a366d9515f4bf69b88ac2 (Cargo.toml 1.30.0):
# src/cli/rustup_mode.rs:234-245,1237-1246; src/config.rs:478-488.
# This strict fixture implements only the four read-only probes below. It
# rejects every option in the toolchain slot, including the real --install
# option, because preflights must never request installation.
RUSTUP_CLI_STUB = r'''#!/bin/sh
[ "$#" -ge 4 ] && [ "$1" = run ] || exit 77
case "$2" in ''|-*) exit 77 ;; esac
toolchain=$2
shift 2
case "$*" in
  'rustc --version'|'cargo --version'|'rustfmt --version')
    [ "$#" -eq 2 ] || exit 77
    probe=$1 ;;
  'cargo clippy --version')
    [ "$#" -eq 3 ] || exit 77
    probe=clippy ;;
  *) exit 77 ;;
esac
[ "${RUSTUP_AUTO_INSTALL-unset}" = 0 ] || exit 78
if [ -n "${FIXTURE_TRACE-}" ]; then
  printf '%s %s %s\n' "$toolchain" "$probe" "$RUSTUP_AUTO_INSTALL" >> "$FIXTURE_TRACE"
fi
[ "${FIXTURE_FAILURE-}" != missing_toolchain ] || exit 1
[ "${FIXTURE_FAILURE-}" != "missing_$probe" ] || exit 1
version=$toolchain
[ "${FIXTURE_FAILURE-}" != "wrong_$probe" ] || version=1.95.0
printf '%s %s (fixture)\n' "$probe" "$version"
'''


def rust_preflights():
    for filename, ids in TARGETS.items():
        for job_id in ids:
            block = jobs(filename)[job_id]
            marker = 'name: Verify provisioned Rust toolchain on Hetzner'
            if marker not in block:
                continue
            step = block.split(marker, 1)[1].split('- name:', 1)[0]
            yield filename, job_id, textwrap.dedent(step.split('run: |\n', 1)[1]).strip()


class HetznerRoutingTest(unittest.TestCase):
    def test_only_existing_labels_are_selected(self):
        for filename, ids in TARGETS.items():
            for job_id in ids:
                with self.subTest(workflow=filename, job=job_id):
                    self.assertEqual(route(jobs(filename)[job_id]), ROUTE)

    def test_event_truth_table_preserves_fork_hosted_route(self):
        repository = 'SaaSy-Solutions/mockforge'
        cases = [
            ('pull_request', repository, LABELS),
            ('pull_request', 'someone/mockforge', 'ubuntu-latest'),
            ('pull_request', None, 'ubuntu-latest'),
            ('push', None, LABELS),
            ('workflow_dispatch', None, LABELS),
        ]
        for filename, ids in TARGETS.items():
            for job_id in ids:
                expression = route(jobs(filename)[job_id])
                for event, head, expected in cases:
                    with self.subTest(workflow=filename, job=job_id, event=event, head=head):
                        self.assertEqual(select_runner(expression, event, head, repository), expected)

    def test_no_pull_request_target_or_other_new_event(self):
        expected = {
            'ci.yml': {'push', 'pull_request', 'workflow_dispatch'},
            'source-install-smoke.yml': {'pull_request'},
            'registry-e2e.yml': {'push', 'pull_request', 'workflow_dispatch'},
            'ashburn-images.yml': {'push', 'workflow_dispatch'},
            'go-publish-sdk.yml': {'push', 'workflow_dispatch'},
        }
        for filename, events in expected.items():
            text = (WORKFLOWS / filename).read_text()
            header = re.search(r'^on:\n(.*?)(?=^\w|\Z)', text, flags=re.M | re.S).group(1)
            found = set(re.findall(r'^  ([\w_]+):', header, flags=re.M))
            self.assertEqual(found, events, filename)

    def test_native_bootstrap_and_system_deletion_are_hosted_only(self):
        for filename in ('ci.yml', 'registry-e2e.yml'):
            for job_id in TARGETS[filename]:
                block = jobs(filename)[job_id]
                steps = re.split(r'(?=^\s+- (?:name|uses):)', block, flags=re.M)
                for step in steps:
                    if re.search(r'^\s*(?:run: )?(?:sudo (?:apt-get|rm)|rustup default)', step, flags=re.M):
                        self.assertIn("if: runner.environment == 'github-hosted'", step,
                                      f'{filename}:{job_id}')

    def test_native_preflight_is_read_only_and_fails_closed(self):
        expected = {'ci.yml': ['test', 'warning-gate', 'warning-ratchet-preview', 'msrv']}
        for filename, ids in expected.items():
            for job_id in ids:
                block = jobs(filename)[job_id]
                preflight = block.split('name: Verify native build dependencies on Hetzner', 1)[1].split('- name:', 1)[0]
                self.assertIn("if: runner.environment == 'self-hosted'", preflight)
                self.assertIn('dpkg -s ', preflight)
                self.assertIn('exit 1', preflight)
                self.assertNotIn('sudo', preflight)
                self.assertNotIn('apt-get', preflight)

    def test_rust_jobs_isolate_cargo_home_by_existing_runner_identity(self):
        for filename in ('ci.yml', 'registry-e2e.yml'):
            for job_id in TARGETS[filename]:
                block = jobs(filename)[job_id]
                if 'uses: ./.github/actions/rust-toolchain' in block:
                    step = block.split('name: Set per-runner CARGO_HOME on Hetzner', 1)[1].split('- name:', 1)[0]
                    self.assertIn("if: runner.environment == 'self-hosted'", step)
                    self.assertIn('CARGO_HOME=/tmp/cargo-mockforge-${{ runner.name }}', step)

    def test_new_self_hosted_jobs_have_bounded_timeouts(self):
        for filename in TARGETS:
            for job_id, block in jobs(filename).items():
                if 'self-hosted' in route(block):
                    self.assertRegex(block, r'(?m)^    timeout-minutes: [1-9][0-9]*$', f'{filename}:{job_id}')

    def test_release_and_publish_jobs_run_on_hetzner_except_other_os(self):
        self.assertEqual(route(jobs('release.yml')['release-windows']), 'windows-latest')
        for job_id in ('release', 'publish', 'helm'):
            self.assertEqual(route(jobs('release.yml')[job_id]), HETZNER, job_id)
        plugin = jobs('plugin-publish.yml')
        self.assertEqual(route(plugin['build']), PLUGIN_MATRIX_ROUTE)
        self.assertIn('os: [ubuntu-latest, macos-latest, windows-latest]', plugin['build'])
        for name in ['go', 'npm', 'python', 'maven', 'nuget']:
            self.assertEqual(route(jobs(f'{name}-publish-sdk.yml')['publish']), HETZNER, name)

    def test_pull_request_light_jobs_use_trusted_route(self):
        # Fork PRs must never reach the shared Hetzner host (#1191).
        for filename, job_id in [('ci.yml', 'typos'), ('ci.yml', 'changelog-validation'),
                                 ('migration-guard.yml', 'check'),
                                 ('k8s-tests.yml', 'validate-manifests'),
                                 ('k8s-tests.yml', 'lint-helm-charts'),
                                 ('k8s-tests.yml', 'cost-estimation'),
                                 ('benchmarks.yml', 'benchmark'),
                                 ('source-install-smoke.yml', 'smoke')]:
            with self.subTest(workflow=filename, job=job_id):
                self.assertEqual(route(jobs(filename)[job_id]), ROUTE)

    def test_only_runner_health_monitor_stays_hosted(self):
        self.assertEqual(route(jobs('runner-health.yml')['check']), 'ubuntu-latest')
        self.assertEqual(route(jobs('production-smoke.yml')['smoke']), HETZNER)
        smoke = (WORKFLOWS / 'production-smoke.yml').read_text()
        self.assertIn('group: production-smoke\n  cancel-in-progress: false', smoke)
        # Docs verification does not compile Rust. It used to take a rust-pool
        # slot on every book edit. It stays GitHub-hosted, like runner-health.
        self.assertEqual(route(jobs('docs-check.yml')['docs']), 'ubuntu-latest')

    def test_docs_smoke_and_runner_health_stay_off_pull_request_flood(self):
        docs = (WORKFLOWS / 'docs-check.yml').read_text()
        paths = re.search(r'^    paths:\n(?P<body>(?:      - .*\n)+)', docs, flags=re.M).group('body')
        for required in ("- 'book/**'", "- 'docs/**'", "- '*.md'", "- '*.markdown'",
                         "- '**/*.md'", "- '**/*.markdown'"):
            self.assertIn(required, paths)
        self.assertNotIn('main.rs', paths)
        self.assertIn("cancel-in-progress: ${{ github.event_name == 'pull_request' }}", docs)
        smoke = (WORKFLOWS / 'production-smoke.yml').read_text()
        header = re.search(r'^on:\n(.*?)(?=^[A-Za-z#])', smoke, flags=re.M | re.S).group(1)
        events = set(re.findall(r'^  ([\w_]+):', header, flags=re.M))
        self.assertEqual(events, {'push', 'schedule', 'workflow_dispatch'})
        self.assertIn('branches: [main]', header)
        self.assertEqual(re.findall(r"cron: '([^']+)'", header), ['17 * * * *'])
        health = (WORKFLOWS / 'runner-health.yml').read_text()
        health_header = re.search(r'^on:\n(.*?)(?=^[A-Za-z#])', health, flags=re.M | re.S).group(1)
        health_events = set(re.findall(r'^  ([\w_]+):', health_header, flags=re.M))
        self.assertEqual(health_events, {'schedule', 'workflow_dispatch'})
        self.assertEqual(re.findall(r"cron: '([^']+)'", health_header), ['47 * * * *'])

    def test_every_job_selects_hetzner_or_an_audited_exception(self):
        allowed = {HETZNER, ROUTE}
        exceptions = {('runner-health.yml', 'check'): 'ubuntu-latest',
                      ('docs-check.yml', 'docs'): 'ubuntu-latest',
                      ('release.yml', 'release-windows'): 'windows-latest',
                      ('plugin-publish.yml', 'build'): PLUGIN_MATRIX_ROUTE}
        for wf in sorted(WORKFLOWS.glob('*.yml')):
            for job_id, block in jobs(wf.name).items():
                selector = re.search(r'^    runs-on: (.+)$', block, flags=re.M)
                if selector is None:
                    continue  # reusable-workflow caller
                with self.subTest(workflow=wf.name, job=job_id):
                    expected = exceptions.get((wf.name, job_id))
                    if expected is not None:
                        self.assertEqual(selector.group(1), expected)
                    else:
                        self.assertIn(selector.group(1), allowed)

    def test_moved_rust_jobs_isolate_cargo_home_and_never_apt_install(self):
        moved = {'benchmarks.yml': ['benchmark'], 'deploy-docs.yml': ['deploy'],
                 'plugin-publish.yml': ['validate', 'security-scan', 'build', 'publish', 'dry-run'],
                 'release.yml': ['release', 'publish'], 'source-install-smoke.yml': ['smoke']}
        for filename, ids in moved.items():
            for job_id in ids:
                block = jobs(filename)[job_id]
                with self.subTest(workflow=filename, job=job_id):
                    first = re.split(r'(?=^\s+- (?:name|uses):)', block.split('steps:\n', 1)[1], flags=re.M)[1]
                    self.assertIn('Set per-runner CARGO_HOME on Hetzner', first)
                    self.assertIn("if: runner.environment == 'self-hosted'", first)
                    self.assertIn('CARGO_HOME=/tmp/cargo-mockforge-${{ runner.name }}', first)
                    steps = re.split(r'(?=^\s+- (?:name|uses):)', block, flags=re.M)
                    for step in steps:
                        if re.search(r'^\s*(?:run: )?sudo (?:apt-get|rm)', step, flags=re.M):
                            self.assertIn("if: runner.environment == 'github-hosted'", step)

    def test_registry_service_jobs_use_trusted_route_and_read_only_preflight(self):
        for job_id in ('registry-e2e', 'registry-e2e-rls'):
            block = jobs('registry-e2e.yml')[job_id]
            self.assertEqual(route(block), ROUTE)
            preflight = block.split('name: Verify registry fixture capabilities on Hetzner', 1)[1].split('- name:', 1)[0]
            self.assertIn('docker info', preflight)
            self.assertIn('docker compose version', preflight)
            self.assertIn('timeout', preflight)
            self.assertIn('pidfd_open', preflight)
            self.assertIn('GNU coreutils', preflight)
            self.assertIn('libc.prctl', preflight)
            self.assertNotIn('sudo', preflight)
            self.assertNotIn('apt-get', preflight)

    def test_newly_moved_rust_actions_cannot_change_shared_defaults(self):
        for filename, ids in TARGETS.items():
            for job_id in ids:
                block = jobs(filename)[job_id]
                if 'uses: ./.github/actions/rust-toolchain' not in block:
                    continue
                steps = re.split(r'(?=^\s+- (?:name|uses):)', block, flags=re.M)
                for step in steps:
                    if 'uses: ./.github/actions/rust-toolchain' in step:
                        self.assertIn("if: runner.environment == 'github-hosted'", step)
                preflight = block.split('name: Verify provisioned Rust toolchain on Hetzner', 1)[1].split('- name:', 1)[0]
                self.assertIn("if: runner.environment == 'self-hosted'", preflight)
                self.assertNotIn('--no-auto-install', preflight)
                self.assertNotIn('--install', preflight)
                self.assertNotIn('export RUSTUP_AUTO_INSTALL', preflight)
                calls = re.findall(r'([^\n]*rustup run[^\n]*)', preflight)
                self.assertGreater(len(calls), 0)
                for call in calls:
                    self.assertIn('RUSTUP_AUTO_INSTALL=0 rustup run "$EXPECTED_TOOLCHAIN" ', call)
                self.assertNotIn('rustup default', preflight)
                self.assertNotIn('toolchain install', preflight)

    def test_rustup_cli_fixture_rejects_unknown_flags_and_auto_install(self):
        with tempfile.TemporaryDirectory() as tmp:
            stub = Path(tmp) / 'rustup'
            stub.write_text(RUSTUP_CLI_STUB)
            stub.chmod(0o755)
            for flag in ('--no-auto-install', '--install', '--unknown'):
                result = subprocess.run([str(stub), 'run', flag, '1.96.0', 'rustc', '--version'],
                                        env={'RUSTUP_AUTO_INSTALL': '0'}, capture_output=True)
                self.assertEqual(result.returncode, 77, flag)
            for value in (None, '1'):
                env = {} if value is None else {'RUSTUP_AUTO_INSTALL': value}
                result = subprocess.run([str(stub), 'run', '1.96.0', 'rustc', '--version'],
                                        env=env, capture_output=True)
                self.assertEqual(result.returncode, 78, value)
            result = subprocess.run([str(stub), 'run', '1.96.0', 'rustc', '--unsupported'],
                                    env={'RUSTUP_AUTO_INSTALL': '0'}, capture_output=True)
            self.assertEqual(result.returncode, 77)

    def test_rust_preflights_use_supported_command_scoped_cli(self):
        count = 0
        for filename, job_id, script in rust_preflights():
            count += 1
            probes = ['rustc', 'cargo']
            if 'rustfmt --version' in script:
                probes += ['rustfmt', 'clippy']
            with self.subTest(workflow=filename, job=job_id), tempfile.TemporaryDirectory() as tmp:
                stub = Path(tmp) / 'rustup'
                stub.write_text(RUSTUP_CLI_STUB)
                stub.chmod(0o755)
                trace = Path(tmp) / 'calls'
                for inherited in (None, '1'):
                    trace.unlink(missing_ok=True)
                    env = {'PATH': tmp, 'EXPECTED_TOOLCHAIN': '1.96.0', 'FIXTURE_TRACE': str(trace)}
                    if inherited is not None:
                        env['RUSTUP_AUTO_INSTALL'] = inherited
                    original = 'unset' if inherited is None else inherited
                    result = subprocess.run(['/bin/bash', '-e', '-o', 'pipefail', '-c',
                                             script + '\n[ "${RUSTUP_AUTO_INSTALL-unset}" = "' + original + '" ]'],
                                            env=env, text=True, capture_output=True)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual(trace.read_text().splitlines(),
                                     [f'1.96.0 {probe} 0' for probe in probes])
        self.assertGreater(count, 0)

    def test_missing_toolchains_components_and_wrong_versions_fail_closed(self):
        for filename, job_id, script in rust_preflights():
            failures = ['missing_toolchain', 'wrong_rustc', 'wrong_cargo']
            if 'rustfmt --version' in script:
                failures += ['missing_rustfmt', 'missing_clippy']
            for failure in failures:
                with self.subTest(workflow=filename, job=job_id, failure=failure), tempfile.TemporaryDirectory() as tmp:
                    stub = Path(tmp) / 'rustup'
                    stub.write_text(RUSTUP_CLI_STUB)
                    stub.chmod(0o755)
                    result = subprocess.run(['/bin/bash', '-e', '-o', 'pipefail', '-c', script],
                                            env={'PATH': tmp, 'EXPECTED_TOOLCHAIN': '1.96.0',
                                                 'FIXTURE_FAILURE': failure}, text=True, capture_output=True)
                    self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_security_installer_cannot_provision_shared_host_packages(self):
        block = jobs('ci.yml')['security-audit']
        installer = block.split('name: Install pinned security audit tools', 1)[1].split('- name:', 1)[0]
        self.assertIn("if: runner.environment == 'github-hosted'", installer)
        preflight = block.split('name: Verify provisioned security audit tools on Hetzner', 1)[1].split('- name:', 1)[0]
        self.assertIn("if: runner.environment == 'self-hosted'", preflight)
        self.assertIn('cargo-audit 0.22.2', preflight)
        self.assertIn('cargo-deny 0.20.2', preflight)
        self.assertNotIn('install', preflight)



if __name__ == '__main__':
    unittest.main()
