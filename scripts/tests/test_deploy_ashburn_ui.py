"""Release artifact, atomic pointer, and protected-main guard regressions."""

import importlib
import importlib.util
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest import mock


SCRIPT = Path(__file__).resolve().parents[1] / "deploy-ashburn-ui.py"
sys.path.insert(0, str(SCRIPT.parent))
artifact = importlib.import_module("ashburn_ui_artifact")
release = importlib.import_module("ashburn_ui_release")

SPEC = importlib.util.spec_from_file_location("deploy_ashburn_ui", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
deploy = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(deploy)


class AshburnUiReleaseTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="mockforge-ui-release-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.dist = self.root / "dist"
        (self.dist / "assets").mkdir(parents=True)
        (self.dist / "index.html").write_text('<div id="root"></div>\n')
        (self.dist / "assets/app.js").write_text(
            "const api='https://api.mockforge.dev';\n"
        )
        self.base = self.root / "host/mockforge-ui"
        self.owner_uid = os.geteuid()

    def install(self, sha: str, digest: str, artifact: Path) -> str:
        return release.install_release(
            self.base, sha, digest, artifact, owner_uid=self.owner_uid
        )

    def artifact(self, name: str):
        archive = self.root / name
        digest = artifact.package_dist(self.dist, archive)
        return archive, digest

    def test_deterministic_artifact_and_immutable_release(self) -> None:
        first, digest = self.artifact("first.tar.gz")
        second, second_digest = self.artifact("second.tar.gz")
        self.assertEqual(digest, second_digest)
        with tarfile.open(first, "r:gz") as archive:
            self.assertIn("dist/index.html", archive.getnames())
            self.assertIn("dist/assets/app.js", archive.getnames())
        sha = "a" * 40
        target = self.install(sha, digest, first)
        self.assertEqual(target, f"releases/{sha}/dist")
        self.assertEqual(os.readlink(self.base / "current"), target)
        self.assertIsNone(
            release.valid_release_target(
                self.base, self.base / "previous", self.owner_uid
            )
        )
        self.assertEqual(self.install(sha, digest, second), target)
        manifest = json.loads(
            (self.base / "releases" / sha / "manifest.json").read_text()
        )
        self.assertEqual(manifest["source_sha"], sha)
        self.assertEqual(manifest["artifact_sha256"], digest)
        self.assertEqual((self.base / "releases" / sha).stat().st_mode & 0o777, 0o755)
        self.assertEqual(
            (self.base / "releases" / sha / "dist/index.html").stat().st_mode & 0o777,
            0o444,
        )

    def test_new_release_and_rollback_switch_symlinks_atomically(self) -> None:
        first, first_digest = self.artifact("first.tar.gz")
        first_sha = "a" * 40
        second_sha = "b" * 40
        self.install(first_sha, first_digest, first)
        (self.dist / "assets/app.js").write_text(
            "const api='https://api.mockforge.dev/v2';\n"
        )
        second, second_digest = self.artifact("second.tar.gz")
        self.install(second_sha, second_digest, second)
        self.assertEqual(
            os.readlink(self.base / "previous"), f"releases/{first_sha}/dist"
        )
        self.assertEqual(
            os.readlink(self.base / "current"), f"releases/{second_sha}/dist"
        )
        self.assertEqual(
            release.rollback_release(self.base, owner_uid=self.owner_uid),
            f"releases/{first_sha}/dist",
        )
        self.assertEqual(
            os.readlink(self.base / "current"), f"releases/{first_sha}/dist"
        )
        self.assertEqual(
            os.readlink(self.base / "previous"), f"releases/{second_sha}/dist"
        )

    def test_corrupt_and_changed_artifacts_cannot_replace_current(self) -> None:
        first, digest = self.artifact("first.tar.gz")
        sha = "a" * 40
        self.install(sha, digest, first)
        original = first.read_bytes()
        first.write_bytes(original + b"corrupt")
        with self.assertRaisesRegex(RuntimeError, "SHA256"):
            self.install("b" * 40, digest, first)
        (self.dist / "index.html").write_text("changed")
        changed, changed_digest = self.artifact("changed.tar.gz")
        with self.assertRaisesRegex(RuntimeError, "different content"):
            self.install(sha, changed_digest, changed)
        self.assertEqual(os.readlink(self.base / "current"), f"releases/{sha}/dist")
        first.write_bytes(original)
        installed_archive = self.base / "releases" / sha / "artifact.tar.gz"
        installed_archive.chmod(0o644)
        installed_archive.write_bytes(b"tampered")
        with self.assertRaisesRegex(RuntimeError, "failed verification"):
            self.install(sha, digest, first)
        self.assertEqual(os.readlink(self.base / "current"), f"releases/{sha}/dist")

    def test_artifact_symlink_is_refused(self) -> None:
        (self.dist / "assets/link").symlink_to("/etc/passwd")
        with self.assertRaisesRegex(RuntimeError, "unsupported entry"):
            self.artifact("unsafe.tar.gz")

    def test_uploaded_archive_symlink_is_refused_before_pointer_change(self) -> None:
        artifact = self.root / "linked.tar.gz"
        with tarfile.open(artifact, "w:gz") as archive:
            directory = tarfile.TarInfo("dist")
            directory.type = tarfile.DIRTYPE
            archive.addfile(directory)
            index = tarfile.TarInfo("dist/index.html")
            index.size = 2
            archive.addfile(index, io.BytesIO(b"ok"))
            linked = tarfile.TarInfo("dist/assets")
            linked.type = tarfile.SYMTYPE
            linked.linkname = "/etc"
            archive.addfile(linked)
        digest = hashlib.sha256(artifact.read_bytes()).hexdigest()
        with self.assertRaisesRegex(RuntimeError, "unsafe path or entry"):
            self.install("a" * 40, digest, artifact)
        self.assertFalse((self.base / "current").exists())

    def test_existing_group_writable_release_refuses_pointer_change(self) -> None:
        artifact, digest = self.artifact("first.tar.gz")
        sha = "a" * 40
        self.install(sha, digest, artifact)
        index = self.base / "releases" / sha / "dist/index.html"
        index.chmod(0o664)
        with self.assertRaisesRegex(RuntimeError, "unsafe owner or permissions"):
            self.install(sha, digest, artifact)
        self.assertEqual(os.readlink(self.base / "current"), f"releases/{sha}/dist")
        with self.assertRaisesRegex(RuntimeError, "unsafe owner or permissions"):
            release.rollback_release(self.base, owner_uid=self.owner_uid)

    def test_linked_lock_file_is_refused_without_writing_target(self) -> None:
        artifact, digest = self.artifact("first.tar.gz")
        self.base.mkdir(parents=True)
        target = self.root / "do-not-touch"
        target.write_text("original")
        (self.base / ".deploy.lock").symlink_to(target)
        with self.assertRaises(OSError):
            self.install("a" * 40, digest, artifact)
        self.assertEqual(target.read_text(), "original")
        self.assertFalse((self.base / "current").exists())

    def test_protected_main_guard_rejects_dirty_and_diverged_checkouts(self) -> None:
        sha = "a" * 40
        origin = "git@github.com:SaaSy-Solutions/mockforge.git"
        with (
            mock.patch.object(deploy, "output", side_effect=[origin, " M foo"]),
            mock.patch.object(
                deploy.subprocess,
                "run",
                return_value=subprocess.CompletedProcess([], 0),
            ),
        ):
            with self.assertRaisesRegex(RuntimeError, "dirty"):
                deploy.require_protected_main()
        with (
            mock.patch.object(
                deploy, "output", side_effect=[origin, "", sha, "b" * 40]
            ),
            mock.patch.object(
                deploy.subprocess,
                "run",
                return_value=subprocess.CompletedProcess([], 0),
            ),
        ):
            with self.assertRaisesRegex(RuntimeError, "not exactly origin/main"):
                deploy.require_protected_main()

    def test_remote_upload_includes_installer_modules(self) -> None:
        sha = "a" * 40
        with (
            mock.patch.object(deploy, "build_ui", return_value=self.dist),
            mock.patch.object(deploy, "package_dist", return_value="b" * 64),
            mock.patch.object(
                deploy,
                "remote_stage",
                return_value="/var/lib/saasy/mockforge-ui/.incoming.abcdefgh",
            ),
            mock.patch.object(
                deploy, "remote_command", return_value=f"releases/{sha}/dist"
            ),
            mock.patch.object(deploy.subprocess, "run") as run,
        ):
            deploy.deploy(sha)
        upload = run.call_args_list[0].args[0]
        self.assertEqual(upload[0], "scp")
        self.assertEqual(
            [Path(path).name for path in upload[3:6]],
            [
                "deploy-ashburn-ui.py",
                "ashburn_ui_artifact.py",
                "ashburn_ui_release.py",
            ],
        )


if __name__ == "__main__":
    unittest.main()
