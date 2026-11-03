"""Runner-persistent cargo targets, and no workflow writes GitHub cache storage.

The org's GitHub Actions storage is billed and runs out early each month.
Rust jobs on Hetzner keep warm targets on the runner disk instead
(scripts/ci/runner-cargo-target.sh). These fixtures never touch /tmp outside
their own temporary root.
"""
from pathlib import Path
import os
import re
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/ci/runner-cargo-target.sh"
WORKFLOWS = ROOT / ".github/workflows"
ACTIONS = ROOT / ".github/actions"


def run(root, *args, runner="runner-a", **env):
    full = {"PATH": os.environ["PATH"], "MOCKFORGE_RUNNER_TARGET_ROOT": str(root),
            "MOCKFORGE_RUNNER_TARGET_USED_PCT": "10"}
    if runner is not None:
        full["RUNNER_NAME"] = runner
    full.update(env)
    return subprocess.run(["bash", str(SCRIPT), *args], env=full, text=True, capture_output=True)


class RunnerCargoTargetTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def fill(self, path, size=4096):
        path.mkdir(parents=True, exist_ok=True)
        (path / "blob").write_bytes(b"x" * size)

    def test_prints_one_env_line_for_a_per_class_per_runner_dir(self):
        result = run(self.root, "ci-test")
        self.assertEqual(result.returncode, 0, result.stderr)
        expected = self.root / "cargo-mockforge-target-runner-a/ci-test"
        self.assertEqual(result.stdout, f"CARGO_TARGET_DIR={expected}\n")
        self.assertTrue(expected.is_dir())
        other = run(self.root, "ci-test", runner="runner-b")
        self.assertIn("cargo-mockforge-target-runner-b/ci-test", other.stdout)

    def test_rejects_bad_class_and_runner_names(self):
        for args, runner in ((("",), "runner-a"), (("../x",), "runner-a"), (("a b",), "runner-a"), (("..",), "runner-a"), ((".",), "runner-a"),
                             (("ok",), None), (("ok",), "bad/runner")):
            with self.subTest(args=args, runner=runner):
                result = run(self.root, *args, runner=runner)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, "")
        self.assertEqual(list(self.root.iterdir()), [])

    def test_refuses_symlink_and_preserves_its_destination(self):
        foreign = self.root / "foreign"
        self.fill(foreign)
        (self.root / "cargo-mockforge-target-runner-a").mkdir()
        (self.root / "cargo-mockforge-target-runner-a/ci-test").symlink_to(foreign)
        result = run(self.root, "ci-test")
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, "")
        self.assertTrue((foreign / "blob").exists())

    def test_oversized_own_dir_is_not_deleted(self):
        own = self.root / "cargo-mockforge-target-runner-a/ci-test"
        self.fill(own, 64 * 1024)
        result = run(self.root, "ci-test", MOCKFORGE_RUNNER_TARGET_MAX_KB="8")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(own.is_dir())
        self.assertTrue((own / "blob").exists())
        warm = run(self.root, "ci-test")
        self.assertEqual(warm.returncode, 0)

    def test_pressure_preserves_all_existing_caches(self):
        own = self.root / "cargo-mockforge-target-runner-a/ci-test"
        sibling = self.root / "cargo-mockforge-target-runner-a/ci-msrv"
        other_runner = self.root / "cargo-mockforge-target-runner-b/ci-msrv"
        cargo_home = self.root / "cargo-mockforge-runner-a"
        unrelated = self.root / "cargo-rust-compile-gate-runner-a"
        for path in (own, sibling, other_runner, cargo_home, unrelated):
            self.fill(path)
        result = run(self.root, "ci-test", MOCKFORGE_RUNNER_TARGET_USED_PCT="95")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((own / "blob").exists())
        self.assertTrue((sibling / "blob").exists())
        for kept in (other_runner, cargo_home, unrelated):
            self.assertTrue((kept / "blob").exists(), kept)

    def test_pressure_preserves_runner_name_suffix_collision(self):
        foreign = self.root / "cargo-mockforge-target-ci-test-other-runner-a"
        self.fill(foreign)
        result = run(self.root, "ci-test", MOCKFORGE_RUNNER_TARGET_USED_PCT="95")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((foreign / "blob").exists())

    def test_class_runner_pairs_have_distinct_targets(self):
        first = run(self.root, "ci", runner="other-runner-a")
        second = run(self.root, "ci-other", runner="runner-a")
        self.assertEqual(first.returncode, 0, first.stderr)
        self.assertEqual(second.returncode, 0, second.stderr)
        self.assertNotEqual(first.stdout, second.stdout)

    def test_refuses_symlinked_runner_namespace(self):
        foreign = self.root / "foreign"
        self.fill(foreign)
        (self.root / "cargo-mockforge-target-runner-a").symlink_to(foreign)
        result = run(self.root, "ci-test")
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertEqual(list(foreign.iterdir()), [foreign / "blob"])

    def test_no_pressure_keeps_siblings(self):
        sibling = self.root / "cargo-mockforge-target-runner-a/ci-msrv"
        self.fill(sibling)
        self.assertEqual(run(self.root, "ci-test").returncode, 0)
        self.assertTrue((sibling / "blob").exists())


