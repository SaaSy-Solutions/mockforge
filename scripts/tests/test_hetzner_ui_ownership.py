"""Offline owned runtime/package-manager regressions. No network or installers."""
from contextlib import redirect_stdout
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import tarfile
import tempfile
import textwrap
import time
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
CI = (ROOT / '.github/workflows/ci.yml').read_text()
UI = CI.split('  ui-test:\n', 1)[1].split('\n  # Cross-Platform Tests', 1)[0]
SPEC = importlib.util.spec_from_file_location('owned_ui', ROOT / 'scripts/ci-owned-ui-tools.py')
TOOLS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(TOOLS)


def step(name):
    return UI.split('      - name: ' + name + '\n', 1)[1].split('      - name:', 1)[0]


def script(name):
    return textwrap.dedent(step(name).split('        run: |\n', 1)[1])


VERIFY = script('Verify UI tool and store ownership on Hetzner')


class UiOwnershipTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='ui ownership ')
        self.root = Path(self.tmp.name)
        self.temp = self.root / 'runner-temp'; self.temp.mkdir()
        self.home = self.root / 'shared-home'; self.home.mkdir()
        (self.home / 'setup-pnpm').mkdir(); (self.home / 'setup-pnpm/foreign').write_text('keep')
        self.env = {**os.environ, 'HOME': str(self.home), 'RUNNER_TEMP': str(self.temp),
                    'GITHUB_RUN_ID': '123', 'GITHUB_RUN_ATTEMPT': '2', 'GITHUB_JOB': 'ui-test',
                    'RUNNER_NAME': 'test-runner', 'RUNNER_OS': 'Linux', 'RUNNER_ARCH': 'X64'}
        for key in ('GITHUB_OUTPUT', 'GITHUB_ENV', 'GITHUB_PATH'):
            target = self.root / key; target.touch(); self.env[key] = str(target)
        self.payload = self.archive()

    def tearDown(self):
        self.tmp.cleanup()

    def archive(self, node='v20.20.2', npm='10.8.2', extras=()):
        output = io.BytesIO()
        with tarfile.open(fileobj=output, mode='w:xz') as archive:
            files = [('bin/node', '#!/bin/sh\nif [ "$1" = "--version" ]; then echo ' + node + '; elif [ "${2:-}" = "--version" ]; then echo ' + npm + '; elif [ "${2:-} ${3:-} ${4:-}" = "config get cache" ]; then echo "${FAKE_NPM_CACHE-$npm_config_cache}"; else exit 77; fi\n'),
                     ('lib/node_modules/npm/bin/npm-cli.js', '#!/usr/bin/env node\n')]
            for path, text in files:
                data = text.encode(); member = tarfile.TarInfo(TOOLS.NODE_TOP + '/' + path)
                member.size = len(data); member.mode = 0o755
                archive.addfile(member, io.BytesIO(data))
            member = tarfile.TarInfo(TOOLS.NODE_TOP + '/bin/npm'); member.type = tarfile.SYMTYPE
            member.linkname = '../lib/node_modules/npm/bin/npm-cli.js'; archive.addfile(member)
            for member, data in extras:
                archive.addfile(member, io.BytesIO(data) if data else None)
        return output.getvalue()

    def prepare(self, env=None, payload=None, checksum=None, download_failure=None):
        data = self.payload if payload is None else payload
        def download(destination):
            if download_failure:
                destination.write_bytes(b'partial')
                raise download_failure
            destination.write_bytes(data)
        with patch.object(TOOLS, 'download', download), patch.object(TOOLS, 'NODE_SHA256', checksum or hashlib.sha256(data).hexdigest()), redirect_stdout(io.StringIO()):
            return TOOLS.prepare(env or self.env)

    def fields(self):
        return dict(line.split('=', 1) for line in Path(self.env['GITHUB_ENV']).read_text().splitlines())

    def installed(self):
        root, token = self.prepare()
        fields = self.fields()
        dest = root / 'setup-pnpm'
        binary = dest / 'node_modules/.bin'; binary.mkdir(parents=True)
        pnpm = binary / 'pnpm'
        pnpm.write_text('#!/bin/sh\ncase "$*" in "--version") echo "${FAKE_PNPM_VERSION-9.15.9}" ;; "store path --silent") echo "${FAKE_PNPM_STORE-$npm_config_store_dir/v3}" ;; *) exit 78 ;; esac\n')
        pnpm.chmod(0o755)
        return root, token, {**self.env, **fields, 'PNPM_HOME': str(binary), 'PATH': str(binary) + os.pathsep + str(root / TOOLS.NODE_TOP / 'bin') + os.pathsep + self.env['PATH']}

    def verify(self, env):
        return subprocess.run(['/bin/bash', '-e', '-o', 'pipefail', '-c', VERIFY], env=env, text=True, capture_output=True)

    def assert_empty(self):
        self.assertEqual(list(self.temp.iterdir()), [])
        self.assertEqual((self.home / 'setup-pnpm/foreign').read_text(), 'keep')
        for key in ('GITHUB_OUTPUT', 'GITHUB_ENV', 'GITHUB_PATH'):
            self.assertEqual(Path(self.env[key]).read_text(), '')

    def test_official_release_version_checksum_and_platform_are_pinned(self):
        self.assertEqual(TOOLS.NODE_VERSION, '20.20.2')
        self.assertEqual(TOOLS.NPM_VERSION, '10.8.2')
        self.assertEqual(TOOLS.NODE_URL, 'https://nodejs.org/dist/v20.20.2/node-v20.20.2-linux-x64.tar.xz')
        self.assertEqual(TOOLS.NODE_SHA256, 'df770b2a6f130ed8627c9782c988fda9669fa23898329a61a871e32f965e007d')
        self.assertEqual(TOOLS.MAX_DOWNLOAD, 64 * 1024 * 1024)
        self.assertEqual(TOOLS.MAX_EXTRACTED, 256 * 1024 * 1024)

    def test_normal_ci_selects_ownership_tests_before_classification(self):
        changes = CI.split('  changes:\n', 1)[1].split('\n  test:', 1)[0]
        self.assertLess(changes.index('name: Checkout repository'), changes.index('name: Verify owned UI tool setup and cleanup'))
        self.assertLess(changes.index('name: Verify owned UI tool setup and cleanup'), changes.index('name: Determine whether non-docs files changed'))
        regression = changes.split('      - name: Verify owned UI tool setup and cleanup\n', 1)[1].split('      - name:', 1)[0]
        self.assertNotIn('if:', regression); self.assertNotIn('continue-on-error', regression)
        self.assertIn('python3 scripts/tests/test_hetzner_ui_ownership.py', regression)
        self.assertIn('command -v python3', regression); self.assertIn('command -v bash', regression)

    def test_shared_installers_remain_hosted_only_and_native_post_is_noop(self):
        for name in ('Setup Node.js', 'Install pnpm'):
            self.assertIn("if: runner.environment == 'github-hosted'", step(name))
        self.assertIn('version: 9\n', step('Install pnpm'))
        native = step('Install pnpm in the owned job directory')
        for value in ("if: runner.environment == 'self-hosted'", 'uses: pnpm/action-setup@0977fd99725f1db4007ccb2928dbb4e90d06cc86', 'version: 9.15.9', 'run_install: false', 'cache: false', 'dest: ${{ steps.ui-state.outputs.pnpm_dest }}'):
            self.assertIn(value, native)
        self.assertNotIn('||', native)
        self.assertNotIn('RUNNER_TOOL_CACHE', UI)
        self.assertNotIn('sudo ', UI)

    def test_two_roots_are_distinct_and_first_cleanup_preserves_sibling(self):
        a, at = self.prepare(); b, bt = self.prepare()
        self.assertNotEqual(a, b); self.assertNotEqual(at, bt)
        self.assertEqual(a.parent, self.temp)
        self.assertEqual(b.parent, self.temp)
        self.assertTrue((a / TOOLS.NODE_TOP / 'bin/npm').resolve().is_relative_to(a))
        TOOLS.remove_owned(a, at, self.env)
        self.assertFalse(a.exists()); self.assertTrue((b / TOOLS.NODE_TOP / 'bin/node').exists())
        self.assertEqual((self.home / 'setup-pnpm/foreign').read_text(), 'keep')
        TOOLS.remove_owned(b, bt, self.env)

    def test_all_state_and_path_exports_are_job_owned(self):
        root, token, env = self.installed()
        fields = self.fields()
        self.assertEqual(fields['UI_TOOL_ID'], token)
        for key in ('npm_config_cache', 'npm_config_store_dir', 'XDG_CACHE_HOME', 'XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_STATE_HOME'):
            self.assertTrue(Path(fields[key]).is_relative_to(root))
        self.assertEqual(Path(self.env['GITHUB_PATH']).read_text(), str(root / TOOLS.NODE_TOP / 'bin') + '\n')
        self.assertEqual(self.verify(env).returncode, 0)

    def test_wrong_checksum_fails_before_extraction_and_cleans_own_root(self):
        with self.assertRaisesRegex(RuntimeError, 'SHA256'):
            self.prepare(checksum='0' * 64)
        self.assert_empty()

    def test_wrong_node_or_npm_version_fails_and_cleans_own_root(self):
        for versions in (('v22.0.0', '10.8.2'), ('v20.20.1', '10.8.2'), ('v20.20.2', '11.0.0')):
            with self.subTest(versions=versions), self.assertRaisesRegex(RuntimeError, 'version verification'):
                self.prepare(payload=self.archive(*versions))
            self.assert_empty()

    def test_download_and_extraction_failures_clean_only_allocated_root(self):
        for failure in (TimeoutError('download timeout'), OSError('download failed')):
            with self.subTest(failure=failure), self.assertRaises(type(failure)):
                self.prepare(download_failure=failure)
            self.assert_empty()
        with self.assertRaises(tarfile.ReadError):
            self.prepare(payload=b'not an xz archive')
        self.assert_empty()

    def test_unsafe_archive_members_and_escaping_links_fail_closed(self):
        cases = []
        for path in ('../foreign', '/tmp/foreign', TOOLS.NODE_TOP + '/../foreign'):
            member = tarfile.TarInfo(path); member.size = 1
            cases.append((member, b'x'))
        link = tarfile.TarInfo(TOOLS.NODE_TOP + '/escape'); link.type = tarfile.SYMTYPE; link.linkname = '../../foreign'
        cases.append((link, None))
        device = tarfile.TarInfo(TOOLS.NODE_TOP + '/device'); device.type = tarfile.CHRTYPE
        cases.append((device, None))
        for member in cases:
            with self.subTest(name=member[0].name), self.assertRaises(RuntimeError):
                self.prepare(payload=self.archive(extras=[member]))
            self.assert_empty()

    def test_missing_owned_npm_payload_fails_before_version_execution(self):
        output = io.BytesIO()
        with tarfile.open(fileobj=output, mode='w:xz') as archive:
            data = b'#!/bin/sh\necho v20.20.2\n'
            member = tarfile.TarInfo(TOOLS.NODE_TOP + '/bin/node'); member.size = len(data); member.mode = 0o755
            archive.addfile(member, io.BytesIO(data))
        with self.assertRaisesRegex(RuntimeError, 'expected owned Node/npm files'):
            self.prepare(payload=output.getvalue())
        self.assert_empty()

    def test_prerequisite_and_identity_failure_allocates_no_state(self):
        for override in ({'RUNNER_OS': 'Windows'}, {'RUNNER_ARCH': 'ARM64'}, {'GITHUB_RUN_ID': '../foreign'}, {'GITHUB_JOB': ''}):
            with self.subTest(override=override), self.assertRaises(RuntimeError):
                self.prepare(env={**self.env, **override})
            self.assert_empty()
        with patch.object(TOOLS.sys, 'version_info', (3, 8)), self.assertRaisesRegex(RuntimeError, 'Python 3.9'):
            self.prepare()
        self.assert_empty()

    def test_cleanup_rejects_foreign_identity_token_inode_and_symlink(self):
        root, token = self.prepare()
        for bad_env, bad_token in (({**self.env, 'GITHUB_JOB': 'foreign'}, token), (self.env, '0' * 32)):
            with self.assertRaises(RuntimeError):
                TOOLS.remove_owned(root, bad_token, bad_env)
            self.assertTrue(root.exists())
        receipt = root / TOOLS.OWNER_FILE; original = receipt.read_text()
        wrong = json.loads(original); wrong['inode'] += 1; receipt.write_text(json.dumps(wrong))
        with self.assertRaises(RuntimeError): TOOLS.remove_owned(root, token, self.env)
        receipt.write_text(original)
        renamed = root.with_name(root.name + '-saved'); root.rename(renamed); root.symlink_to(renamed, target_is_directory=True)
        with self.assertRaises(RuntimeError): TOOLS.remove_owned(root, token, self.env)
        self.assertTrue((renamed / TOOLS.NODE_TOP / 'bin/node').exists())
        root.unlink(); renamed.rename(root)
        TOOLS.remove_owned(root, token, self.env)

    def test_foreign_cache_store_pnpm_and_runtime_fail_before_dependencies(self):
        root, token, env = self.installed()
        for override in ({'FAKE_NPM_CACHE': str(self.home / 'npm-cache')}, {'FAKE_PNPM_STORE': str(self.home / 'pnpm-store')}, {'FAKE_PNPM_VERSION': '10.15.0'}, {'PNPM_HOME': str(self.home / 'setup-pnpm')}):
            self.assertNotEqual(self.verify({**env, **override}).returncode, 0)
        self.assertEqual((self.home / 'setup-pnpm/foreign').read_text(), 'keep')

    def test_foreign_npm_with_matching_version_is_rejected(self):
        root, token, env = self.installed()
        foreign = self.root / 'foreign-bin'; foreign.mkdir()
        npm = foreign / 'npm'
        npm.write_text('#!/bin/sh\ncase "$*" in "--version") echo 10.8.2 ;; "config get cache") echo "$npm_config_cache" ;; *) exit 78 ;; esac\n')
        npm.chmod(0o755)
        result = self.verify({**env, 'PATH': str(foreign) + os.pathsep + env['PATH']})
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue(npm.exists())
        self.assertTrue(root.exists())

    def test_cleanup_selection_preserves_upload_order_and_failure(self):
        cleanup = step('Clean up owned UI tools on Hetzner')
        self.assertIn("if: always() && runner.environment == 'self-hosted' && steps.ui-state.outcome == 'success'", cleanup)
        self.assertNotIn('continue-on-error', cleanup)
        self.assertLess(UI.index('name: Upload coverage'), UI.index('name: Clean up owned UI tools on Hetzner'))
        self.assertIn('python3 scripts/ci-owned-ui-tools.py cleanup', cleanup)

    def test_all_original_dependency_test_and_coverage_commands_remain(self):
        for name, command in (('Install dependencies', 'pnpm install --frozen-lockfile'), ('Type check', 'pnpm type-check'), ('Lint', 'pnpm lint'), ('Build UI', 'pnpm build'), ('Run tests', 'pnpm test')):
            self.assertIn('run: ' + command, step(name))
            self.assertIn('working-directory: crates/mockforge-ui/ui', step(name))
        self.assertIn('uses: codecov/codecov-action@v7', step('Upload coverage'))
        self.assertIn("node-version: '20'", step('Setup Node.js'))

    def test_repeated_termination_during_setup_cleans_only_own_partial_root(self):
        ready = self.root / 'download-ready'
        cleaning = self.root / 'cleanup-ready'
        code = """
import importlib.util, os, sys, time
from pathlib import Path
spec = importlib.util.spec_from_file_location('owned', sys.argv[1])
module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
original = module.remove_owned
ready_args = list(sys.argv)
sys.argv = [sys.argv[1], 'prepare']
# The closures use fixed paths, rather than the subsequently changed CLI args.
def download(destination):
    destination.write_bytes(b'partial'); Path(ready_args[2]).touch(); time.sleep(10)
def remove(root, token, env):
    Path(ready_args[3]).touch(); time.sleep(0.3); original(root, token, env)
module.download = download; module.remove_owned = remove
module.main()
"""
        with subprocess.Popen([sys.executable, '-c', code, str(ROOT / 'scripts/ci-owned-ui-tools.py'), str(ready), str(cleaning)], env=self.env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True) as child:
            try:
                deadline = time.monotonic() + 5
                while not ready.exists() and child.poll() is None and time.monotonic() < deadline:
                    time.sleep(0.01)
                self.assertTrue(ready.exists())
                child.send_signal(signal.SIGTERM)
                while not cleaning.exists() and child.poll() is None and time.monotonic() < deadline:
                    time.sleep(0.01)
                self.assertTrue(cleaning.exists())
                child.send_signal(signal.SIGTERM)
                _, error = child.communicate(timeout=5)
                self.assertEqual(child.returncode, 143, error)
                self.assert_empty()
            finally:
                if child.poll() is None:
                    child.kill(); child.wait()

    def test_nonofficial_or_downgraded_redirects_are_rejected(self):
        handler = TOOLS.OfficialRedirects()
        for url in ('http://nodejs.org/archive', 'https://example.com/archive', 'https://user@nodejs.org/archive', 'https://nodejs.org:8443/archive'):
            with self.subTest(url=url), self.assertRaises(RuntimeError):
                handler.redirect_request(None, None, 302, None, {}, url)


if __name__ == '__main__':
    unittest.main()
