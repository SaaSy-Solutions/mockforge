"""Offline synthetic tool/ownership tests; no downloads, scans or native binaries."""
from contextlib import redirect_stdout
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
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
JOB = CI.split('  security-audit:\n', 1)[1].split('\n  coverage:', 1)[0]
SPEC = importlib.util.spec_from_file_location('owned_security', ROOT / 'scripts/ci-owned-security-tools.py')
TOOLS = importlib.util.module_from_spec(SPEC); SPEC.loader.exec_module(TOOLS)


def step(name):
    return JOB.split('    - name: ' + name + '\n', 1)[1].split('    - name:', 1)[0]


def script(name):
    return textwrap.dedent(step(name).split('      run: |\n', 1)[1])


VERIFY = script('Verify provisioned security audit tools on Hetzner')


class SecurityToolOwnershipTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='security ownership '); self.root = Path(self.tmp.name)
        self.temp = self.root / 'runner-temp'; self.temp.mkdir()
        self.cargo_home = self.root / 'existing-cargo'; (self.cargo_home / 'bin').mkdir(parents=True)
        self.system_bin = self.root / 'system-bin'; self.system_bin.mkdir()
        self.env = {**os.environ, 'RUNNER_TEMP': str(self.temp), 'GITHUB_RUN_ID': '123', 'GITHUB_RUN_ATTEMPT': '2',
                    'GITHUB_JOB': 'security-audit', 'RUNNER_NAME': 'test-runner', 'RUNNER_OS': 'Linux', 'RUNNER_ARCH': 'X64',
                    'CARGO_HOME': str(self.cargo_home), 'PATH': str(self.system_bin) + os.pathsep + os.environ['PATH']}
        for key in ('GITHUB_ENV', 'GITHUB_PATH'):
            path = self.root / key; path.touch(); self.env[key] = str(path)
        for spec in TOOLS.SPECS:
            self.tool(self.cargo_home / 'bin' / spec['tool'], spec['tool'] + ' 0.0.0')
        # Model the pinned Cargo1.96 search_directories/find_external_subcommand
        # source. This is a synthetic command, not native Cargo qualification.
        cargo = self.system_bin / 'cargo'
        cargo.write_text('#!' + sys.executable + '\nimport os,sys\nfrom pathlib import Path\nif sys.argv[1:]==["--version"]: print(os.environ.get("FAKE_CARGO_VERSION","cargo 1.96.0 (reviewed 2026-05-25)"));sys.exit(0)\npaths=os.environ["PATH"].split(os.pathsep)\nhome=str(Path(os.environ["CARGO_HOME"])/"bin")\nif home not in paths: paths.insert(0,home)\nfor directory in paths:\n target=Path(directory)/("cargo-"+sys.argv[1])\n if target.is_file(): os.execv(str(target),[str(target),*sys.argv[1:]])\nsys.exit(127)\n')
        cargo.chmod(0o755)
        self.payloads = {spec['tool']: self.archive(spec) for spec in TOOLS.SPECS}

    def tearDown(self): self.tmp.cleanup()

    def tool(self, path, version):
        path.write_text('#!/bin/sh\nprintf "%s\\n" "' + version + '"\n'); path.chmod(0o755)

    def archive(self, spec, version=None, extra=None, omit_binary=False):
        output = io.BytesIO()
        with tarfile.open(fileobj=output, mode='w:gz') as archive:
            if not omit_binary:
                data = ('#!/bin/sh\nprintf "%s\\n" "' + spec['tool'] + ' ' + (version or spec['version']) + '"\n').encode()
                member = tarfile.TarInfo(spec['top'] + '/' + spec['tool']); member.size = len(data); member.mode = 0o755
                archive.addfile(member, io.BytesIO(data))
            if extra:
                member, data = extra; archive.addfile(member, io.BytesIO(data) if data else None)
        return output.getvalue()

    def fake_specs(self):
        return tuple({**spec, 'sha256': hashlib.sha256(self.payloads[spec['tool']]).hexdigest()} for spec in TOOLS.SPECS)

    def prepare(self, env=None, bad_checksum=False, fail_second=False):
        specs = self.fake_specs()
        if bad_checksum: specs = ({**specs[0], 'sha256': '0' * 64}, specs[1])
        def download(destination, url):
            spec = next(spec for spec in specs if spec['url'] == url)
            if fail_second and spec['tool'] == 'cargo-deny':
                destination.write_bytes(b'partial')
                raise OSError('second download failed')
            destination.write_bytes(self.payloads[spec['tool']])
        with patch.object(TOOLS, 'SPECS', specs), patch.object(TOOLS, 'download', download), redirect_stdout(io.StringIO()):
            return TOOLS.prepare(env or self.env)

    def installed_env(self):
        root, token = self.prepare()
        values = dict(line.split('=', 1) for line in Path(self.env['GITHUB_ENV']).read_text().splitlines())
        added = Path(self.env['GITHUB_PATH']).read_text().splitlines()
        return root, token, {**self.env, **values, 'PATH': os.pathsep.join(reversed(added)) + os.pathsep + self.env['PATH']}

    def verify(self, env):
        return subprocess.run(['/bin/bash', '-e', '-o', 'pipefail', '-c', VERIFY], env=env, text=True, capture_output=True)

    def assert_unmodified(self):
        for spec in TOOLS.SPECS:
            self.assertIn(spec['tool'] + ' 0.0.0', (self.cargo_home / 'bin' / spec['tool']).read_text())
        self.assertEqual(self.env['CARGO_HOME'], str(self.cargo_home))

    def assert_empty(self):
        self.assertEqual(list(self.temp.iterdir()), []); self.assert_unmodified()
        for key in ('GITHUB_ENV', 'GITHUB_PATH'): self.assertEqual(Path(self.env[key]).read_text(), '')

    def test_exact_official_assets_versions_and_digests(self):
        audit, deny = TOOLS.SPECS
        self.assertEqual((audit['tool'], audit['version'], audit['sha256']), ('cargo-audit', '0.22.2', '7fb9497f8594b389e5fce5ef9b92db08432996895b2e0c5a0167a69ed445c428'))
        self.assertEqual((deny['tool'], deny['version'], deny['sha256']), ('cargo-deny', '0.20.2', '9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f'))
        self.assertEqual(audit['url'], 'https://github.com/rustsec/rustsec/releases/download/cargo-audit/v0.22.2/cargo-audit-x86_64-unknown-linux-musl-v0.22.2.tgz')
        self.assertEqual(deny['url'], 'https://github.com/EmbarkStudios/cargo-deny/releases/download/0.20.2/cargo-deny-0.20.2-x86_64-unknown-linux-musl.tar.gz')

    def test_normal_ci_selects_regression_and_forks_keep_original_installer(self):
        changes = CI.split('  changes:\n', 1)[1].split('\n  test:', 1)[0]
        self.assertLess(changes.index('Checkout repository'), changes.index('Verify owned security tool setup and cleanup'))
        self.assertLess(changes.index('Verify owned security tool setup and cleanup'), changes.index('Determine whether non-docs'))
        self.assertIn('python3 scripts/tests/test_owned_security_tools.py', changes)
        self.assertIn("if: runner.environment == 'github-hosted'", step('Install pinned security audit tools'))
        self.assertIn('uses: taiki-e/install-action@v2.87.22', step('Install pinned security audit tools'))
        self.assertIn("if: runner.environment == 'self-hosted'", step('Prepare job-owned security audit tools on Hetzner'))

    def test_direct_and_cargo_subcommands_use_owned_exact_tools(self):
        root, token, env = self.installed_env()
        self.assertEqual(Path(self.env['GITHUB_PATH']).read_text().splitlines(), [str(self.cargo_home / 'bin'), str(root / 'bin')])
        result = self.verify(env)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(result.stdout.count('Security version probe:'), 4)
        self.assert_unmodified()

    def test_wrong_or_missing_cargo_fails_before_plugin_probes(self):
        root, token, env = self.installed_env()
        for version in ('cargo 1.95.0 (old)', 'cargo 1.96.1 (different)', 'cargo 1.96.0-nightly (different)', ''):
            with self.subTest(version=version):
                result = self.verify({**env, 'FAKE_CARGO_VERSION': version})
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('Cargo version probe: actual=' + version, result.stdout)
                self.assertNotIn('Security version probe:', result.stdout)
        (self.system_bin / 'cargo').unlink()
        # Avoid finding any host cargo in this negative control.
        result = self.verify({**env, 'PATH': str(root / 'bin') + os.pathsep + str(self.cargo_home / 'bin')})
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Cargo version probe: actual= exit=127', result.stdout)
        self.assertNotIn('Security version probe:', result.stdout)

    def test_missing_explicit_cargo_home_path_exposes_foreign_plugin_and_fails(self):
        root, token, env = self.installed_env()
        env['PATH'] = os.pathsep.join(path for path in env['PATH'].split(os.pathsep) if path != str(self.cargo_home / 'bin'))
        result = self.verify(env)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('actual=cargo-audit 0.0.0', result.stdout)
        self.assert_unmodified()

    def test_foreign_same_version_direct_tools_fail_path_ownership(self):
        root, token, env = self.installed_env()
        foreign = self.root / 'foreign'; foreign.mkdir()
        for spec in TOOLS.SPECS: self.tool(foreign / spec['tool'], spec['tool'] + ' ' + spec['version'])
        result = self.verify({**env, 'PATH': str(foreign) + os.pathsep + env['PATH']})
        self.assertNotEqual(result.returncode, 0); self.assert_unmodified()

    def test_wrong_version_prints_diagnostics_before_failing(self):
        root, token, env = self.installed_env()
        self.tool(root / 'bin/cargo-audit', 'cargo-audit 0.1.0')
        result = self.verify(env)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('expected=cargo-audit 0.22.2 actual=cargo-audit 0.1.0 exit=0', result.stdout)

    def test_checksum_mismatch_and_partial_second_download_clean_own_root(self):
        with self.assertRaisesRegex(RuntimeError, 'SHA256'): self.prepare(bad_checksum=True)
        self.assert_empty()
        with self.assertRaisesRegex(OSError, 'second download failed'): self.prepare(fail_second=True)
        self.assert_empty()

    def test_wrong_binary_version_is_not_published_or_retained(self):
        spec = TOOLS.SPECS[1]; self.payloads[spec['tool']] = self.archive(spec, version='0.1.0')
        with self.assertRaisesRegex(RuntimeError, 'version verification'): self.prepare()
        self.assert_empty()

    def test_invalid_archive_missing_binary_and_extraction_bounds_fail(self):
        spec = TOOLS.SPECS[0]
        self.payloads[spec['tool']] = b'not a gzip archive'
        with self.assertRaises(tarfile.ReadError): self.prepare()
        self.assert_empty()
        self.payloads[spec['tool']] = self.archive(spec, omit_binary=True)
        with self.assertRaisesRegex(RuntimeError, 'expected binary'): self.prepare()
        self.assert_empty()
        self.payloads[spec['tool']] = self.archive(spec)
        with patch.object(TOOLS, 'MAX_EXTRACTED', 0), self.assertRaisesRegex(RuntimeError, 'extraction limits'): self.prepare()
        self.assert_empty()

    def test_links_traversal_and_unexpected_members_are_rejected(self):
        spec = TOOLS.SPECS[0]
        for name, kind, link in (('../foreign', tarfile.REGTYPE, ''), (spec['top']+'/link', tarfile.SYMTYPE, '../../foreign'),
                                 (spec['top']+'/link', tarfile.LNKTYPE, 'cargo-audit'), (spec['top']+'/extra', tarfile.REGTYPE, '')):
            member = tarfile.TarInfo(name); member.type = kind; member.linkname = link
            self.payloads[spec['tool']] = self.archive(spec, extra=(member, None))
            with self.subTest(name=name, kind=kind), self.assertRaises(RuntimeError): self.prepare()
            self.assert_empty()

    def test_platform_identity_and_existing_cargo_home_fail_before_allocation(self):
        for override in ({'RUNNER_OS':'Windows'}, {'RUNNER_ARCH':'ARM64'}, {'GITHUB_RUN_ID':'../foreign'}, {'GITHUB_JOB':''}, {'CARGO_HOME':'relative'}, {'CARGO_HOME':'/tmp/a:b'}):
            with self.subTest(override=override), self.assertRaises(RuntimeError): self.prepare({**self.env, **override})
            self.assert_empty()

    def test_sibling_cleanup_and_foreign_receipt_refusal(self):
        a, at = self.prepare(); b, bt = self.prepare()
        self.assertNotEqual(a, b); self.assertNotEqual(at, bt)
        with self.assertRaises(RuntimeError): TOOLS.remove_owned(a, bt, self.env)
        self.assertTrue(a.exists()); self.assertTrue(b.exists())
        TOOLS.remove_owned(a, at, self.env)
        self.assertTrue((b/'bin/cargo-audit').is_file()); self.assert_unmodified()
        receipt=b/TOOLS.OWNER_FILE; original=receipt.read_text(); owner=json.loads(original); owner['inode']+=1;receipt.write_text(json.dumps(owner))
        with self.assertRaises(RuntimeError): TOOLS.remove_owned(b,bt,self.env)
        receipt.write_text(original); TOOLS.remove_owned(b,bt,self.env)

    def test_cleanup_failure_remains_a_failure_and_preserves_locator(self):
        root, token = self.prepare()
        with patch.object(TOOLS.shutil, 'rmtree', side_effect=OSError('cleanup failed')), self.assertRaisesRegex(OSError, 'cleanup failed'):
            # Preserve the platform capability attribute while injecting failure.
            TOOLS.shutil.rmtree.avoids_symlink_attacks = True
            TOOLS.remove_owned(root, token, self.env)
        self.assertTrue((root / TOOLS.OWNER_FILE).exists()); self.assert_unmodified()

    def test_repeated_term_during_second_setup_preserves_existing_sibling(self):
        a, at = self.prepare(); ready=self.root/'ready'; cleaning=self.root/'cleaning'
        specs=self.fake_specs(); payload_dir=self.root/'archives';payload_dir.mkdir()
        for tool,payload in self.payloads.items():(payload_dir/tool).write_bytes(payload)
        code='''
import importlib.util,json,sys,time
from pathlib import Path
helper,ready,cleaning,payloads,specs=sys.argv[1:]
spec=importlib.util.spec_from_file_location('owned',helper);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
m.SPECS=json.loads(specs)
def download(destination,url):
 tool=next(s['tool'] for s in m.SPECS if s['url']==url)
 if tool=='cargo-deny':
  destination.write_bytes(b'partial');Path(ready).touch();time.sleep(10)
 else:destination.write_bytes((Path(payloads)/tool).read_bytes())
original=m.remove_owned
def remove(root,token,env):
 Path(cleaning).touch();time.sleep(0.3);original(root,token,env)
m.download=download;m.remove_owned=remove;sys.argv=[helper,'prepare'];m.main()
'''
        with subprocess.Popen([sys.executable,'-c',code,str(ROOT/'scripts/ci-owned-security-tools.py'),str(ready),str(cleaning),str(payload_dir),json.dumps(specs)],env=self.env,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE,text=True) as child:
            try:
                deadline=time.monotonic()+5
                while not ready.exists() and child.poll() is None and time.monotonic()<deadline:time.sleep(.01)
                self.assertTrue(ready.exists());child.send_signal(signal.SIGTERM)
                while not cleaning.exists() and child.poll() is None and time.monotonic()<deadline:time.sleep(.01)
                self.assertTrue(cleaning.exists());child.send_signal(signal.SIGTERM)
                _,error=child.communicate(timeout=5);self.assertEqual(child.returncode,143,error)
                self.assertEqual(list(self.temp.iterdir()),[a]);self.assertTrue((a/'bin/cargo-deny').is_file());self.assert_unmodified()
            finally:
                if child.poll() is None:child.kill();child.wait()
        TOOLS.remove_owned(a,at,self.env)

    def test_scan_policies_and_job_limits_are_unchanged(self):
        self.assertIn('timeout-minutes: 30',JOB)
        self.assertIn('run: python3 scripts/precommit/cargo_audit.py',step('Run RustSec advisory scan'))
        self.assertIn('run: cargo deny check licenses sources bans',step('Run license and source compliance check'))
        cleanup=step('Clean up owned security tools on Hetzner')
        self.assertIn("if: always() && runner.environment == 'self-hosted' && steps.security-tools.outcome == 'success'",cleanup)
        self.assertNotIn('continue-on-error',cleanup)
        self.assertLess(JOB.index('name: Check for unsafe code blocks'),JOB.index('name: Clean up owned security tools'))

    def test_nonofficial_redirects_fail_closed(self):
        handler=TOOLS.OfficialRedirects()
        for url in ('http://github.com/asset','https://example.com/asset','https://user@github.com/asset','https://github.com:8443/asset'):
            with self.subTest(url=url),self.assertRaises(RuntimeError):handler.redirect_request(None,None,302,None,{},url)


if __name__=='__main__':unittest.main()
