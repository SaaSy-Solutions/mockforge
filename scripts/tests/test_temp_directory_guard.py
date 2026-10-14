"""Causal allocation failures preserve existing files and close owned probes."""
import errno
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("temp_guard", ROOT / "scripts/ci/check_temp_directory.py")
GUARD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GUARD)


class TempDirectoryGuardTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.sentinel = self.root / "existing-cache"
        self.sentinel.write_bytes(b"foreign file must remain unchanged")

    def assert_preserved(self):
        self.assertEqual(self.sentinel.read_bytes(), b"foreign file must remain unchanged")
        self.assertEqual(list(self.root.iterdir()), [self.sentinel])

    def test_real_allocation_releases_owned_file_and_preserves_other_files(self):
        result = GUARD.inspect(self.root)
        self.assertEqual(result["errors"], [])
        self.assertEqual(result["allocated_bytes"], 1024 * 1024)
        self.assert_preserved()

    def test_user_quota_failure_after_partial_write_closes_owned_probe(self):
        real_write = GUARD.os.write
        descriptors = []

        def write(fd, data):
            descriptors.append(fd)
            if len(descriptors) == 2:
                raise OSError(errno.EDQUOT, "quota exceeded")
            return real_write(fd, data)

        with patch.object(GUARD.os, "write", side_effect=write):
            result = GUARD.inspect(self.root)
        self.assertEqual(result["allocated_bytes"], GUARD.CHUNK_BYTES)
        self.assertIn("EDQUOT", result["errors"][0])
        with self.assertRaises(OSError):
            GUARD.os.fstat(descriptors[0])
        self.assert_preserved()

    def test_fsync_space_failure_is_reported_and_owned_probe_closed(self):
        with patch.object(GUARD.os, "fsync", side_effect=OSError(errno.ENOSPC, "no space")):
            result = GUARD.inspect(self.root)
        self.assertIn("ENOSPC", result["errors"][0])
        self.assert_preserved()

    def test_short_writes_are_completed_without_claiming_unused_capacity(self):
        real_write = GUARD.os.write
        with patch.object(GUARD.os, "write", side_effect=lambda fd, data: real_write(fd, data[:4096])):
            result = GUARD.inspect(self.root)
        self.assertEqual(result["errors"], [])
        self.assertEqual(result["allocated_bytes"], GUARD.PROBE_BYTES)
        self.assert_preserved()

    def test_zero_write_is_an_error_instead_of_an_infinite_loop(self):
        with patch.object(GUARD.os, "write", return_value=0):
            result = GUARD.inspect(self.root)
        self.assertIn("EIO", result["errors"][0])
        self.assert_preserved()

    def test_missing_directory_does_not_create_or_repair_it(self):
        missing = self.root / "missing"
        result = GUARD.inspect(missing)
        self.assertIn("ENOENT", result["errors"][0])
        self.assertFalse(missing.exists())
        self.assert_preserved()


if __name__ == "__main__":
    unittest.main()
