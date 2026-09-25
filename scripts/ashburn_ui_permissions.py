"""Ownership checks and atomic release pointers for the Ashburn UI."""

from __future__ import annotations

import fcntl
import os
from pathlib import Path
import re
import stat


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
