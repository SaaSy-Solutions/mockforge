"""Offline request/path contracts; no Rust builds, credentials, or uploads."""

import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
HELPER = ROOT / "scripts/plugin_publish_request.py"
WORKFLOW = ROOT / ".github/workflows/plugin-publish.yml"
SPEC = importlib.util.spec_from_file_location("plugin_publish_request", HELPER)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def run_script(step_name):
    text = WORKFLOW.read_text().split(f"      - name: {step_name}\n", 1)[1]
    text = text.split("      - name:", 1)[0]
    value = text.split("        run: ", 1)[1]
    if not value.startswith("|\n"):
        return value.splitlines()[0]
    lines = []
    for line in value.splitlines()[1:]:
        if line and not line.startswith("          "):
            break
        lines.append(line[10:])
    return "\n".join(lines)


class PluginRequestTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def plugin(self, path="."):
        selected = self.root / path
        selected.mkdir(parents=True, exist_ok=True)
        (selected / "plugin.yaml").write_text("name: test-plugin\nversion: 1.0.0\n")
        (selected / "Cargo.toml").write_text("[package]\nname = 'test-plugin'\n")
        return selected

    def choose(self, path=".", event="workflow_dispatch", ref="refs/heads/main"):
        return MODULE.plugin_directory(self.root, event, ref, path)

    def test_crate_tag_never_requires_a_plugin_manifest(self):
        for ref in ("refs/tags/v0.3.231", "refs/tags/v1.0.0-rc.1", "refs/heads/main"):
            with self.subTest(ref=ref):
                self.assertIsNone(self.choose(event="push", ref=ref))

    def test_crate_tag_does_not_fall_through_even_with_root_manifest(self):
        self.plugin()
        self.assertIsNone(self.choose(event="push", ref="refs/tags/v0.3.231"))

    def test_plugin_tag_requires_root_manifest(self):
        with self.assertRaises(ValueError):
            self.choose(event="push", ref="refs/tags/plugin-test-v1.0.0")

    def test_plugin_tag_retains_root_project_selection(self):
        self.plugin()
        self.assertEqual(self.choose("ignored", "push", "refs/tags/plugin-test-v1.0.0"), ".")

    def test_empty_plugin_tag_is_rejected(self):
        self.plugin()
        with self.assertRaises(ValueError):
            self.choose(event="push", ref="refs/tags/plugin-")

    def test_dispatch_selects_nested_plugin_with_spaces(self):
        self.plugin("examples/my plugin")
        self.assertEqual(self.choose("./examples/my plugin"), "examples/my plugin")

    def test_leading_dash_directory_is_a_valid_local_selection(self):
        self.plugin("-plugin")
        self.assertEqual(self.choose("./-plugin"), "-plugin")

    def test_dispatch_does_not_fall_back_to_root_manifest(self):
        self.plugin()
        (self.root / "nested").mkdir()
        with self.assertRaises(ValueError):
            self.choose("nested")

    def test_missing_directory_fails(self):
        with self.assertRaises(OSError):
            self.choose("absent")

    def test_file_is_not_a_plugin_directory(self):
        (self.root / "file").write_text("x")
        with self.assertRaises(ValueError):
            self.choose("file")

    def test_manifest_directory_is_rejected(self):
        (self.root / "plugin.yaml").mkdir()
        with self.assertRaises(ValueError):
            self.choose()

    def test_missing_manifest_is_rejected(self):
        with self.assertRaises(ValueError):
            self.choose()

    def test_missing_cargo_manifest_fails_before_compilation(self):
        selected = self.plugin()
        (selected / "Cargo.toml").unlink()
        with self.assertRaises(ValueError):
            self.choose()

    def test_cargo_manifest_symlink_is_rejected(self):
        selected = self.plugin()
        (selected / "Cargo.toml").unlink()
        (selected / "real.toml").write_text("[package]\nname = 'test'\n")
        (selected / "Cargo.toml").symlink_to(selected / "real.toml")
        with self.assertRaises(ValueError):
            self.choose()

    def test_path_traversal_and_control_characters_are_rejected(self):
        self.plugin()
        for path in ("", "../outside", "a/../b", "/tmp", "C:/tmp", "a\\b", "a\nb",
                     "a\rb", "a\tb", "a=evil", "$(touch nope)", " a", "a ", "a\x00b"):
            with self.subTest(path=path), self.assertRaises(ValueError):
                self.choose(path)

    def test_directory_symlinks_are_rejected_inside_and_outside_checkout(self):
        self.plugin("real")
        for name, target in (("inside", self.root / "real"), ("outside", self.root.parent)):
            (self.root / name).symlink_to(target, target_is_directory=True)
            with self.subTest(name=name), self.assertRaises(ValueError):
                self.choose(name)

    def test_manifest_symlink_is_rejected(self):
        (self.root / "real.yaml").write_text("name: test")
        (self.root / "plugin.yaml").symlink_to(self.root / "real.yaml")
        with self.assertRaises(ValueError):
            self.choose()

    def test_unexpected_event_is_rejected(self):
        self.plugin()
        with self.assertRaises(ValueError):
            self.choose(event="pull_request")

    def invoke(self, path=".", event="workflow_dispatch", ref="refs/heads/main"):
        output = self.root / "output"
        output.write_text("")
        result = subprocess.run(
            [sys.executable, str(HELPER)], cwd=self.root,
            env={**os.environ, "GITHUB_WORKSPACE": str(self.root), "GITHUB_OUTPUT": str(output),
                 "GITHUB_EVENT_NAME": event, "GITHUB_REF": ref, "INPUT_PLUGIN_PATH": path},
            text=True, capture_output=True, check=False,
        )
        return result, output.read_text()

    def test_failed_preflight_writes_no_partial_success_output(self):
        result, output = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(output, "")

    def test_successful_preflight_outputs_only_validated_selection(self):
        self.plugin("examples/my plugin")
        result, output = self.invoke("./examples/my plugin")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "requested=true\nplugin_path=examples/my plugin\n")

    def test_crate_tag_outputs_explicit_non_plugin_result(self):
        result, output = self.invoke(event="push", ref="refs/tags/v0.3.231")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "requested=false\nplugin_path=\n")

    def shell(self, step, extra_env=None):
        return subprocess.run(
            ["bash", "--noprofile", "--norc", "-e", "-o", "pipefail", "-c", run_script(step)],
            cwd=self.root, env={**os.environ, "GITHUB_WORKSPACE": str(self.root),
                                "PLUGIN_PATH": "examples/my plugin", **(extra_env or {})},
            text=True, capture_output=True, check=False,
        )

    def test_actual_test_command_uses_selected_manifest(self):
        self.plugin("examples/my plugin")
        bin_dir = self.root / "bin"
        bin_dir.mkdir()
        cargo = bin_dir / "cargo"
        cargo.write_text("#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$GITHUB_WORKSPACE/args\"\n")
        cargo.chmod(0o755)
        result = self.shell("Run tests", {"PATH": f"{bin_dir}:{os.environ['PATH']}"})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.root / "args").read_text().splitlines(),
                         ["test", "--release", "--manifest-path", "./examples/my plugin/Cargo.toml"])

    def test_actual_publish_commands_use_selected_cwd_without_upload(self):
        selected = self.plugin("examples/my plugin")
        binary = self.root / "target/release/mockforge"
        binary.parent.mkdir(parents=True)
        binary.write_text("#!/bin/sh\nprintf '%s\\n' \"$PWD\" \"$@\" > \"$GITHUB_WORKSPACE/args\"\n")
        binary.chmod(0o755)
        for step, args in (("Publish plugin", ["plugin", "registry", "publish"]),
                           ("Dry run publish", ["plugin", "registry", "publish", "--dry-run"])):
            with self.subTest(step=step):
                result = self.shell(step)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual((self.root / "args").read_text().splitlines(), [str(selected), *args])

    def test_actual_manifest_reader_does_not_treat_selected_path_as_an_option(self):
        self.plugin("-plugin")
        bin_dir = self.root / "bin"
        bin_dir.mkdir()
        yq = bin_dir / "yq"
        yq.write_text("#!/bin/sh\ncase \"$3\" in -*) exit 7;; esac\n"
                      "test -f \"$3\" || exit 8\nprintf '%s\\n' \"$3\" >> \"$GITHUB_WORKSPACE/args\"\n"
                      "printf 'synthetic\\n'\n")
        yq.chmod(0o755)
        output = self.root / "output"
        result = self.shell("Read plugin manifest", {
            "PATH": f"{bin_dir}:{os.environ['PATH']}", "PLUGIN_PATH": "-plugin",
            "GITHUB_OUTPUT": str(output),
        })
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.root / "args").read_text().splitlines(),
                         ["./-plugin/plugin.yaml", "./-plugin/plugin.yaml"])

    def test_actual_packaging_copies_selected_manifest_for_dash_and_space_paths(self):
        for path in ("-plugin", "examples/my plugin"):
            with self.subTest(path=path):
                selected = self.plugin(path)
                target = self.root / "target/fixture-target/release"
                target.mkdir(parents=True, exist_ok=True)
                (target / "test.so").write_text("synthetic native artifact")
                script = run_script("Package plugin").replace(
                    "${{ matrix.target }}", "fixture-target"
                ).replace("${{ matrix.ext }}", "so")
                result = subprocess.run(
                    ["bash", "--noprofile", "--norc", "-e", "-o", "pipefail", "-c", script],
                    cwd=self.root, env={**os.environ, "PLUGIN_PATH": path},
                    text=True, capture_output=True, check=False,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual((self.root / "artifacts/plugin.yaml").read_bytes(),
                                 (selected / "plugin.yaml").read_bytes())


class WorkflowWiringTests(unittest.TestCase):
    def test_only_plugin_tags_enter_plugin_workflow(self):
        triggers = WORKFLOW.read_text().split("\n# Serialize", 1)[0]
        self.assertIn("- 'plugin-*'", triggers)
        self.assertNotIn("- 'v*.*.*'", triggers)
        self.assertIn("workflow_dispatch:", triggers)

    def test_classifier_precedes_compilation_and_validated_path_is_threaded(self):
        workflow = WORKFLOW.read_text()
        self.assertIn("run: python3 scripts/plugin_publish_request.py", workflow)
        self.assertIn("if: needs.request.outputs.requested == 'true'", workflow)
        self.assertLess(workflow.index("- name: Read plugin manifest"), workflow.index("- name: Setup Rust"))
        self.assertEqual(workflow.count("PLUGIN_PATH: ${{ needs.request.outputs.plugin_path }}"), 4)
        self.assertEqual(workflow.count("${{ inputs.plugin_path }}"), 1)
        self.assertIn("needs: [request, validate, security-scan]", workflow)
        self.assertIn("needs: [request, validate]", workflow)
        self.assertIn("needs: [request, validate, build]", workflow)
        self.assertIn("needs: [request, validate, security-scan, build]", workflow)
        self.assertIn('plugin validate "$GITHUB_WORKSPACE/$PLUGIN_PATH"', workflow)
        self.assertIn('cp "./$PLUGIN_PATH/plugin.yaml" artifacts/', workflow)
        self.assertIn('CARGO_TARGET_DIR: ${{ github.workspace }}/target', workflow)
        self.assertIn('run: cargo build --release --manifest-path "./$PLUGIN_PATH/Cargo.toml"', workflow)
        self.assertIn("shell: bash", workflow)

    def test_security_scans_select_the_same_plugin_project(self):
        security = WORKFLOW.read_text().split("\n  security-scan:\n", 1)[1].split("\n  build:\n", 1)[0]
        self.assertIn("needs: [request, validate]", security)
        self.assertEqual(security.count("working-directory: ${{ needs.request.outputs.plugin_path }}"), 2)
        self.assertIn("run: cargo clippy --all-features -- -D warnings", security)
        self.assertIn("command: check licenses sources", security)
        self.assertIn("manifest-path: ./${{ needs.request.outputs.plugin_path }}/Cargo.toml", security)

    def test_publisher_scope_and_destination_are_unchanged(self):
        workflow = WORKFLOW.read_text()
        self.assertIn("cancel-in-progress: false", workflow)
        self.assertIn("MOCKFORGE_REGISTRY_URL: ${{ secrets.MOCKFORGE_REGISTRY_URL || 'https://registry.mockforge.dev' }}", workflow)
        self.assertIn("if: github.event_name == 'push' || (github.event_name == 'workflow_dispatch' && !inputs.dry_run)", workflow)
        self.assertIn("if: github.event_name == 'workflow_dispatch' && inputs.dry_run", workflow)
        self.assertNotIn("packages: write", workflow)
        self.assertNotIn("contents: write", workflow)
        self.assertIn("MOCKFORGE_REGISTRY_TOKEN: ''", workflow)

    def test_contracts_run_in_required_security_audit_context(self):
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        security = workflow.split("\n  security-audit:\n", 1)[1].split("\n  coverage:\n", 1)[0]
        self.assertIn("run: python3 scripts/tests/test_plugin_publish_request.py", security)
        self.assertNotIn("continue-on-error", security)


if __name__ == "__main__":
    unittest.main()
