#!/usr/bin/env python3
"""Exercise the actual registry scope shell with production dependency inputs."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
import textwrap

ROOT = Path(__file__).resolve().parents[2]


class RegistrySecurityScope(unittest.TestCase):
    def run_filter(self, files, event="pull_request"):
        workflow = (ROOT / ".github/workflows/registry-e2e.yml").read_text()
        block = workflow.split("        id: filter\n", 1)[1].split("        run: |\n", 1)[1]
        lines = []
        for line in block.splitlines():
            if line.strip() and not line.startswith("          "):
                break
            lines.append(line)
        script = textwrap.dedent("\n".join(lines))
        with tempfile.TemporaryDirectory(prefix="registry-security-scope-") as directory:
            path = Path(directory)
            git = path / "git"
            git.write_text('#!/bin/sh\nprintf "%s\\n" "$CHANGED_FILES"\n')
            git.chmod(0o755)
            output = path / "output"
            env = dict(os.environ, PATH=str(path) + os.pathsep + os.environ["PATH"], CHANGED_FILES="\n".join(files), EVENT_NAME=event, BASE_REF="main", GITHUB_OUTPUT=str(output))
            subprocess.run(["bash", "-e", "-c", script], env=env, check=True, capture_output=True)
            return output.read_text().strip()

    def test_security_dependency_inputs_run_real_registry_validation(self):
        for filename in ("Cargo.lock", "Cargo.toml", "vendor/aws-sigv4/Cargo.toml", "vendor/aws-sigv4/src/sign/v4a.rs"):
            with self.subTest(filename=filename):
                self.assertEqual(self.run_filter([filename]), "registry=true")

    def test_registry_source_still_runs(self):
        self.assertEqual(self.run_filter(["crates/mockforge-registry-server/src/storage.rs"]), "registry=true")

    def test_unrelated_docs_still_skip(self):
        self.assertEqual(self.run_filter(["docs/development/README.md"]), "registry=false")

    def test_dispatch_still_runs(self):
        self.assertEqual(self.run_filter([], "workflow_dispatch"), "registry=true")


if __name__ == "__main__":
    unittest.main()
