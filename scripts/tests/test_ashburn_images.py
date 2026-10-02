"""Guard MockForge's Ashburn image supply inventory and demo command."""

from pathlib import Path
import json
import os
import re
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]
FILENAMES = ['ashburn-images.yml', 'docker-build.yml']


class AshburnImagesTest(unittest.TestCase):
    def test_publishers_build_on_self_hosted_runners_with_private_state(self) -> None:
        smoke = (ROOT / ".github/workflows/ashburn-image-smoke.yml").read_text()
        self.assertNotIn("packages: write", smoke)
        for filename in FILENAMES:
            workflow = (ROOT / ".github/workflows" / filename).read_text()
            self.assertIn("runs-on: [self-hosted, linux, x64, rust]", workflow)
            self.assertNotIn("fly", workflow.lower())
            self.assertIn("persist-credentials: false", workflow)
            self.assertIn("packages: write", workflow)
            # GHCR credential and buildx state live in a private per-job dir.
            self.assertIn('mktemp -d "$RUNNER_TEMP/ghcr-auth-XXXXXXXX")', workflow)
            self.assertIn('>> "$GITHUB_ENV"', workflow)
            # A fresh builder per run, removed explicitly before the private
            # config dir it is registered in is deleted.
            self.assertIn("docker/setup-buildx-action", workflow)
            self.assertIn("cleanup: false", workflow)
            self.assertIn("${{ github.run_id }}-${{ github.run_attempt }}", workflow)
            self.assertIn('docker buildx rm --force "$BUILDER"', workflow)
            self.assertLess(workflow.index("docker buildx rm"), workflow.index('rm -rf -- "$DOCKER_CONFIG"'))
            self.assertIn("docker/build-push-action", workflow)
            self.assertIn("provenance: false", workflow)
            self.assertIn(":buildcache", workflow)

    def test_fly_publisher_is_gone(self) -> None:
        for path in (
            "scripts/ci/fly_buildkit_publish.py",
            "scripts/verify-image-publisher.sh",
            ".github/workflows/fly-publisher-cleanup.yml",
            ".github/actions/setup-flyctl",
        ):
            self.assertFalse((ROOT / path).exists(), path)

    def test_registry_and_tunnel_dockerfiles_are_published_serially(self) -> None:
        workflow = (ROOT / ".github/workflows/ashburn-images.yml").read_text()
        core_workflow = (ROOT / ".github/workflows/docker-build.yml").read_text()
        smoke = (ROOT / ".github/workflows/ashburn-image-smoke.yml").read_text()
        for dockerfile in ("Dockerfile.egress", "Dockerfile.registry", "Dockerfile.runner", "Dockerfile.tunnel"):
            self.assertTrue((ROOT / dockerfile).is_file())
            self.assertIn(dockerfile, workflow)
            # A Dockerfile-only change must still trigger the publisher.
            self.assertIn(f"      - '{dockerfile}'", workflow.split("workflow_dispatch:", 1)[0])
        self.assertIn("max-parallel: 1", workflow)
        self.assertIn("group: mockforge-image-builds", workflow)
        self.assertIn("group: mockforge-image-builds", core_workflow)
        self.assertIn("github.ref == 'refs/heads/main' && github.ref_protected", workflow)
        self.assertIn("github.ref == 'refs/heads/main'", core_workflow)
        self.assertIn("github.ref_protected", core_workflow)
        self.assertNotIn("refs/heads/develop", core_workflow)
        self.assertIn("startsWith(github.ref, 'refs/tags/v')", core_workflow)
        self.assertIn("github.event_name == 'workflow_dispatch' && github.ref == 'refs/heads/main'", core_workflow)
        isolation = (ROOT / "docs/IMAGE_PUBLISHER_ISOLATION.md").read_text()
        self.assertIn("[self-hosted, linux, x64, rust]", isolation)
        self.assertIn("Threat model", isolation)
        self.assertIn('protected `v*`', isolation)
        for publish in (workflow, core_workflow):
            self.assertNotIn("  pull_request:", publish)
            self.assertIn("packages: write", publish)
        self.assertIn("pull_request:", smoke)
        self.assertIn("contents: read", smoke)
        self.assertNotIn("packages: write", smoke)
        self.assertNotIn("docker/login-action", smoke)
        self.assertIn("push: false", smoke)
        self.assertIn("Dockerfile.registry", smoke)
        self.assertIn("Dockerfile.runner", smoke)
        self.assertIn("Dockerfile.egress", smoke)
        self.assertIn("bash scripts/tests/k6_egress_e2e.sh", smoke)
        self.assertIn("BUILD_DATE=${{ steps.build-date.outputs.value }}", core_workflow)
        self.assertIn("docker buildx imagetools create", core_workflow)
        self.assertIn("image_digest: ${{ steps.build-and-push.outputs.digest }}", core_workflow)
        self.assertIn("@${{ needs.build-and-push.outputs.image_digest }}", core_workflow)

    def test_no_write_planner_and_summary_use_hosted_runners(self) -> None:
        workflow = (ROOT / ".github/workflows/ashburn-images.yml").read_text()
        plan = workflow.split("\n  plan:\n", 1)[1].split("\n  build:\n", 1)[0]
        build = workflow.split("\n  build:\n", 1)[1].split("\n  images-built:\n", 1)[0]
        summary = workflow.split("\n  images-built:\n", 1)[1]
        for no_write_job in (plan, summary):
            self.assertIn("runs-on: ubuntu-latest", no_write_job)
            self.assertNotIn("packages: write", no_write_job)
            self.assertNotIn("docker/login-action", no_write_job)
        self.assertIn("runs-on: [self-hosted, linux, x64, rust]", build)
        self.assertIn("packages: write", build)

    def test_auto_publish_is_opt_in_without_blocking_manual_canary(self) -> None:
        ashburn = (ROOT / ".github/workflows/ashburn-images.yml").read_text()
        root_image = (ROOT / ".github/workflows/docker-build.yml").read_text()
        self.assertEqual(ashburn.count("vars.MOCKFORGE_AUTO_IMAGE_PUBLISH == 'true'"), 3)
        self.assertIn("vars.MOCKFORGE_AUTO_IMAGE_PUBLISH == 'true'", root_image)
        self.assertIn("github.event_name == 'workflow_dispatch'", ashburn)
        self.assertIn("github.event_name == 'workflow_dispatch' && github.ref == 'refs/heads/main'", root_image)

    def test_manual_canary_selects_one_known_image(self) -> None:
        workflow = (ROOT / ".github/workflows/ashburn-images.yml").read_text()
        script = workflow.split("python3 - <<'PY' >> \"$GITHUB_OUTPUT\"\n", 1)[1].split("\n          PY", 1)[0]
        script = "\n".join(line.removeprefix("          ") for line in script.splitlines())
        env = {**os.environ, "WANTED_IMAGE": "mockforge-tunnel-relay"}
        result = subprocess.run(["python3", "-c", script], env=env, text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        matrix = json.loads(result.stdout.removeprefix("matrix="))
        self.assertEqual(matrix["include"], [{"app": "mockforge-tunnel-relay", "dockerfile": "Dockerfile.tunnel"}])
        env["WANTED_IMAGE"] = "mockforge-test-runner"
        runner = subprocess.run(["python3", "-c", script], env=env, text=True, capture_output=True)
        self.assertEqual(runner.returncode, 0, runner.stderr)
        self.assertEqual(
            json.loads(runner.stdout.removeprefix("matrix="))["include"],
            [{"app": "mockforge-test-runner", "dockerfile": "Dockerfile.runner"}],
        )
        env["WANTED_IMAGE"] = ""
        everything = subprocess.run(["python3", "-c", script], env=env, text=True, capture_output=True)
        self.assertEqual(everything.returncode, 0, everything.stderr)
        self.assertEqual(
            [entry["app"] for entry in json.loads(everything.stdout.removeprefix("matrix="))["include"]],
            ["mockforge-egress", "mockforge-registry", "mockforge-test-runner", "mockforge-tunnel-relay"],
        )
        env["WANTED_IMAGE"] = "not-an-image"
        invalid = subprocess.run(["python3", "-c", script], env=env, text=True, capture_output=True)
        self.assertNotEqual(invalid.returncode, 0)
        self.assertIn("unknown image", invalid.stderr)

    def test_runner_env_docs_match_what_the_runner_reads(self) -> None:
        # fly.runner.toml once documented REDIS_URL, MOCKFORGE_RUNNER_REGISTRY_URL
        # and MOCKFORGE_RUNNER_CALLBACK_TOKEN, none of which the runner reads,
        # so a deploy following it would exit at startup on a missing variable.
        config = (ROOT / "crates/mockforge-test-runner/src/config.rs").read_text()
        read = set(re.findall(r'"(MOCKFORGE_RUNNER_[A-Z0-9_]+)"', config))
        required = set(re.findall(r'required_env\("(MOCKFORGE_RUNNER_[A-Z0-9_]+)"\)', config))
        self.assertEqual(
            required,
            {
                "MOCKFORGE_RUNNER_REDIS_URL",
                "MOCKFORGE_RUNNER_REGISTRY_INTERNAL_BASE_URL",
                "MOCKFORGE_RUNNER_REGISTRY_INTERNAL_TOKEN",
            },
        )
        guide = (ROOT / "docs/ASHBURN_IMAGE_SUPPLY.md").read_text()
        fly = (ROOT / "fly.runner.toml").read_text()
        for name in read:
            self.assertIn(f"`{name}`", guide)
        for name in required:
            self.assertIn(name, fly)
        for stale in ("MOCKFORGE_RUNNER_REGISTRY_URL ", "MOCKFORGE_RUNNER_CALLBACK_TOKEN", "#   REDIS_URL"):
            self.assertNotIn(stale, fly)
        # The registry hardcodes the queue key; the runner default must match it.
        run_queue = (ROOT / "crates/mockforge-registry-server/src/run_queue.rs").read_text()
        self.assertIn('DEFAULT_QUEUE_KEY: &str = "test_runs:queued"', run_queue)
        self.assertIn('"test_runs:queued"', config)
        self.assertIn("MOCKFORGE_SSRF_ALLOW_LOOPBACK = \"0\"", fly)

    def test_demo_command_is_documented_before_repointing_image(self) -> None:
        fly_config = (ROOT / "fly.demo.toml").read_text()
        guide = (ROOT / "docs/ASHBURN_IMAGE_SUPPLY.md").read_text()
        self.assertIn("serve --spec", fly_config)
        for path in (
            "ecommerce-store/openapi.json",
            "weather-geo/openapi.json",
            "chat-api/openapi.json",
        ):
            self.assertIn(path, guide)
        self.assertIn("command:", guide)


if __name__ == "__main__":
    unittest.main()
