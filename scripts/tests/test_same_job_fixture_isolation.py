#!/usr/bin/env python3
"""Same-job proof and failure orchestration regressions. No Docker used here."""
import copy
from datetime import datetime, timedelta, timezone
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
from types import SimpleNamespace
import unittest
from unittest.mock import patch

ROOT=Path(__file__).resolve().parents[2]
SPEC=importlib.util.spec_from_file_location('same_job',ROOT/'scripts/ci-same-job-fixture-isolation.py')
same_job=importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(same_job)


class SameJobProofTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.root=Path(self.tmp.name)
        self.paths=[self.root/'first.jsonl',self.root/'second.jsonl']
        self.identities=[{'version':1,'pid':100+i,'start_time':1000+i,
                          'context':same_job.owned.process_context()} for i in range(2)]
        start=datetime(2026,10,11,tzinfo=timezone.utc)
        self.groups=[]
        for index, phases in enumerate([[('started',0),('healthy',1),('cleaned',10)],
                                        [('started',0),('healthy',9),('healthy',16),('cleaned',20)]]):
            project=f'rlsgate-101-2-{str(index)*32}-runner'
            rows=[]
            for phase,offset in phases:
                row={'at':(start+timedelta(seconds=offset)).isoformat(),'phase':phase,
                     'run_id':'101','project':project,'compose':'docker-compose.rls-gate.yml','daemon':'daemon-12345678'}
                if phase!='cleaned':
                    row.update(registry_healthy=True,
                        ports={key:f'127.0.0.1:{41000+index*10+n}' for n,key in enumerate(['postgres','minio','minio_console','registry'])},
                        services={name:{'container':str(index*2+n)*64,'image_id':'sha256:'+'a'*64,
                                        'repo_digests':['postgres@sha256:'+'b'*64]} for n,name in enumerate(['db','minio'])})
                rows.append(row)
            self.groups.append(rows)

    def tearDown(self):self.tmp.cleanup()

    def verify(self):
        for path,rows in zip(self.paths,self.groups):path.write_text(''.join(json.dumps(row)+'\n' for row in rows))
        return same_job.verify_same_job(*self.paths,*self.identities,'101','2')

    def test_same_job_proof_has_narrow_name_and_cross_run_comparator_stays_red(self):
        result=self.verify()
        self.assertEqual(result['status'],'same_job_fixture_isolation_proven')
        with self.assertRaisesRegex(RuntimeError,'no same-daemon cross-run'):
            same_job.evidence.compare(self.paths)

    def test_rejects_different_daemon(self):
        for row in self.groups[1]:row['daemon']='other-daemon'
        with self.assertRaisesRegex(ValueError,'different daemons'):self.verify()

    def test_rejects_forged_run_or_attempt(self):
        for mode in ['run','attempt']:
            with self.subTest(mode=mode):
                original=copy.deepcopy(self.groups)
                for row in self.groups[1]:
                    if mode=='run':row['run_id']='202'
                    else:row['project']=row['project'].replace('-101-2-','-101-3-')
                with self.assertRaisesRegex(ValueError,'actual run/attempt'):self.verify()
                self.groups=original

    def test_rejects_shared_process_or_wrong_context(self):
        original=copy.deepcopy(self.identities)
        self.identities[1]=copy.deepcopy(self.identities[0])
        with self.assertRaisesRegex(ValueError,'different real processes'):self.verify()
        self.identities=original
        self.identities[1]['context']['boot_id']='different-boot'
        with self.assertRaisesRegex(ValueError,'same process context'):self.verify()

    def test_rejects_invalid_kernel_receipt(self):
        self.identities[0]['pid']=1
        with self.assertRaisesRegex(ValueError,'kernel identity'):self.verify()

    def test_rejects_shared_endpoint_and_container(self):
        for field in ['ports','services']:
            with self.subTest(field=field):
                original=copy.deepcopy(self.groups)
                for row in self.groups[1]:
                    if row['phase']!='cleaned':row[field]=self.groups[0][0][field]
                with self.assertRaises(ValueError):self.verify()
                self.groups=original

    def test_rejects_absent_or_too_early_survivor_sample(self):
        for offset in [1,40]:
            with self.subTest(offset=offset):
                original=copy.deepcopy(self.groups)
                self.groups[1][2]['at']=(datetime.fromisoformat(self.groups[0][-1]['at'])+timedelta(seconds=offset)).isoformat()
                self.groups[1][-1]['at']=(datetime.fromisoformat(self.groups[1][2]['at'])+timedelta(seconds=1)).isoformat()
                with self.assertRaisesRegex(RuntimeError,'post-cleanup'):self.verify()
                self.groups=original

    def test_state_parser_rejects_shell_code_and_unknown_keys(self):
        path=self.root/'state'
        for value in ['COMPOSE_PROJECT_NAME="$(touch unwanted)"\n','EVIL="1"\n','PG_PORT="0"\nPG_PORT="1"\n']:
            path.write_text(value)
            with self.assertRaises(ValueError):same_job.read_state(path)

    def test_native_script_does_not_forge_run_ids_or_change_cross_run_source(self):
        source=(ROOT/'scripts/ci-same-job-fixture-isolation.py').read_text()
        self.assertNotIn('killpg',source)
        self.assertNotIn('docker ps',source)
        self.assertNotIn('GITHUB_RUN_ID=',source)
        self.assertIn('"label=com.docker.compose.project=" + self.project',source)


