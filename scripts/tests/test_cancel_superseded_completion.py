"""Safety boundaries for cancelling only an obsolete, partially cancelled CI tail."""
import copy
import importlib.util
from pathlib import Path
import unittest
import sys
import io
from contextlib import redirect_stdout
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("cancel_tail", ROOT / "scripts/ci/cancel_superseded_completion.py")
MODULE = importlib.util.module_from_spec(SPEC)
sys.path.insert(0, str(ROOT / "scripts/ci"))
SPEC.loader.exec_module(MODULE)
REPO = "SaaSy-Solutions/mockforge"


def fixture():
    pull = {"number":1348,"state":"open","head":{"sha":"current","ref":"fix/example","repo":{"full_name":REPO}}}
    run = {"id":42,"event":"pull_request","path":".github/workflows/ci.yml","head_branch":"fix/example","head_sha":"old","head_repository":{"full_name":REPO},"pull_requests":[{"number":1348}],"status":"queued"}
    jobs = [{"id":1,"name":"Detect change scope","status":"completed","conclusion":"cancelled"},
            {"id":2,"name":"Test (1.96.0)","status":"queued","runner_name":"","runner_id":0,"runner_group_id":0,"labels":["self-hosted","linux","x64","rust"]}]
    return pull,run,jobs


class CleanupTest(unittest.TestCase):
    def invoke(self):
        return patch.object(sys,"argv",["cancel-tail","--pull-request","1348","--run-id","42","--expected-head","current","--apply"])

    def test_real_stuck_tail_shape_is_eligible(self):
        pull,run,jobs=fixture()
        plan=MODULE.eligible(REPO,pull,run,jobs,"current")
        self.assertEqual(plan["completion_job_id"],2)
        self.assertEqual(plan["old_head"],"old")

    def test_current_head_closed_forks_and_wrong_workflow_are_refused(self):
        mutations=[("pull","state","closed"),("run","head_sha","current"),("run","event","push"),
                   ("run","path",".github/workflows/deploy.yml"),("run","head_branch","other"),
                   ("run","status","completed"),("run","pull_requests",[]),
                   ("run","head_repository",{"full_name":"fork/repo"})]
        for target,key,value in mutations:
            with self.subTest(target=target,key=key):
                pull,run,jobs=fixture();(pull if target=="pull" else run)[key]=value
                with self.assertRaises(MODULE.Refused):MODULE.eligible(REPO,pull,run,jobs,"current")
        pull,run,jobs=fixture();pull["head"]["repo"]["full_name"]="fork/repo"
        with self.assertRaises(MODULE.Refused):MODULE.eligible(REPO,pull,run,jobs,"current")
        pull,run,jobs=fixture()
        with self.assertRaises(MODULE.Refused):MODULE.eligible(REPO,pull,run,jobs,"outdated-plan")

    def test_active_assigned_and_normal_queued_jobs_are_refused(self):
        for key,value in [("status","in_progress"),("name","Rust default checks"),("runner_name","busy-runner"),
                          ("runner_id",1),("runner_group_id",1),("labels",["ubuntu-latest"])]:
            with self.subTest(key=key):
                pull,run,jobs=fixture();jobs[1][key]=value
                with self.assertRaises(MODULE.Refused):MODULE.eligible(REPO,pull,run,jobs,"current")
        pull,run,jobs=fixture();jobs[0]["conclusion"]="success"
        with self.assertRaises(MODULE.Refused):MODULE.eligible(REPO,pull,run,jobs,"current")
        pull,run,jobs=fixture();jobs.append(copy.deepcopy(jobs[1]))
        with self.assertRaises(MODULE.Refused):MODULE.eligible(REPO,pull,run,jobs,"current")

    def test_default_dry_run_does_not_cancel(self):
        pull,run,jobs=fixture();plan=MODULE.eligible(REPO,pull,run,jobs,"current")
        argv=["cancel-tail","--pull-request","1348","--run-id","42","--expected-head","current"]
        with patch.object(sys,"argv",argv), patch.object(MODULE,"snapshot",return_value=plan), patch.object(MODULE,"api") as api, redirect_stdout(io.StringIO()):
            MODULE.main()
        api.assert_not_called()

    def test_normal_cancellation_finishes_without_force(self):
        pull,run,jobs=fixture();plan=MODULE.eligible(REPO,pull,run,jobs,"current")
        with self.invoke(), patch.object(MODULE,"snapshot",return_value=plan) as inspect, patch.object(MODULE,"api",side_effect=[None,{"status":"completed","conclusion":"cancelled"}]) as api, redirect_stdout(io.StringIO()):
            MODULE.main()
        self.assertEqual(inspect.call_count,1)
        self.assertFalse(any("force-cancel" in call.args[0] for call in api.call_args_list))

    def test_unresponsive_cancel_revalidates_before_force(self):
        pull,run,jobs=fixture();plan=MODULE.eligible(REPO,pull,run,jobs,"current")
        with self.invoke(), patch.object(MODULE,"snapshot",return_value=plan) as inspect, patch.object(MODULE,"api",side_effect=[None,*[{"status":"queued"}]*3,None]) as api, patch.object(MODULE.time,"sleep"), redirect_stdout(io.StringIO()):
            MODULE.main()
        self.assertEqual(inspect.call_count,2)
        self.assertTrue(api.call_args_list[-1].args[0].endswith("/force-cancel"))

    def test_changed_head_or_assigned_worker_prevents_force(self):
        pull,run,jobs=fixture();plan=MODULE.eligible(REPO,pull,run,jobs,"current")
        with self.invoke(), patch.object(MODULE,"snapshot",side_effect=[plan,MODULE.Refused("state changed")]), patch.object(MODULE,"api",side_effect=[None,*[{"status":"queued"}]*3]) as api, patch.object(MODULE.time,"sleep"), redirect_stdout(io.StringIO()):
            with self.assertRaises(MODULE.Refused):MODULE.main()
        self.assertFalse(any("force-cancel" in call.args[0] for call in api.call_args_list))


if __name__ == "__main__":unittest.main()
