"""Deterministic MockForge UI archive creation and validation."""

from __future__ import annotations

import gzip
import hashlib
import io
from pathlib import Path, PurePosixPath
import re
import shutil
import tarfile
import tempfile


API_BASE_URL = "https://api.mockforge.dev"
DIGEST_PATTERN = re.compile(r"[0-9a-f]{64}\Z")


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
