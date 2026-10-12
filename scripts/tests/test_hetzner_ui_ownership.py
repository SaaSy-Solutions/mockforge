"""Offline ownership regressions; no package downloads or runner access."""
from pathlib import Path
import os
import re
import shutil
import subprocess
import tempfile
import textwrap
import unittest

ROOT=Path(__file__).resolve().parents[2]
CI=(ROOT/'.github/workflows/ci.yml').read_text()
UI=CI.split('  ui-test:\n',1)[1].split('\n  # Cross-Platform Tests',1)[0]

def step(name):
    block=UI.split('      - name: '+name+'\n',1)[1]
    return block.split('      - name:',1)[0]

def script(name):
    return textwrap.dedent(step(name).split('        run: |\n',1)[1])

PREPARE=script('Prepare job-owned UI tool state on Hetzner')
VERIFY=script('Verify UI tool and store ownership on Hetzner')

class UiOwnershipTest(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory(prefix='ui ownership ')
        self.root=Path(self.tmp.name)
        self.bin=self.root/'bin';self.bin.mkdir()
        self.temp=self.root/'runner-temp';self.temp.mkdir()
        self.home=self.root/'shared-home';self.home.mkdir()
        (self.home/'setup-pnpm').mkdir();(self.home/'setup-pnpm'/'foreign').write_text('keep')
        self.env={'PATH':str(self.bin)+os.pathsep+'/usr/bin:/bin','HOME':str(self.home),
                  'RUNNER_TEMP':str(self.temp),'GITHUB_RUN_ID':'123','GITHUB_RUN_ATTEMPT':'2'}
        self.tool('node','printf "%s\\n" "${FAKE_NODE_VERSION-v20.20.0}"')
        self.tool('npm','case "$*" in "--version") echo 10.8.2 ;; "config get cache") printf "%s\\n" "${FAKE_NPM_CACHE-$npm_config_cache}" ;; *) exit 77 ;; esac')

    def tearDown(self):self.tmp.cleanup()

    def tool(self,name,body,directory=None):
        path=(directory or self.bin)/name;path.parent.mkdir(parents=True,exist_ok=True)
        path.write_text('#!/bin/sh\n'+body+'\n');path.chmod(0o755);return path

    def prepare(self,extra=None):
        index=len(list(self.root.glob('env-*')))
        output=self.root/f'output-{index}';environment=self.root/f'env-{index}'
        output.touch();environment.touch()
        env={**self.env,'GITHUB_OUTPUT':str(output),'GITHUB_ENV':str(environment),**(extra or {})}
        result=subprocess.run(['/bin/bash','-e','-o','pipefail','-c',PREPARE],env=env,text=True,capture_output=True)
        fields=lambda p:dict(line.split('=',1) for line in p.read_text().splitlines())
        return result,fields(output),fields(environment)

    def install_fixture(self,outputs,env):
        # Model only the action's reviewed rm(dest), mkdir(dest), PATH and
        # PNPM_HOME contract, never run an installer or delete outside tmp.
        dest=Path(outputs['pnpm_dest']);dest.mkdir();(dest/'stale').touch()
        self.assertTrue(dest.is_relative_to(self.temp))
        shutil.rmtree(dest);dest.mkdir()
        binary=dest/'node_modules/.bin';binary.mkdir(parents=True)
        self.tool('pnpm','case "$*" in "--version") echo "${FAKE_PNPM_VERSION-9.15.9}" ;; "store path --silent") echo "${FAKE_PNPM_STORE-$npm_config_store_dir/v3}" ;; *) exit 78 ;; esac',binary)
        return {**self.env,**env,'PNPM_HOME':str(binary),'PATH':str(binary)+os.pathsep+self.env['PATH']}

    def verify(self,env):
        return subprocess.run(['/bin/bash','-e','-o','pipefail','-c',VERIFY],env=env,text=True,capture_output=True)

    def test_shared_installers_only_run_on_hosted_fallback(self):
        for name in ['Setup Node.js','Install pnpm']:
            self.assertIn("if: runner.environment == 'github-hosted'",step(name))
        native=step('Install pnpm in the owned job directory')
        self.assertIn("if: runner.environment == 'self-hosted'",native)
        self.assertIn('uses: pnpm/action-setup@v6',native)
        self.assertIn('version: 9',native)
        self.assertIn('dest: ${{ steps.ui-state.outputs.pnpm_dest }}',native)
        # No empty-output fallback to shared HOME, no cache/prune opt-in.
        self.assertNotIn('||',native)
        self.assertNotIn('cache:',native)
        self.assertNotIn('run_install:',native)
        self.assertNotIn('RUNNER_TOOL_CACHE',UI)
        self.assertNotRegex(PREPARE,r"(?:^|[\s'\"])HOME=")

    def test_two_same_identity_preparations_own_distinct_directories(self):
        a,ao,ae=self.prepare();b,bo,be=self.prepare()
        self.assertEqual(a.returncode,0,a.stderr);self.assertEqual(b.returncode,0,b.stderr)
        self.assertNotEqual(ae['UI_TOOL_ROOT'],be['UI_TOOL_ROOT'])
        for out,env in [(ao,ae),(bo,be)]:
            root=Path(env['UI_TOOL_ROOT']);self.assertEqual(root.parent,self.temp)
            self.assertRegex(root.name,r'^mockforge-ui-123-2\.[A-Za-z0-9]{8}$')
            self.assertEqual(out['pnpm_dest'],str(root/'setup-pnpm'))
            for key in ['npm_config_cache','npm_config_store_dir','XDG_CACHE_HOME','XDG_CONFIG_HOME','XDG_DATA_HOME','XDG_STATE_HOME']:
                self.assertTrue(Path(env[key]).is_relative_to(root),key)
        b_env=self.install_fixture(bo,be);sibling=Path(bo['pnpm_dest'])/'live';sibling.write_text('keep')
        a_env=self.install_fixture(ao,ae)
        self.assertEqual(self.verify(a_env).returncode,0)
        self.assertEqual(self.verify(b_env).returncode,0)
        self.assertEqual(sibling.read_text(),'keep')
        self.assertEqual((self.home/'setup-pnpm/foreign').read_text(),'keep')

    def test_unavailable_or_wrong_node_fails_before_allocating_state(self):
        for value in ['v18.20.0','v22.12.0','']:
            result,out,env=self.prepare({'FAKE_NODE_VERSION':value})
            self.assertNotEqual(result.returncode,0);self.assertEqual(out,{})
            self.assertEqual(env,{});self.assertEqual(list(self.temp.iterdir()),[])
        self.tool('node','exit 1')
        result,out,env=self.prepare();self.assertNotEqual(result.returncode,0)
        self.assertEqual(list(self.temp.iterdir()),[])

    def test_missing_npm_fails_before_allocating_state(self):
        self.tool('npm','exit 1')
        result,out,env=self.prepare();self.assertNotEqual(result.returncode,0)
        self.assertEqual(out,{});self.assertEqual(env,{})
        self.assertEqual(list(self.temp.iterdir()),[])

    def test_invalid_run_metadata_fails_before_allocating_state(self):
        for key in ['GITHUB_RUN_ID','GITHUB_RUN_ATTEMPT']:
            result,out,env=self.prepare({key:'../foreign'})
            self.assertNotEqual(result.returncode,0);self.assertEqual(out,{})
            self.assertEqual(list(self.temp.iterdir()),[])

    def test_foreign_cache_store_and_wrong_pnpm_fail_before_dependencies(self):
        result,out,env=self.prepare();self.assertEqual(result.returncode,0)
        installed=self.install_fixture(out,env)
        for override in [{'FAKE_NPM_CACHE':str(self.home/'npm-cache')},
                         {'FAKE_PNPM_STORE':str(self.home/'pnpm-store')},
                         {'FAKE_PNPM_VERSION':'10.15.0'},
                         {'PNPM_HOME':str(self.home/'setup-pnpm')}]:
            self.assertNotEqual(self.verify({**installed,**override}).returncode,0)
        self.assertEqual((self.home/'setup-pnpm/foreign').read_text(),'keep')

    def test_all_existing_dependency_and_test_commands_remain(self):
        commands=['pnpm install --frozen-lockfile','pnpm type-check','pnpm lint','pnpm build','pnpm test']
        actual=re.findall(r'^        run: (.+)$',UI,flags=re.M)
        self.assertEqual([x for x in actual if x!='|'],commands)
        self.assertIn('uses: codecov/codecov-action@v7',UI)
        self.assertIn("node-version: '20'",UI)

if __name__=='__main__':unittest.main()
