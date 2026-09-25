#!/usr/bin/env python3
"""Publish the exact protected-main cloud UI as an immutable Ashburn release."""

from __future__ import annotations

import argparse
import fcntl
import gzip
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import shlex
import shutil
import socket
import stat
import subprocess
import sys
import tarfile
import tempfile


REPO = Path(__file__).resolve().parents[1]
UI = REPO / "crates/mockforge-ui/ui"
REMOTE_HOST = "saasy-ash-01"
REMOTE_BASE = Path("/var/lib/saasy/mockforge-ui")
API_BASE_URL = "https://api.mockforge.dev"
SHA_PATTERN = re.compile(r"[0-9a-f]{40}\Z")
DIGEST_PATTERN = re.compile(r"[0-9a-f]{64}\Z")
STAGE_PATTERN = re.compile(r"/var/lib/saasy/mockforge-ui/\.incoming\.[A-Za-z0-9]{8,}\Z")
TARGET_PATTERN = re.compile(r"releases/[0-9a-f]{40}/dist\Z")


def output(command: list[str], *, cwd: Path = REPO) -> str:
    return subprocess.run(
        command, cwd=cwd, capture_output=True, text=True, check=True
    ).stdout.strip()


def require_protected_main() -> str:
    origin = output(["git", "remote", "get-url", "origin"])
    if origin not in (
        "git@github.com:SaaSy-Solutions/mockforge.git",
        "https://github.com/SaaSy-Solutions/mockforge.git",
    ):
        raise RuntimeError("origin is not SaaSy-Solutions/mockforge")
    subprocess.run(["git", "fetch", "--quiet", "origin", "main"], cwd=REPO, check=True)
    if output(["git", "status", "--porcelain=v1", "--untracked-files=all"]):
        raise RuntimeError("checkout is dirty")
    head = output(["git", "rev-parse", "HEAD"])
    main = output(["git", "rev-parse", "refs/remotes/origin/main"])
    if head != main or not SHA_PATTERN.fullmatch(head):
        raise RuntimeError("checkout is not exactly origin/main")
    protected = output(
        [
            "gh",
            "api",
            "repos/SaaSy-Solutions/mockforge/branches/main",
            "--jq",
            ".protected",
        ]
    )
    if protected != "true":
        raise RuntimeError("GitHub main branch is not protected")
    return head


def build_ui() -> Path:
    version = output(["corepack", "pnpm", "--version"], cwd=UI)
    if version != "10.15.0":
        raise RuntimeError(f"expected pnpm 10.15.0, got {version}")
    subprocess.run(
        ["corepack", "pnpm", "install", "--frozen-lockfile"], cwd=UI, check=True
    )
    env = os.environ.copy()
    env.update(
        VITE_MOCKFORGE_MODE="cloud",
        VITE_API_BASE_URL=API_BASE_URL,
        NODE_OPTIONS="--max-old-space-size=2048",
    )
    subprocess.run(["corepack", "pnpm", "build"], cwd=UI, env=env, check=True)
    dist = UI / "dist"
    if not (dist / "index.html").is_file() or not list((dist / "assets").glob("*.js")):
        raise RuntimeError("cloud UI build produced no index.html or JavaScript assets")
    return dist


def package_dist(dist: Path, artifact: Path) -> str:
    if dist.is_symlink() or not dist.is_dir():
        raise RuntimeError("dist is missing or linked")
    files = sorted(dist.rglob("*"))
    if not (dist / "index.html").is_file() or not files:
        raise RuntimeError("dist has no application entrypoint")
    with tempfile.TemporaryFile() as raw:
        with tarfile.open(fileobj=raw, mode="w") as archive:
            for path in [dist, *files]:
                if path.is_symlink() or not (path.is_file() or path.is_dir()):
                    raise RuntimeError(f"dist contains an unsupported entry: {path}")
                name = (
                    "dist"
                    if path == dist
                    else f"dist/{path.relative_to(dist).as_posix()}"
                )
                info = archive.gettarinfo(str(path), arcname=name)
                info.uid = info.gid = 0
                info.uname = info.gname = ""
                info.mtime = 0
                info.mode = 0o755 if path.is_dir() else 0o644
                with path.open("rb") if path.is_file() else io.BytesIO() as source:
                    archive.addfile(info, source if path.is_file() else None)
        raw.seek(0)
        with artifact.open("wb") as destination:
            with gzip.GzipFile(
                fileobj=destination, mode="wb", filename="", mtime=0
            ) as compressed:
                shutil.copyfileobj(raw, compressed)
    return hashlib.sha256(artifact.read_bytes()).hexdigest()


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