def text_files():
    for path in sorted(WORKFLOWS.glob("*.yml")) + sorted(ACTIONS.glob("*/action.yml")):
        yield path, path.read_text()


def steps(text):
    return [s for s in re.split(r"(?m)^(?=\s+- (?:name|uses):)", text) if s.strip()]


class NoGitHubCacheStorageTest(unittest.TestCase):
    def test_no_cache_actions_or_gha_compile_cache(self):
        forbidden = re.compile(r"uses:\s*(?:actions/cache|Swatinem/rust-cache|mozilla-actions/sccache-action)|"
                               r"SCCACHE_GHA_ENABLED|type=gha")
        for path, text in text_files():
            for number, line in enumerate(text.splitlines(), 1):
                if line.lstrip().startswith("#"):
                    continue
                with self.subTest(file=path.name, line=number):
                    self.assertIsNone(forbidden.search(line), line)

    def test_setup_actions_with_default_caching_disable_it(self):
        # These setup actions cache by default; the others only cache when a
        # `cache:` input is given, which must never name a package manager.
        default_on = ("actions/setup-go@", "actions-rust-lang/setup-rust-toolchain@")
        for path, text in text_files():
            for step in steps(text):
                with self.subTest(file=path.name, step=step.strip().splitlines()[0]):
                    if any("uses: " + name in step for name in default_on):
                        self.assertRegex(step, r"(?m)^\s+cache: false\s*$")
                    self.assertNotRegex(step, r"(?m)^\s+cache: ['\"]?(?:npm|pnpm|yarn|pip|pipenv|poetry|gradle|maven|sbt|true)")

    def test_sbom_action_does_not_upload_a_second_artifact(self):
        for path, text in text_files():
            for step in steps(text):
                if "uses: anchore/sbom-action" in step:
                    with self.subTest(file=path.name):
                        self.assertIn("upload-artifact: false", step)

    def test_artifact_uploads_are_short_lived(self):
        # Workflow artifacts are billed storage too. Diagnostics and in-run
        # handoffs keep one day; tiny crash/seed reproducers and receipts
        # keep at most seven.
        for path, text in text_files():
            if path.parent.name != "workflows":
                continue
            for step in steps(text):
                if re.search(r"uses: actions/upload-artifact@", step):
                    with self.subTest(file=path.name, step=step.strip().splitlines()[0]):
                        match = re.search(r"(?m)^\s+retention-days: (\d+)\s*$", step)
                        self.assertIsNotNone(match, "upload-artifact needs retention-days")
                        self.assertLessEqual(int(match.group(1)), 7)

    def test_rust_jobs_use_the_runner_target_script_with_a_valid_class(self):
        pattern = re.compile(r'bash scripts/ci/runner-cargo-target\.sh ([^ ]+) >> "\$GITHUB_ENV"')
        seen = {}
        for path, text in text_files():
            for cls in pattern.findall(text):
                with self.subTest(file=path.name, cls=cls):
                    self.assertRegex(cls, r"^[A-Za-z0-9._-]+$")
                    self.assertNotIn(cls, seen, f"class reused by {seen.get(cls)}")
                    seen[cls] = path.name
        self.assertIn("ci-test", seen)


if __name__ == "__main__":
    unittest.main()
