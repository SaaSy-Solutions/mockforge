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
TRIM_SCRIPT = ROOT / "scripts/ci/trim-runner-cargo-target.sh"
GUARD_SCRIPT = ROOT / "scripts/ci/guard-runner-cargo-targets.sh"
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


class TrimRunnerCargoTargetTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def fill(self, path, size=4096):
        path.mkdir(parents=True, exist_ok=True)
        (path / "blob").write_bytes(b"x" * size)

    def run_trim(self, target, **env):
        full = {"PATH": os.environ["PATH"], "RUNNER_NAME": "runner-a",
                "MOCKFORGE_RUNNER_TARGET_ROOT": str(self.root)}
        if target is not None:
            full["CARGO_TARGET_DIR"] = str(target)
        full.update(env)
        return subprocess.run(["bash", str(TRIM_SCRIPT)], env=full, text=True,
                              capture_output=True)

    def test_refuses_empty_target(self):
        result = self.run_trim(None)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("CARGO_TARGET_DIR is empty", result.stderr)

    def test_refuses_symlink_target(self):
        foreign = self.root / "foreign"
        self.fill(foreign)
        target = self.root / "cargo-mockforge-target-runner-a/release"
        target.parent.mkdir(parents=True)
        target.symlink_to(foreign)
        result = self.run_trim(target)
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue((foreign / "blob").exists())

    def test_refuses_symlinked_parent(self):
        foreign = self.root / "foreign"
        self.fill(foreign / "release")
        (self.root / "cargo-mockforge-target-runner-a").symlink_to(foreign)
        target = self.root / "cargo-mockforge-target-runner-a/release"
        result = self.run_trim(target, MOCKFORGE_RELEASE_TARGET_MAX_GB="0")
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue((foreign / "release/blob").exists())

    def test_refuses_trailing_slash_and_dotdot_spellings(self):
        target = self.root / "cargo-mockforge-target-runner-a/release"
        self.fill(target)
        for spelling in (f"{target}/", f"{target}/../release"):
            result = self.run_trim(spelling, MOCKFORGE_RELEASE_TARGET_MAX_GB="0")
            self.assertNotEqual(result.returncode, 0, spelling)
            self.assertTrue((target / "blob").exists(), spelling)

    def test_refuses_missing_runner_name(self):
        target = self.root / "cargo-mockforge-target-runner-a/release"
        self.fill(target)
        result = self.run_trim(target, RUNNER_NAME="", MOCKFORGE_RELEASE_TARGET_MAX_GB="0")
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue((target / "blob").exists())

    def test_refuses_path_outside_expected_target(self):
        foreign = self.root / "foreign"
        self.fill(foreign)
        result = self.run_trim(foreign)
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue((foreign / "blob").exists())

    def test_keeps_target_under_cap(self):
        target = self.root / "cargo-mockforge-target-runner-a/release"
        self.fill(target)
        result = self.run_trim(target, MOCKFORGE_RELEASE_TARGET_MAX_GB="40")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("size before trim", result.stdout)
        self.assertTrue((target / "blob").exists())

    def test_removes_target_over_zero_cap(self):
        target = self.root / "cargo-mockforge-target-runner-a/release"
        self.fill(target)
        result = self.run_trim(target, MOCKFORGE_RELEASE_TARGET_MAX_GB="0")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("size after trim", result.stdout)
        self.assertFalse(target.exists())

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


class GuardRunnerCargoTargetsTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def fill(self, path, size=4096):
        path.mkdir(parents=True, exist_ok=True)
        (path / "blob").write_bytes(b"x" * size)

    def run_guard(self, root=None, runner="runner-a", **env):
        target = self.root if root is None else Path(root)
        full = {"PATH": os.environ["PATH"], "MOCKFORGE_RUNNER_TARGET_ROOT": str(target)}
        if runner is not None:
            full["RUNNER_NAME"] = runner
        full.update(env)
        return subprocess.run(["bash", str(GUARD_SCRIPT)], env=full, text=True, capture_output=True)

    def namespace(self, runner="runner-a"):
        return self.root / f"cargo-mockforge-target-{runner}"

    def test_missing_runner_dir_exits_zero(self):
        self.assertEqual(self.run_guard().returncode, 0)
        self.assertEqual(list(self.root.iterdir()), [])

    def test_under_cap_keeps_everything(self):
        own = self.namespace() / "ci-test"
        self.fill(own)
        result = self.run_guard(MOCKFORGE_RUNNER_TARGET_MAX_GB="1")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("size before guard", result.stdout)
        self.assertIn("under cap", result.stdout)
        self.assertTrue((own / "blob").exists())

    def test_over_cap_removes_class_dirs_but_keeps_the_namespace(self):
        first = self.namespace() / "ci-test"
        second = self.namespace() / "ci-msrv"
        self.fill(first)
        self.fill(second)
        result = self.run_guard(MOCKFORGE_RUNNER_TARGET_MAX_GB="0")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("size before guard", result.stdout)
        self.assertIn("size after guard", result.stdout)
        self.assertFalse(first.exists())
        self.assertFalse(second.exists())
        self.assertTrue(self.namespace().is_dir())

    def test_other_runner_namespaces_are_untouched_including_suffix_collision(self):
        own = self.namespace("runner-a") / "ci-test"
        other = self.namespace("runner-b") / "ci-test"
        suffix = self.namespace("other-runner-a") / "ci-test"
        for path in (own, other, suffix):
            self.fill(path)
        result = self.run_guard(MOCKFORGE_RUNNER_TARGET_MAX_GB="0")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse(own.exists())
        for kept in (other, suffix):
            self.assertTrue((kept / "blob").exists(), kept)

    def test_refuses_symlinked_root_and_preserves_its_destination(self):
        foreign = self.root / "real-root"
        self.fill(foreign / "cargo-mockforge-target-runner-a/ci-test")
        (self.root / "root-link").symlink_to(foreign)
        result = self.run_guard(root=self.root / "root-link")
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, "")
        self.assertTrue((foreign / "cargo-mockforge-target-runner-a/ci-test/blob").exists())

    def test_refuses_symlinked_namespace_and_preserves_its_destination(self):
        foreign = self.root / "foreign"
        self.fill(foreign)
        self.namespace().symlink_to(foreign)
        result = self.run_guard()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, "")
        self.assertEqual(list(foreign.iterdir()), [foreign / "blob"])

    def test_refuses_non_directory_namespace(self):
        self.namespace().write_bytes(b"not a directory")
        result = self.run_guard()
        self.assertEqual(result.returncode, 1)
        self.assertTrue(self.namespace().is_file())

    def test_symlink_child_is_skipped_while_real_class_dirs_are_removed(self):
        foreign = self.root / "foreign"
        self.fill(foreign)
        real = self.namespace() / "ci-test"
        self.fill(real)
        (self.namespace() / "ci-link").symlink_to(foreign)
        result = self.run_guard(MOCKFORGE_RUNNER_TARGET_MAX_GB="0")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse(real.exists())
        self.assertTrue((self.namespace() / "ci-link").is_symlink())
        self.assertTrue((foreign / "blob").exists())

    def test_invalid_basename_child_is_skipped_with_a_warning(self):
        bad = self.namespace() / "bad name"
        self.fill(bad)
        real = self.namespace() / "ci-test"
        self.fill(real)
        result = self.run_guard(MOCKFORGE_RUNNER_TARGET_MAX_GB="0")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("not a valid class name", result.stderr)
        self.assertTrue((bad / "blob").exists())
        self.assertFalse(real.exists())

    def test_bad_cap_value_exits_two(self):
        self.fill(self.namespace() / "ci-test")
        for cap in ("", "abc", "-1", "4.0"):
            with self.subTest(cap=cap):
                result = self.run_guard(MOCKFORGE_RUNNER_TARGET_MAX_GB=cap)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, "")

    def test_invalid_runner_name_exits_two(self):
        for runner in (None, "", "bad/runner", "../runner"):
            with self.subTest(runner=runner):
                result = self.run_guard(runner=runner)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, "")
        self.assertEqual(list(self.root.iterdir()), [])

    def test_every_selector_call_site_runs_the_guard_first(self):
        pattern = re.compile(r'bash scripts/ci/runner-cargo-target\.sh [^ ]+ >> "\$GITHUB_ENV"')
        seen = 0
        for path, text in text_files():
            for number, line in enumerate(text.splitlines(), 1):
                if pattern.search(line):
                    seen += 1
                    with self.subTest(file=path.name, line=number):
                        self.assertIn("bash scripts/ci/guard-runner-cargo-targets.sh && ", line)
                        self.assertLess(line.index("guard-runner-cargo-targets.sh"),
                                        line.index("runner-cargo-target.sh "))
        self.assertGreaterEqual(seen, 17)


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
