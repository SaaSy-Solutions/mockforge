"""Managed CVE lab engine operations."""

import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import time
import http.client
from contextlib import contextmanager
from cve_lab_catalog import LABEL
from cve_lab_transport import request
import re


def command(args, timeout=120, check=True):
    result = subprocess.run(args, capture_output=True, text=True, timeout=timeout)
    if check and result.returncode:
        raise RuntimeError(f"{args[0]} command failed: {result.stderr[-3000:]}")
    return result


def doctor(engine):
    if sys.version_info < (3, 11):
        raise RuntimeError("Install Python 3.11 or newer")
    if platform.system() != "Linux" or platform.machine() not in ("x86_64", "amd64"):
        raise RuntimeError("This pack currently requires Linux amd64")
    if not shutil.which(engine):
        raise RuntimeError(f"Install {engine} first, then rerun mockforge cve doctor")
    version = command(
        [engine, "version", "--format", "{{.Server.Version}}"], timeout=20
    )
    # Host networking and loopback publications must refer to this machine.
    if any(
        os.environ.get(key)
        for key in (
            "DOCKER_HOST",
            "DOCKER_CONTEXT",
            "CONTAINER_HOST",
            "CONTAINER_CONNECTION",
        )
    ):
        raise RuntimeError(
            "Use the default local engine without remote/context environment overrides"
        )
    info = json.loads(
        command([engine, "info", "--format", "{{json .}}"], timeout=20).stdout
    )
    is_podman = "host" in info and "version" in info
    if is_podman and info["host"].get("serviceIsRemote", False):
        raise RuntimeError("Remote Podman engines are unsupported")
    if not is_podman:
        context = command([engine, "context", "inspect"], check=False, timeout=20)
        if context.returncode:
            raise RuntimeError("Cannot verify local Docker context")
        if context.returncode == 0:
            contexts = json.loads(context.stdout)
            if len(contexts) != 1:
                raise RuntimeError("Cannot identify local Docker context")
            host = contexts[0]["Endpoints"]["docker"]["Host"]
            if not host.startswith("unix://"):
                raise RuntimeError("Remote container engines are unsupported")
    return {
        "python": platform.python_version(),
        "engine": engine,
        "server": version.stdout.strip(),
        "platform": platform.machine(),
    }


def prepare(engine, pack):
    # Refuse unreviewed images before any engine interaction: reviewed packs
    # pin every image by digest, so an unpinned reference (for example a pack
    # whose lab image is still pending its lab-host review) must stop here.
    for image in (pack["catalog"]["application_image"], pack["catalog"]["runner_image"]):
        if "@sha256:" not in image:
            raise RuntimeError(
                f"{image} is not pinned by digest; reviewed packs refuse "
                "unreviewed images. Build and review the lab image per its "
                "image directory, then pin the digest in the pack catalog."
            )
    doctor(engine)
    for image in (pack["catalog"]["application_image"], pack["catalog"]["runner_image"]):
        if command([engine, "image", "inspect", image], check=False).returncode:
            print(f"Downloading pinned image {image}", flush=True)
            command([engine, "pull", image], timeout=900)
        # An inspect by digest ensures the exact requested image exists locally.
        command([engine, "image", "inspect", image])


def owned_remove(engine, name, run_id):
    result = command([engine, "inspect", name], check=False)
    if result.returncode:
        listing = command([engine, "ps", "-a", "--format", "{{.Names}}"], check=False)
        if listing.returncode or name in listing.stdout.splitlines():
            raise RuntimeError(
                f"Cannot verify removal of {name}: {result.stderr[-1000:]}"
            )
        return
    item = json.loads(result.stdout)[0]
    if item.get("Config", {}).get("Labels", {}).get(LABEL) != run_id:
        raise RuntimeError(f"Refusing to remove unowned container {name}")
    command([engine, "rm", "-f", name])


def container_running(engine, run_id):
    """Whether this run's application container is still alive.

    Crash-evidence packs use this to distinguish a reviewed exploit outcome
    from a healthy application.
    """
    result = command(
        [engine, "inspect", "mockforge-cve-" + run_id, "--format", "{{.State.Running}}"],
        check=False,
    )
    if result.returncode:
        return False
    return result.stdout.strip().lower() == "true"


def container_log(engine, run_id, tail=400):
    """The application container's recent log output, mid-run.

    Evidence packs whose exploit outcome lives in the application log (an
    Apache prefork child segmentation fault, for example) compare marker
    counts before and after the exploit request.
    """
    result = command(
        [engine, "logs", "--tail", str(tail), "mockforge-cve-" + run_id],
        check=False,
    )
    if result.returncode:
        return ""
    return result.stdout + result.stderr


@contextmanager
def lab(engine, run_id, output, pack):
    app = pack["app"]
    name = "mockforge-cve-" + run_id
    try:
        run_args = [
            engine,
            "run",
            "-d",
            "--name",
            name,
            "--label",
            f"{LABEL}={run_id}",
            "--cap-drop=ALL",
            "--security-opt=no-new-privileges",
            "--memory=1g",
            "--pids-limit=256",
            "-p",
            f"127.0.0.1::{app['port']}",
        ]
        for key, value in app["env"].items():
            run_args.extend(["-e", f"{key}={value}"])
        for mount in app.get("mounts", ()):
            host = mount["host"].replace("{output}", str(output))
            run_args.extend(["-v", f"{host}:{mount['container']}:ro"])
        if app.get("workdir"):
            run_args.extend(["-w", app["workdir"]])
        run_args.append(pack["catalog"]["application_image"])
        run_args.extend(app.get("command", ()))
        command(run_args)
        ports = command([engine, "port", name, app["port"]]).stdout.strip().splitlines()
        if not ports:
            # No published mapping means the container never bound app["port"]
            # (it exited or failed to start, e.g. the server could not bind the
            # port). Surface the clean readiness diagnostic that run() catches
            # instead of letting an IndexError escape as a raw traceback.
            raise RuntimeError(
                "Application did not expose "
                + app["port"]
                + "; it failed to start (see application.log)"
            )
        backend = "http://" + ports[0]
        deadline = time.monotonic() + 90
        while time.monotonic() < deadline:
            try:
                ready = (
                    request(backend, app["readiness"]["path"])[0]
                    == app["readiness"]["status"]
                )
            except (OSError, ValueError, http.client.HTTPException):
                ready = False
            if ready:
                break
            time.sleep(0.5)
        else:
            raise RuntimeError("Application did not become ready within 90 seconds")
        yield backend
    finally:
        try:
            logs = command([engine, "logs", "--tail", "200", name], check=False)
            (output / "application.log").write_text(logs.stdout + logs.stderr)
        finally:
            owned_remove(engine, name, run_id)


def cleanup(args):
    data = json.loads((Path(args.output_dir) / "ownership.json").read_text())
    run_id = data["run_id"]
    if not re.fullmatch("[0-9a-f]{32}", run_id):
        raise ValueError("invalid ownership record")
    if data["engine"] != args.engine:
        raise ValueError("use the engine recorded in ownership.json")
    doctor(args.engine)
    for name in (
        "mockforge-cve-" + run_id,
        "mockforge-cve-client-" + run_id + "-baseline",
        "mockforge-cve-client-" + run_id + "-target",
    ):
        owned_remove(args.engine, name, run_id)
    print("Owned containers removed; reports and image cache retained")
