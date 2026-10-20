"""Keep benchmark comparison failures visible after report publication."""

from pathlib import Path
import json
import os
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/benchmarks.yml"
COMPARATOR = ROOT / ".github/scripts/compare-benchmarks.js"


class BenchmarkGateTest(unittest.TestCase):
    def compare(self, current_mean: int | None, base_samples=None, current_samples=None):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            baseline = root / "baseline"
            baseline.mkdir()
            base = {"mean": 100}
            if base_samples is not None:
                base = {"mean": sum(base_samples) / len(base_samples), "samples": base_samples}
            (baseline / "baseline.json").write_text(json.dumps({"example": {"new": base}}))
            criterion = root / "criterion"
            if current_mean is not None:
                estimates = criterion / "example/new/estimates.json"
                estimates.parent.mkdir(parents=True)
                estimate = {
                    "mean": {"point_estimate": current_mean},
                    "std_dev": {"point_estimate": 1},
                }
                if current_samples is not None:
                    estimate["samples"] = current_samples
                estimates.write_text(json.dumps(estimate))
            report = root / "report.md"
            result = subprocess.run(
                ["node", str(COMPARATOR), "compare"],
                env={
                    **os.environ,
                    "BASELINE_DIR": str(baseline),
                    "CRITERION_DIR": str(criterion),
                    "REPORT_PATH": str(report),
                    "REGRESSION_THRESHOLD": "5.0",
                },
                text=True,
                capture_output=True,
                check=False,
            )
            return result, report.read_text() if report.exists() else None

    def test_stable_comparison_succeeds(self):
        result, report = self.compare(104)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("**Regressions**: 0", report)

    def test_regression_fails_but_keeps_report(self):
        result, report = self.compare(120)
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("**Regressions**: 1", report)
        self.assertIn("+20.00%", report)

    def test_paired_mean_shift_within_overlapping_samples_is_stable(self):
        # #1309: identical code, means 8% apart, but the samples overlap.
        result, report = self.compare(108, base_samples=[95, 105], current_samples=[100, 116])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("**Regressions**: 0", report)

    def test_paired_samples_separated_beyond_threshold_fail(self):
        result, report = self.compare(120, base_samples=[99, 101], current_samples=[118, 122])
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("**Regressions**: 1", report)

    def test_paired_samples_separated_by_less_than_threshold_are_stable(self):
        result, _ = self.compare(105, base_samples=[100, 101], current_samples=[104, 106])
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_paired_improvement_needs_separation_too(self):
        result, report = self.compare(80, base_samples=[99, 101], current_samples=[79, 81])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("**Improvements**: 1", report)

    def test_comparison_error_fails(self):
        result, _ = self.compare(None)
        self.assertNotEqual(result.returncode, 0)

    def test_workflow_gates_original_outcome_after_reporting(self):
        workflow = WORKFLOW.read_text()
        comparison = workflow.split("- name: Compare benchmarks against baseline\n", 1)[1]
        comparison = comparison.split("\n      - name:", 1)[0]
        self.assertIn("id: compare", comparison)
        self.assertIn("continue-on-error: true", comparison)
        self.assertIn("node .github/scripts/compare-benchmarks.js compare", comparison)
        gate = workflow.split("- name: Check for performance regression\n", 1)[1]
        # outcome retains failure when continue-on-error makes conclusion success.
        # Outputs written after a failing command are absent under bash -e.
        self.assertIn(
            "if: github.event_name == 'pull_request' && steps.compare.outcome == 'failure'",
            gate,
        )
        self.assertIn("exit 1", gate)
        self.assertNotIn("steps.compare.outputs.comparison_status", gate)
        self.assertLess(
            workflow.index("- name: Comment PR with benchmark results"),
            workflow.index("- name: Check for performance regression"),
        )


if __name__ == "__main__":
    unittest.main()
