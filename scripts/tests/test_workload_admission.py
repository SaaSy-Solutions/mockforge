"""Exercise real admission locks, warm evidence, FIFO, crash and memory recovery."""

from contextlib import ExitStack, contextmanager
import importlib.util
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

SCRIPTS = Path(__file__).resolve().parents[1]
MODULE = next(
    p
    for p in (SCRIPTS / "rust_build_policy.py", SCRIPTS / "dev/rust_build_policy.py")
    if p.exists()
)
SPEC = importlib.util.spec_from_file_location("workload_contract", MODULE)
policy = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(policy)
ARGS = ["check", "-p", "founderos-proto", "--lib"]
REPOSITORY = "SaaSy-Solutions/aether"


def ready(process, timeout=2):
    if select.select([process.stdout], [], [], timeout)[0]:
        return json.loads(process.stdout.readline())
    return None


@contextmanager
def worker(directory, context, arguments=ARGS, memory_file=None):
    code = """import importlib.util,json,os,sys
from pathlib import Path
s=importlib.util.spec_from_file_location('p',sys.argv[1]);p=importlib.util.module_from_spec(s);s.loader.exec_module(p)
sys.modules["rust_policy_config"].host_budgets=lambda:(65536,800)
sys.modules["rust_policy_runtime"].available_memory_mib=lambda:int(Path(sys.argv[5]).read_text()) if sys.argv[5] else 65536
c=json.loads(sys.argv[3]); c['environment']=dict(os.environ,RUSTC=c.pop('compiler'))
with p.Session(json.loads(sys.argv[4]),Path(sys.argv[2])) as a:
 a.configure(**c); wait=a.acquire();print(json.dumps(dict(a.event(),wait=wait)),flush=True);sys.stdin.read(1)
"""
    process = subprocess.Popen(
        [
            sys.executable,
            "-c",
            code,
            str(MODULE),
            str(directory),
            json.dumps(context),
            json.dumps(arguments),
            str(memory_file or ""),
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
            process.communicate(timeout=4)
        except subprocess.TimeoutExpired:
            process.kill()
            process.communicate()
        if process.returncode not in (0, -9):
            raise AssertionError(f"Fixture exited {process.returncode}")


class WorkloadAdmission(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.runtime = self.root / "admission"
        self.worktree = self.root / "worktree"
        self.worktree.mkdir()
        (self.worktree / "Cargo.lock").write_text("fixture lock")
        (self.worktree / "Cargo.toml").write_text("fixture manifest")
        subprocess.run(["git", "init", "--quiet", str(self.worktree)], check=True)
        self.target = self.root / "target"
        (self.target / "debug/.fingerprint/example").mkdir(parents=True)
        self.compiler = self.root / "compiler"
        self.compiler.write_text(
            "#!" + sys.executable + '\nprint("fixture rustc version")\n'
        )
        self.compiler.chmod(0o700)
        self.environment = dict(os.environ, RUSTC=str(self.compiler))
        self.context = dict(
            repository=REPOSITORY,
            worktree=str(self.worktree),
            target=str(self.target),
            compiler=str(self.compiler),
        )
        self.budgets = patch.object(
            sys.modules["rust_policy_config"], "host_budgets", return_value=(65536, 800)
        )
        self.budgets.start()
        self.addCleanup(self.budgets.stop)
        self.memory = patch.object(
            sys.modules["rust_policy_runtime"],
            "available_memory_mib",
            return_value=65536,
        )
        self.memory.start()
        self.addCleanup(self.memory.stop)
        policy.apply_policy(policy.default_policy(), self.runtime)

    def configured(self, arguments=ARGS, **changes):
        session = policy.Session(arguments, self.runtime)
        context = dict(
            repository=REPOSITORY,
            worktree=self.worktree,
            target=self.target,
            environment=self.environment,
        )
        context.update(changes)
        session.configure(**context)
        return session

    def seed(self):
        with self.configured() as session:
            self.assertEqual(session.admission_class, "STANDARD_CHECK")
            session.acquire()
            session.record_success()

    def test_default_measured_recipes_are_exact_and_native_library_only(self):
        recipes = policy.default_policy()["light_checks"]
        selected = {(rule["repository"], tuple(rule["arguments"])) for rule in recipes}
        expected = {
            ("SaaSy-Solutions/" + repo, ("check", "-p", package, "--lib"))
            for repo, package in (
                ("aether", "founderos-proto"),
                ("aether", "founderos-core"),
                ("saas-platform", "platform-events-core"),
                ("codedig", "codedig-change-kind"),
                ("mockforge", "mockforge-platform-signing"),
            )
        }
        self.assertEqual(selected, expected)
        self.assertTrue(all(rule["evidence"].endswith(".md") for rule in recipes))

    def test_only_successful_admitted_compatible_warm_recipe_is_light(self):
        with self.configured() as session:
            session.record_success()  # Unadmitted outer scope is not proof.
        with self.configured() as session:
            self.assertEqual(session.admission_class, "STANDARD_CHECK")
        self.seed()
        with self.configured(
            ["check", "-p", "founderos-proto", "--lib", "--offline", "--locked"]
        ) as session:
            self.assertEqual(session.admission_class, "LIGHT_CHECK")

    def test_unknown_features_profiles_targets_and_packages_are_conservative(self):
        self.seed()
        for args in (
            [*ARGS, "--features", "large"],
            [*ARGS, "--all-targets"],
            [*ARGS, "--profile=dev"],
            ["check", "-p", "other", "--lib"],
            ["check", "--workspace"],
            ["check", "-p", "founderos-proto"],
            ["clippy", "-p", "founderos-proto", "--lib"],
        ):
            with self.subTest(arguments=args), self.configured(args) as session:
                self.assertNotEqual(session.admission_class, "LIGHT_CHECK")
        for args in (
            ["test", "-p", "founderos-proto", "--no-run"],
            ["build", "-p", "founderos-proto"],
            [*ARGS, "--release"],
        ):
            with self.configured(args) as session:
                self.assertEqual(session.admission_class, "HEAVY_BUILD")

    def test_environment_compiler_mode_overrides_cannot_rewarm_into_light(self):
        self.seed()
        for override in (
            {"CARGO_BUILD_TARGET": "aarch64-unknown-linux-gnu"},
            {"CARGO_BUILD_TARGET": ""},
            {"CARGO_PROFILE_DEV_OPT_LEVEL": "3"},
            {"CARGO_PROFILE_TEST_DEBUG": ""},
            {"RUSTFLAGS": "-C opt-level=3"},
            {"RUSTFLAGS": ""},
            {"CARGO_ENCODED_RUSTFLAGS": "-C\x1fopt-level=3"},
            {"CARGO_BUILD_RUSTFLAGS": "-C opt-level=3"},
            {"CARGO_INCREMENTAL": "0"},
            {"CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS": "-C opt-level=3"},
        ):
            environment = dict(self.environment, **override)
            with self.subTest(override=override):
                with self.configured(environment=environment) as session:
                    self.assertEqual(session.admission_class, "STANDARD_CHECK")
                    self.assertIn("Unmeasured", session.classification_reason)
                    session.acquire()
                    session.record_success()
                    self.assertIsNone(session.receipt)
                with self.configured(environment=environment) as session:
                    self.assertEqual(session.admission_class, "STANDARD_CHECK")
        with self.configured() as session:
            self.assertEqual(session.admission_class, "LIGHT_CHECK")

    def test_lock_flags_toolchain_target_and_receipt_expiry_invalidate_warmth(self):
        self.seed()
        with self.configured(
            environment=dict(self.environment, RUSTFLAGS="-C debuginfo=0")
        ) as session:
            self.assertEqual(session.admission_class, "STANDARD_CHECK")
        (self.worktree / "Cargo.toml").write_text("changed manifest features")
        with self.configured() as session:
            self.assertEqual(session.admission_class, "STANDARD_CHECK")
        (self.worktree / "Cargo.toml").write_text("fixture manifest")
        (self.worktree / "Cargo.lock").write_text("changed dependency versions")
        with self.configured() as session:
            self.assertEqual(session.admission_class, "STANDARD_CHECK")
        (self.worktree / "Cargo.lock").write_text("fixture lock")
        (self.worktree / "Cargo.toml").write_text("fixture manifest")
        subprocess.run(["git", "init", "--quiet", str(self.worktree)], check=True)
        self.compiler.write_text(
            "#!" + sys.executable + '\nprint("new fixture rustc")\n'
        )
        with self.configured() as session:
            self.assertEqual(session.admission_class, "STANDARD_CHECK")
        self.compiler.write_text(
            "#!" + sys.executable + '\nprint("fixture rustc version")\n'
        )
        with self.configured() as session:
            data = json.loads(session.receipt.read_text())
            data["success_unix"] = time.time() - 4000
            session.receipt.write_text(json.dumps(data))
        with self.configured() as session:
            self.assertEqual(session.admission_class, "STANDARD_CHECK")
        other = self.root / "other-target"
        other.mkdir()
        with self.configured(target=other) as session:
            self.assertEqual(session.admission_class, "STANDARD_CHECK")

    def test_two_heavy_allow_two_light_and_never_more_than_four_total(self):
        self.seed()
        heavy = ["build", "-p", "other"]
        with ExitStack() as stack:
            running = [
                stack.enter_context(worker(self.runtime, self.context, heavy))
                for _ in range(2)
            ]
            for process in running:
                self.assertEqual(ready(process)["admission_class"], "HEAVY_BUILD")
            blocked_heavy = stack.enter_context(
                worker(self.runtime, self.context, heavy)
            )
            self.assertIsNone(ready(blocked_heavy, 0.2))
            lights = [
                stack.enter_context(worker(self.runtime, self.context))
                for _ in range(2)
            ]
            for process in lights:
                self.assertEqual(ready(process)["admission_class"], "LIGHT_CHECK")
            fifth = stack.enter_context(worker(self.runtime, self.context))
            self.assertIsNone(ready(fifth, 0.2))
            lights[0].stdin.write("x")
            lights[0].stdin.flush()
            lights[0].wait(timeout=3)
            self.assertIsNotNone(ready(fifth))
            self.assertIsNone(ready(blocked_heavy, 0.1))
            running[0].stdin.write("x")
            running[0].stdin.flush()
            running[0].wait(timeout=3)
            self.assertIsNotNone(ready(blocked_heavy))

    def test_four_light_checks_can_run_together(self):
        self.seed()
        with ExitStack() as stack:
            running = [
                stack.enter_context(worker(self.runtime, self.context))
                for _ in range(4)
            ]
            for process in running:
                self.assertEqual(ready(process)["admission_class"], "LIGHT_CHECK")
            blocked = stack.enter_context(worker(self.runtime, self.context))
            self.assertIsNone(ready(blocked, 0.2))
            running[0].stdin.write("x")
            running[0].stdin.flush()
            running[0].wait(timeout=3)
            self.assertIsNotNone(ready(blocked))

    def test_older_heavy_is_not_starved_by_new_light_requests(self):
        self.seed()
        with ExitStack() as stack:
            running = [
                stack.enter_context(worker(self.runtime, self.context))
                for _ in range(4)
            ]
            admitted_rows = [ready(process) for process in running]
            self.assertTrue(all(admitted_rows))
            older = stack.enter_context(
                worker(self.runtime, self.context, ["build", "-p", "other"])
            )
            self.assertIsNone(ready(older, 0.2))
            newer = stack.enter_context(worker(self.runtime, self.context))
            self.assertIsNone(ready(newer, 0.2))
            ordinary = next(
                process
                for process, row in zip(running, admitted_rows)
                if row["admission_slot"] <= 2
            )
            reserved = next(
                process
                for process, row in zip(running, admitted_rows)
                if row["admission_slot"] > 2
            )
            ordinary.stdin.write("x")
            ordinary.stdin.flush()
            ordinary.wait(timeout=3)
            self.assertIsNotNone(ready(older))
            self.assertIsNone(ready(newer, 0.2))
            reserved.stdin.write("x")
            reserved.stdin.flush()
            reserved.wait(timeout=3)
            self.assertIsNotNone(ready(newer))

    def test_nonregular_queue_input_fails_without_blocking(self):
        queue = self.runtime / "queue"
        policy.private_directory(queue)
        os.mkfifo(queue / "invalid-ticket")
        with self.assertRaisesRegex(ValueError, "Unsafe queue ticket"):
            policy.live_waiters(queue)

    def test_dead_waiter_does_not_block_later_requests(self):
        heavy = ["build", "-p", "other"]
        with ExitStack() as stack:
            active = [
                stack.enter_context(worker(self.runtime, self.context, heavy))
                for _ in range(2)
            ]
            for process in active:
                self.assertIsNotNone(ready(process))
            dead = stack.enter_context(worker(self.runtime, self.context, heavy))
            self.assertIsNone(ready(dead, 0.2))
            dead.kill()
            dead.wait(timeout=3)
            later = stack.enter_context(worker(self.runtime, self.context, heavy))
            self.assertIsNone(ready(later, 0.2))
            active[0].stdin.write("x")
            active[0].stdin.flush()
            active[0].wait(timeout=3)
            self.assertIsNotNone(ready(later))

    def test_inherited_child_keeps_slot_after_launcher_death(self):
        release = self.root / "release-child"
        code = """import importlib.util,json,subprocess,sys,time
from pathlib import Path
s=importlib.util.spec_from_file_location('p',sys.argv[1]);p=importlib.util.module_from_spec(s);s.loader.exec_module(p)
sys.modules["rust_policy_config"].host_budgets=lambda:(65536,800);sys.modules["rust_policy_runtime"].available_memory_mib=lambda:65536
with p.Session(['build','-p','other'],Path(sys.argv[2])) as a:
 a.acquire()
 child="import pathlib,sys,time; p=pathlib.Path(sys.argv[1]); end=time.monotonic()+10; exec('while not p.exists() and time.monotonic()<end: time.sleep(.02)')"
 subprocess.Popen([sys.executable,'-c',child,sys.argv[3]],pass_fds=a.descriptors)
 print('child holds descriptors',flush=True);time.sleep(10)
"""
        parent = subprocess.Popen(
            [sys.executable, "-c", code, str(MODULE), str(self.runtime), str(release)],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        try:
            self.assertTrue(select.select([parent.stdout], [], [], 3)[0])
            self.assertEqual(
                parent.stdout.readline().strip(), "child holds descriptors"
            )
            parent.kill()
            parent.wait(timeout=3)
            with worker(self.runtime, self.context, ["build", "-p", "other"]) as second:
                self.assertIsNotNone(ready(second))
                with worker(
                    self.runtime, self.context, ["build", "-p", "other"]
                ) as queued:
                    self.assertIsNone(ready(queued, 0.2))
                    with self.assertRaisesRegex(ValueError, "Drain"):
                        policy.apply_policy(policy.default_policy(), self.runtime)
                    release.touch()
                    self.assertIsNotNone(ready(queued))
        finally:
            release.touch()
            if parent.poll() is None:
                parent.kill()
            parent.communicate(timeout=3)

    def test_memory_pressure_waits_without_reserving_a_slot_then_recovers(self):
        memory = self.root / "memory"
        memory.write_text("1024")
        with worker(
            self.runtime, self.context, ["build", "-p", "other"], memory
        ) as process:
            self.assertIsNone(ready(process, 0.2))
            with self.assertRaisesRegex(ValueError, "Drain"):
                policy.apply_policy(policy.default_policy(), self.runtime)
            memory.write_text("65536")
            result = ready(process)
            self.assertIsNotNone(result)
            self.assertGreater(result["wait"], 0.1)

    def test_failed_or_missing_artifact_receipt_does_not_promote(self):
        (self.target / "debug/.fingerprint/example").rmdir()
        with self.configured() as session:
            session.acquire()
            session.record_success()
        with self.configured() as session:
            self.assertEqual(session.admission_class, "STANDARD_CHECK")


if __name__ == "__main__":
    unittest.main()
