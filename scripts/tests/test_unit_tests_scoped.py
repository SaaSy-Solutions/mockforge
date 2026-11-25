"""Plan the tests of a push without a repository, cargo or a network."""

import importlib.util
from pathlib import Path
import unittest

SCRIPT = Path(__file__).resolve().parents[2] / "scripts" / "prepush" / "unit_tests_scoped.py"


class UnitTestsScopedTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location("unit_tests_scoped", SCRIPT)
        cls.hook = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.hook)

    def plan(self, *files):
        return self.hook.plan(list(files))

    def test_a_file_under_a_crate_selects_that_crate(self):
        self.assertEqual(
            self.plan("crates/mockforge-http/src/lib.rs", "crates/mockforge-cli/src/main.rs"),
            (["mockforge-cli", "mockforge-http"], [], []),
        )

    def test_a_file_that_is_not_rust_still_selects_its_crate(self):
        # A suite a crate includes when it compiles is tested by its tests.
        crates, tests, left = self.plan(
            "crates/mockforge-contracts/src/agent_redteam/suites/tool-poisoning.json"
        )
        self.assertEqual((crates, tests, left), (["mockforge-contracts"], [], []))

    def test_a_file_under_tests_selects_the_integration_test_it_belongs_to(self):
        self.assertEqual(
            self.plan("crates/mockforge-cli/tests/agent_redteam_cli.rs", "crates/mockforge-cli/src/main.rs"),
            (["mockforge-cli"], ["agent_redteam_cli"], []),
        )
        # Two files of one test are one test, and the names are sorted.
        self.assertEqual(
            self.plan("crates/mockforge-cli/tests/zeta.rs", "crates/mockforge-cli/tests/alpha.rs")[1],
            ["alpha", "zeta"],
        )

    def test_a_file_deeper_under_tests_selects_every_integration_test(self):
        # Any test may include a helper, so none of them can be left out.
        self.assertIsNone(self.plan("crates/mockforge-http/tests/common/mod.rs")[1])
        self.assertIsNone(self.plan("crates/mockforge-http/tests/fixtures/answer.json")[1])
        # A test in a directory of its own is one test.
        self.assertEqual(
            self.plan("crates/mockforge-http/tests/contract/main.rs")[1], ["contract"]
        )

    def test_a_module_of_the_library_is_not_an_integration_test(self):
        self.assertEqual(self.plan("crates/mockforge-http/src/tests/helpers.rs")[1], [])

    def test_the_workspace_manifest_is_left_to_ci_and_the_crates_are_still_tested(self):
        crates, _, left = self.plan("Cargo.lock", "crates/mockforge-k8s-operator/Cargo.toml")
        self.assertEqual(crates, ["mockforge-k8s-operator"])
        self.assertEqual(left, ["the workspace manifest (`Cargo.lock`)"])
        self.assertIn(
            "CI runs the whole workspace's tests",
            self.hook.LEFT_TO_CI.format(what=left[0]),
        )
        # A crate's own manifest is not the workspace's.
        self.assertEqual(self.plan("crates/mockforge-k8s-operator/Cargo.toml")[2], [])

    def test_rust_outside_crates_is_left_to_ci(self):
        crates, _, left = self.plan("examples/client.rs", "crates/mockforge-core/src/lib.rs")
        self.assertEqual(crates, ["mockforge-core"])
        self.assertEqual(left, ["a Rust file outside `crates/` (`examples/client.rs`)"])

    def test_files_that_are_neither_select_nothing(self):
        self.assertEqual(
            self.plan("README.md", "book/src/SUMMARY.md", ".github/workflows/ci.yml"), ([], [], [])
        )

    def test_a_crate_the_hooks_leave_out_is_left_out(self):
        self.assertEqual(self.plan("crates/mockforge-desktop/src/main.rs"), ([], [], []))

    def test_the_crate_of_a_path(self):
        crate_for = self.hook.crate_for
        self.assertEqual(crate_for("crates/mockforge-cli/src/main.rs"), "mockforge-cli")
        self.assertEqual(crate_for("crates/a/b/c/d.txt"), "a")
        for path in ("Cargo.toml", "src/lib.rs", "crates", "crates/", "crates//src/lib.rs"):
            self.assertIsNone(crate_for(path), path)

    def test_one_run_names_every_crate(self):
        self.assertEqual(
            self.hook.test_args(["mockforge-http", "mockforge-cli"], []),
            ["-p", "mockforge-http", "-p", "mockforge-cli", "--lib", "--bins"],
        )
        self.assertEqual(
            self.hook.test_args(["mockforge-cli"], ["zeta", "alpha"]),
            ["-p", "mockforge-cli", "--test", "alpha", "--test", "zeta", "--lib", "--bins"],
        )
        self.assertEqual(self.hook.test_args(["mockforge-http", "mockforge-cli"], None),
                         ["-p", "mockforge-http", "-p", "mockforge-cli", "--tests"])

    def test_cargo_is_the_repository_wrapper(self):
        self.assertEqual(self.hook.CARGO, SCRIPT.parents[2] / "scripts" / "dev" / "cargo")


if __name__ == "__main__":
    unittest.main()
