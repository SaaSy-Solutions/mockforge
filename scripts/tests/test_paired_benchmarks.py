"""Validate the diagnostic without compiling or measuring Rust code."""

from contextlib import redirect_stdout
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
SCRIPTS = next(p for p in (HERE, HERE.parent) if (p / "paired_benchmarks.py").is_file())
sys.path.insert(0, str(SCRIPTS))
import paired_benchmarks as paired


def fixture(root, mean=100.0):
    for name, count in paired.EXPECTED.items():
        directory = root / name / "new"
        directory.mkdir(parents=True)
        paired.write_json(directory / "benchmark.json", {"full_id": name})
        paired.write_json(directory / "estimates.json", {"mean": {
            "point_estimate": mean,
            "confidence_interval": {"lower_bound": mean * .99, "upper_bound": mean * 1.01,
                                    "confidence_level": .95},
        }})
        paired.write_json(directory / "sample.json", {
            "sampling_mode": "Linear", "iters": [1] * count, "times": [mean] * count,
        })


def full_plan(root, deltas=(0, 0, 0)):
    shas = {"base": "a" * 40, "candidate": "b" * 40}
    settings = {"RUSTUP_TOOLCHAIN": "1.96.0", "CARGO_BUILD_JOBS": "2"}
    paired.write_json(root / "metadata.json", {
        "schema": 1, "threshold_percent": 5.0, "expected_benchmarks": paired.EXPECTED,
        "command": paired.COMMAND, "shas": shas, "settings": settings,
    })
    for variant in shas:
        (root / f"{variant}-Cargo.lock").write_bytes((variant + "\n\n").encode())
    records = []
    for ordinal, (run_id, variant, pair) in enumerate(paired.PLAN, 1):
        mean = 100.0 + (deltas[pair - 1] if variant == "candidate" and pair else 0)
        fixture(root / "runs" / run_id / "criterion", mean)
        records.append({
            "id": run_id, "ordinal": ordinal, "variant": variant, "pair": pair,
            "measured": pair != 0, "sha": shas[variant], "state": "complete", "exit_code": 0,
            "command": paired.COMMAND, "settings": settings,
            "lock_sha256": hashlib.sha256((root / f"{variant}-Cargo.lock").read_bytes()).hexdigest(),
            "started_utc": "2026-10-08T00:00:00Z", "finished_utc": "2026-10-08T00:01:00Z",
            "elapsed_seconds": 60,
        })
    paired.write_json(root / "runs.json", records)
    return records


class PairedDiagnosticTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def test_complete_raw_inventory(self):
        fixture(self.root)
        result = paired.parse_criterion(self.root)
        self.assertEqual(set(result), set(paired.EXPECTED))
        self.assertEqual(result["memory/large_spec_parsing"]["sample_count"], 10)

    def test_stale_base_results_are_not_used(self):
        fixture(self.root)
        directory = self.root / "template_rendering/simple"
        (directory / "new").rename(directory / "base")
        with self.assertRaises(paired.InvalidData):
            paired.parse_criterion(self.root)

    def test_missing_or_short_samples_fail(self):
        fixture(self.root)
        sample = self.root / "template_rendering/simple/new/sample.json"
        value = paired.read_json(sample)
        value["times"].pop()
        paired.write_json(sample, value)
        with self.assertRaises(paired.InvalidData):
            paired.parse_criterion(self.root)

    def test_invalid_interval_and_nonfinite_numbers_fail(self):
        fixture(self.root)
        path = self.root / "template_rendering/simple/new/estimates.json"
        value = paired.read_json(path)
        value["mean"]["confidence_interval"]["lower_bound"] = 200
        paired.write_json(path, value)
        with self.assertRaises(paired.InvalidData):
            paired.parse_criterion(self.root)
        with self.assertRaises(paired.InvalidData):
            paired.positive(float("nan"), "test")

    def test_threshold_is_strict_and_no_pair_is_averaged_away(self):
        self.assertEqual(paired.classify([5, 5, 5]), "within_threshold")
        self.assertEqual(paired.classify([5.01, 6, 7]), "reproducible_regression")
        self.assertEqual(paired.classify([-6, -7, -8]), "reproducible_improvement")
        self.assertEqual(paired.classify([6, -20, -20]), "mixed")
        with self.assertRaises(paired.InvalidData):
            paired.classify([0, 0])

    def test_full_report_preserves_all_three_pairs(self):
        full_plan(self.root, (1, 2, 3))
        with redirect_stdout(io.StringIO()):
            self.assertEqual(paired.report(self.root), 0)
        report = paired.read_json(self.root / "report.json")
        self.assertEqual(len(report["benchmarks"]), 13)
        self.assertEqual(len(report["benchmarks"]["template_rendering/simple"]["pairs"]), 3)

    def test_mixed_result_does_not_pass(self):
        full_plan(self.root, (6, -20, -20))
        with redirect_stdout(io.StringIO()):
            self.assertEqual(paired.report(self.root), 1)

    def test_reproducible_regression_does_not_pass(self):
        full_plan(self.root, (8, 9, 10))
        with redirect_stdout(io.StringIO()):
            self.assertEqual(paired.report(self.root), 1)

    def test_order_lockfile_and_settings_mismatches_fail(self):
        records = full_plan(self.root)
        records[2]["settings"] = {"RUSTUP_TOOLCHAIN": "1.98.0"}
        paired.write_json(self.root / "runs.json", records)
        with self.assertRaises(paired.InvalidData):
            paired.report(self.root)
        records[2]["settings"] = records[0]["settings"]
        records[2]["ordinal"] = 9
        paired.write_json(self.root / "runs.json", records)
        with self.assertRaises(paired.InvalidData):
            paired.report(self.root)
        records[2]["ordinal"] = 3
        paired.write_json(self.root / "runs.json", records)
        (self.root / "base-Cargo.lock").write_text("different\n")
        with self.assertRaises(paired.InvalidData):
            paired.report(self.root)

    def test_incomplete_cli_report_is_explicit_and_fails(self):
        records = full_plan(self.root)
        paired.write_json(self.root / "runs.json", records[:-1])
        result = subprocess.run([sys.executable, str(SCRIPTS / "paired_benchmarks.py"),
                                 "report", "--root", str(self.root)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 2)
        self.assertIn("INCOMPLETE", (self.root / "report.txt").read_text())

    def test_environment_capture_omits_credentials(self):
        with patch.dict(os.environ, {"GITHUB_TOKEN": "not-a-real-token",
                                   "ACTIONS_RUNTIME_TOKEN": "not-a-real-token",
                                   "CARGO_REGISTRIES_CRATES_IO_TOKEN": "not-a-real-token"}):
            settings = paired.build_settings()
        self.assertNotIn("GITHUB_TOKEN", settings)
        self.assertNotIn("ACTIONS_RUNTIME_TOKEN", settings)
        self.assertNotIn("CARGO_REGISTRIES_CRATES_IO_TOKEN", settings)


if __name__ == "__main__":
    unittest.main()
