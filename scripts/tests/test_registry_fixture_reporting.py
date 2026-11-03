"""Exercise the workflow's stale-fixture reporting without a Docker daemon."""

import os
from pathlib import Path
import re
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[2]


class FixtureReportingTest(unittest.TestCase):
    def run_report(self, listing_fails=False):
        workflow = (ROOT / ".github/workflows/registry-e2e.yml").read_text()
        step = re.search(
            r"(?ms)^      - name: (?:Reap|Report) fixtures left by completed runs\n(.*?)(?=^      - name:)",
            workflow,
        ).group(1)
        script = textwrap.dedent(step.split("        run: |\n", 1)[1])
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            log = root / "docker.log"
            docker = root / "docker"
            docker.write_text(
                '#!/bin/bash\nprintf "%s\\n" "$*" >> "$MOCK_DOCKER_LOG"\n'
                '[ "$*" = "compose ls -a -q" ] || exit 99\n'
                '[ "$MOCK_LIST_FAIL" = 0 ] || exit 1\n'
                'printf "%s\\n" mf-e2e-101-1-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa '
                'mf-e2e-102-1-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb '
                'mf-e2e-103-1-cccccccccccccccccccccccccccccccc '
                'mf-e2e-104-1-dddddddddddddddddddddddddddddddd unrelated\n'
            )
            gh = root / "gh"
            gh.write_text(
                '#!/bin/bash\ncase "$2" in\n'
                '*/101) echo completed ;;\n'
                '*/102) echo in_progress ;;\n'
                '*) exit 1 ;;\nesac\n'
            )
            docker.chmod(0o700)
            gh.chmod(0o700)
            result = subprocess.run(
                ["bash", "-e", "-o", "pipefail", "-c", script],
                env={**os.environ, "PATH": f"{root}:{os.defpath}",
                     "MOCK_DOCKER_LOG": str(log),
                     "MOCK_LIST_FAIL": str(int(listing_fails)),
                     "GH_REPO": "SaaSy-Solutions/mockforge", "GITHUB_RUN_ID": "104"},
                text=True, capture_output=True, timeout=10,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(log.read_text().splitlines(), ["compose ls -a -q"])
            return result.stdout

    def test_only_completed_foreign_runs_are_reported_without_deletion(self):
        output = self.run_report()
        self.assertIn("mf-e2e-101-1-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", output)
        for run in ("102", "103", "104"):
            self.assertNotIn(f"mf-e2e-{run}-", output)
        self.assertIn("retained", output)

    def test_listing_failure_is_non_destructive(self):
        self.assertIn("cannot list", self.run_report(listing_fails=True))


if __name__ == "__main__":
    unittest.main()
