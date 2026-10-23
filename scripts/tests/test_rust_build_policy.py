"""Shared admission contract: real locks/processes, no Rust compilation."""

from contextlib import contextmanager
import fcntl
import importlib.util
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

SCRIPTS = Path(__file__).resolve().parents[1]
MODULE = next(
    path
    for path in (
        SCRIPTS / "dev" / "rust_build_policy.py",
        SCRIPTS / "rust_build_policy.py",
    )
    if path.exists()
)
spec = importlib.util.spec_from_file_location("contract_under_test", MODULE)
contract = importlib.util.module_from_spec(spec)
spec.loader.exec_module(contract)


def configuration(slots=2, reserve=0):
    policy = contract.default_policy()
    policy.update(
        max_concurrent=slots,
        reserved_checks=reserve,
        memory_budget_mib=65536,
        cpu_budget_percent=800,
    )
    for limits in policy["classes"].values():
        limits.update(memory_mib=256, cpu_percent=100, jobs=1)
    return policy


@contextmanager
def worker(directory, arguments):
    program = """import importlib.util,json,sys
spec=importlib.util.spec_from_file_location('policy',sys.argv[1]); m=importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
m.host_budgets=lambda:(65536,800)
with m.Session(json.loads(sys.argv[3]),m.Path(sys.argv[2])) as session:
 session.acquire(); print('admitted',flush=True); sys.stdin.read(1)
"""
    process = subprocess.Popen(
        [
            sys.executable,
            "-c",
            program,
            str(MODULE),
            str(directory),
            json.dumps(arguments),
        ],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        yield process
    finally:
        if process.poll() is None:
            process.stdin.write("x")
            process.stdin.flush()
        try:
            process.communicate(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.communicate()
        if process.returncode != 0:
            raise AssertionError(f"Policy fixture worker exited {process.returncode}")


def admitted(process, timeout=3):
    ready, _, _ = select.select([process.stdout], [], [], timeout)
    if ready:
        return process.stdout.readline().strip() == "admitted"
    return False


class SharedBuildPolicyTests(unittest.TestCase):
    def setUp(self):
        # These idle-process tests exercise contract semantics on even tiny CI
        # hosts; no child performs CPU or memory work at these synthetic budgets.
        self.budgets = patch.object(contract, "host_budgets", return_value=(65536, 800))
        self.budgets.start()
        self.addCleanup(self.budgets.stop)
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name) / "admission"

    def test_default_is_provisional_and_valid(self):
        value = contract.validate(contract.default_policy())
        self.assertLessEqual(value["max_concurrent"], 2)
        self.assertEqual(value["reserved_checks"], 0)

    def test_classification_for_five_workloads(self):
        cases = [
            (["check", "-p", "leaf"], "check"),
            (["clippy", "--package=leaf"], "check"),
            (["test", "-p", "leaf"], "targeted-test"),
            (["build", "-p", "leaf"], "executable"),
            (["check", "--workspace"], "workspace"),
            (["test", "-p", "leaf", "--all-features"], "workspace"),
            (["build", "-p", "leaf", "--release"], "release"),
            (["bench", "-p", "leaf"], "release"),
            (["bench"], "release"),
            (["bench", "--workspace"], "release"),
        ]
        for arguments, expected in cases:
            with self.subTest(arguments=arguments):
                self.assertEqual(contract.classify(arguments), expected)

    def test_aliases_and_executable_arguments(self):
        for verb, expected in [
            ("c", "check"),
            ("t", "targeted-test"),
            ("b", "executable"),
            ("r", "executable"),
        ]:
            self.assertEqual(contract.classify([verb, "-pleaf"]), expected)
        self.assertEqual(
            contract.classify(["test", "-p", "leaf", "--", "--release"]),
            "targeted-test",
        )

    def test_custom_profile_is_conservative(self):
        for option in [
            ["--profile", "production"],
            ["--profile=production"],
            ["--profile=bench"],
        ]:
            self.assertEqual(
                contract.classify(["build", "-p", "leaf", *option]), "release"
            )

    def test_exact_capacity_one_two_four_with_real_processes(self):
        from contextlib import ExitStack

        for capacity in (1, 2, 4):
            with self.subTest(capacity=capacity):
                contract.apply_policy(configuration(capacity), self.directory)
                with ExitStack() as stack:
                    active = [
                        stack.enter_context(
                            worker(self.directory, ["build", "-p", "leaf"])
                        )
                        for _ in range(capacity)
                    ]
                    for process in active:
                        self.assertTrue(admitted(process))
                    queued = stack.enter_context(
                        worker(self.directory, ["build", "-p", "leaf"])
                    )
                    self.assertFalse(admitted(queued, 0.2))
                    active[0].stdin.write("x")
                    active[0].stdin.flush()
                    active[0].wait(timeout=3)
                    self.assertTrue(admitted(queued))

    def test_reserved_check_progress_while_heavy_waits(self):
        contract.apply_policy(configuration(2, 1), self.directory)
        with worker(self.directory, ["build", "-p", "leaf"]) as heavy:
            self.assertTrue(admitted(heavy))
            with worker(self.directory, ["test", "-p", "leaf"]) as queued:
                self.assertFalse(admitted(queued, 0.2))
                with worker(self.directory, ["check", "-p", "leaf"]) as feedback:
                    self.assertTrue(admitted(feedback))
                    self.assertFalse(admitted(queued, 0.1))
                heavy.stdin.write("x")
                heavy.stdin.flush()
                heavy.wait(timeout=3)
                self.assertTrue(admitted(queued))

    def test_active_lease_refuses_update(self):
        contract.apply_policy(configuration(), self.directory)
        with contract.Session(["build", "-p", "leaf"], self.directory) as session:
            session.acquire()
            with self.assertRaisesRegex(ValueError, "Drain"):
                contract.apply_policy(configuration(4), self.directory)
        contract.apply_policy(configuration(4), self.directory)

    def test_queued_policy_snapshot_refuses_update(self):
        contract.apply_policy(configuration(), self.directory)
        with contract.Session(["test", "-p", "leaf"], self.directory):
            with self.assertRaisesRegex(ValueError, "Drain"):
                contract.apply_policy(configuration(1), self.directory)

    def test_legacy_lock_refuses_update(self):
        contract.private_directory(self.directory)
        fd = contract.open_lock(self.directory / "slot-2")
        try:
            fcntl.flock(fd, fcntl.LOCK_EX)
            with self.assertRaisesRegex(ValueError, "Drain"):
                contract.apply_policy(configuration(4), self.directory)
        finally:
            os.close(fd)

    def test_malformed_unsafe_and_overbudget_refused(self):
        variants = []
        value = configuration()
        value["max_concurrent"] = True
        variants.append(value)
        value = configuration()
        value["schema_version"] = 2
        variants.append(value)
        value = configuration()
        value["reserved_checks"] = 2
        variants.append(value)
        value = configuration(4)
        value["cpu_budget_percent"] = 300
        variants.append(value)
        value = configuration(4)
        value["memory_budget_mib"] = 1024
        value["classes"]["release"]["memory_mib"] = 512
        variants.append(value)
        value = configuration()
        value["classes"]["check"]["jobs"] = 0
        variants.append(value)
        for value in variants:
            with self.subTest(value=value), self.assertRaises(ValueError):
                contract.validate(value)
        contract.private_directory(self.directory)
        (self.directory / "policy.json").write_text("{malformed")
        with self.assertRaises(ValueError):
            contract.Session(["check"], self.directory)
        (self.directory / "policy.json").write_text(json.dumps(configuration()))
        (self.directory / "policy.json").chmod(0o666)
        with self.assertRaisesRegex(ValueError, "Unsafe"):
            contract.Session(["check"], self.directory)

    def test_fingerprint_and_limits_survive_session(self):
        value = configuration(4, 1)
        value["classes"]["check"].update(cpu_percent=200, jobs=3)
        contract.apply_policy(value, self.directory)
        with (
            contract.Session(["check", "-p", "leaf"], self.directory) as first,
            contract.Session(["check", "-p", "other"], self.directory) as second,
        ):
            self.assertEqual(first.fingerprint, second.fingerprint)
            self.assertEqual(first.limits["jobs"], 3)
            self.assertIn("CPUQuota=200%", first.properties())
            first.acquire()
            self.assertEqual(len(first.descriptors), 2)
            self.assertEqual(first.event()["reserved_checks"], 1)


if __name__ == "__main__":
    unittest.main()
