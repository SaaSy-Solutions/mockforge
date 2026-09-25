"""Root-owned immutable release installation and atomic rollback on Ashburn."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import tarfile
import tempfile

from ashburn_ui_artifact import API_BASE_URL, validate_artifact
from ashburn_ui_permissions import (
    atomic_link,
    release_lock,
    require_owned_path,
    require_release_tree,
    valid_release_target,
)


SHA_PATTERN = re.compile(r"[0-9a-f]{40}\Z")


def install_release(
    base: Path, sha: str, digest: str, artifact: Path, *, owner_uid: int = 0
) -> str:
    if not SHA_PATTERN.fullmatch(sha):
        raise RuntimeError("invalid source SHA")
    validate_artifact(artifact, digest)
    if base.exists() or base.is_symlink():
        require_owned_path(base, owner_uid, directory=True)
    else:
        base.mkdir(mode=0o755, parents=True)
    require_owned_path(base, owner_uid, directory=True)
    releases = base / "releases"
    releases.mkdir(mode=0o755, exist_ok=True)
    require_owned_path(releases, owner_uid, directory=True)
    with release_lock(base, owner_uid):
        release = releases / sha
        if release.exists() or release.is_symlink():
            if release.is_symlink() or not release.is_dir():
                raise RuntimeError("existing release path is invalid")
            require_release_tree(release, owner_uid)
            manifest = json.loads((release / "manifest.json").read_text())
            if manifest != {
                "source_sha": sha,
                "artifact_sha256": digest,
                "api_base_url": API_BASE_URL,
            }:
                raise RuntimeError("existing immutable release has different content")
            if (
                not (release / "dist/index.html").is_file()
                or hashlib.sha256(
                    (release / "artifact.tar.gz").read_bytes()
                ).hexdigest()
                != digest
            ):
                raise RuntimeError("existing immutable release failed verification")
        else:
            with tempfile.TemporaryDirectory(
                prefix=".release.", dir=base
            ) as staging_name:
                staging = Path(staging_name)
                with tarfile.open(artifact, "r:gz") as archive:
                    archive.extractall(staging, filter="data")
                shutil.copyfile(artifact, staging / "artifact.tar.gz")
                manifest = {
                    "source_sha": sha,
                    "artifact_sha256": digest,
                    "api_base_url": API_BASE_URL,
                }
                (staging / "manifest.json").write_text(
                    json.dumps(manifest, sort_keys=True) + "\n"
                )
                for path in staging.rglob("*"):
                    path.chmod(0o755 if path.is_dir() else 0o444)
                staging.chmod(0o755)
                require_release_tree(staging, owner_uid)
                os.rename(staging, release)
        new_target = f"releases/{sha}/dist"
        old_target = valid_release_target(base, base / "current", owner_uid)
        if old_target != new_target:
            if old_target:
                atomic_link(base, "previous", old_target)
            atomic_link(base, "current", new_target)
        return new_target


def rollback_release(base: Path, *, owner_uid: int = 0) -> str:
    require_owned_path(base, owner_uid, directory=True)
    require_owned_path(base / "releases", owner_uid, directory=True)
    with release_lock(base, owner_uid):
        previous = valid_release_target(base, base / "previous", owner_uid)
        current = valid_release_target(base, base / "current", owner_uid)
        if not previous or not current:
            raise RuntimeError("rollback needs valid current and previous releases")
        atomic_link(base, "current", previous)
        atomic_link(base, "previous", current)
        return previous
