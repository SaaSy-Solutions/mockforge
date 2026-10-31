"""Paired measurements retain the real comparison gate and immutable sources."""

import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/ci"))
SPEC = importlib.util.spec_from_file_location(
    "paired", ROOT / "scripts/ci/paired_benchmarks.py"
)
PAIRED = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PAIRED)
import benchmark_measurements as MEASUREMENTS


class PairedBenchmarkTest(unittest.TestCase):
    def exercise(self, candidate, extra=False):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output = root / "results"
            calls = []

            def build(checkout, target):
                binary = root / "shared-cargo-executable"
                binary.write_text("base" if checkout.name == "source" else "head")
                return binary

            def names(binary, checkout):
                return (
                    ["example", "new_case"]
                    if extra and binary.name == "head"
                    else ["example"]
                )

            def measure(binary, checkout, target, name, label):
                self.assertEqual(binary.read_text(), binary.name)
                calls.append((name, binary.name))
                return 100 if binary.name == "base" else candidate

            with (
                patch.object(
                    PAIRED.subprocess, "check_output", side_effect=["a" * 40, "b" * 40]
                ),
                patch.object(PAIRED.subprocess, "run"),
                patch.object(PAIRED, "build", side_effect=build),
                patch.object(PAIRED, "names", side_effect=names),
                patch.object(PAIRED, "measure", side_effect=measure),
            ):
                PAIRED.run(root, "a" * 40, output)
            # Execute the existing comparator, including its unchanged 5% limit.
            compared = subprocess.run(
                [
                    "node",
                    str(ROOT / ".github/scripts/compare-benchmarks.js"),
                    "compare",
                ],
                env={
                    **os.environ,
                    "BASELINE_DIR": str(output),
                    "CRITERION_DIR": str(output / "criterion"),
                    "REPORT_PATH": str(root / "report.md"),
                    "REGRESSION_THRESHOLD": "5.0",
                },
                capture_output=True,
                text=True,
            )
            return (
                calls,
                compared.returncode,
                json.loads((output / "sources.json").read_text()),
                (root / "report.md").read_text(),
            )

    def test_stable_pair_passes_and_balances_order(self):
        calls, status, sources, report = self.exercise(104)
        self.assertEqual(
            calls, [("example", side) for side in ["base", "head", "head", "base"]]
        )
        self.assertEqual(status, 0)
        self.assertEqual(sources["base_sha"], "a" * 40)
        self.assertEqual(sources["head_sha"], "b" * 40)
        self.assertRegex(sources["base_binary_sha256"], r"^[0-9a-f]{64}$")
        self.assertRegex(sources["head_binary_sha256"], r"^[0-9a-f]{64}$")
        self.assertIn("**Regressions**: 0", report)

    def test_real_twenty_percent_regression_still_fails(self):
        _, status, _, report = self.exercise(120)
        self.assertEqual(status, 1)
        self.assertIn("+20.00%", report)

    def test_new_case_is_measured_without_fabricating_a_baseline(self):
        calls, status, _, report = self.exercise(104, extra=True)
        self.assertEqual(
            [side for name, side in calls if name == "new_case"], ["head", "head"]
        )
        self.assertEqual(status, 0)
        self.assertIn("**New**: 1", report)

    def test_incomplete_measurement_fails_closed(self):
        with self.assertRaises(ValueError):
            PAIRED.aggregate({"example": [100]})

    def test_cached_timing_cannot_replace_a_missing_measurement(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            estimates = root / "criterion/example/paired-head-1/estimates.json"
            estimates.parent.mkdir(parents=True)
            estimates.write_text(json.dumps({"mean": {"point_estimate": 100}}))
            with (
                patch.object(PAIRED.subprocess, "run"),
                self.assertRaises(FileNotFoundError),
            ):
                PAIRED.measure(Path("unused"), root, root, "example", "paired-head-1")
            self.assertFalse(estimates.exists())

    def test_moving_reference_or_shell_input_is_rejected(self):
        for revision in ("main", "a" * 39, "a" * 40 + "; exit 0"):
            with self.subTest(revision=revision), self.assertRaises(ValueError):
                PAIRED.run(Path.cwd(), revision, Path("unused"))

    def test_workflow_uses_event_base_and_paired_results(self):
        workflow = (ROOT / ".github/workflows/benchmarks.yml").read_text()
        self.assertIn(
            "BENCHMARK_BASE_SHA: ${{ github.event.pull_request.base.sha }}", workflow
        )
        self.assertIn("BASELINE_DIR: paired-benchmarks", workflow)
        self.assertIn("CRITERION_DIR: paired-benchmarks/criterion", workflow)
        self.assertNotIn("- name: Fetch baseline", workflow)
        self.assertIn("REGRESSION_THRESHOLD: 5.0", workflow)
        # Hetzner for trusted events, hosted for fork PRs (#1191).
        self.assertIn(
            "runs-on: ${{ (github.event_name != 'pull_request' || "
            "github.event.pull_request.head.repo.full_name == github.repository) && "
            "fromJSON('[\"self-hosted\",\"linux\",\"x64\",\"rust\"]') || 'ubuntu-latest' }}",
            workflow,
        )


class CargoSourceFreshnessTest(unittest.TestCase):
    def checkout(self, root, marker):
        (root / "src").mkdir(parents=True)
        (root / "benches").mkdir()
        (root / "Cargo.toml").write_text(
            '[package]\nname="mockforge-core"\nversion="0.0.0"\nedition="2021"\n'
            '[workspace]\n[[bench]]\nname="core_benchmarks"\nharness=false\n'
        )
        (root / "Cargo.lock").write_text(
            'version = 4\n\n[[package]]\nname = "mockforge-core"\nversion = "0.0.0"\n'
        )
        (root / "src/lib.rs").write_text(f"pub fn marker() -> u32 {{ {marker} }}\n")
        (root / "benches/core_benchmarks.rs").write_text(
            'fn main() { println!("{}", mockforge_core::marker()); }\n'
        )
        for args in (["git", "init", "-q"], ["git", "add", "."]):
            subprocess.run(args, cwd=root, check=True, capture_output=True)

    @unittest.skipUnless(
        shutil.which("cargo") and shutil.which("rustc") and shutil.which("cc"),
        "Cargo and a C linker are required for the isolated build regression",
    )
    def test_older_candidate_sources_do_not_reuse_base_executable(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            base, head, target = root / "base", root / "head", root / "target"
            self.checkout(base, 1)
            self.checkout(head, 2)
            for source in head.rglob("*.rs"):
                os.utime(source, (time.time() - 3600, time.time() - 3600))
            host = subprocess.check_output(["rustc", "-vV"], text=True)
            triple = next(line.removeprefix("host: ") for line in host.splitlines()
                          if line.startswith("host: "))
            # Keep the fixture independent of project-specific linker flags
            # and compiler wrappers inherited from a developer's environment.
            env = {
                "RUSTFLAGS": "", "CARGO_ENCODED_RUSTFLAGS": "",
                "RUSTC_WRAPPER": "", "RUSTC_WORKSPACE_WRAPPER": "",
                "RUSTUP_TOOLCHAIN": os.environ.get("RUSTUP_TOOLCHAIN")
                    or tomllib.loads((ROOT / "rust-toolchain.toml").read_text())["toolchain"]["channel"],
                "CARGO_BUILD_TARGET": triple,
                "CARGO_TARGET_" + triple.upper().replace("-", "_") + "_LINKER":
                    shutil.which("cc"),
            }
            with patch.dict(os.environ, env):
                before = (head / "src/lib.rs").read_bytes()
                base_copy = root / "base-binary"
                shutil.copy2(MEASUREMENTS.build(base, target), base_copy)
                candidate = MEASUREMENTS.build(head, target)
                self.assertEqual(subprocess.check_output([str(base_copy)], text=True).strip(), "1")
                self.assertEqual(subprocess.check_output([str(candidate)], text=True).strip(), "2")
                self.assertEqual((head / "src/lib.rs").read_bytes(), before)

    def test_source_refresh_refuses_symlinks_outside_checkout(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            checkout = root / "checkout"
            self.checkout(checkout, 1)
            outside = root / "outside.rs"
            outside.write_text("pub fn outside() {}\n")
            source = checkout / "src/lib.rs"
            source.unlink()
            source.symlink_to(outside)
            before = outside.stat().st_mtime_ns
            with self.assertRaisesRegex(RuntimeError, "escapes benchmark checkout"):
                MEASUREMENTS.refresh_rust_sources(checkout)
            self.assertEqual(outside.stat().st_mtime_ns, before)


if __name__ == "__main__":
    unittest.main()
