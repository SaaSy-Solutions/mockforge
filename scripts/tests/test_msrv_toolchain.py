"""Verify the MSRV lane selects and checks the intended Rust toolchain."""

from pathlib import Path
import os
import subprocess
import tempfile
import unittest
import tomllib

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/ci.yml"


def msrv_job():
    workflow = WORKFLOW.read_text()
    return workflow.split("\n  msrv:\n", 1)[1].split("\n  typos:\n", 1)[0]


class MsrvToolchainTest(unittest.TestCase):
    def test_job_overrides_global_toolchain_and_installs_same_version(self):
        workflow = WORKFLOW.read_text()
        self.assertIn('RUSTUP_TOOLCHAIN: "1.96.0"', workflow.split("\njobs:\n", 1)[0])
        job = msrv_job()
        self.assertIn('RUSTUP_TOOLCHAIN: "1.96.0"', job)
        self.assertIn('toolchain: ${{ env.RUSTUP_TOOLCHAIN }}', job)
        self.assertIn("cargo check --workspace", job)
        self.assertIn("python3 scripts/tests/test_msrv_toolchain.py", job)

    def verify_versions(self, rustc_version, cargo_version):
        step = msrv_job().split("- name: Verify active MSRV toolchain\n", 1)[1]
        step = step.split("\n    - name:", 1)[0]
        script = step.split("      run: |\n", 1)[1]
        script = "\n".join(line.removeprefix("        ") for line in script.splitlines())
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for tool, version in (("rustc", rustc_version), ("cargo", cargo_version)):
                executable = root / tool
                executable.write_text(f"#!/bin/sh\nprintf '%s\\n' '{tool} {version} (test)'\n")
                executable.chmod(0o755)
            return subprocess.run(
                ["bash", "--noprofile", "--norc", "-e", "-o", "pipefail", "-c", script],
                env={**os.environ, "PATH": f"{root}:{os.environ['PATH']}", "RUSTUP_TOOLCHAIN": "1.96.0"},
                text=True,
                capture_output=True,
                check=False,
            )

    def test_intended_versions_pass_and_are_printed(self):
        result = self.verify_versions("1.96.0", "1.96.0")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("rustc 1.96.0", result.stdout)
        self.assertIn("cargo 1.96.0", result.stdout)

    def test_overridden_compiler_fails(self):
        result = self.verify_versions("1.98.0", "1.96.0")
        self.assertNotEqual(result.returncode, 0)

    def test_overridden_cargo_fails(self):
        result = self.verify_versions("1.96.0", "1.98.0")
        self.assertNotEqual(result.returncode, 0)

    def test_older_compiler_fails(self):
        result = self.verify_versions("1.91.1", "1.91.1")
        self.assertNotEqual(result.returncode, 0)

    def test_every_workspace_member_inherits_declared_minimum(self):
        workspace = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]
        self.assertEqual(workspace["package"]["rust-version"], "1.96.0")
        for member in workspace["members"]:
            manifest = tomllib.loads((ROOT / member / "Cargo.toml").read_text())
            with self.subTest(member=member):
                # Include private packages: publish=false does not exempt builds.
                self.assertEqual(manifest["package"]["rust-version"], {"workspace": True})

    def test_public_minimum_and_build_pin_stay_in_sync(self):
        toolchain = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())
        self.assertEqual(toolchain["toolchain"]["channel"], "1.96.0")
        for filename in (
            "README.md", "CONTRIBUTING.md", "docs/STABILITY_GUARANTEES.md",
            "book/src/reference/bench-capacity-sizing.md",
            "desktop-app/README.md", "ui-builder/README.md",
            "book/src/development/plugin-development.md",
        ):
            with self.subTest(filename=filename):
                text = (ROOT / filename).read_text()
                self.assertIn("1.96.0", text)
                self.assertNotIn("Rust 1.70", text)
                self.assertNotIn("**Rust**: 1.70", text)
                self.assertNotIn("| Rust | >= 1.74", text)

    def test_packaging_workflow_is_non_publishing(self):
        smoke = (ROOT / ".github/workflows/source-install-smoke.yml").read_text()
        self.assertIn('RUSTUP_TOOLCHAIN: "1.96.0"', smoke)
        self.assertIn("scripts/smoke-test-install.sh", smoke)
        self.assertIn("cargo package --locked --no-verify", smoke)
        self.assertIn("cargo metadata --locked --no-deps", smoke)
        self.assertNotIn("cargo publish", smoke)
        self.assertNotIn("contents: write", smoke)
        self.assertNotIn("packages: write", smoke)

    def test_install_smoke_has_a_fresh_compilation_directory(self):
        script = (ROOT / "scripts/smoke-test-install.sh").read_text()
        self.assertIn('--root "$TMP" --target-dir "$TMP/target" --force', script)


if __name__ == "__main__":
    unittest.main()
