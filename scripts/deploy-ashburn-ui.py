#!/usr/bin/env python3
"""Publish the exact protected-main cloud UI as an immutable Ashburn release."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import re
import shlex
import socket
import subprocess
import sys
import tarfile
import tempfile

from ashburn_ui_artifact import API_BASE_URL, package_dist
from ashburn_ui_release import (
    SHA_PATTERN,
    TARGET_PATTERN,
    install_release,
    rollback_release,
)


REPO = Path(__file__).resolve().parents[1]
UI = REPO / "crates/mockforge-ui/ui"
REMOTE_HOST = "saasy-ash-01"
REMOTE_BASE = Path("/var/lib/saasy/mockforge-ui")
STAGE_PATTERN = re.compile(r"/var/lib/saasy/mockforge-ui/\.incoming\.[A-Za-z0-9]{8,}\Z")


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
                    str(Path(__file__).with_name("ashburn_ui_artifact.py")),
                    str(Path(__file__).with_name("ashburn_ui_release.py")),
                    f"{REMOTE_HOST}:{stage}/",
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
