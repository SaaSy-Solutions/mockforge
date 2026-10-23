"""Local policy behavior without compiling Rust or claiming cache speedups."""

import fcntl
import importlib.machinery
import importlib.util
import os
import signal
import time
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

LAUNCHER = Path(__file__).resolve().parents[1] / "dev" / "cargo"
loader = importlib.machinery.SourceFileLoader("dev_cargo", str(LAUNCHER))
spec = importlib.util.spec_from_loader(loader.name, loader)
policy = importlib.util.module_from_spec(spec)
loader.exec_module(policy)


class DevelopmentCargoTests(unittest.TestCase):
    def test_scope_enforces_bounds(self):
        command = policy.scope_command(["+stable", "test", "-p", "example"])
        self.assertIn("--property=MemoryMax=12G", command)
        self.assertIn("--property=MemorySwapMax=0", command)
        self.assertIn("--property=CPUQuota=200%", command)
        self.assertEqual(
            command[-5:], ["--inside-scope", "+stable", "test", "-p", "example"]
        )

    def test_environment_preserves_compiler_inputs(self):
        with patch.dict(
            os.environ,
            {"RUSTFLAGS": "-C debuginfo=1", "CARGO_INCREMENTAL": "1"},
            clear=True,
        ):
            env = policy.build_environment(["check"])
        self.assertEqual(env["RUSTFLAGS"], "-C debuginfo=1")
        self.assertEqual(env["CARGO_INCREMENTAL"], "1")
        self.assertEqual(env["CARGO_BUILD_JOBS"], "2")
        self.assertEqual(env["CARGO_TARGET_DIR"], str(policy.ROOT / "target"))

    def test_worktrees_do_not_share_targets(self):
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.dict(os.environ, {}, clear=True),
        ):
            targets = []
            for name in ("first", "second"):
                with patch.object(policy, "ROOT", Path(directory) / name):
                    targets.append(
                        policy.build_environment(["check"])["CARGO_TARGET_DIR"]
                    )
            self.assertNotEqual(*targets)

    def test_repeated_target_is_stable(self):
        with patch.dict(os.environ, {}, clear=True):
            self.assertEqual(
                policy.build_environment(["check"])["CARGO_TARGET_DIR"],
                policy.build_environment(["test"])["CARGO_TARGET_DIR"],
            )

    def test_reject_unsafe_overrides(self):
        with patch.dict(os.environ, {}, clear=True):
            for args in (
                ["check", "-j8"],
                ["test", "--jobs=0"],
                ["build", "--target-dir=/tmp/shared"],
                ["check", "--config", "build.jobs=8"],
                ["check", "--manifest-path=/tmp/Cargo.toml"],
            ):
                with self.subTest(args=args), self.assertRaises(ValueError):
                    policy.build_environment(args)
        with patch.dict(os.environ, {"CARGO_TARGET_DIR": "/tmp/shared"}, clear=True):
            with self.assertRaises(ValueError):
                policy.build_environment(["check"])

    def test_executable_arguments_are_not_cargo_options(self):
        with patch.dict(os.environ, {}, clear=True):
            policy.build_environment(["run", "--", "--jobs=20"])

    def test_second_shared_slot_when_first_busy(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "admission"
            first, _ = policy.acquire_slot(root)
            second, _ = policy.acquire_slot(root)
            self.assertTrue((root / "slot-2").exists())
            self.assertFalse((root / "slot-3").exists())
            os.close(first)
            os.close(second)
            again, _ = policy.acquire_slot(root)
            os.close(again)

    def test_unsafe_lock_directory_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "unsafe"
            path.mkdir(mode=0o755)
            with self.assertRaises(ValueError):
                policy.acquire_slot(path)

    def test_child_failure_preserved_and_wait_reported(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            runtime = root / "runtime"
            runtime.mkdir()
            for name, body in [
                (
                    "systemd-run",
                    '#!/bin/sh\nwhile test "$1" != --; do shift; done\nshift\nexec "$@"\n',
                ),
                ("cargo", "#!/bin/sh\nexit 7\n"),
            ]:
                path = root / name
                path.write_text(body)
                path.chmod(0o700)
            env = {"PATH": f"{root}:/usr/bin:/bin", "XDG_RUNTIME_DIR": str(runtime)}
            program = f"import runpy; p=runpy.run_path({str(LAUNCHER)!r}); p['run'].__globals__['verify_scope']=lambda:None; raise SystemExit(p['run'](['--inside-scope','check','-p','fake']))"
            result = subprocess.run(
                ["/usr/bin/python3", "-c", program],
                env=env,
                text=True,
                capture_output=True,
            )
            self.assertEqual(result.returncode, 7, result.stderr)
            self.assertIn('"admission_seconds":', result.stderr)
            self.assertIn('"exit_status": 7', result.stderr)
            # A failed child releases both slots; a second invocation finishes.
            again = subprocess.run(
                ["/usr/bin/python3", "-c", program],
                env=env,
                capture_output=True,
                timeout=5,
            )
            self.assertEqual(again.returncode, 7)

    def test_inherited_lock_survives_launcher_kill(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            runtime = root / "runtime"
            runtime.mkdir()
            ready = root / "ready"
            cargo = root / "cargo"
            cargo.write_text(
                "#!/usr/bin/python3\nimport os,time\nfrom pathlib import Path\nPath(os.environ['READY']).write_text(str(os.getpid()))\ntime.sleep(30)\n"
            )
            cargo.chmod(0o700)
            program = f"import runpy; p=runpy.run_path({str(LAUNCHER)!r}); p['run'].__globals__['verify_scope']=lambda:None; raise SystemExit(p['run'](['--inside-scope','check']))"
            env = {
                "PATH": f"{root}:/usr/bin:/bin",
                "XDG_RUNTIME_DIR": str(runtime),
                "READY": str(ready),
            }
            parent = subprocess.Popen(
                ["/usr/bin/python3", "-c", program],
                env=env,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            compiler_pid = None
            try:
                deadline = time.monotonic() + 5
                while not ready.exists() and time.monotonic() < deadline:
                    time.sleep(0.02)
                self.assertTrue(ready.exists())
                compiler_pid = int(ready.read_text())
                parent.kill()
                parent.wait(timeout=5)
                fd = os.open(
                    runtime / "saasy-dev-cargo-admission" / "slot-1", os.O_RDWR
                )
                try:
                    with self.assertRaises(BlockingIOError):
                        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
                finally:
                    os.close(fd)
            finally:
                if compiler_pid is not None:
                    os.kill(compiler_pid, signal.SIGTERM)
                if parent.poll() is None:
                    parent.kill()
                parent.wait()

    def test_scoped_hook_routes_through_launcher(self):
        source = (LAUNCHER.parents[1] / "precommit" / "clippy_scoped.py").read_text()
        self.assertIn('/ "dev" / "cargo"', source)
        self.assertNotIn('"cargo",', source)


if __name__ == "__main__":
    unittest.main()