class SameJobOrchestrationTests(unittest.TestCase):
    def test_real_double_term_during_popen_assignment_kills_only_owned_invocation(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);(root/'scripts').mkdir()
            plugin=root/'plugin.py'
            plugin.write_text("import os,signal,time\nfrom pathlib import Path\nsignal.signal(signal.SIGTERM,signal.SIG_IGN)\nPath("+repr(str(root/'plugin.pid'))+").write_text(str(os.getpid()))\ntime.sleep(30)\n")
            gate=root/'scripts/rls-e2e-gate.sh'
            gate.write_text('#!/bin/sh\n'+sys.executable+' '+str(plugin)+' &\nwait\n')
            controller=root/'controller.py'
            controller.write_text('''import importlib.util,os,signal,subprocess,sys,time
from pathlib import Path
spec=importlib.util.spec_from_file_location('gate',sys.argv[1]);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
m.ROOT=Path(sys.argv[2])
real=m.subprocess.Popen
def delayed(*args,**kwargs):
 process=real(*args,**kwargs)
 deadline=time.monotonic()+3
 while not (m.ROOT/'plugin.pid').exists() and time.monotonic()<deadline:time.sleep(.01)
 (m.ROOT/'assignment-window').touch();time.sleep(.25)
 return process
m.subprocess.Popen=delayed
def interrupted(signum,frame):m.CANCELLED=signum
signal.signal(signal.SIGTERM,interrupted)
fixture=m.Fixture(m.ROOT,'owned',dict(os.environ))
try:fixture.command('up')
except InterruptedError:sys.exit(143)
sys.exit(99)
''')
            sibling=subprocess.Popen([sys.executable,'-c','import time;time.sleep(10)'])
            child=subprocess.Popen([sys.executable,str(controller),str(ROOT/'scripts/ci-same-job-fixture-isolation.py'),str(root)],
                                   stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
            try:
                deadline=time.monotonic()+4
                while not (root/'assignment-window').exists() and time.monotonic()<deadline:
                    self.assertIsNone(child.poll());time.sleep(.01)
                self.assertTrue((root/'assignment-window').exists())
                child.send_signal(signal.SIGTERM);time.sleep(.03);child.send_signal(signal.SIGTERM)
                out,err=child.communicate(timeout=4)
                self.assertEqual(child.returncode,143,out+err)
                self.assertIsNone(sibling.poll())
                pid=int((root/'plugin.pid').read_text())
                deadline=time.monotonic()+1
                while time.monotonic()<deadline:
                    try:state=Path(f'/proc/{pid}/stat').read_text().rsplit(') ',1)[1].split()[0]
                    except FileNotFoundError:break
                    if state=='Z':break
                    time.sleep(.01)
                else:self.fail('owned plugin survived the cancelled supervisor')
            finally:
                if child.poll() is None:child.kill();child.communicate(timeout=3)
                sibling.terminate();sibling.wait(timeout=3)

    def test_partial_second_setup_cleans_both_and_preserves_primary_failure(self):
        self.run_failure_case(cancel=False)

    def test_cancellation_and_repeated_term_do_not_interrupt_owned_cleanup(self):
        self.run_failure_case(cancel=True)

    def run_failure_case(self,cancel):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);binary=root/'target/debug/mockforge-registry-server'
            binary.parent.mkdir(parents=True);binary.write_text('already-built');binary.chmod(0o755)
            env={**os.environ,'GITHUB_RUN_ID':'101','GITHUB_RUN_ATTEMPT':'2','RUNNER_NAME':'runner',
                 'CARGO_TARGET_DIR':str(root/'target')}
            calls=[]
            class FakeFixture:
                def __init__(self,output,name,environment):self.name=name
                def up(self):
                    calls.append(('up',self.name))
                    if self.name=='survivor':
                        if cancel:
                            signal.getsignal(signal.SIGTERM)(signal.SIGTERM,None)
                            same_job.check_cancelled()
                        raise same_job.GateFailure('up',19)
                    return SimpleNamespace()
                def down(self):
                    calls.append(('down',self.name))
                    # A second TERM is ignored while all owned cleanups run.
                    if cancel:self.assert_ignored()
                    if self.name=='survivor':raise same_job.GateFailure('down',23)
                def assert_ignored(self):
                    if signal.getsignal(signal.SIGTERM)!=signal.SIG_IGN:
                        raise AssertionError('repeated TERM can interrupt cleanup')
            with patch.object(same_job,'Fixture',FakeFixture),patch.object(same_job.subprocess,'check_output',return_value='timeout (GNU coreutils) 9.5\n'):
                with self.assertRaises(InterruptedError if cancel else same_job.GateFailure) as caught:
                    same_job.run(root/'output',env)
            if not cancel:self.assertEqual(caught.exception.code,19)
            self.assertEqual(calls,[('up','first'),('up','survivor'),('down','survivor'),('down','first')])
            self.assertFalse((root/'output/result.json').exists())


if __name__=='__main__':unittest.main()
