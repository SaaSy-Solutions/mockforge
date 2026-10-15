"""Keep trusted Rust lanes sequential and the original Test context fail-closed."""

import itertools
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import textwrap
import unittest


ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/ci.yml"
TRUSTED = "(github.event_name != 'pull_request' || github.event.pull_request.head.repo.full_name == github.repository)"
FORK = "github.event_name == 'pull_request' && github.event.pull_request.head.repo.full_name != github.repository"
EXTENSION_IF = "${{ needs.changes.outputs.code == 'true' && needs.test.result == 'success' && " + TRUSTED + " }}"
COMPLETE_IF = "${{ always() && " + TRUSTED + " && (needs.changes.result != 'success' || needs.changes.outputs.code != 'false') }}"
LABELS = "[self-hosted, linux, x64, rust]"
ALL_FEATURES = "cargo test --workspace --exclude mockforge-desktop --exclude mockforge-tui --all-features --verbose"
DOCS = "cargo doc --workspace --exclude mockforge-desktop --no-deps"
# Actual original check name: CI run38189713874/job114633625345.
# Run38213487657/job114723000285 demonstrated that the dynamic 'Test'
# expression did not automatically receive the matrix suffix.
REQUIRED_CONTEXT = "Test (1.96.0)"


def block(job):
    match = re.search(r"(?ms)^  " + re.escape(job) + r":\n(.*?)(?=^  [A-Za-z][A-Za-z0-9_-]*:\n|\Z)", WORKFLOW.read_text())
    if match is None:
        raise AssertionError("missing job " + job)
    return match.group(1)


def field(job, name):
    match = re.search(r"(?m)^    " + re.escape(name) + r": (.*)$", block(job))
    if match is None:
        raise AssertionError("missing field " + name)
    return match.group(1)


def trusted(event, head, repository="SaaSy-Solutions/mockforge"):
    return event != "pull_request" or head == repository


def aggregate_should_run(event, head, scope_result, code):
    return trusted(event, head) and (scope_result != "success" or code != "false")


def display_name(expression, event, head):
    # Evaluate only our two literal branches, not GitHub's implicit naming.
    pattern = re.escape("${{ " + TRUSTED + " && ") + r"'([^']*)' \|\| '([^']*)' \}\}"
    match = re.fullmatch(pattern, expression)
    if match is None:
        raise AssertionError("unexpected job name expression")
    return match.group(1 if trusted(event, head) else 2)


