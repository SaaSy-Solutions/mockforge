#!/usr/bin/env python3
"""Source contracts and fake-Docker orchestration tests; no Docker daemon used."""

from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import tempfile
import sys
import time
import unittest


ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = (ROOT / ".github/workflows/registry-e2e.yml").read_text()
RLS = (ROOT / "scripts/rls-e2e-gate.sh").read_text()


def step_script(name):
    step = WORKFLOW.split(f"      - name: {name}\n", 1)[1].split("\n      - name:", 1)[0]
    script = step.split("        run: |\n", 1)[1]
    return "\n".join(line[10:] if line.startswith("          ") else line for line in script.splitlines())


class FixtureIsolationTests(unittest.TestCase):
    def test_compose_never_uses_global_container_names(self):
        for filename in ["docker-compose.e2e.yml", "docker-compose.rls-gate.yml"]:
            text = (ROOT / filename).read_text()
            self.assertNotRegex(text, r"(?m)^\s*container_name:")
            self.assertEqual(len(re.findall(r'"127\.0\.0\.1:\$\{(?:PG|MINIO|MINIO_CONSOLE)_PORT', text)), 3)

    def test_cleanup_never_selects_processes_or_containers_globally(self):
        for text in [WORKFLOW, RLS]:
            self.assertNotRegex(text, r"(?m)^\s*(?:fuser|pkill|killall)\b")
            self.assertNotIn('docker ps -aq --filter "name=', text)
            self.assertNotRegex(text, r"docker\s+(?:system|container|volume|network)\s+prune")
        self.assertIn('ci-owned-process.py stop "$FIXTURE_ROOT/server.json"', WORKFLOW)
        self.assertIn('ci-owned-process.py" stop "$SERVER_PID_FILE"', RLS)
        for text in [WORKFLOW, RLS]:
            self.assertNotIn('timeout --foreground', text)
            self.assertIn('timeout --signal=KILL 60s docker compose', text)

    def test_timeout_bounds_only_its_owned_client_and_plugin_group(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory=Path(tmp)
            plugin=directory/'plugin.py'
            plugin.write_text("import os,signal,time\nfrom pathlib import Path\nsignal.signal(signal.SIGTERM,signal.SIG_IGN)\nPath("+repr(str(directory/'plugin.pid'))+").write_text(str(os.getpid()))\ntime.sleep(5)\n")
            client=directory/'client.py'
            client.write_text("import os,signal,subprocess,sys,time\nfrom pathlib import Path\nPath("+repr(str(directory/'client.pid'))+").write_text(str(os.getpid()))\nsubprocess.Popen([sys.executable,"+repr(str(plugin))+"])\ntime.sleep(5)\n")
            sibling=subprocess.Popen([sys.executable,'-c','import time;time.sleep(5)'])
            try:
                result=subprocess.run(['timeout','--signal=KILL','.4s',sys.executable,str(client)],timeout=3,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
                self.assertIn(result.returncode,(124,137,-9))
                self.assertIsNone(sibling.poll())
                for name in ['client.pid','plugin.pid']:
                    pid=int((directory/name).read_text())
                    deadline=time.monotonic()+1
                    while time.monotonic()<deadline:
                        try: state=Path(f'/proc/{pid}/stat').read_text().rsplit(') ',1)[1].split()[0]
                        except FileNotFoundError: break
                        if state=='Z': break
                        time.sleep(.01)
                    else: self.fail(f'{name} survived its owned timeout process group')
            finally:
                sibling.terminate();sibling.wait(timeout=3)

    def test_rls_uses_atomic_docker_ports_and_cleanup_on_failure(self):
        for variable in ["PG_PORT", "MINIO_PORT", "MINIO_CONSOLE_PORT"]:
            self.assertIn(f'{variable}="${{{variable}:-0}}"', RLS)
        self.assertIn("trap cleanup_on_exit EXIT", RLS)
        self.assertIn("trap 'exit 130' INT", RLS)
        self.assertIn("trap 'exit 143' TERM", RLS)
        self.assertIn('docker compose -f "$COMPOSE_FILE_GATE" port db 5432', RLS)
        self.assertIn('export DATABASE_URL="$OWNER_URL"', RLS)

    def test_both_lanes_retain_registry_e2e_targets(self):
        # The Hetzner split kept these thirteen on both lanes. Main later added
        # hosted_capture_workspaces_e2e (#1210) and diff_finding_incident_e2e
        # (#1207) to the base lane only; the RLS array was not extended.
        original = {
            "workspace_content_e2e", "signup_flow_e2e", "marketplace_e2e",
            "cloud_verification_e2e", "cloud_ai_contract_diff_e2e", "cloud_conformance_e2e",
            "paid_flow_e2e", "hosted_mock_overrides_e2e", "audit_integrity_e2e",
            "rls_coverage_e2e", "hosted_mock_rls_routes_e2e", "rls_join_coverage_e2e",
            "cross_tenant_chaos_capture_e2e",
        }
        base = original.union({"hosted_capture_workspaces_e2e", "diff_finding_incident_e2e"})
        self.assertEqual(set(re.findall(r"--test ([a-z0-9_]+)", step_script("Run E2E tests"))), base)
        array = RLS.split("E2E_TESTS=(", 1)[1].split(")", 1)[0]
        self.assertEqual(set(re.findall(r"--test ([a-z0-9_]+)", array)), original)
        self.assertIn("workers::contract_probe::postgres_tests::due_claims_serialize_and_failed_timestamp_rolls_back_run", WORKFLOW)
        self.assertIn("-- --ignored --nocapture --test-threads=1", WORKFLOW)
        self.assertIn("--no-fail-fast", RLS)

    def test_cleanup_without_initialized_project_is_noop(self):
        with tempfile.TemporaryDirectory() as tmp:
            script = Path(tmp) / "docker"
            script.write_text("#!/bin/sh\nexit 77\n")
            script.chmod(0o755)
            env = {**os.environ, "PATH": tmp + os.pathsep + os.environ["PATH"]}
            env.pop("COMPOSE_PROJECT_NAME", None)
            result = subprocess.run(["bash", "-euo", "pipefail", "-c", step_script("Tear down Postgres + MinIO")], env=env)
            self.assertEqual(result.returncode, 0)

    def test_two_simulated_jobs_keep_distinct_projects_and_observed_ports(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            docker = directory / "docker"
            docker.write_text('''#!/usr/bin/env python3
import fcntl,json,os,sys
from pathlib import Path
root=Path(os.environ['FAKE_DOCKER_ROOT'])
project=os.environ['COMPOSE_PROJECT_NAME']
with (root/'lock').open('a') as lock:
 fcntl.flock(lock,fcntl.LOCK_EX)
 state_path=root/'state.json'
 state=json.loads(state_path.read_text()) if state_path.exists() else {}
 if project not in state: state[project]=41000+10*len(state)
 state_path.write_text(json.dumps(state))
 with (root/'calls.jsonl').open('a') as stream: stream.write(json.dumps({'project':project,'args':sys.argv[1:]})+'\\n')
 port=state[project]
args=sys.argv[1:]
if args[0]=='inspect': print('healthy')
elif 'ps' in args: print(project+'-'+args[-1])
elif 'port' in args: print('127.0.0.1:'+str(port+(0 if 'db' in args else 1)))
''')
            docker.chmod(0o755)

            def run_job(index):
                env_file = directory / f"job-{index}.env"
                env = {**os.environ, "PATH": tmp + os.pathsep + os.environ["PATH"],
                       "FAKE_DOCKER_ROOT": tmp, "RUNNER_TEMP": tmp, "GITHUB_ENV": str(env_file),
                       "GITHUB_RUN_ID": str(index), "GITHUB_RUN_ATTEMPT": "1"}
                env.pop("COMPOSE_PROJECT_NAME", None)
                subprocess.run(["bash", "-euo", "pipefail", "-c", step_script("Allocate run-owned fixture state")], env=env, check=True)
                env.update(dict(line.split("=", 1) for line in env_file.read_text().splitlines()))
                self.assertEqual(env["PG_PORT"], "0")
                subprocess.run(["bash", "-euo", "pipefail", "-c", step_script("Start Postgres + MinIO")], env=env, check=True)
                env.update(dict(line.split("=", 1) for line in env_file.read_text().splitlines()))
                subprocess.run(["bash", "-euo", "pipefail", "-c", step_script("Tear down Postgres + MinIO")], env=env, check=True)
                return env

            with ThreadPoolExecutor(max_workers=2) as pool:
                first, second = list(pool.map(run_job, [101, 102]))
            self.assertNotEqual(first["COMPOSE_PROJECT_NAME"], second["COMPOSE_PROJECT_NAME"])
            self.assertNotEqual(first["FIXTURE_ROOT"], second["FIXTURE_ROOT"])
            self.assertNotEqual(first["DATABASE_URL"], second["DATABASE_URL"])
            self.assertNotEqual(first["S3_ENDPOINT"], second["S3_ENDPOINT"])
            calls = [json.loads(line) for line in (directory / "calls.jsonl").read_text().splitlines()]
            self.assertEqual({call["project"] for call in calls}, {first["COMPOSE_PROJECT_NAME"], second["COMPOSE_PROJECT_NAME"]})
            for project in {call["project"] for call in calls}:
                own = [call["args"] for call in calls if call["project"] == project]
                self.assertEqual(sum("down" in args for args in own), 1)
                self.assertEqual(sum("up" in args for args in own), 1)

    def test_rls_setup_failure_cleans_only_its_project_and_preserves_failure(self):
        self.run_cleanup_failure_case()

    def test_standalone_up_failure_uses_owned_cleanup(self):
        self.run_cleanup_failure_case(command="up")

    def test_standalone_up_cleanup_failure_retains_locator_and_primary_status(self):
        self.run_cleanup_failure_case(command="up", docker_failure=True)

    def test_rls_process_cleanup_failure_still_tears_down_owned_docker_project(self):
        self.run_cleanup_failure_case(bad_receipt=True)

    def test_rls_docker_cleanup_failure_retains_project_locator(self):
        self.run_cleanup_failure_case(docker_failure=True)

    def run_cleanup_failure_case(self, bad_receipt=False, docker_failure=False, command="all"):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            docker = directory / "docker"
            docker.write_text('''#!/usr/bin/env python3
import json,os,sys
from pathlib import Path
with Path(os.environ['DOCKER_CALLS']).open('a') as stream:
 stream.write(json.dumps({'project':os.environ['COMPOSE_PROJECT_NAME'],'args':sys.argv[1:]})+'\\n')
args=sys.argv[1:]
if 'down' in args and os.environ.get('FAIL_DOWN')=='true': sys.exit(23)
if args[0]=='inspect': print('healthy')
elif 'ps' in args: print('owned-'+args[-1])
elif 'port' in args: print('127.0.0.1:'+{'5432':'42000','9000':'42001','9001':'42002'}[args[-1]])
''')
            docker.chmod(0o755)
            for tool in ["cargo", "curl", "psql"]:
                path = directory / tool
                path.write_text("#!/bin/sh\nexit " + ("19" if tool == "cargo" else "0") + "\n")
                path.chmod(0o755)
            state = directory / "state"
            calls_path = directory / "calls.jsonl"
            env = {**os.environ, "PATH": tmp + os.pathsep + os.environ["PATH"],
                   "STATE_FILE": str(state), "SERVER_PID_FILE": str(directory / "server.json"),
                   "COMPOSE_PROJECT_NAME": "rlsgate-owned-test", "DOCKER_CALLS": str(calls_path)}
            if bad_receipt:
                (directory / "server.json").write_text("invalid receipt")
            if docker_failure:
                env["FAIL_DOWN"] = "true"
            result = subprocess.run(["bash", str(ROOT / "scripts/rls-e2e-gate.sh"), command], env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 19, result.stdout + result.stderr)
            self.assertEqual(state.exists(), bad_receipt or docker_failure)
            calls = [json.loads(line) for line in calls_path.read_text().splitlines()]
            self.assertTrue(calls)
            self.assertEqual({call["project"] for call in calls}, {"rlsgate-owned-test"})
            self.assertEqual(sum("down" in call["args"] for call in calls), 1)
            self.assertEqual(sum("up" in call["args"] for call in calls), 1)

    def test_partial_second_setup_and_repeated_term_preserve_live_sibling(self):
        # Real shell signals and an unrelated real live process; Docker is a
        # deterministic stub. The native two-fixture test supplies real Docker.
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            first_state = directory / 'first.state'
            first_state.write_text('COMPOSE_PROJECT_NAME="rlsgate-first"\n')
            original_state = first_state.read_bytes()
            second_state = directory / 'second.state'
            calls = directory / 'calls.jsonl'
            docker = directory / 'docker'
            docker.write_text('''#!/usr/bin/env python3
import json,os,sys,time
from pathlib import Path
root=Path(os.environ['TEST_ROOT']);args=sys.argv[1:]
with (root/'calls.jsonl').open('a') as f:f.write(json.dumps({'project':os.environ['COMPOSE_PROJECT_NAME'],'args':args})+'\\n')
if 'down' in args:
 (root/'cleaning').touch()
 deadline=time.monotonic()+3
 while not (root/'release').exists() and time.monotonic()<deadline:time.sleep(.01)
elif args[0]=='inspect':print('healthy')
elif 'ps' in args:print('second-'+args[-1])
elif 'port' in args:print('127.0.0.1:'+{'5432':'42000','9000':'42001','9001':'42002'}[args[-1]])
''')
            docker.chmod(0o755)
            cargo = directory / 'cargo'
            cargo.write_text('#!/bin/sh\ntouch "$TEST_ROOT/building"\nsleep .2\nexit 19\n')
            cargo.chmod(0o755)
            for tool in ['curl', 'psql']:
                path=directory/tool;path.write_text('#!/bin/sh\nexit 0\n');path.chmod(0o755)
            sibling=subprocess.Popen([sys.executable,'-c','import time;time.sleep(10)'])
            env={**os.environ,'PATH':tmp+os.pathsep+os.environ['PATH'], 'TEST_ROOT':tmp,
                 'STATE_FILE':str(second_state),'SERVER_PID_FILE':str(directory/'second.json'),
                 'COMPOSE_PROJECT_NAME':'rlsgate-second'}
            child=subprocess.Popen(['bash',str(ROOT/'scripts/rls-e2e-gate.sh'),'up'],env=env,
                                   stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
            def wait_marker(name):
                deadline=time.monotonic()+4
                while not (directory/name).exists() and time.monotonic()<deadline:
                    self.assertIsNone(child.poll());time.sleep(.01)
                self.assertTrue((directory/name).exists(),name)
            try:
                wait_marker('building')
                child.send_signal(signal.SIGTERM)
                wait_marker('cleaning')
                child.send_signal(signal.SIGTERM)
                (directory/'release').touch()
                out,err=child.communicate(timeout=5)
                self.assertEqual(child.returncode,143,out+err)
                self.assertIsNone(sibling.poll())
                self.assertEqual(first_state.read_bytes(),original_state)
                self.assertFalse(second_state.exists())
                operations=[json.loads(line) for line in calls.read_text().splitlines()]
                self.assertEqual({op['project'] for op in operations},{'rlsgate-second'})
                self.assertEqual(sum('down' in op['args'] for op in operations),1)
            finally:
                if child.poll() is None:child.kill();child.communicate(timeout=5)
                sibling.terminate();sibling.wait(timeout=3)

    def test_successful_standalone_up_disarms_cleanup_and_keeps_state(self):
        # Focus on the transaction wrapper, retaining production state claiming
        # and cleanup code; fixture bootstrap itself is covered by native CI.
        prefix=RLS.rsplit('case "${1:-all}" in',1)[0]
        suffix=RLS.rsplit('case "${1:-all}" in',1)[1]
        with tempfile.TemporaryDirectory() as tmp:
            directory=Path(tmp);state=directory/'state';calls=directory/'cleanup'
            script=directory/'transaction.sh'
            script.write_text(prefix+'\ncmd_up() { write_state new; }\ncmd_down() { touch "$CLEANUP_MARKER"; }\ncase "${1:-all}" in'+suffix)
            env={**os.environ,'STATE_FILE':str(state),'COMPOSE_PROJECT_NAME':'rlsgate-success',
                 'CLEANUP_MARKER':str(calls),'REGISTRY_PORT':'41000'}
            result=subprocess.run(['bash',str(script),'up'],env=env,text=True,capture_output=True)
            self.assertEqual(result.returncode,0,result.stdout+result.stderr)
            self.assertTrue(state.exists());self.assertFalse(calls.exists())

    def test_rls_stale_project_refusal_never_claims_state_or_cleans_residuals(self):
        for kind in ['container','network','volume','read_failure']:
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as tmp:
                directory=Path(tmp);calls=directory/'calls.jsonl';state=directory/'state'
                docker=directory/'docker'
                docker.write_text("#!/usr/bin/env python3\nimport json,os,sys\nfrom pathlib import Path\na=sys.argv[1:]\nwith Path(os.environ['CALLS']).open('a') as f:f.write(json.dumps(a)+'\\n')\nif a[0]==os.environ['RESIDUAL']:print('prior-run-resource')\nif os.environ['RESIDUAL']=='read_failure':sys.exit(23)\n")
                docker.chmod(0o755)
                for tool in ['cargo','curl','psql']:
                    path=directory/tool;path.write_text('#!/bin/sh\nexit 0\n');path.chmod(0o755)
                env={**os.environ,'PATH':tmp+os.pathsep+os.environ['PATH'],'STATE_FILE':str(state),
                     'COMPOSE_PROJECT_NAME':'rlsgate-prior-job','CALLS':str(calls),'RESIDUAL':kind,
                     'OWNS_GATE_STATE':'true'}
                for command in ['all','up']:
                    result=subprocess.run(['bash',str(ROOT/'scripts/rls-e2e-gate.sh'),command],env=env,text=True,capture_output=True)
                    self.assertEqual(result.returncode,1,result.stdout+result.stderr)
                    self.assertFalse(state.exists())
                operations=[json.loads(line) for line in calls.read_text().splitlines()]
                self.assertTrue(operations)
                self.assertFalse(any('up' in args or 'down' in args for args in operations))
                for args in operations:
                    self.assertIn('label=com.docker.compose.project=rlsgate-prior-job',args)
                    self.assertIn(args[0],['container','network','volume'])

    def test_rls_uuid_generator_failure_never_claims_state_or_calls_docker(self):
        for program, code in [('#!/bin/sh\nexit 29\n',29),('#!/bin/sh\nprintf invalid\n',1)]:
            with self.subTest(code=code), tempfile.TemporaryDirectory() as tmp:
                directory=Path(tmp);state=directory/'state';called=directory/'docker-called'
                python=directory/'python3';python.write_text(program);python.chmod(0o755)
                docker=directory/'docker';docker.write_text('#!/bin/sh\ntouch '+str(called)+'\n');docker.chmod(0o755)
                env={**os.environ,'PATH':tmp+os.pathsep+os.environ['PATH'],'STATE_FILE':str(state)}
                env.pop('COMPOSE_PROJECT_NAME',None)
                result=subprocess.run(['bash',str(ROOT/'scripts/rls-e2e-gate.sh'),'all'],env=env,capture_output=True,text=True)
                self.assertEqual(result.returncode,code,result.stdout+result.stderr)
                self.assertFalse(state.exists());self.assertFalse(called.exists())

    def test_rls_default_project_remains_unique_for_same_run_and_attempt(self):
        # Force the same CI run/attempt and runner identity twice. UUID, rather
        # than process-ID uniqueness, must distinguish the generated projects.
        prefix=RLS.split('# Docker allocates service ports',1)[0]
        with tempfile.TemporaryDirectory() as tmp:
            env={**os.environ,'STATE_FILE':str(Path(tmp)/'state'),'GITHUB_RUN_ID':'101',
                 'GITHUB_RUN_ATTEMPT':'2','RUNNER_NAME':'same-runner'}
            env.pop('COMPOSE_PROJECT_NAME',None)
            script=prefix+'\nprintf "%s\\n" "$COMPOSE_PROJECT_NAME"\n'
            path=Path(tmp)/'identity.sh';path.write_text(script)
            results=[subprocess.check_output(['bash',str(path),'up'],env=env,text=True).strip() for _ in range(2)]
            self.assertNotEqual(results[0],results[1])
            for project in results:self.assertRegex(project,r'^rlsgate-101-2-[a-f0-9]{32}-same-runner$')

    def test_repeated_local_up_refuses_existing_state_without_side_effects(self):
        with tempfile.TemporaryDirectory() as tmp:
            state = Path(tmp) / "state"
            original = 'COMPOSE_PROJECT_NAME="original-live-project"\n'
            state.write_text(original)
            env = {**os.environ, "STATE_FILE": str(state)}
            for command in ["up", "all"]:
                result = subprocess.run(["bash", str(ROOT / "scripts/rls-e2e-gate.sh"), command],
                                        env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode, 1)
                self.assertIn("gate state already exists", result.stderr)
                self.assertEqual(state.read_text(), original)


if __name__ == "__main__":
    unittest.main()
