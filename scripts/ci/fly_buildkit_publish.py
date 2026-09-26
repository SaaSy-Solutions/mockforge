#!/usr/bin/env python3
"""Publish one exact-main image through a disposable Fly rootless BuildKit VM.

Only the trusted main-branch workflow calls this. The Fly app must already
exist, with an app-scoped token stored in FLY_IMAGE_PUBLISHER_TOKEN.
"""

from __future__ import annotations

import base64
import gzip
import json
import os
import re
import signal
import subprocess
import sys
import tempfile
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path

APP = "mockforge-image-publisher"
REGION = "iad"
BUILDKIT_IMAGE = (
    "moby/buildkit:rootless@sha256:"
    "f5a131a50b4dd414d1847904603d0f19a28ddd413f43a703c5c657196ca429a0"
)
SOCKET = "unix:///run/user/1000/buildkit/buildkitd.sock"
MACHINE_ID_RE = re.compile(r"Machine ID:\s*([0-9a-f]{14})")
DIGEST_RE = re.compile(r"sha256:[0-9a-f]{64}")
PUBLISHER_NAME_RE = re.compile(r"pub-[0-9]+-[0-9]+-[a-z0-9-]+")
STALE_AFTER = timedelta(hours=6)


def fly(*args: str, capture: bool = False, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["flyctl", *args],
        env={**os.environ, "FLY_API_TOKEN": os.environ["FLY_IMAGE_PUBLISHER_TOKEN"]},
        capture_output=capture,
        text=True,
        check=check,
        timeout=180,
    )


def guest(machine_id: str, command: str, *, user: str = "user", timeout: int = 3600) -> str:
    result = subprocess.run(
        [
            "flyctl", "ssh", "console", "-a", APP, "--machine", machine_id,
            "-u", user, "-C", command,
        ],
        env={**os.environ, "FLY_API_TOKEN": os.environ["FLY_IMAGE_PUBLISHER_TOKEN"]},
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )
    if result.returncode:
        # Fly/BuildKit errors are useful, but never print the Docker auth file.
        raise RuntimeError(f"guest command failed ({result.returncode}): {result.stderr[-3000:]}")
    return result.stdout


def machine_name() -> str:
    run_id = os.environ["GITHUB_RUN_ID"]
    attempt = os.environ["GITHUB_RUN_ATTEMPT"]
    app = os.environ["IMAGE_APP"]
    if not re.fullmatch(r"[0-9]+", run_id) or not re.fullmatch(r"[0-9]+", attempt):
        raise ValueError("invalid GitHub run identity")
    if not re.fullmatch(r"[a-z0-9-]+", app):
        raise ValueError("invalid image app")
    name = f"pub-{run_id}-{attempt}-{app}"
    if len(name) > 63:
        raise ValueError("Fly machine name is too long")
    return name


def machine_rows() -> list[dict[str, object]]:
    result = fly("machine", "list", "-a", APP, "--json", capture=True)
    rows = json.loads(result.stdout)
    if not isinstance(rows, list):
        raise RuntimeError("unexpected Fly machine list")
    return rows


def live_machine_ids(name: str) -> list[str]:
    return [str(row["id"]) for row in machine_rows() if row.get("name") == name]


def destroy_named(name: str, known_id: str | None = None, *, attempts: int = 12) -> None:
    """Delete a returned ID first, then observe a timed-out create by unique name.

    Empty early listings are insufficient after a failed create: Fly may have
    accepted the request but not yet exposed the Machine in a list response.
    The hourly stale cleanup is the backstop after this bounded observation.
    """
    if not PUBLISHER_NAME_RE.fullmatch(name):
        raise ValueError("invalid publisher Machine name")
    if known_id:
        try:
            fly("machine", "destroy", known_id, "-a", APP, "--force")
        except subprocess.CalledProcessError:
            pass  # Readback below decides whether the Machine is gone.
    empty_reads = 0
    for attempt in range(attempts):
        ids = live_machine_ids(name)
        if ids:
            empty_reads = 0
            for machine_id in ids:
                try:
                    fly("machine", "destroy", machine_id, "-a", APP, "--force")
                except subprocess.CalledProcessError:
                    pass  # A stale list may still show an already destroyed ID.
        else:
            empty_reads += 1
            if empty_reads >= 3:
                return
        if attempt + 1 < attempts:
            time.sleep(5)
    raise RuntimeError(f"Fly publisher cleanup did not settle for {name}")


