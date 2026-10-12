"""Causal cache/pressure fixtures preserve caches and symlink destinations."""
import importlib.util
from pathlib import Path
import shutil
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("guard", ROOT / "scripts/ci/check_rust_cache.py")
GUARD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GUARD)


class CacheGuardTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.disk = patch.object(GUARD.shutil, "disk_usage", return_value=SimpleNamespace(free=80 * GUARD.GIB))
        self.disk.start()
        self.addCleanup(self.disk.stop)

    def fingerprint(self, profile="debug", crate="example", suffix=".rlib"):
        base = self.root / "target" / profile
        fingerprint = base / ".fingerprint" / (crate + "-" + "a" * 16)
        fingerprint.mkdir(parents=True)
        (fingerprint / ("lib-" + crate)).write_text("fingerprint")
        artifact = base / "deps" / ("lib" + crate.replace("-", "_") + "-" + "a" * 16 + suffix)
        artifact.parent.mkdir(parents=True)
        artifact.write_bytes(b"owned artifact")
        return artifact

    def test_missing_target_and_intact_library(self):
        self.assertEqual(GUARD.inspect(self.root)["errors"], [])
        artifact = self.fingerprint()
        self.assertEqual(GUARD.inspect(self.root)["errors"], [])
        self.assertEqual(artifact.read_bytes(), b"owned artifact")

    def test_removed_or_empty_artifact_is_rejected_without_repair(self):
        artifact = self.fingerprint()
        artifact.write_bytes(b"")
        self.assertTrue(GUARD.inspect(self.root)["errors"])
        self.assertTrue(artifact.exists())
        artifact.unlink()
        self.assertTrue(GUARD.inspect(self.root)["errors"])
        self.assertFalse(artifact.exists())

    def test_check_metadata_proc_macro_and_cross_target_profiles(self):
        self.fingerprint("debug", "metadata", ".rmeta")
        self.fingerprint("release", "macro", ".so")
        self.fingerprint("x86_64-unknown-linux-gnu/debug", "cross", ".rlib")
        result = GUARD.inspect(self.root)
        self.assertEqual(result["library_fingerprints"], 3)
        self.assertEqual(result["errors"], [])

    def test_declared_rlib_missing_with_metadata_present_is_rejected(self):
        artifact = self.fingerprint()
        metadata = artifact.with_suffix(".rmeta")
        metadata.write_bytes(b"metadata")
        dep_info = artifact.parent / ("example-" + "a" * 16 + ".d")
        dep_info.write_text("/old/runner/deps/" + artifact.name + ": source.rs\n/old/runner/deps/" + metadata.name + ": source.rs\n")
        self.assertEqual(GUARD.inspect(self.root)["errors"], [])
        artifact.unlink()
        self.assertTrue(GUARD.inspect(self.root)["errors"])
        self.assertEqual(metadata.read_bytes(), b"metadata")

    def test_unhashed_dynamic_library_and_symlinked_deps(self):
        artifact = self.fingerprint(suffix=".so")
        artifact.rename(artifact.parent / "libexample.so")
        self.assertEqual(GUARD.inspect(self.root)["errors"], [])
        deps = artifact.parent
        foreign = self.root / "foreign"
        deps.rename(foreign)
        deps.symlink_to(foreign, target_is_directory=True)
        self.assertTrue(GUARD.inspect(self.root)["errors"])
        self.assertTrue((foreign / "libexample.so").exists())

    def test_low_headroom_does_not_remove_healthy_cache(self):
        artifact = self.fingerprint()
        with patch.object(GUARD.shutil, "disk_usage", return_value=SimpleNamespace(free=GUARD.GIB)):
            self.assertTrue(GUARD.inspect(self.root)["errors"])
        self.assertEqual(artifact.read_bytes(), b"owned artifact")

    def test_target_and_nested_symlinks_preserve_foreign_contents(self):
        foreign = self.root / "foreign"
        foreign.mkdir()
        sentinel = foreign / "sentinel"
        sentinel.write_text("foreign")
        target = self.root / "target"
        target.symlink_to(foreign, target_is_directory=True)
        self.assertTrue(GUARD.inspect(self.root)["errors"])
        target.unlink()
        target.mkdir()
        (target / "nested").symlink_to(foreign, target_is_directory=True)
        self.assertTrue(GUARD.inspect(self.root)["errors"])
        self.assertEqual(sentinel.read_text(), "foreign")

    def test_artifact_symlink_and_non_directory_target(self):
        artifact = self.fingerprint()
        foreign = self.root / "foreign-artifact"
        foreign.write_bytes(b"foreign")
        artifact.unlink()
        artifact.symlink_to(foreign)
        self.assertTrue(GUARD.inspect(self.root)["errors"])
        self.assertEqual(foreign.read_bytes(), b"foreign")
        shutil.rmtree(self.root / "target")
        (self.root / "target").write_text("keep")
        self.assertTrue(GUARD.inspect(self.root)["errors"])
        self.assertEqual((self.root / "target").read_text(), "keep")

    def test_bin_and_incomplete_fingerprints_do_not_claim_missing_libraries(self):
        directory = self.root / "target/debug/.fingerprint/example-aaaaaaaaaaaaaaaa"
        directory.mkdir(parents=True)
        (directory / "bin-example").write_text("binary pruned by rust-cache")
        (directory / "lib-example.json").write_text("{}")
        self.assertEqual(GUARD.inspect(self.root)["errors"], [])


if __name__ == "__main__":
    unittest.main()
