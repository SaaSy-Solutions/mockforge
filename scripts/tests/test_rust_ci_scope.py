"""Guards for affected-package scope, persistent targets, mold, and nightly (#1449-#1451)."""

import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github/workflows"
CI = (WORKFLOWS / "ci.yml").read_text()
NIGHTLY = (WORKFLOWS / "nightly-rust.yml").read_text()
FULL = "--workspace --exclude mockforge-desktop"


def load(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / "scripts/ci" / (name + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


SCOPE = load("rust_scope")
TARGET = load("persistent_target")


def package(name, directory, deps=(), kinds=("lib",)):
    return {"id": name + " 0.1.0", "name": name,
            "manifest_path": "/w/" + directory + "/Cargo.toml",
            "targets": [{"kind": list(kinds)}],
            "dependencies": [{"name": dep, "path": "/w/x"} for dep in deps] + [{"name": "serde"}]}


METADATA = {
    "workspace_root": "/w",
    "packages": [
        package("core", "crates/core"),
        package("http", "crates/http", ["core"]),
        package("cli", "crates/cli", ["http"], kinds=("bin",)),
        package("test", "crates/test"),
        package("integration", "tests", ["test"]),
        package("leaf", "crates/leaf"),
        package("other", "crates/other"),
        package("more", "crates/more"),
        package("bin-only", "crates/bin-only", kinds=("bin",)),
        package("mockforge-desktop", "desktop-app", ["core"], kinds=("bin",)),
    ],
}
METADATA["workspace_members"] = [p["id"] for p in METADATA["packages"]]


def run_scope(files, metadata=METADATA, extra=()):
    with tempfile.TemporaryDirectory() as directory:
        meta = Path(directory, "metadata.json")
        meta.write_text(json.dumps(metadata) if isinstance(metadata, dict) else metadata)
        output = Path(directory, "output")
        result = subprocess.run(
            [sys.executable, str(ROOT / "scripts/ci/rust_scope.py"), "--metadata", str(meta), *extra],
            input="\n".join(files) + ("\n" if files else ""), text=True, capture_output=True,
            env={**os.environ, "GITHUB_OUTPUT": str(output)}, check=True)
        values = dict(line.split("=", 1) for line in output.read_text().splitlines())
        return values, result.stdout


class ScopeTest(unittest.TestCase):
    def setUp(self):
        # The fixture names stand in for the real binary edge packages.
        self.provider, self.resolver = SCOPE.BINARY_PROVIDER, SCOPE.BINARY_RESOLVER
        SCOPE.BINARY_PROVIDER, SCOPE.BINARY_RESOLVER = "cli", "test"
        self.addCleanup(setattr, SCOPE, "BINARY_PROVIDER", self.provider)
        self.addCleanup(setattr, SCOPE, "BINARY_RESOLVER", self.resolver)

    def decide(self, files):
        packages, by_dir, libs = SCOPE.load_graph("/w", json.dumps(METADATA))
        return SCOPE.decide(files, packages, by_dir, libs)

    def test_empty_diff_runs_everything_instead_of_skipping(self):
        for files in ([], [""], ["  "]):
            with self.subTest(files=files):
                result = self.decide(files)
                self.assertEqual((result["code"], result["scope"], result["cargo_args"]), ("true", "full", FULL))
        values, _ = run_scope([])
        self.assertEqual(values, {"code": "true", "scope": "full", "cargo_args": FULL})

    def test_docs_only_still_skips(self):
        result = self.decide(["docs/a.md", "book/src/x.md", ".claude/rules/y.md", "crates/core/README.md"])
        self.assertEqual(result["code"], "false")

    def test_leaf_change_selects_package_and_dependents(self):
        self.assertEqual(self.decide(["crates/leaf/src/lib.rs"])["cargo_args"], "-p leaf")
        result = self.decide(["crates/http/src/lib.rs", "docs/x.md"])
        self.assertEqual(result["scope"], "partial")
        # cli spawns nothing itself; integration depends on cli via the binary edge.
        self.assertEqual(result["cargo_args"], "-p cli -p http -p integration -p test")

    def test_desktop_is_never_selected(self):
        result = self.decide(["crates/core/src/lib.rs"])
        self.assertNotIn("mockforge-desktop", result["cargo_args"])

    def test_unattributable_paths_run_everything(self):
        for path in ("Cargo.lock", "Cargo.toml", "rust-toolchain.toml", ".github/workflows/ci.yml",
                     "scripts/ci/rust_scope.py", "fixtures/spec.json", "proto/a.proto", "crates/README-only/x.rs"):
            with self.subTest(path=path):
                result = self.decide(["crates/leaf/src/lib.rs", path])
                self.assertEqual((result["scope"], result["cargo_args"]), ("full", FULL))

    def test_bin_only_and_wide_selections_run_everything(self):
        self.assertEqual(self.decide(["crates/bin-only/src/main.rs"])["scope"], "full")
        self.assertEqual(self.decide(["crates/core/src/lib.rs"])["scope"], "partial")
        threshold = SCOPE.FULL_THRESHOLD
        self.addCleanup(setattr, SCOPE, "FULL_THRESHOLD", threshold)
        SCOPE.FULL_THRESHOLD = 0.4  # core + http + cli + integration = 4 of 9
        self.assertEqual(self.decide(["crates/core/src/lib.rs"])["scope"], "full")

    def test_broken_graph_and_hostile_names_fail_closed(self):
        values, _ = run_scope(["crates/leaf/src/lib.rs"], metadata="not json")
        self.assertEqual(values["scope"], "full")
        hostile = json.loads(json.dumps(METADATA))
        hostile["packages"][0]["name"] = "core; rm -rf /"
        values, _ = run_scope(["crates/leaf/src/lib.rs"], metadata=hostile)
        self.assertEqual((values["code"], values["scope"]), ("true", "full"))
        root_package = json.loads(json.dumps(METADATA))
        root_package["packages"][0]["manifest_path"] = "/w/Cargo.toml"
        values, _ = run_scope(["crates/leaf/src/lib.rs"], metadata=root_package)
        self.assertEqual(values["scope"], "full")

    def test_full_flag_for_non_pr_events(self):
        values, _ = run_scope(["crates/leaf/src/lib.rs"], extra=("--full",))
        self.assertEqual(values["scope"], "full")


class RealWorkspaceTest(unittest.TestCase):
    def test_real_workspace_graph_is_readable(self):
        packages, by_dir, libs = SCOPE.load_graph(str(ROOT), None)
        self.assertIn("mockforge-core", packages)
        self.assertIn("mockforge-cli", packages["mockforge-integration-tests"])
        result = SCOPE.decide(["crates/mockforge-tcp/src/lib.rs"], packages, by_dir, libs)
        self.assertEqual(result["scope"], "partial")
        self.assertIn("-p mockforge-tcp", result["cargo_args"])
        self.assertIn("-p mockforge-cli", result["cargo_args"])


class WorkflowWiringTest(unittest.TestCase):
    def changes_step(self):
        changes = CI.split("\n  changes:\n", 1)[1].split("\n  test:\n", 1)[0]
        return changes.split("      - name: Determine whether non-docs files changed\n", 1)[1]

    def test_changes_step_uses_scope_script_and_fails_closed(self):
        step = self.changes_step()
        self.assertIn("set -euo pipefail", step)
        self.assertIn("python3 scripts/ci/rust_scope.py --full", step)
        self.assertIn('printf \'%s\\n\' "$files" | python3 scripts/ci/rust_scope.py', step)
        self.assertNotIn("code=false", step)
        for output in ("code: ${{ steps.filter.outputs.code }}", "cargo_args: ${{ steps.filter.outputs.cargo_args }}"):
            self.assertIn(output, CI)
        self.assertIn("-p test_rust_ci_scope.py", CI)

    def test_changes_step_runs_everything_on_empty_pull_request_diff(self):
        script = textwrap.dedent(self.changes_step().split("        run: |\n", 1)[1].split("\n\n", 1)[0])
        with tempfile.TemporaryDirectory() as directory:
            bin_dir = Path(directory, "bin")
            bin_dir.mkdir()
            git = bin_dir / "git"
            git.write_text("#!/bin/sh\nexit 0\n")  # empty diff
            git.chmod(0o700)
            output = Path(directory, "output")
            subprocess.run(["bash", "-c", script], cwd=ROOT, check=True, capture_output=True, text=True,
                           env={**os.environ, "PATH": str(bin_dir) + os.pathsep + os.environ["PATH"],
                                "EVENT_NAME": "pull_request", "BASE_REF": "main", "GITHUB_OUTPUT": str(output)})
            self.assertIn("code=true", output.read_text())
            self.assertIn("scope=full", output.read_text())
            git.write_text("#!/bin/sh\nexit 3\n")  # broken diff fails the job
            failed = subprocess.run(["bash", "-c", script], cwd=ROOT, capture_output=True, text=True,
                                    env={**os.environ, "PATH": str(bin_dir) + os.pathsep + os.environ["PATH"],
                                         "EVENT_NAME": "pull_request", "BASE_REF": "main", "GITHUB_OUTPUT": str(output)})
            self.assertNotEqual(failed.returncode, 0)

    def test_rust_jobs_use_persistent_target_and_mold_not_rust_cache(self):
        self.assertNotIn("Swatinem/rust-cache", CI)
        self.assertNotIn("\n  warning-ratchet-preview:\n", CI)
        for job in ("test", "warning-gate", "msrv"):
            content = re.search(r"(?ms)^  " + re.escape(job) + r":\n(.*?)(?=^  [a-z][a-z0-9_-]*:\n|\Z)", CI).group(1)
            with self.subTest(job=job):
                step = content.split("name: Use the runner's persistent Cargo target dir on Hetzner", 1)[1].split("- name:", 1)[0]
                self.assertIn("if: runner.environment == 'self-hosted'", step)
                self.assertIn("run: python3 scripts/ci/persistent_target.py", step)
                self.assertIn("uses: rui314/setup-mold@v1", content)
                self.assertIn("run: bash scripts/ci/setup_mold.sh", content)
                for flag in ('CARGO_INCREMENTAL: "0"', 'CARGO_BUILD_JOBS: "4"',
                             'CARGO_PROFILE_DEV_DEBUG: "0"', 'CARGO_PROFILE_TEST_DEBUG: "0"'):
                    self.assertIn(flag, content)

    def test_warning_gate_runs_gate_and_preview_in_one_job(self):
        gate = (ROOT / "scripts/lint-warning-gate.sh").read_text()
        self.assertIn("--force-warn \"$PREVIEW_LINT\"", gate)
        self.assertIn("-Dunused_qualifications", gate)
        self.assertNotIn("RUSTFLAGS=", gate)
        self.assertIn("run: bash scripts/lint-warning-gate.sh", CI)
        self.assertNotIn("lint-warning-ratchet-preview.sh", CI)

    def test_nightly_runs_full_suite_on_existing_pool_off_peak(self):
        self.assertIn("runs-on: [self-hosted, linux, x64, rust]", NIGHTLY)
        self.assertNotIn("ubuntu-latest", NIGHTLY)
        self.assertNotIn("pull_request", NIGHTLY)
        self.assertIn("cron: '23 7 * * *'", NIGHTLY)
        self.assertIn("github.ref == 'refs/heads/main'", NIGHTLY)
        self.assertIn("cancel-in-progress: false", NIGHTLY)
        self.assertRegex(NIGHTLY, r"(?m)^    timeout-minutes: [1-9][0-9]*$")
        self.assertIn("run: cargo test --workspace --exclude mockforge-desktop --verbose", NIGHTLY)
        self.assertIn("MOCKFORGE_TEST_BINARY: ${{ runner.temp }}/mockforge-all-protocols", NIGHTLY)
        for forbidden in ("sudo ", "apt-get", "rustup default", "secrets."):
            self.assertNotIn(forbidden, NIGHTLY)


class PersistentTargetTest(unittest.TestCase):
    def test_runner_name_is_validated(self):
        self.assertEqual(TARGET.target_for("saasy-ci-runner-01"), Path("/tmp/mockforge-target-saasy-ci-runner-01"))
        for bad in ("", "../x", "a/b", "a b", "-x"):
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                TARGET.target_for(bad)

    def test_reuses_within_budget_and_resets_only_its_own_dir(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            sibling = root / "mockforge-target-other"
            sibling.mkdir()
            (sibling / "keep").write_bytes(b"x" * 8192)
            target = TARGET.target_for("runner-1", root)
            self.assertEqual(TARGET.prepare(target, 10 * TARGET.GIB, 0), "reused")
            (target / "artifact").write_bytes(b"x" * 8192)
            self.assertEqual(TARGET.prepare(target, 10 * TARGET.GIB, 0), "reused")
            self.assertTrue((target / "artifact").exists())
            self.assertIn("budget", TARGET.prepare(target, 1024, 0))
            self.assertTrue(target.is_dir())
            self.assertFalse((target / "artifact").exists())
            self.assertTrue((sibling / "keep").exists())

    def test_refuses_symlinked_target(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            elsewhere = root / "elsewhere"
            elsewhere.mkdir()
            (elsewhere / "precious").write_text("x")
            target = TARGET.target_for("runner-1", root)
            target.symlink_to(elsewhere, target_is_directory=True)
            with self.assertRaises(ValueError):
                TARGET.prepare(target, 0, 0)
            self.assertTrue((elsewhere / "precious").exists())


class MoldSetupTest(unittest.TestCase):
    def test_missing_mold_keeps_default_linker_without_failing(self):
        with tempfile.TemporaryDirectory() as directory:
            bin_dir = Path(directory, "bin")
            bin_dir.mkdir()
            for tool in ("mktemp", "rm"):
                real = subprocess.run(["bash", "-c", "command -v " + tool], text=True, capture_output=True, check=True)
                (bin_dir / tool).symlink_to(real.stdout.strip())
            env_file = Path(directory, "env")
            env_file.write_text("")
            result = subprocess.run(["/bin/bash", str(ROOT / "scripts/ci/setup_mold.sh")],
                                    text=True, capture_output=True,
                                    env={"PATH": str(bin_dir), "GITHUB_ENV": str(env_file), "RUNNER_TEMP": directory})
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("mold is not installed", result.stdout)
            self.assertEqual(env_file.read_text(), "")


if __name__ == "__main__":
    unittest.main()
