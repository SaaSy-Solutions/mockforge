"""Exercise verifier subprocess cleanup with real child processes and no Docker."""
import contextlib
import importlib.util
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import threading
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location('root_archive', Path(__file__).resolve().parents[1] / 'verify-root-image-archive.py')
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class VerifierLifecycle(unittest.TestCase):
    def exercise(self, interrupt=None, ignore_term=False, keyboard=False):
        with tempfile.TemporaryDirectory(prefix='mockforge-verifier-lifecycle-') as directory:
            output = Path(directory)
            child = output / 'child.py'
            child.write_text('''import pathlib, signal, sys, time
root = pathlib.Path(sys.argv[1])
ignore = sys.argv[2] == 'True'
def stop(*_):
    raise InterruptedError('requested cleanup')
signal.signal(signal.SIGTERM, signal.SIG_IGN if ignore else stop)
try:
    (root / 'ready').write_text('ready')
    time.sleep(1.5 if ignore else 10)
finally:
    (root / 'cleaned').write_text('cleaned')
''')
            receipt = {}
            timer = None
            if interrupt:
                def send_when_ready():
                    import time
                    deadline = time.monotonic() + 3
                    while not (output / 'ready').exists() and time.monotonic() < deadline:
                        time.sleep(0.01)
                    os.kill(os.getpid(), interrupt)
                timer = threading.Thread(target=send_when_ready)
                timer.start()
            original_wait = subprocess.Popen.wait
            injected = False

            def interrupted_wait(process, *args, **kwargs):
                nonlocal injected
                if not injected:
                    import time
                    deadline = time.monotonic() + 3
                    while not (output / 'ready').exists() and time.monotonic() < deadline:
                        time.sleep(0.01)
                    injected = True
                    raise KeyboardInterrupt()
                return original_wait(process, *args, **kwargs)

            patch = mock.patch.object(subprocess.Popen, 'wait', interrupted_wait) if keyboard else contextlib.nullcontext()
            try:
                with patch, self.assertRaises(KeyboardInterrupt if keyboard else InterruptedError if interrupt else subprocess.TimeoutExpired):
                    MODULE.run_owned_verifier([sys.executable, str(child), str(output), str(ignore_term)], output,
                                              receipt, timeout=3 if interrupt else 0.5,
                                              cleanup_timeout=0.1 if ignore_term else 2)
            finally:
                if timer:
                    timer.join(timeout=4)
            state = receipt['http_verifier_process']
            if ignore_term:
                self.assertEqual(state['state'], 'unresolved_cleanup')
                self.assertTrue(state['operator_followup_required'])
                os.kill(state['pid'], 0)  # No unconditional SIGKILL on cleanup timeout.
                os.waitpid(state['pid'], 0)  # Fixture exits itself; leave no child behind.
            else:
                self.assertEqual(state['state'], 'exited_after_termination')
            self.assertTrue((output / 'cleaned').exists())
            self.assertTrue(state['termination_requested'])

    def test_timeout_allows_child_finally(self):
        self.exercise()

    def test_sigint_allows_child_finally(self):
        self.exercise(signal.SIGINT)

    def test_sigterm_allows_child_finally(self):
        self.exercise(signal.SIGTERM)

    def test_keyboard_interrupt_allows_child_finally(self):
        self.exercise(keyboard=True)

    def test_cleanup_timeout_preserves_live_child_identity(self):
        self.exercise(ignore_term=True)

    def test_success(self):
        with tempfile.TemporaryDirectory(prefix='mockforge-verifier-success-') as directory:
            receipt = {}
            result = MODULE.run_owned_verifier([sys.executable, '-c', 'print("done")'], Path(directory), receipt)
            self.assertEqual(result, 0)
            self.assertEqual(receipt['http_verifier_process']['state'], 'exited')
            self.assertEqual((Path(directory) / 'http.stdout').read_text().strip(), 'done')


if __name__ == '__main__':
    unittest.main()
