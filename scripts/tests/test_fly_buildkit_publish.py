#!/usr/bin/env python3
"""Provider-side failure and cleanup contracts for disposable image builds."""

from __future__ import annotations

import base64
import importlib.util
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
MODULE = ROOT / "scripts/ci/fly_buildkit_publish.py"
spec = importlib.util.spec_from_file_location("fly_buildkit_publish", MODULE)
assert spec is not None and spec.loader is not None
publisher = importlib.util.module_from_spec(spec)
spec.loader.exec_module(publisher)


class FlyBuildkitPublishTest(unittest.TestCase):
    def test_unprotected_tag_is_rejected_before_machine_creation(self) -> None:
        env = {
            "GITHUB_SHA": "a" * 40,
            "GITHUB_REPOSITORY": "SaaSy-Solutions/mockforge",
            "GITHUB_REF": "refs/tags/v1.2.3",
            "GITHUB_REF_PROTECTED": "false",
            "IMAGE_APP": "mockforge",
            "IMAGE_DOCKERFILE": "Dockerfile",
        }
        with patch.dict(os.environ, env), patch.object(publisher, "fly") as fly:
            with self.assertRaisesRegex(RuntimeError, "protected main or root release tag"):
                publisher.publish()
            fly.assert_not_called()

    def test_registry_auth_is_private_and_contains_expected_principal(self) -> None:
        with tempfile.TemporaryDirectory() as directory, patch.dict(
            os.environ, {"GITHUB_ACTOR": "release-bot", "GITHUB_TOKEN": "fixture-token"}
        ):
            path = Path(directory) / "config.json"
            publisher.write_auth(path)
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)
            encoded = json.loads(path.read_text())["auths"]["ghcr.io"]["auth"]
            self.assertEqual(base64.b64decode(encoded), b"release-bot:fixture-token")

    def test_archive_requires_exact_checkout_sha(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(RuntimeError, "differs from requested main SHA"):
                publisher.archive_head(Path(directory) / "source.tar.gz", "0" * 40)

    def test_machine_name_rejects_untrusted_fields(self) -> None:
        with patch.dict(os.environ, {"GITHUB_RUN_ID": "123", "GITHUB_RUN_ATTEMPT": "1", "IMAGE_APP": "mockforge-registry"}):
            self.assertEqual(publisher.machine_name(), "pub-123-1-mockforge-registry")
        with patch.dict(os.environ, {"GITHUB_RUN_ID": "123", "GITHUB_RUN_ATTEMPT": "1", "IMAGE_APP": "x;id"}):
            with self.assertRaises(ValueError):
                publisher.machine_name()

    def test_cleanup_recovers_machine_by_unique_name(self) -> None:
        name = "pub-123-1-mockforge-registry"
        rows = iter([
            json.dumps([{"id": "d8911154b66358", "name": name}]),
            json.dumps([]),
        ])
        calls: list[tuple[str, ...]] = []

        def fake_fly(*args: str, **_kwargs: object) -> subprocess.CompletedProcess[str]:
            calls.append(args)
            if args[:2] == ("machine", "list"):
                return subprocess.CompletedProcess([], 0, next(rows), "")
            return subprocess.CompletedProcess([], 0, "", "")

        with patch.object(publisher, "fly", side_effect=fake_fly):
            publisher.destroy_named(name)
        self.assertIn(("machine", "destroy", "d8911154b66358", "-a", publisher.APP, "--force"), calls)

    def test_guest_build_failure_still_deletes_machine(self) -> None:
        sha = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
        env = {
            "GITHUB_SHA": sha,
            "GITHUB_REPOSITORY": "SaaSy-Solutions/mockforge",
            "GITHUB_REF": "refs/heads/main",
            "GITHUB_REF_PROTECTED": "true",
            "GITHUB_RUN_ID": "123",
            "GITHUB_RUN_ATTEMPT": "1",
            "GITHUB_ACTOR": "tester",
            "GITHUB_TOKEN": "fake-package-token",
            "FLY_IMAGE_PUBLISHER_TOKEN": "fake-fly-token",
            "IMAGE_APP": "mockforge-registry",
            "IMAGE_DOCKERFILE": "Dockerfile.registry",
        }
        assert (ROOT / env["IMAGE_DOCKERFILE"]).is_file()
        rows = iter([
            [{"id": "d8911154b66358", "name": "pub-123-1-mockforge-registry"}],
            [{"id": "d8911154b66358", "name": "pub-123-1-mockforge-registry"}],
            [],
        ])
        destroyed: list[str] = []

        def fake_fly(*args: str, **_kwargs: object) -> subprocess.CompletedProcess[str]:
            if args[:2] == ("machine", "run"):
                return subprocess.CompletedProcess([], 0, "Machine ID: d8911154b66358\n", "")
            if args[:2] == ("machine", "list"):
                return subprocess.CompletedProcess([], 0, json.dumps(next(rows)), "")
            if args[:2] == ("machine", "destroy"):
                destroyed.append(args[2])
            return subprocess.CompletedProcess([], 0, "", "")

        def fake_guest(_machine: str, command: str, **_kwargs: object) -> str:
            if "buildctl" in command:
                raise RuntimeError("synthetic build failure")
            return ""

        with (
            patch.dict(os.environ, env),
            patch.object(publisher, "fly", side_effect=fake_fly),
            patch.object(publisher, "guest", side_effect=fake_guest),
            patch.object(publisher, "archive_head", side_effect=lambda p, _s: p.write_bytes(b"tar")),
            patch.object(publisher, "write_auth", side_effect=lambda p: p.write_bytes(b"auth")),
            patch.object(publisher, "upload"),
            patch.object(publisher.Path, "is_file", return_value=True),
        ):
            with self.assertRaisesRegex(RuntimeError, "synthetic build failure"):
                publisher.publish()
        self.assertEqual(destroyed, ["d8911154b66358"])

    def test_root_image_preserves_release_build_args_and_digest(self) -> None:
        sha = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
        env = {
            "GITHUB_SHA": sha,
            "GITHUB_REPOSITORY": "SaaSy-Solutions/mockforge",
            "GITHUB_REF": "refs/tags/v1.2.3",
            "GITHUB_REF_PROTECTED": "true",
            "GITHUB_RUN_ID": "124",
            "GITHUB_RUN_ATTEMPT": "1",
            "GITHUB_ACTOR": "tester",
            "GITHUB_TOKEN": "fake-package-token",
            "FLY_IMAGE_PUBLISHER_TOKEN": "fake-fly-token",
            "IMAGE_APP": "mockforge",
            "IMAGE_DOCKERFILE": "Dockerfile",
            "BUILD_VERSION": "1.2.3",
            "BUILD_DATE": "2026-09-26T20:00:00Z",
        }
        rows = iter([
            [{"id": "d8911154b66358", "name": "pub-124-1-mockforge"}],
            [{"id": "d8911154b66358", "name": "pub-124-1-mockforge"}],
            [],
        ])
        commands: list[str] = []
        destroyed: list[str] = []

        def fake_fly(*args: str, **_kwargs: object) -> subprocess.CompletedProcess[str]:
            if args[:2] == ("machine", "run"):
                return subprocess.CompletedProcess([], 0, "Machine ID: d8911154b66358\n", "")
            if args[:2] == ("machine", "list"):
                return subprocess.CompletedProcess([], 0, json.dumps(next(rows)), "")
            if args[:2] == ("machine", "destroy"):
                destroyed.append(args[2])
            return subprocess.CompletedProcess([], 0, "", "")

        def fake_guest(_machine: str, command: str, **_kwargs: object) -> str:
            commands.append(command)
            if "cat /tmp/publisher/metadata.json" in command:
                return json.dumps({"containerimage.digest": "sha256:" + "a" * 64})
            return ""

        with (
            patch.dict(os.environ, env),
            patch.object(publisher, "fly", side_effect=fake_fly),
            patch.object(publisher, "guest", side_effect=fake_guest),
            patch.object(publisher, "archive_head", side_effect=lambda p, _s: p.write_bytes(b"tar")),
            patch.object(publisher, "write_auth", side_effect=lambda p: p.write_bytes(b"auth")),
            patch.object(publisher, "upload"),
            patch.object(publisher.Path, "is_file", return_value=True),
        ):
            digest = publisher.publish()
        self.assertEqual(digest, "sha256:" + "a" * 64)
        build = next(command for command in commands if "buildctl" in command)
        self.assertIn("--opt build-arg:VERSION=1.2.3", build)
        self.assertIn("--opt build-arg:COMMIT_SHA=" + sha, build)
        self.assertIn("--opt build-arg:BUILD_DATE=2026-09-26T20:00:00Z", build)
        self.assertNotIn("fake-package-token", build)
        self.assertEqual(destroyed, ["d8911154b66358"])


if __name__ == "__main__":
    unittest.main()