def cleanup_stale(now: datetime | None = None) -> list[str]:
    """Delete only this app's old publisher Machines; never touch active builds."""
    now = now or datetime.now(timezone.utc)
    stale: list[tuple[str, str]] = []
    for row in machine_rows():
        name, machine_id, created = row.get("name"), row.get("id"), row.get("created_at")
        if not isinstance(name, str) or not PUBLISHER_NAME_RE.fullmatch(name):
            continue
        if not isinstance(machine_id, str) or not isinstance(created, str):
            raise RuntimeError(f"publisher Machine {name} lacks ID or creation time")
        created_at = datetime.fromisoformat(created.replace("Z", "+00:00"))
        if created_at.tzinfo is None:
            raise RuntimeError(f"publisher Machine {name} has naive creation time")
        if now - created_at >= STALE_AFTER:
            stale.append((name, machine_id))
    for name, machine_id in stale:
        destroy_named(name, machine_id)
    return [name for name, _ in stale]


def archive_head(path: Path, sha: str) -> None:
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    if head != sha:
        raise RuntimeError(f"checkout {head} differs from requested main SHA {sha}")
    with path.open("wb") as raw, gzip.GzipFile(fileobj=raw, mode="wb") as compressed:
        subprocess.run(["git", "archive", "--format=tar", "HEAD"], stdout=compressed, check=True)


def write_auth(path: Path) -> None:
    actor = os.environ["GITHUB_ACTOR"]
    token = os.environ["GITHUB_TOKEN"]
    if not token or not actor:
        raise RuntimeError("GitHub package token/actor missing")
    encoded = base64.b64encode(f"{actor}:{token}".encode()).decode()
    path.write_text(json.dumps({"auths": {"ghcr.io": {"auth": encoded}}}), encoding="utf-8")
    path.chmod(0o600)


def upload(machine_id: str, source: Path, target: str) -> None:
    fly(
        "ssh", "sftp", "put", str(source), target, "-a", APP,
        "--machine", machine_id, "-m", "0600",
    )


