"""Guard MockForge's Ashburn image supply inventory and demo command."""

from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
FILENAMES = ['ashburn-images.yml', 'docker-build.yml']


class AshburnImagesTest(unittest.TestCase):
    def test_publisher_uses_trusted_hosted_orchestrator(self) -> None:
        smoke = (ROOT / ".github/workflows/ashburn-image-smoke.yml").read_text()
        guard = (ROOT / "scripts/verify-image-publisher.sh").read_text()
        self.assertIn("name=rootless", guard)
        self.assertIn("isolated-host", guard)
        self.assertIn("attestation directory must be root owned", guard)
        self.assertIn("saasy-ci-fsn-02", guard)
        self.assertNotIn("packages: write", smoke)
        for filename in FILENAMES:
            workflow = (ROOT / ".github/workflows" / filename).read_text()
            self.assertIn("runs-on: ubuntu-latest", workflow)
            self.assertNotIn("group: mockforge-image-publish", workflow)
            self.assertIn("fly_buildkit_publish.py", workflow)
            self.assertIn("FLY_IMAGE_PUBLISHER_TOKEN", workflow)
            self.assertIn("persist-credentials: false", workflow)
            self.assertIn("--cleanup-only", workflow)
            self.assertIn("packages: write", workflow)

    def test_registry_and_tunnel_dockerfiles_are_published_serially(self) -> None:
        workflow = (ROOT / ".github/workflows/ashburn-images.yml").read_text()
        core_workflow = (ROOT / ".github/workflows/docker-build.yml").read_text()
        smoke = (ROOT / ".github/workflows/ashburn-image-smoke.yml").read_text()
        for dockerfile in ("Dockerfile.registry", "Dockerfile.tunnel"):
            self.assertTrue((ROOT / dockerfile).is_file())
            self.assertIn(dockerfile, workflow)
        self.assertIn("max-parallel: 1", workflow)
        self.assertIn("group: mockforge-image-builds", workflow)
        self.assertIn("group: mockforge-image-builds", core_workflow)
        self.assertIn("if: github.ref == 'refs/heads/main' && github.ref_protected", workflow)
        self.assertIn("github.ref == 'refs/heads/main'", core_workflow)
        self.assertIn("github.ref_protected", core_workflow)
        self.assertNotIn("refs/heads/develop", core_workflow)
        self.assertIn("startsWith(github.ref, 'refs/tags/v')", core_workflow)
        self.assertIn("github.event_name == 'workflow_dispatch' && github.ref == 'refs/heads/main'", core_workflow)
        isolation = (ROOT / "docs/IMAGE_PUBLISHER_ISOLATION.md").read_text()
        self.assertIn('mockforge-image-publisher', isolation)
        self.assertIn('FLY_IMAGE_PUBLISHER_TOKEN', isolation)
        self.assertIn('protected `v*`', isolation)
        for publish in (workflow, core_workflow):
            self.assertNotIn("  pull_request:", publish)
            self.assertIn("packages: write", publish)
            self.assertIn("fly_buildkit_publish.py", publish)
        self.assertIn("pull_request:", smoke)
        self.assertIn("contents: read", smoke)
        self.assertNotIn("packages: write", smoke)
        self.assertNotIn("docker/login-action", smoke)
        self.assertIn("push: false", smoke)
        self.assertIn("Dockerfile.registry", smoke)
        self.assertIn("BUILD_DATE: ${{ steps.build-date.outputs.value }}", core_workflow)
        self.assertIn("docker buildx imagetools create", core_workflow)

    def test_no_write_planner_and_summary_use_hosted_runners(self) -> None:
        workflow = (ROOT / ".github/workflows/ashburn-images.yml").read_text()
        plan = workflow.split("\n  plan:\n", 1)[1].split("\n  build:\n", 1)[0]
        build = workflow.split("\n  build:\n", 1)[1].split("\n  images-built:\n", 1)[0]
        summary = workflow.split("\n  images-built:\n", 1)[1]
        for no_write_job in (plan, summary):
            self.assertIn("runs-on: ubuntu-latest", no_write_job)
            self.assertNotIn("packages: write", no_write_job)
            self.assertNotIn("docker/login-action", no_write_job)
        self.assertIn("runs-on: ubuntu-latest", build)
        self.assertIn("packages: write", build)

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
