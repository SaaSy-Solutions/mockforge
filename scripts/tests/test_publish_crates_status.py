"""Exercise publisher exit status offline, without uploads or credentials."""

import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/publish-crates.sh"
# The publisher reports "<published>/<total>"; derive the total from its own
# CRATES list so adding a crate does not break these tests.
CRATES = re.search(r"CRATES=\((.*?)\n\)", SCRIPT.read_text(), re.S).group(1).split()
TOTAL = len(CRATES)


class PublishStatusTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Exercise the real ordering validator against the actual workspace graph,
        # while keeping publication and registry queries fully stubbed below.
        cls.metadata = json.loads(subprocess.check_output(
            ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"], cwd=ROOT,
        ))

    def run_publisher(self, mode, dry_run=False, stale_bundle=False):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "scripts").mkdir()
            shutil.copyfile(SCRIPT, root / "scripts/publish-crates.sh")
            shutil.copyfile(ROOT / "scripts/check_publish_order.py", root / "scripts/check_publish_order.py")
            metadata = root / "metadata.json"
            metadata.write_text(json.dumps(self.metadata))
            (root / "Cargo.toml").write_text(
                '[workspace.package]\nversion = "0.3.232"\n'
            )
            for name in CRATES:
                crate = root / "crates" / name
                crate.mkdir(parents=True)
                (crate / "Cargo.toml").write_text('[package]\nname = "fixture"\n')
            shutil.copyfile(ROOT / "scripts/bundle_cli_tools.py", root / "scripts/bundle_cli_tools.py")
            bundle_path = Path("crates/mockforge-cli/src/bundled_tools.json")
            bundled = json.loads((ROOT / bundle_path).read_text())
            for name in bundled:
                source = ROOT / name
                if source.is_file():
                    target = root / name
                    target.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(source, target)
            (root / bundle_path).parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / bundle_path, root / bundle_path)
            if stale_bundle:
                (root / "scripts/export_github_issue.py").write_text("# changed after bundling\n")
            tools = root / "tools"
            tools.mkdir()
            curl = tools / "curl"
            curl.write_text(
                "#!/usr/bin/env python3\nimport json, os, sys\n"
                'if os.environ["PUBLISH_TEST_MODE"] == "query-failure":\n'
                '    print("unavailable"); sys.exit(0)\n'
                'stale = os.environ["PUBLISH_TEST_MODE"] == "stale" '
                'and sys.argv[-1].endswith("/mockforge-cli")\n'
                'print(json.dumps({"crate": {"max_version": '
                '"0.3.231" if stale else "0.3.232"}}))\n'
            )
            cargo = tools / "cargo"
            cargo.write_text(
                '#!/bin/sh\ncase " $* " in\n'
                '  *" metadata "*) cat "$PUBLISH_TEST_METADATA"; exit $?;;\n'
                '  *" --dry-run "*) exit 0;;\nesac\n'
                'echo "fixture upload refused" >&2\nexit 1\n'
            )
            curl.chmod(0o700)
            cargo.chmod(0o700)
            env = dict(os.environ, PATH=f"{tools}:{os.environ['PATH']}")
            env["PUBLISH_TEST_MODE"] = mode
            env["PUBLISH_TEST_METADATA"] = str(metadata)
            for name in ("CARGO_REGISTRY_TOKEN", "CRATES_IO_TOKEN"):
                env.pop(name, None)
            args = ["bash", str(root / "scripts/publish-crates.sh")]
            if dry_run:
                args.append("--dry-run")
            return subprocess.run(
                args, cwd=root, env=env, capture_output=True, text=True, timeout=30
            )

    def test_stale_tools_prevent_publication(self):
        result = self.run_publisher("published", stale_bundle=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("CLI tools bundle is stale", result.stderr)
        self.assertNotIn("=== mockforge", result.stdout)

    def test_all_published_returns_success(self):
        result = self.run_publisher("published")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(f"{TOTAL}/{TOTAL} at 0.3.232", result.stdout)

    def test_incomplete_publication_returns_failure(self):
        result = self.run_publisher("stale")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(f"{TOTAL - 1}/{TOTAL} at 0.3.232", result.stdout)
        self.assertIn("publish failed", result.stdout)

    def test_registry_query_failure_returns_failure(self):
        result = self.run_publisher("query-failure")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(f"0/{TOTAL} at 0.3.232", result.stdout)

    def test_dry_run_does_not_require_public_versions(self):
        result = self.run_publisher("stale", dry_run=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("DRY RUN", result.stdout)


if __name__ == "__main__":
    unittest.main()
