"""Exercise the exact argument ordering used by the installed source hooks."""

from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "precommit/offline_checks.py"


class OfflineChecksTests(unittest.TestCase):
    def test_large_file_guard_accepts_files_after_configured_option(self):
        with tempfile.TemporaryDirectory() as directory:
            small = Path(directory) / "small.txt"
            large = Path(directory) / "large.txt"
            small.write_bytes(b"x" * 1024)
            large.write_bytes(b"x" * 1025)
            command = [sys.executable, str(SCRIPT), "check-added-large-files", "--maxkb", "1"]
            accepted = subprocess.run(command + [str(small)], capture_output=True, text=True)
            self.assertEqual(accepted.returncode, 0, accepted.stderr)
            rejected = subprocess.run(command + [str(small), str(large)], capture_output=True, text=True)
            self.assertEqual(rejected.returncode, 1, rejected.stderr)
            self.assertIn(f"large file: {large}", rejected.stdout)


if __name__ == "__main__":
    unittest.main()
