#!/usr/bin/env python3
"""Metadata-only evidence regressions; no Docker daemon or network required."""
import contextlib
from datetime import datetime, timedelta, timezone
import importlib.util
import io
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

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('evidence', ROOT / 'scripts/ci-fixture-evidence.py')
evidence = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(evidence)


def observed_process_state(pid):
    try:
        return Path(f'/proc/{pid}/stat').read_text().rsplit(') ',1)[1].split()[0]
    except (FileNotFoundError, ProcessLookupError):
        return None


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        evidence.STOPPING = False
        self.args = SimpleNamespace(project='mf-e2e-owned', compose='docker-compose.e2e.yml',
                                    run_id='101', output=self.root / 'evidence.jsonl',
                                    port=43210, receipt=self.root / 'server.json', interval=5, max_seconds=2700,
                                    db_cid='a'*64, minio_cid='b'*64)
        self.args.receipt.write_text(json.dumps({'pid':os.getpid(), 'start_time':evidence.owned.start_time(os.getpid()),
                                               'context':evidence.owned.process_context()}))
        self.calls = []

    def tearDown(self):
        evidence.STOPPING = False
        self.tmp.cleanup()

    def docker(self, *args):
        self.calls.append(args)
        if args[0] == 'info': return 'daemon-12345678'
        if args[:2] == ('container','ls'):
            self.assertEqual(args,('container','ls','--all','--filter','label=com.docker.compose.project='+self.args.project,'--format','{{.ID}}'))
            return ''
        if args[0] == 'inspect':
            ports={port:[{'HostIp':'127.0.0.1','HostPort':host}] for port,host in
                   [('5432/tcp','43000'),('9000/tcp','43001'),('9001/tcp','43002')]}
            return json.dumps({'project':self.args.project, 'healthy':'healthy', 'running':True,
                               'image':'sha256:'+'c'*64,'ports':ports})
        if args[:2] == ('image', 'inspect'):
            return json.dumps(['postgres@sha256:'+'d'*64])
        self.fail(f'unexpected Docker operation {args}')

    def sample(self, phase='started'):
        response = SimpleNamespace(status=200)
        with patch.object(evidence, 'docker', side_effect=self.docker), patch.object(evidence.LOCAL_HTTP, 'open', return_value=contextlib.nullcontext(response)) as request:
            evidence.sample(self.args, phase)
        request.assert_called_once_with('http://127.0.0.1:43210/health', timeout=2)

    def test_narrow_metadata_records_owned_images_without_secrets(self):
        self.sample()
        with patch.object(evidence, 'docker', side_effect=self.docker): evidence.cleaned(self.args)
        data = evidence.events(self.args.output)
        self.assertEqual(data[0]['services']['db']['image_id'], 'sha256:'+'c'*64)
        self.assertEqual(data[-1]['phase'], 'cleaned')
        self.assertEqual(data[0]['ports'], {'postgres':'127.0.0.1:43000','minio':'127.0.0.1:43001','minio_console':'127.0.0.1:43002','registry':'127.0.0.1:43210'})
        content = self.args.output.read_text()
        for forbidden in ['DATABASE_URL', 'S3_ENDPOINT', 'password', 'JWT_SECRET', 'Config.Env']:
            self.assertNotIn(forbidden, content)
        self.assertTrue(all(call[0] in {'info','container','inspect','image'} for call in self.calls))
        self.assertFalse(any(call[0] == 'ps' for call in self.calls))

    def test_verified_log_export_allows_only_nonsecret_receipt_fields(self):
        self.sample('started')
        self.sample('healthy')
        with patch.object(evidence,'docker',side_effect=self.docker): evidence.cleaned(self.args)
        data=evidence.verify(self.args.output,self.args.project)
        # Accidental future fields must not cross into ordinary job logs.
        for item in data:
            item['environment']={'DATABASE_URL':'must-not-be-logged'}
            for service in item.get('services',{}).values(): service['secret']='must-not-be-logged'
        with contextlib.redirect_stdout(io.StringIO()) as output: evidence.emit_verified(data)
        lines=output.getvalue().splitlines()
        self.assertEqual(len(lines),3)
        self.assertTrue(all(line.startswith('FIXTURE_EVIDENCE ') for line in lines))
        self.assertNotIn('must-not-be-logged',output.getvalue())
        self.assertEqual(json.loads(lines[0].split(' ',1)[1])['ports']['registry'],'127.0.0.1:43210')
        # A failed validation cannot reach the export at the CLI call site.
        source=(ROOT/'scripts/ci-fixture-evidence.py').read_text()
        self.assertIn('emit_verified(verify(args.output, args.project))',source)

    def test_foreign_project_rejected_before_image_read(self):
        original = self.docker
        def foreign(*args):
            value = original(*args)
            if args[0] == 'inspect':
                state=json.loads(value); state['project']='someone-else'; return json.dumps(state)
            return value
        with patch.object(evidence, 'docker', side_effect=foreign):
            with self.assertRaisesRegex(ValueError, 'foreign'): evidence.sample(self.args)
        self.assertFalse(any(call[:2] == ('image','inspect') for call in self.calls))
        self.assertFalse(self.args.output.exists())

    def test_started_file_is_exclusive_and_failure_cannot_overwrite(self):
        self.sample()
        original = self.args.output.read_text()
        with self.assertRaises(FileExistsError): self.sample()
        self.assertEqual(self.args.output.read_text(), original)

    def test_nonloopback_endpoint_fails(self):
        original=self.docker
        def nonloopback(*args):
            value=original(*args)
            if args[0]=='inspect':
                state=json.loads(value); state['ports']['5432/tcp'][0]['HostIp']='0.0.0.0'; return json.dumps(state)
            return value
        with patch.object(evidence,'docker',side_effect=nonloopback):
            with self.assertRaisesRegex(ValueError,'loopback'): evidence.sample(self.args)

    def test_server_receipt_identity_mismatch_fails(self):
        value=json.loads(self.args.receipt.read_text()); value['start_time']+=1
        self.args.receipt.write_text(json.dumps(value))
        with patch.object(evidence,'docker',side_effect=self.docker):
            with self.assertRaisesRegex(ValueError,'ownership'): evidence.sample(self.args)

    def test_cleanup_only_queries_exact_project_and_fails_if_nonempty(self):
        with patch.object(evidence,'docker',return_value='container-still-present'):
            with self.assertRaisesRegex(RuntimeError,'still has containers'): evidence.cleaned(self.args)
        self.assertEqual(evidence.events(self.args.output)[0]['phase'],'cleanup_error')

    def test_monitor_bounds_reject_nan_and_excessive_or_zero_duration(self):
        for key, value in [('interval',float('nan')),('interval',0),('interval',31),('max_seconds',2701),('max_seconds',0)]:
            args=SimpleNamespace(**vars(self.args)); setattr(args,key,value)
            with self.assertRaises(ValueError): evidence.monitor(args)

    def test_monitor_stop_is_prompt_and_does_not_record_failure(self):
        def stop(args):
            evidence.STOPPING=True
            raise evidence.Stopping()
        with patch.object(evidence,'sample',side_effect=stop), patch.object(evidence.signal,'signal'):
            evidence.monitor(self.args)
        self.assertFalse(self.args.output.exists())

    def test_monitor_read_failure_is_fail_closed(self):
        with patch.object(evidence,'sample',side_effect=RuntimeError('sensitive value not logged')), patch.object(evidence.signal,'signal'):
            with self.assertRaises(RuntimeError): evidence.monitor(self.args)
        data=evidence.events(self.args.output)
        self.assertEqual(data[0]['error'],'RuntimeError')
        self.assertNotIn('sensitive', self.args.output.read_text())

    def test_http_redirects_are_refused(self):
        with self.assertRaisesRegex(RuntimeError,'redirects'):
            evidence.NoRedirect().redirect_request(None,None,302,'',{},'https://other.example')

    def write_events(self, filename, project, run_id, times, daemon='daemon-12345678'):
        start=datetime(2026,10,11,tzinfo=timezone.utc)
        data=[]
        for index, moment in enumerate(times):
            phase='started' if index==0 else 'cleaned' if index==len(times)-1 else 'healthy'
            item={'project':project,'run_id':run_id,'compose':'docker-compose.e2e.yml',
                  'at':(start+timedelta(seconds=moment)).isoformat(),'phase':phase,'daemon':daemon}
            if phase!='cleaned':
                item['services']={'db':{'container':project+'db'},'minio':{'container':project+'minio'}}
                item['ports']={'registry':'127.0.0.1:'+('43000' if project=='project-a' else '43001')}
            data.append(item)
        path=self.root / filename; path.write_text(''.join(json.dumps(item)+'\n' for item in data)); return path

    def test_actual_cross_run_same_daemon_interval_is_required(self):
        first=self.write_events('a','project-a','101',[0,5,10])
        peer=self.write_events('b','project-b','102',[0,8,17,20])
        with contextlib.redirect_stdout(io.StringIO()) as result: evidence.compare([first,peer])
        self.assertEqual(json.loads(result.getvalue())['status'],'overlap_proven')
        for times, daemon, run_id in [([11,16,20],'daemon-12345678','102'),([0,8,17,20],'other-daemon','102'),([0,8,17,20],'daemon-12345678','101'),([0,8,45,50],'daemon-12345678','102')]:
            peer=self.write_events('b','project-b',run_id,times,daemon)
            with self.assertRaisesRegex(RuntimeError,'no same-daemon'): evidence.compare([first,peer])

    def test_incomplete_or_mixed_evidence_rejected(self):
        path=self.write_events('a','project-a','101',[0,5,10])
        data=evidence.events(path)
        for change in ['incomplete','daemon','identity','unordered','error']:
            altered=json.loads(json.dumps(data))
            if change=='incomplete': altered.pop()
            elif change=='daemon': altered[-1]['daemon']='different-daemon'
            elif change=='identity': altered[1]['services']['db']['container']='other-container'
            elif change=='unordered': altered[1]['at']=altered[-1]['at']; altered[-1]['at']=data[0]['at']
            else: altered[1]['phase']='health_error'
            path.write_text(''.join(json.dumps(item)+'\n' for item in altered))
            with self.assertRaises((ValueError,RuntimeError)): evidence.verify(path,'project-a')

    def run_metadata_descendant_case(self, kill_observer, client_ignores_term=True):
        docker=self.root/'docker'; child_receipt=self.root/'child.pid'; plugin_receipt=self.root/'plugin.pid'
        plugin=self.root/'plugin.py'
        plugin.write_text('import os,signal,time\nfrom pathlib import Path\nsignal.signal(signal.SIGTERM,signal.SIG_IGN)\nPath(os.environ["PLUGIN_PID"]).write_text(str(os.getpid()))\ntime.sleep(10)\n')
        docker.write_text('#!/usr/bin/env python3\nimport os,signal,subprocess,sys,time\nfrom pathlib import Path\nsignal.signal(signal.SIGTERM,signal.SIG_IGN)\nPath(os.environ["CHILD_PID"]).write_text(str(os.getpid()))\nsubprocess.Popen([sys.executable,os.environ["PLUGIN_PATH"]])\ntime.sleep(10)\n')
        if not client_ignores_term:
            docker.write_text(docker.read_text().replace('signal.signal(signal.SIGTERM,signal.SIG_IGN)', 'signal.signal(signal.SIGTERM,signal.SIG_DFL)'))
        docker.chmod(0o755)
        program='import importlib.util; s=importlib.util.spec_from_file_location("e",'+repr(str(ROOT/'scripts/ci-fixture-evidence.py'))+'); e=importlib.util.module_from_spec(s); s.loader.exec_module(e); e.docker("info")'
        parent=subprocess.Popen([sys.executable,'-c',program],env={**os.environ,'PATH':str(self.root)+os.pathsep+os.environ['PATH'],'CHILD_PID':str(child_receipt),'PLUGIN_PID':str(plugin_receipt),'PLUGIN_PATH':str(plugin)},stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        sibling=subprocess.Popen([sys.executable,'-c','import time;time.sleep(10)'])
        try:
            deadline=time.monotonic()+3
            while not plugin_receipt.exists() and time.monotonic()<deadline: time.sleep(.02)
            self.assertTrue(plugin_receipt.exists())
            if kill_observer: parent.kill()
            self.assertNotEqual(parent.wait(timeout=8),0)
            for receipt in [child_receipt,plugin_receipt]:
                pid=int(receipt.read_text())
                deadline=time.monotonic()+3
                while time.monotonic()<deadline:
                    state=observed_process_state(pid)
                    if state in (None,'Z'): break
                    time.sleep(.02)
                else: self.fail('Docker metadata client/plugin survived owned supervisor cleanup')
            self.assertIsNone(sibling.poll())
        finally:
            if parent.poll() is None: parent.kill(); parent.wait(timeout=3)
            sibling.terminate(); sibling.wait(timeout=3)

    def test_docker_metadata_client_and_plugin_die_when_observer_is_killed(self):
        self.run_metadata_descendant_case(kill_observer=True)

    def test_process_observation_accepts_only_disappearance_errors(self):
        for error in [FileNotFoundError(),ProcessLookupError()]:
            with patch.object(Path,'read_text',side_effect=error):
                self.assertIsNone(observed_process_state(123))
        for state in ['S','R','Z']:
            with patch.object(Path,'read_text',return_value=f'123 (owned fixture) {state} 1'):
                self.assertEqual(observed_process_state(123),state)
        for error in [PermissionError(),OSError('unrelated failure')]:
            with patch.object(Path,'read_text',side_effect=error):
                with self.assertRaises(type(error)):observed_process_state(123)

    def test_docker_metadata_client_and_plugin_are_bounded_on_timeout(self):
        self.run_metadata_descendant_case(kill_observer=False)

    def test_parent_death_kills_plugin_even_when_client_would_exit_on_term(self):
        self.run_metadata_descendant_case(kill_observer=True, client_ignores_term=False)

    def test_deadline_kills_plugin_even_when_client_would_exit_on_term(self):
        self.run_metadata_descendant_case(kill_observer=False, client_ignores_term=False)


class ProbeMissTests(unittest.TestCase):
    """A stalled /health probe on a busy runner is retried, not fatal (main run 38780287954)."""

    def setUp(self):
        self.base = EvidenceTests('setUp')
        self.base.setUp()
        self.args = self.base.args

    def tearDown(self):
        self.base.tearDown()

    def miss(self):
        error = evidence.ProbeUnavailable('daemon-12345678')
        error.__cause__ = TimeoutError()
        return error

    def run_monitor(self, outcomes):
        clock = iter(range(0, 10**6, 10))
        def fake(args):
            outcome = outcomes.pop(0)
            if not outcomes:
                evidence.STOPPING = True
            if outcome:
                raise outcome
        with patch.object(evidence, 'sample', side_effect=fake), patch.object(evidence.signal, 'signal'), \
                patch.object(evidence.time, 'monotonic', side_effect=lambda: next(clock)):
            evidence.monitor(self.args)

    def test_probe_timeout_is_retryable_but_http_status_is_not(self):
        with patch.object(evidence, 'docker', side_effect=self.base.docker), \
                patch.object(evidence.LOCAL_HTTP, 'open', side_effect=TimeoutError()):
            with self.assertRaises(evidence.ProbeUnavailable) as caught:
                evidence.sample(self.args, 'healthy')
        self.assertEqual(caught.exception.daemon, 'daemon-12345678')
        unavailable = evidence.urllib.error.HTTPError('http://127.0.0.1:43210/health', 503, 'unavailable', {}, None)
        with patch.object(evidence, 'docker', side_effect=self.base.docker), \
                patch.object(evidence.LOCAL_HTTP, 'open', side_effect=unavailable):
            with self.assertRaises(evidence.urllib.error.HTTPError):
                evidence.sample(self.args, 'healthy')

    def test_isolated_probe_misses_are_recorded_as_retries(self):
        self.run_monitor([self.miss(), None, self.miss(), self.miss(), None])
        rows = [(row['phase'], row['consecutive'], row['daemon']) for row in evidence.events(self.args.output)]
        self.assertEqual(rows, [('health_retry', 1, 'daemon-12345678'),
                                ('health_retry', 1, 'daemon-12345678'),
                                ('health_retry', 2, 'daemon-12345678')])

    def test_consecutive_probe_misses_still_fail_closed(self):
        with self.assertRaises(evidence.ProbeUnavailable):
            self.run_monitor([self.miss(), self.miss(), self.miss(), None])
        last = evidence.events(self.args.output)[-1]
        self.assertEqual((last['phase'], last['consecutive'], last['error']), ('health_error', 3, 'TimeoutError'))

    def test_evidence_with_a_retry_row_verifies(self):
        self.base.sample('started')
        self.base.sample('healthy')
        evidence.emit(self.args, 'health_retry', daemon='daemon-12345678', error='TimeoutError', consecutive=1)
        self.base.sample('healthy')
        with patch.object(evidence, 'docker', side_effect=self.base.docker):
            evidence.cleaned(self.args)
        data = evidence.verify(self.args.output, self.args.project)
        self.assertIn('health_retry', [row['phase'] for row in data])
        with contextlib.redirect_stdout(io.StringIO()) as output:
            evidence.emit_verified(data)
        self.assertEqual(len(output.getvalue().splitlines()), 5)


if __name__=='__main__': unittest.main()
