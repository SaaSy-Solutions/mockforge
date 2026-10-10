"""Exercise publisher exit status offline, without uploads or credentials."""

import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/publish-crates.sh"


class PublishStatusTests(unittest.TestCase):
    def run_publisher(self, mode, dry_run=False):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "scripts").mkdir()
            shutil.copyfile(SCRIPT, root / "scripts/publish-crates.sh")
            (root / "Cargo.toml").write_text(
                '[workspace.package]\nversion = "0.3.232"\n'
            )
            crates = re.search(r"CRATES=\((.*?)\n\)", SCRIPT.read_text(), re.S)
            for name in crates.group(1).split():
                crate = root / "crates" / name
                crate.mkdir(parents=True)
                (crate / "Cargo.toml").write_text('[package]\nname = "fixture"\n')
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
                '#!/bin/sh\ncase " $* " in *" --dry-run "*) exit 0;; esac\n'
                'echo "fixture upload refused" >&2\nexit 1\n'
            )
            curl.chmod(0o700)
            cargo.chmod(0o700)
            env = dict(os.environ, PATH=f"{tools}:{os.environ['PATH']}")
            env["PUBLISH_TEST_MODE"] = mode
            for name in ("CARGO_REGISTRY_TOKEN", "CRATES_IO_TOKEN"):
                env.pop(name, None)
            args = ["bash", str(root / "scripts/publish-crates.sh")]
            if dry_run:
                args.append("--dry-run")
            return subprocess.run(
                args, cwd=root, env=env, capture_output=True, text=True, timeout=30
            )

    def test_all_published_returns_success(self):
        result = self.run_publisher("published")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("53/53 at 0.3.232", result.stdout)

    def test_incomplete_publication_returns_failure(self):
        result = self.run_publisher("stale")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("52/53 at 0.3.232", result.stdout)
        self.assertIn("publish failed", result.stdout)

    def test_registry_query_failure_returns_failure(self):
        result = self.run_publisher("query-failure")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("0/53 at 0.3.232", result.stdout)

    def test_dry_run_does_not_require_public_versions(self):
        result = self.run_publisher("stale", dry_run=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("DRY RUN", result.stdout)


if __name__ == "__main__":
    unittest.main()
