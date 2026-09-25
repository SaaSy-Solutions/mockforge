"""Root-owned immutable release installation and atomic rollback on Ashburn."""

from __future__ import annotations

import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import tarfile
import tempfile

from ashburn_ui_artifact import API_BASE_URL, validate_artifact


SHA_PATTERN = re.compile(r"[0-9a-f]{40}\Z")
TARGET_PATTERN = re.compile(r"releases/[0-9a-f]{40}/dist\Z")


def require_owned_path(path: Path, owner_uid: int, *, directory: bool) -> None:
    info = path.lstat()
    if info.st_uid != owner_uid or stat.S_IMODE(info.st_mode) & 0o022:
        raise RuntimeError(f"release path has unsafe owner or permissions: {path}")
    if directory and not stat.S_ISDIR(info.st_mode):
        raise RuntimeError(f"release path is not a directory: {path}")
    if not directory and not stat.S_ISREG(info.st_mode):
        raise RuntimeError(f"release path is not a regular file: {path}")


def require_release_tree(release: Path, owner_uid: int) -> None:
    require_owned_path(release, owner_uid, directory=True)
    for path in release.rglob("*"):
        require_owned_path(path, owner_uid, directory=path.is_dir())


def release_lock(base: Path, owner_uid: int):
    path = base / ".deploy.lock"
    descriptor = os.open(path, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    lock = os.fdopen(descriptor, "r+")
    try:
        require_owned_path(path, owner_uid, directory=False)
        fcntl.flock(lock, fcntl.LOCK_EX)
        return lock
    except Exception:
        lock.close()
        raise


def valid_release_target(base: Path, link: Path, owner_uid: int = 0) -> str | None:
    if not link.exists() and not link.is_symlink():
        return None
    if not link.is_symlink():
        raise RuntimeError(f"release pointer is not a symlink: {link}")
    if link.lstat().st_uid != owner_uid:
        raise RuntimeError(f"release pointer has an unsafe owner: {link}")
    target = os.readlink(link)
    if not TARGET_PATTERN.fullmatch(target) or not (base / target).is_dir():
        raise RuntimeError(f"release pointer has an invalid target: {link}")
    require_release_tree((base / target).parent, owner_uid)
    return target


def atomic_link(base: Path, name: str, target: str) -> None:
    temporary = base / f".{name}.{os.getpid()}.tmp"
    try:
        temporary.symlink_to(target)
        os.replace(temporary, base / name)
    finally:
        temporary.unlink(missing_ok=True)


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