class SequentialRustTest(unittest.TestCase):
    def regression_step(self):
        return block("changes").split("      - name: Verify Rust lane and required-context routing\n", 1)[1].split("\n      - name:", 1)[0]

    def test_routing_regression_is_selected_by_normal_change_scope_job(self):
        changes = block("changes")
        self.assertLess(changes.index("name: Checkout repository"), changes.index("name: Verify Rust lane and required-context routing"))
        self.assertLess(changes.index("name: Verify Rust lane and required-context routing"), changes.index("name: Determine whether non-docs files changed"))
        step = self.regression_step()
        self.assertIn("        shell: sh\n", step)
        self.assertNotIn("if:", step)
        self.assertNotIn("continue-on-error", step)
        self.assertTrue(step.rstrip().endswith("python3 scripts/tests/test_rust_sequential_ci.py"))

    def test_missing_python_or_bash_fails_with_clear_prerequisite_error(self):
        script = textwrap.dedent(self.regression_step().split("        run: |\n", 1)[1])
        for missing in ("python3", "bash"):
            with self.subTest(missing=missing), tempfile.TemporaryDirectory() as directory:
                present = "bash" if missing == "python3" else "python3"
                Path(directory, present).symlink_to(sys.executable)
                result = subprocess.run(["/bin/sh", "-c", script], env={**os.environ, "PATH": directory}, text=True, capture_output=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("Python 3 is required" if missing == "python3" else "Bash is required", result.stdout)

    def test_change_scope_step_propagates_regression_failure(self):
        script = textwrap.dedent(self.regression_step().split("        run: |\n", 1)[1])
        with tempfile.TemporaryDirectory() as directory:
            stub = Path(directory, "python3")
            stub.write_text('#!/bin/sh\n[ "$1" = "scripts/tests/test_rust_sequential_ci.py" ] || exit 9\nexit 7\n')
            stub.chmod(0o700)
            Path(directory, "bash").symlink_to(sys.executable)
            result = subprocess.run(["/bin/sh", "-c", script], env={**os.environ, "PATH": directory}, text=True, capture_output=True)
            self.assertEqual(result.returncode, 7)

    def gate(self, scope="success", code="true", default="success", extended="success"):
        script = textwrap.dedent(block("test-complete").split("        run: |\n", 1)[1])
        return subprocess.run(
            ["bash", "-c", script],
            env={**os.environ, "SCOPE_RESULT": scope or "", "CODE_SCOPE": code or "",
                 "DEFAULT_RESULT": default or "", "EXTENDED_RESULT": extended or ""},
            text=True, capture_output=True, check=False,
        )

    def test_aggregate_condition_does_not_skip_failed_or_cancelled_dependencies(self):
        self.assertEqual(field("test-complete", "if"), COMPLETE_IF)
        self.assertEqual(field("test-complete", "needs"), "[changes, test, test-extended]")
        self.assertNotIn("cancelled()", COMPLETE_IF)
        for result, code in itertools.product(("success", "failure", "cancelled", "skipped", "", None), ("true", "false", "", None)):
            with self.subTest(result=result, code=code):
                self.assertEqual(aggregate_should_run("pull_request", "SaaSy-Solutions/mockforge", result, code), not (result == "success" and code == "false"))

    def test_gate_accepts_only_all_success_and_explicit_true_scope(self):
        states = ("success", "failure", "cancelled", "skipped", "", None)
        for scope, default, extended in itertools.product(states, repeat=3):
            with self.subTest(scope=scope, default=default, extended=extended):
                result = self.gate(scope=scope, default=default, extended=extended)
                self.assertEqual(result.returncode == 0, (scope, default, extended) == ("success", "success", "success"))

    def test_missing_or_invalid_scope_is_not_a_green_skip(self):
        for code in ("", None, "false", "TRUE", "success", "true; exit 0"):
            with self.subTest(code=code):
                self.assertNotEqual(self.gate(code=code).returncode, 0)
        self.assertEqual(self.gate().returncode, 0)

    def test_gate_environment_is_bound_to_all_four_dependency_fields(self):
        gate = block("test-complete")
        for binding in ("SCOPE_RESULT: ${{ needs.changes.result }}", "CODE_SCOPE: ${{ needs.changes.outputs.code }}",
                        "DEFAULT_RESULT: ${{ needs.test.result }}", "EXTENDED_RESULT: ${{ needs['test-extended'].result }}"):
            self.assertIn(binding, gate)
        self.assertNotIn("continue-on-error", gate)
        self.assertNotIn("uses:", gate)

    def test_original_context_is_aggregate_only_for_trusted_events(self):
        self.assertEqual(field("test", "name"), "${{ " + TRUSTED + " && 'Rust default checks' || 'Test (1.96.0)' }}")
        self.assertEqual(field("test-complete", "name"), "${{ " + TRUSTED + " && 'Test (1.96.0)' || 'Trusted Rust completion' }}")
        for job in ("test", "test-extended", "test-complete"):
            self.assertIn('        rust:\n          - "1.96.0"', block(job))

    def test_literal_required_name_matches_native_context_on_trusted_and_fork_paths(self):
        for event, head in (("pull_request", "SaaSy-Solutions/mockforge"), ("pull_request", "untrusted/fork"),
                            ("push", None), ("workflow_dispatch", None)):
            for scope_result, code in (("success", "true"), ("success", "false"), ("failure", "")):
                with self.subTest(event=event, head=head, scope=scope_result, code=code):
                    names = {job: display_name(field(job, "name"), event, head) for job in ("test", "test-complete")}
                    owner = "test-complete" if trusted(event, head) else "test"
                    self.assertEqual(names[owner], REQUIRED_CONTEXT)
                    self.assertNotEqual(names['test' if owner == 'test-complete' else 'test-complete'], REQUIRED_CONTEXT)
                    self.assertEqual(list(names.values()).count(REQUIRED_CONTEXT), 1)
                    if scope_result == 'success' and code == 'false':
                        # Docs-only preserves the existing accepted skipped context.
                        self.assertFalse(aggregate_should_run(event, head, scope_result, code))
                    if not trusted(event, head):
                        self.assertFalse(aggregate_should_run(event, head, scope_result, code))

    def test_native_unsuffixed_counterfactual_cannot_satisfy_original_context(self):
        for job, event, head in (("test-complete", "pull_request", "SaaSy-Solutions/mockforge"),
                                 ("test", "pull_request", "untrusted/fork")):
            current = field(job, "name")
            previous = current.replace("'Test (1.96.0)'", "'Test'")
            self.assertNotEqual(previous, current)
            self.assertEqual(display_name(previous, event, head), "Test")
            self.assertNotEqual(display_name(previous, event, head), REQUIRED_CONTEXT)
            self.assertEqual(display_name(current, event, head), REQUIRED_CONTEXT)

    def test_no_new_hosted_fork_jobs_or_privileged_fork_execution(self):
        self.assertEqual(field("test-extended", "if"), EXTENSION_IF)
        self.assertEqual(field("test-complete", "if"), COMPLETE_IF)
        for job in ("test-extended", "test-complete"):
            self.assertEqual(field(job, "runs-on"), LABELS)
            self.assertNotIn("ubuntu-latest", block(job))
        for head in ("untrusted/fork", "", None):
            with self.subTest(head=head):
                self.assertFalse(trusted("pull_request", head))
                self.assertFalse(aggregate_should_run("pull_request", head, "success", "true"))
        for event in ("push", "workflow_dispatch"):
            self.assertTrue(trusted(event, None))
        self.assertTrue(trusted("pull_request", "SaaSy-Solutions/mockforge"))

    def test_extension_starts_only_after_successful_default_lane(self):
        self.assertEqual(field("test-extended", "needs"), "[changes, test]")
        self.assertEqual(field("test-extended", "if"), EXTENSION_IF)
        self.assertNotIn("always()", EXTENSION_IF)
        self.assertEqual(field("test-extended", "timeout-minutes"), "60")
        self.assertEqual(field("test", "timeout-minutes"), "60")
        self.assertEqual(field("test-complete", "timeout-minutes"), "5")

    def test_all_features_and_docs_remain_on_full_fork_path(self):
        default = block("test")
        for name, command in (("Run tests with all features", ALL_FEATURES), ("Check documentation", DOCS)):
            step = default.split("    - name: " + name + "\n", 1)[1].split("\n    - name:", 1)[0]
            self.assertIn("run: " + command, step)
            self.assertIn("if: matrix.rust == '1.96.0' && (" + FORK + ")", step)
        self.assertIn("continue-on-error: true", default.split("    - name: Run tests with all features\n", 1)[1].split("\n    - name:", 1)[0])

    def test_extension_keeps_original_commands_and_all_features_policy(self):
        extension = block("test-extended")
        self.assertIn("run: " + ALL_FEATURES, extension)
        self.assertIn("run: " + DOCS, extension)
        self.assertIn("continue-on-error: true", extension)
        self.assertLess(extension.index(ALL_FEATURES), extension.index(DOCS))
        self.assertNotIn("MOCKFORGE_TEST_BINARY", extension)
        self.assertIn("Default, all-features, and documentation commands completed under their existing policies.", block("test-complete"))
        self.assertNotIn("all-features tests passed", block("test-complete"))

    def test_cache_namespaces_and_artifacts_are_not_shared_between_jobs(self):
        for job in ("test", "test-extended"):
            content = block(job)
            self.assertIn("uses: Swatinem/rust-cache@v2", content)
            self.assertIn('env-vars: "CARGO_HOME"', content)
            self.assertIn("CARGO_HOME=/tmp/cargo-mockforge-${{ runner.name }}", content)
            for forbidden in ("shared-key:", "cache-on-failure:", "cache-workspace-crates:", "download-artifact", "upload-artifact"):
                self.assertNotIn(forbidden, content)
        self.assertIn('cp target/debug/mockforge "$RUNNER_TEMP/mockforge-all-protocols"', block("test"))
        self.assertIn("MOCKFORGE_TEST_BINARY: ${{ runner.temp }}/mockforge-all-protocols", block("test"))
        self.assertNotIn("mockforge-all-protocols", block("test-extended"))

    def test_extension_uses_read_only_prerequisites_and_same_resource_flags(self):
        extension = block("test-extended")
        self.assertIn("RUSTUP_AUTO_INSTALL=0 rustup run", extension)
        self.assertIn("dpkg -s protobuf-compiler", extension)
        for flag in ('CARGO_INCREMENTAL: "0"', 'CARGO_BUILD_JOBS: "2"', 'CARGO_PROFILE_DEV_DEBUG: "0"', 'CARGO_PROFILE_TEST_DEBUG: "0"'):
            self.assertIn(flag, extension)
        for forbidden in ("sudo ", "apt-get", "rustup default", "dtolnay/", "uses: ./.github/actions/rust-toolchain"):
            self.assertNotIn(forbidden, extension)


if __name__ == "__main__":
    unittest.main()
