#!/usr/bin/env python3
"""Local lifecycle tests use only processes created by this test suite."""

import importlib.util
import json
import os
from pathlib import Path
import signal
import select
import subprocess
import sys
import tempfile
import time
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("owned_process", ROOT / "scripts/ci-owned-process.py")
owned = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(owned)


class OwnedProcessTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.receipt = Path(self.directory.name) / "fixture.json"
        self.processes = []

    def tearDown(self):
        for process in self.processes:
            if process.poll() is None:
                process.kill()
            process.wait()
            if process.stdout:
                process.stdout.close()
            if process.stderr:
                process.stderr.close()
        self.directory.cleanup()

    def process(self, ignore_term=False):
        code = "import time; time.sleep(60)"
        if ignore_term:
            code = "import signal,time; signal.signal(signal.SIGTERM, signal.SIG_IGN); print('ready', flush=True); time.sleep(60)"
        process = subprocess.Popen([sys.executable, "-c", code], stdout=subprocess.PIPE)
        self.processes.append(process)
        if ignore_term:
            self.assertEqual(process.stdout.readline(), b"ready\n")
        return process

    def test_stop_only_recorded_process_leaves_sibling_alive(self):
        target, sibling = self.process(), self.process()
        owned.record(target.pid, self.receipt)
        owned.stop(self.receipt, timeout=1)
        self.assertEqual(target.wait(timeout=2), -signal.SIGTERM)
        self.assertIsNone(sibling.poll())
        self.assertFalse(self.receipt.exists())

    def test_reused_pid_identity_is_not_killed(self):
        process = self.process()
        owned.record(process.pid, self.receipt)
        identity = json.loads(self.receipt.read_text())
        identity["start_time"] -= 1
        self.receipt.write_text(json.dumps(identity))
        owned.stop(self.receipt, timeout=0)
        self.assertIsNone(process.poll())
        self.assertFalse(self.receipt.exists())

    def test_exact_owned_process_can_be_killed_after_term_timeout(self):
        process = self.process(ignore_term=True)
        owned.record(process.pid, self.receipt)
        owned.stop(self.receipt, timeout=0.1)
        self.assertEqual(process.wait(timeout=2), -signal.SIGKILL)

    def test_already_exited_process_is_safe(self):
        process = self.process()
        owned.record(process.pid, self.receipt)
        process.terminate()
        process.wait(timeout=2)
        owned.stop(self.receipt)
        self.assertFalse(self.receipt.exists())

    def test_absent_receipt_is_noop(self):
        owned.stop(self.receipt)

    def test_invalid_identity_never_opens_pidfd(self):
        for pid in [0, -1, 1, True, "123"]:
            with self.subTest(pid=pid):
                self.receipt.write_text(json.dumps({"version": 1, "pid": pid, "start_time": 123}))
                with mock.patch.object(os, "pidfd_open") as open_pidfd:
                    with self.assertRaises(ValueError):
                        owned.stop(self.receipt)
                    open_pidfd.assert_not_called()

    def test_record_refuses_to_overwrite_receipt(self):
        process = self.process()
        owned.record(process.pid, self.receipt)
        before = self.receipt.read_text()
        with self.assertRaises(FileExistsError):
            owned.record(process.pid, self.receipt)
        self.assertEqual(self.receipt.read_text(), before)

    def test_start_owns_spawn_and_commits_receipt(self):
        real_popen = subprocess.Popen

        def spawn(*args, **kwargs):
            child = real_popen(*args, **kwargs)
            self.processes.append(child)
            return child

        with mock.patch.object(subprocess, "Popen", side_effect=spawn):
            pid = owned.start(self.receipt, Path(self.directory.name) / "server.log",
                              [sys.executable, "-c", "import time; time.sleep(60)"])
        self.assertEqual(json.loads(self.receipt.read_text())["pid"], pid)
        owned.stop(self.receipt, timeout=1)
        self.assertEqual(self.processes[0].wait(timeout=2), -signal.SIGTERM)

    def test_unsupported_pidfd_is_rejected_before_spawn(self):
        with mock.patch.object(os, "pidfd_open", side_effect=NotImplementedError), \
                mock.patch.object(subprocess, "Popen") as spawn:
            with self.assertRaises(NotImplementedError):
                owned.start(self.receipt, Path(self.directory.name) / "server.log", ["ignored"])
            spawn.assert_not_called()
        self.assertFalse(self.receipt.exists())

    def test_receipt_from_another_boot_or_namespace_cannot_signal_process(self):
        process = self.process()
        owned.record(process.pid, self.receipt)
        original = json.loads(self.receipt.read_text())
        for key in ["boot_id", "pid_namespace"]:
            identity = json.loads(json.dumps(original))
            identity["context"][key] = "different"
            self.receipt.write_text(json.dumps(identity))
            with mock.patch.object(os, "pidfd_open") as open_pidfd:
                with self.assertRaises(ValueError):
                    owned.stop(self.receipt)
                open_pidfd.assert_not_called()
            self.assertIsNone(process.poll())

    def test_existing_receipt_is_rejected_before_spawn(self):
        self.receipt.write_text("keep original")
        with mock.patch.object(subprocess, "Popen") as spawn:
            with self.assertRaises(FileExistsError):
                owned.start(self.receipt, Path(self.directory.name) / "server.log", ["ignored"])
            spawn.assert_not_called()
        self.assertEqual(self.receipt.read_text(), "keep original")

    def test_receipt_write_failure_stops_only_new_child(self):
        sibling = self.process()
        self.assert_start_failure_is_rolled_back(OSError("injected receipt write failure"))
        self.assertIsNone(sibling.poll())

    def test_startup_signal_exception_stops_new_child(self):
        self.assert_start_failure_is_rolled_back(owned.StartupInterrupted("injected SIGTERM"))

    def test_signal_in_spawn_assignment_window_rolls_back_owned_child(self):
        real_popen = subprocess.Popen
        spawned = []

        def spawn(*args, **kwargs):
            child = real_popen(*args, **kwargs)
            spawned.append(child)
            self.processes.append(child)
            # start() has not yet received/assigned this Popen object.
            os.kill(os.getpid(), signal.SIGINT)
            return child

        with mock.patch.object(subprocess, "Popen", side_effect=spawn):
            with self.assertRaises(owned.StartupInterrupted):
                owned.start(self.receipt, Path(self.directory.name) / "server.log",
                            [sys.executable, "-c", "import time; time.sleep(60)"])
        self.assertEqual(len(spawned), 1)
        self.assertIsNotNone(spawned[0].poll())
        self.assertFalse(self.receipt.exists())

    def test_real_repeated_cancellation_does_not_interrupt_owned_rollback(self):
        directory = Path(self.directory.name)
        harness = directory / "cancellation.py"
        harness.write_text('''import importlib.util,json,os,signal,subprocess,sys,time
from pathlib import Path
root=Path(sys.argv[1])
spec=importlib.util.spec_from_file_location('owned',sys.argv[2])
owned=importlib.util.module_from_spec(spec);spec.loader.exec_module(owned)
original_write=os.write
def delayed_launch(fd,data):
 result=original_write(fd,data)
 if data==b'G':
  while not (root/'child-ready').exists(): time.sleep(.01)
  (root/'recording').touch()
  while not (root/'proceed').exists(): time.sleep(.01)
 return result
os.write=delayed_launch
original_terminate=subprocess.Popen.terminate
def terminate(process):
 (root/'rollback').touch()
 return original_terminate(process)
subprocess.Popen.terminate=terminate
child="import signal,time; from pathlib import Path; signal.signal(signal.SIGTERM,signal.SIG_IGN); Path("+repr(str(root/'child-ready'))+").touch(); time.sleep(60)"
owned.start(root/'fixture.json',root/'server.log',[sys.executable,'-c',child])
''')
        helper = subprocess.Popen([sys.executable, str(harness), str(directory),
                                   str(ROOT / "scripts/ci-owned-process.py")],
                                  stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.processes.append(helper)

        def wait_for(path):
            deadline = time.monotonic() + 5
            while not path.exists() and time.monotonic() < deadline:
                if helper.poll() is not None:
                    self.fail("startup helper exited before synchronization point")
                time.sleep(.01)
            self.assertTrue(path.exists(), str(path))

        identity = None
        try:
            wait_for(directory / "recording")
            identity = json.loads(self.receipt.read_text())
            helper.send_signal(signal.SIGINT)
            (directory / "proceed").touch()
            wait_for(directory / "rollback")
            helper.send_signal(signal.SIGTERM)
            _, error = helper.communicate(timeout=15)
            self.assertNotEqual(helper.returncode, 0)
            self.assertIn(b"StartupInterrupted", error)
            self.assertFalse(self.receipt.exists())
            try:
                self.assertNotEqual(owned.start_time(identity["pid"]), identity["start_time"])
            except FileNotFoundError:
                pass
        finally:
            if helper.poll() is None:
                helper.kill()
                helper.wait()
            # Failure-path housekeeping still uses exact test-owned identity.
            if identity is not None:
                rescue = directory / "rescue.json"
                rescue.write_text(json.dumps(identity))
                owned.stop(rescue, timeout=.1)

    def test_sigkill_before_commit_never_executes_fixture(self):
        directory = Path(self.directory.name)
        harness = directory / "killed-startup.py"
        harness.write_text('''import importlib.util,json,sys,time
from pathlib import Path
root=Path(sys.argv[1])
spec=importlib.util.spec_from_file_location('owned',sys.argv[2])
owned=importlib.util.module_from_spec(spec);spec.loader.exec_module(owned)
original_dump=json.dump
def pause(value,stream):
 original_dump(value,stream);stream.flush()
 (root/'recording').touch()
 time.sleep(60)
json.dump=pause
child="from pathlib import Path; import time; Path("+repr(str(root/'fixture-executed'))+").touch(); time.sleep(60)"
owned.start(root/'fixture.json',root/'server.log',[sys.executable,'-c',child])
''')
        helper = subprocess.Popen([sys.executable, str(harness), str(directory),
                                   str(ROOT / "scripts/ci-owned-process.py")],
                                  stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.processes.append(helper)
        descriptor = None
        identity = None
        try:
            deadline = time.monotonic() + 5
            while not (directory / "recording").exists() and time.monotonic() < deadline:
                self.assertIsNone(helper.poll())
                time.sleep(.01)
            self.assertTrue((directory / "recording").exists())
            identity = json.loads(self.receipt.read_text())
            descriptor = os.pidfd_open(identity["pid"])
            helper.kill()
            helper.wait(timeout=2)
            self.assertTrue(select.select([descriptor], [], [], 2)[0], "launch wrapper did not exit on parent pipe EOF")
            self.assertFalse((directory / "fixture-executed").exists())
        finally:
            if helper.poll() is None:
                helper.kill()
                helper.wait()
            if descriptor is not None:
                os.close(descriptor)
            if identity is not None:
                rescue = directory / "rescue.json"
                rescue.write_text(json.dumps(identity))
                owned.stop(rescue, timeout=.1)

    def assert_start_failure_is_rolled_back(self, error):
        real_popen = subprocess.Popen
        spawned = []

        def spawn(*args, **kwargs):
            child = real_popen(*args, **kwargs)
            spawned.append(child)
            self.processes.append(child)
            return child

        with mock.patch.object(subprocess, "Popen", side_effect=spawn), \
                mock.patch.object(json, "dump", side_effect=error):
            with self.assertRaises(type(error)):
                owned.start(self.receipt, Path(self.directory.name) / "server.log",
                            [sys.executable, "-c", "import time; time.sleep(60)"])
        self.assertEqual(len(spawned), 1)
        self.assertIsNotNone(spawned[0].poll())
        self.assertFalse(self.receipt.exists())


if __name__ == "__main__":
    unittest.main()