def publish() -> str:
    sha = os.environ["GITHUB_SHA"]
    repo = os.environ["GITHUB_REPOSITORY"]
    app = os.environ["IMAGE_APP"]
    dockerfile = os.environ["IMAGE_DOCKERFILE"]
    if not re.fullmatch(r"[0-9a-f]{40}", sha) or repo != "SaaSy-Solutions/mockforge":
        raise RuntimeError("publisher requires exact MockForge commit")
    ref = os.environ.get("GITHUB_REF", "")
    protected = os.environ.get("GITHUB_REF_PROTECTED") == "true"
    if not protected or not (ref == "refs/heads/main" or
                             (app == "mockforge" and ref.startswith("refs/tags/v"))):
        raise RuntimeError("publisher requires protected main or root release tag")
    if not re.fullmatch(r"[a-z0-9-]+", app):
        raise ValueError("invalid image name")
    allowed = {
        "mockforge": "Dockerfile",
        "mockforge-registry": "Dockerfile.registry",
        "mockforge-tunnel-relay": "Dockerfile.tunnel",
    }
    if allowed.get(app) != dockerfile:
        raise ValueError("invalid image/Dockerfile pairing")
    version = os.environ.get("BUILD_VERSION", "") if app == "mockforge" else ""
    build_date = os.environ.get("BUILD_DATE", "") if app == "mockforge" else ""
    if version and not re.fullmatch(r"[A-Za-z0-9._-]+", version):
        raise ValueError("invalid image version")
    if build_date and not re.fullmatch(r"[0-9TZ:-]+", build_date):
        raise ValueError("invalid build timestamp")
    if not Path(dockerfile).is_file():
        raise ValueError("Dockerfile missing from exact checkout")
    if not os.environ.get("FLY_IMAGE_PUBLISHER_TOKEN"):
        raise RuntimeError("app-scoped Fly image publisher token missing")

    name = machine_name()
    image = f"ghcr.io/saasy-solutions/{app}:{sha}"
    cache = f"ghcr.io/saasy-solutions/{app}:buildcache"
    with tempfile.TemporaryDirectory(prefix="fly-publish-") as directory:
        scratch = Path(directory)
        source = scratch / "source.tar.gz"
        auth = scratch / "config.json"
        archive_head(source, sha)
        write_auth(auth)
        machine_id: str | None = None
        try:
            # A unique name lets cleanup recover a Machine even if flyctl fails
            # after creation but before returning its ID.
            result = fly(
                "machine", "run", BUILDKIT_IMAGE, "-a", APP,
                "--name", name, "--region", REGION,
                "--vm-cpu-kind", "performance", "--vm-cpus", "8",
                "--vm-memory", "65536", "--rootfs-size", "100",
                "--restart", "no", "--rm", "--detach", "--skip-dns-registration",
                capture=True,
            )
            found = MACHINE_ID_RE.search(result.stdout)
            if not found:
                raise RuntimeError("Fly returned no Machine ID")
            machine_id = found.group(1)
            upload(machine_id, source, "/tmp/publisher-source.tar.gz")
            upload(machine_id, auth, "/tmp/publisher-config.json")
            guest(
                machine_id,
                "sh -lc 'mkdir -p /tmp/publisher/auth /tmp/publisher/src; "
                "mv /tmp/publisher-config.json /tmp/publisher/auth/config.json; "
                "mv /tmp/publisher-source.tar.gz /tmp/publisher/source.tar.gz; "
                "chown -R user:user /tmp/publisher; "
                "chmod 700 /tmp/publisher/auth; chmod 600 /tmp/publisher/auth/config.json'",
                user="root",
            )
            extra_args = (
                f"--opt build-arg:VERSION={version} "
                f"--opt build-arg:COMMIT_SHA={sha} "
                f"--opt build-arg:BUILD_DATE={build_date} "
                if app == "mockforge" else ""
            )
            build = (
                "sh -lc 'set -eu; cd /tmp/publisher; "
                "tar -xzf source.tar.gz -C src; "
                "DOCKER_CONFIG=/tmp/publisher/auth "
                f"buildctl --addr {SOCKET} build --frontend dockerfile.v0 "
                "--local context=/tmp/publisher/src "
                "--local dockerfile=/tmp/publisher/src "
                f"--opt filename={dockerfile} "
                f"{extra_args}"
                f"--import-cache type=registry,ref={cache} "
                f"--export-cache type=registry,ref={cache},mode=max "
                f"--output type=image,name={image},push=true "
                "--metadata-file /tmp/publisher/metadata.json'"
            )
            guest(machine_id, build, timeout=3900)
            metadata = guest(
                machine_id,
                "sh -lc 'cat /tmp/publisher/metadata.json'",
            )
            digest = json.loads(metadata).get("containerimage.digest")
            if not isinstance(digest, str) or not DIGEST_RE.fullmatch(digest):
                raise RuntimeError("BuildKit did not return a manifest digest")
            print(f"{app} {image}@{digest}")
            summary = os.environ.get("GITHUB_STEP_SUMMARY")
            if summary:
                with Path(summary).open("a", encoding="utf-8") as output:
                    output.write(f"### {app}\n\n`{image}@{digest}`\n")
            github_output = os.environ.get("GITHUB_OUTPUT")
            if github_output:
                with Path(github_output).open("a", encoding="utf-8") as output:
                    output.write(f"digest={digest}\n")
            return digest
        except subprocess.CalledProcessError as exc:
            output = exc.stdout or ""
            if isinstance(output, bytes):
                output = output.decode(errors="replace")
            found = MACHINE_ID_RE.search(output)
            if found:
                machine_id = found.group(1)
            raise
        finally:
            try:
                for candidate in ([machine_id] if machine_id else []):
                    try:
                        guest(candidate, "sh -lc 'rm -rf /tmp/publisher'", user="root")
                    except Exception as exc:  # cleanup still destroys the guest
                        print(f"guest auth cleanup failed: {exc}", file=sys.stderr)
            finally:
                destroy_named(name, machine_id)


def main() -> None:
    if len(sys.argv) == 2 and sys.argv[1] == "--cleanup-only":
        destroy_named(machine_name())
        return
    if len(sys.argv) == 2 and sys.argv[1] == "--cleanup-stale":
        for name in cleanup_stale():
            print(f"destroyed stale publisher Machine {name}")
        return
    if len(sys.argv) != 1:
        raise SystemExit("usage: fly_buildkit_publish.py [--cleanup-only|--cleanup-stale]")
    signal.signal(signal.SIGTERM, lambda _sig, _frame: (_ for _ in ()).throw(InterruptedError()))
    publish()


if __name__ == "__main__":
    main()
