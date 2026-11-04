"""Exercise the one-time extraction proof against isolated source mutations."""

from contextlib import redirect_stdout
import importlib.util
import io
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "protocol_extraction", ROOT / "scripts/dev/verify_protocol_extraction.py"
)
proof = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(proof)


class ProtocolExtractionTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.trees = {}
        paths = {f"{proof.CORE}/{path}" for path in proof.FILES}
        paths.update(f"{proof.CORE}/{path}" for path in (
            "lib.rs", "protocol_abstraction/auth.rs", "protocol_abstraction/middleware.rs"
        ))
        paths.add("crates/mockforge-core/Cargo.toml")
        cls.current = {str(path.relative_to(ROOT)): path.read_text()
                       for path in (ROOT / proof.PROTOCOL).rglob("*.rs")}
        cls.current["crates/mockforge-protocol/Cargo.toml"] = (
            ROOT / "crates/mockforge-protocol/Cargo.toml"
        ).read_text()
        for package in ("mockforge-tcp", "mockforge-smtp"):
            prefix = f"crates/{package}"
            files = subprocess.check_output(
                ["git", "ls-tree", "-r", "--name-only", proof.BASELINE, "--", prefix],
                cwd=ROOT, text=True,
            )
            cls.trees[prefix] = files
            paths.update(path for path in files.splitlines() if path.endswith(".rs"))
            paths.add(f"{prefix}/Cargo.toml")
        cls.baseline = {path: proof.original(path) for path in paths}
        cls.current.update({path: (ROOT / path).read_text() for path in paths
                            if path not in {f"{proof.CORE}/{p}" for p in proof.FILES}})
        for path in proof.FILES:
            relative = f"{proof.CORE}/{path}"
            if (ROOT / relative).exists():
                cls.current[relative] = (ROOT / relative).read_text()

    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="protocol-proof-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        for path, text in self.current.items():
            destination = self.root / path
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_text(text)
        self.enterContext(patch.object(proof, "ROOT", self.root))
        self.enterContext(patch.object(proof, "original", self.baseline.__getitem__))
        self.enterContext(patch.object(
            proof.subprocess, "check_output", side_effect=lambda command, **kwargs:
            self.trees[command[-1]],
        ))

    def replace(self, path, before, after):
        destination = self.root / path
        text = destination.read_text()
        self.assertEqual(text.count(before), 1)
        destination.write_text(text.replace(before, after))

    def test_accepted_extraction_passes(self):
        with redirect_stdout(io.StringIO()):
            proof.main()

    def test_fixture_behavior_change_fails(self):
        self.replace(f"{proof.PROTOCOL}/fixture_store.rs",
                     "if !dir.exists() {", "if dir.exists() {")
        with self.assertRaisesRegex(SystemExit, "Source identity mismatch"):
            proof.check_moves()

    def test_missing_facade_export_fails(self):
        self.replace(f"{proof.CORE}/fixture_store.rs",
                     "pub use mockforge_protocol::fixture_store::*;", "")
        with self.assertRaisesRegex(SystemExit, "Source identity mismatch"):
            proof.check_core_wiring()

    def test_core_feature_drift_fails(self):
        self.replace("crates/mockforge-core/Cargo.toml",
                     'default = ["data", "ai", "contracts", "workspace-mgmt", "voice", "advanced", "scripting"]',
                     'default = ["data", "ai", "contracts", "workspace-mgmt", "voice", "advanced"]')
        with self.assertRaisesRegex(SystemExit, "Unexpected core manifest or feature changes"):
            proof.check_core_wiring()

    def test_transport_loses_error_feature_fails(self):
        self.replace("crates/mockforge-tcp/Cargo.toml",
                     'features = ["scripting"]', 'features = []')
        with self.assertRaisesRegex(SystemExit, "Transport feature contract changed"):
            proof.check_transports()

    def test_transport_source_inventory_change_fails(self):
        for operation in ("add", "remove"):
            with self.subTest(operation=operation):
                if operation == "add":
                    path = self.root / "crates/mockforge-tcp/src/unreviewed.rs"
                    path.write_text("pub fn new_behavior() {}\n")
                else:
                    relative = next(path for path in self.current
                                    if path.startswith("crates/mockforge-tcp/")
                                    and path.endswith(".rs"))
                    path = self.root / relative
                    previous = path.read_text()
                    path.unlink()
                with self.assertRaisesRegex(SystemExit, "Transport source inventory changed"):
                    proof.check_transports()
                if operation == "add":
                    path.unlink()
                else:
                    path.write_text(previous)

    def test_target_alias_back_edges_fail(self):
        path = self.root / "crates/mockforge-protocol/Cargo.toml"
        original = path.read_text()
        for section in ("dependencies", "dev-dependencies", "build-dependencies"):
            with self.subTest(section=section):
                path.write_text(original + f'\n[target.\'cfg(unix)\'.{section}]\n'
                                'innocent_alias = { package = "mockforge-core", version = "*" }\n')
                with self.assertRaisesRegex(SystemExit, "Forbidden protocol back edge"):
                    proof.check_protocol_manifest()


if __name__ == "__main__":
    unittest.main()
