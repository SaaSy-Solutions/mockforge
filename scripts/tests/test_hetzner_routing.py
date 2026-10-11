"""Static routing guards; never dispatch jobs or contact a runner."""

from pathlib import Path
import json
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / '.github/workflows'
LABELS = ['self-hosted', 'linux', 'x64', 'rust']
ROUTE = '${{ (github.event_name != \'pull_request\' || github.event.pull_request.head.repo.full_name == github.repository) && fromJSON(\'["self-hosted","linux","x64","rust"]\') || \'ubuntu-latest\' }}'
TARGETS = {
    'ci.yml': ['changes', 'test', 'ui-test', 'warning-gate',
               'warning-ratchet-preview', 'security-audit', 'msrv',
               'typos', 'changelog-validation'],
    'source-install-smoke.yml': ['smoke'],
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
        for filename in ('ci.yml', 'source-install-smoke.yml', 'registry-e2e.yml'):
            for job_id in TARGETS[filename]:
                block = jobs(filename)[job_id]
                steps = re.split(r'(?=^\s+- (?:name|uses):)', block, flags=re.M)
                for step in steps:
                    if re.search(r'^\s*(?:run: )?(?:sudo (?:apt-get|rm)|rustup default)', step, flags=re.M):
                        self.assertIn("if: runner.environment == 'github-hosted'", step,
                                      f'{filename}:{job_id}')

    def test_native_preflight_is_read_only_and_fails_closed(self):
        expected = {'ci.yml': ['test', 'warning-gate', 'warning-ratchet-preview', 'msrv'],
                    'source-install-smoke.yml': ['smoke']}
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
        for filename in ('ci.yml', 'source-install-smoke.yml', 'registry-e2e.yml'):
            for job_id in TARGETS[filename]:
                block = jobs(filename)[job_id]
                if 'uses: ./.github/actions/rust-toolchain' in block:
                    step = block.split('name: Set per-runner CARGO_HOME on Hetzner', 1)[1].split('- name:', 1)[0]
                    self.assertIn("if: runner.environment == 'self-hosted'", step)
                    self.assertIn('CARGO_HOME=/tmp/cargo-mockforge-${{ runner.name }}', step)

    def test_new_self_hosted_jobs_have_bounded_timeouts(self):
        for filename in [*TARGETS, 'ashburn-images.yml', 'go-publish-sdk.yml']:
            for job_id, block in jobs(filename).items():
                if 'self-hosted' in route(block):
                    self.assertRegex(block, r'(?m)^    timeout-minutes: [1-9][0-9]*$', f'{filename}:{job_id}')

    def test_sensitive_release_jobs_and_os_coverage_are_not_rerouted(self):
        self.assertEqual(route(jobs('release.yml')['release-windows']), 'windows-latest')
        self.assertEqual(route(jobs('release.yml')['release']), 'ubuntu-latest')
        plugin = jobs('plugin-publish.yml')
        self.assertEqual(route(plugin['build']), '${{ matrix.os }}')
        self.assertIn('os: [ubuntu-latest, macos-latest, windows-latest]', plugin['build'])
        for name in ['npm', 'python', 'maven', 'nuget']:
            self.assertEqual(route(jobs(f'{name}-publish-sdk.yml')['publish']), 'ubuntu-latest')

    def test_monitoring_and_isolated_benchmark_capacity_are_not_silently_degraded(self):
        for filename, job_id in [('runner-health.yml', 'check'), ('production-smoke.yml', 'smoke'), ('benchmarks.yml', 'benchmark')]:
            self.assertEqual(route(jobs(filename)[job_id]), 'ubuntu-latest')

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
                self.assertIn('rustup run --no-auto-install', preflight)
                self.assertNotIn('rustup default', preflight)
                self.assertNotIn('toolchain install', preflight)

    def test_security_installer_cannot_provision_shared_host_packages(self):
        block = jobs('ci.yml')['security-audit']
        installer = block.split('name: Install pinned security audit tools', 1)[1].split('- name:', 1)[0]
        self.assertIn("if: runner.environment == 'github-hosted'", installer)
        preflight = block.split('name: Verify provisioned security audit tools on Hetzner', 1)[1].split('- name:', 1)[0]
        self.assertIn("if: runner.environment == 'self-hosted'", preflight)
        self.assertIn('cargo-audit 0.22.2', preflight)
        self.assertIn('cargo-deny 0.20.2', preflight)
        self.assertNotIn('install', preflight)

    def test_go_sdk_job_requires_no_publishing_credentials(self):
        text = (WORKFLOWS / 'go-publish-sdk.yml').read_text()
        self.assertIn('contents: read', text)
        self.assertNotIn('secrets.', text)
        self.assertNotIn('id-token:', text)
        self.assertEqual(route(jobs('go-publish-sdk.yml')['publish']), '[self-hosted, linux, x64, rust]')


if __name__ == '__main__':
    unittest.main()