def validate_artifact(artifact: Path, digest: str) -> None:
    if not DIGEST_PATTERN.fullmatch(digest):
        raise RuntimeError("invalid artifact digest")
    if hashlib.sha256(artifact.read_bytes()).hexdigest() != digest:
        raise RuntimeError("artifact SHA256 does not match upload")
    with tarfile.open(artifact, "r:gz") as archive:
        members = archive.getmembers()
        if not members or len(members) > 10_000:
            raise RuntimeError("artifact member count is invalid")
        total = 0
        for member in members:
            path = PurePosixPath(member.name)
            if (
                path.is_absolute()
                or ".." in path.parts
                or path.parts[0] != "dist"
                or not (member.isfile() or member.isdir())
            ):
                raise RuntimeError("artifact contains an unsafe path or entry")
            total += member.size
        if total > 512 * 1024 * 1024:
            raise RuntimeError("artifact is larger than 512 MiB")
        if not any(
            member.name == "dist/index.html" and member.isfile() for member in members
        ):
            raise RuntimeError("artifact has no application entrypoint")


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


def remote_stage() -> str:
    command = (
        "install -d -m 0755 /var/lib/saasy/mockforge-ui/releases && "
        "mktemp -d /var/lib/saasy/mockforge-ui/.incoming.XXXXXXXXXX"
    )
    stage = output(["ssh", "-o", "BatchMode=yes", REMOTE_HOST, command])
    if not STAGE_PATTERN.fullmatch(stage):
        raise RuntimeError("Ashburn returned an unexpected staging path")
    return stage


def remote_command(stage: str, args: list[str]) -> str:
    command = shlex.join(["python3", f"{stage}/deploy-ashburn-ui.py", *args])
    return output(["ssh", "-o", "BatchMode=yes", REMOTE_HOST, command])


def deploy(sha: str, *, rollback: bool = False) -> None:
    with tempfile.TemporaryDirectory(prefix="mockforge-ui-upload-") as temporary:
        folder = Path(temporary)
        artifact = folder / "artifact.tar.gz"
        digest = ""
        if not rollback:
            dist = build_ui()
            digest = package_dist(dist, artifact)
        stage = remote_stage()
        try:
            subprocess.run(
                [
                    "scp",
                    "-o",
                    "BatchMode=yes",
                    str(Path(__file__).resolve()),
                    f"{REMOTE_HOST}:{stage}/deploy-ashburn-ui.py",
                ],
                check=True,
            )
            if rollback:
                target = remote_command(stage, ["--remote-rollback"])
            else:
                subprocess.run(
                    [
                        "scp",
                        "-o",
                        "BatchMode=yes",
                        str(artifact),
                        f"{REMOTE_HOST}:{stage}/artifact.tar.gz",
                    ],
                    check=True,
                )
                target = remote_command(
                    stage, ["--remote-install", sha, digest, f"{stage}/artifact.tar.gz"]
                )
            if not TARGET_PATTERN.fullmatch(target):
                raise RuntimeError("Ashburn returned an unexpected release target")
            print(f"Ashburn MockForge UI current -> {target}")
        finally:
            subprocess.run(
                [
                    "ssh",
                    "-o",
                    "BatchMode=yes",
                    REMOTE_HOST,
                    shlex.join(["rm", "-rf", "--", stage]),
                ],
                check=False,
            )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group()
    action.add_argument("--rollback", action="store_true")
    action.add_argument(
        "--remote-install", nargs=3, metavar=("SHA", "DIGEST", "ARTIFACT")
    )
    action.add_argument("--remote-rollback", action="store_true")
    args = parser.parse_args()
    try:
        if args.remote_install or args.remote_rollback:
            if (
                os.geteuid() != 0
                or socket.gethostname().split(".", 1)[0] != REMOTE_HOST
            ):
                raise RuntimeError(
                    "remote release operation requires root on saasy-ash-01"
                )
            if args.remote_install:
                sha, digest, artifact = args.remote_install
                print(install_release(REMOTE_BASE, sha, digest, Path(artifact)))
            else:
                print(rollback_release(REMOTE_BASE))
        else:
            sha = require_protected_main()
            deploy(sha, rollback=args.rollback)
    except (
        OSError,
        RuntimeError,
        ValueError,
        subprocess.CalledProcessError,
        tarfile.TarError,
    ) as error:
        print(f"MockForge UI deploy refused: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
