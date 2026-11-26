"""Managed CVE lab engine operations."""

import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import time
import uuid
import http.client
from contextlib import contextmanager
from cve_lab_catalog import LABEL
from cve_lab_transport import request
import re


ALLOWED_APP_CAPABILITIES = frozenset(("SETUID", "SETGID", "CHOWN"))

# What every lab application container gets unless its pack, as reviewed
# data, says it needs more: a memory cap and a readiness budget. Both have
# an upper bound here so a pack cannot turn the lab into an unbounded load
# on the host, and both are checked before anything is downloaded.
DEFAULT_MEMORY = "1g"
MAX_MEMORY_MIB = 4096
DEFAULT_READINESS_SECONDS = 90
MAX_READINESS_SECONDS = 300
MIN_READINESS_SECONDS = 10


def memory_limit(app):
    """The application container's memory cap, as `--memory` takes it."""
    value = app.get("memory", DEFAULT_MEMORY)
    match = re.fullmatch(r"([0-9]+)([mg])", value) if isinstance(value, str) else None
    if not match:
        raise RuntimeError("app.memory must be a size such as 1g, 512m or 1536m")
    mib = int(match.group(1)) * (1024 if match.group(2) == "g" else 1)
    if not 64 <= mib <= MAX_MEMORY_MIB:
        raise RuntimeError(f"app.memory must be between 64m and {MAX_MEMORY_MIB // 1024}g")
    return value


def readiness_budget(app):
    """Seconds the application is given to answer its readiness path."""
    value = app.get("readiness_seconds", DEFAULT_READINESS_SECONDS)
    if isinstance(value, bool) or not isinstance(value, int):
        raise RuntimeError("app.readiness_seconds must be a whole number of seconds")
    if not MIN_READINESS_SECONDS <= value <= MAX_READINESS_SECONDS:
        raise RuntimeError(
            f"app.readiness_seconds must be between {MIN_READINESS_SECONDS} "
            f"and {MAX_READINESS_SECONDS}"
        )
    return value

GUIDE_URL = "https://docs.mockforge.dev/user-guide/managed-cve-labs.html"


def command(args, timeout=120, check=True):
    result = subprocess.run(args, capture_output=True, text=True, timeout=timeout)
    if check and result.returncode:
        raise RuntimeError(f"{args[0]} command failed: {result.stderr[-3000:]}")
    return result


def doctor(engine, packs=None):
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
    report = {
        "python": platform.python_version(),
        "engine": engine,
        "server": version.stdout.strip(),
        "platform": platform.machine(),
    }
    if packs:
        # Per-pack pinned image presence (by-digest inspects, never a parse
        # of `docker images`) and the host-network probe: both diagnostics
        # are gathered locally, without downloading anything.
        report["images"] = {
            cve: verify_images(engine, pack) for cve, pack in packs.items()
        }
        report["host_network"] = host_network_report(engine, report["images"])
    return report


def host_network_report(engine, images):
    """Probe whether --network=host containers share this machine's network.

    The probe needs the pinned runner image already present locally (doctor
    never downloads), so without it the check reports unknown: ``prepare``
    or the next run probes again once the image is in place.
    """
    runners = [
        entry["image"]
        for entries in images.values()
        for entry in entries
        if entry["role"] == "runner" and entry["status"] == "present"
    ]
    if not runners:
        return {
            "shared": None,
            "reason": (
                "the pinned runner image is not present locally; the check "
                "runs after prepare or at the start of a run"
            ),
        }
    return host_network_probe(engine, runners[0])


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


def image_status(engine, image):
    """Whether the exact image reference exists in the local engine.

    The check is the same by-digest inspect ``prepare`` performs - never a
    parse of ``docker images`` output, which does not list digest-pulled
    images the way tagged ones appear.
    """
    if "@sha256:" not in image:
        # A tag (a pack still pending review) is not a pinned image: a local
        # image with that tag proves nothing, and prepare refuses it anyway.
        return "unpinned"
    result = command([engine, "image", "inspect", image], check=False)
    return "present" if result.returncode == 0 else "missing"


def verify_images(engine, pack):
    """Report each pack-pinned image with its local presence status.

    Returns one entry per pinned image (the application and the runner) in
    catalog order, so a run report or a dedicated check can attest exactly
    which digest-pinned images this machine holds.
    """
    catalog = pack["catalog"]
    return [
        {
            "role": "application",
            "image": catalog["application_image"],
            "status": image_status(engine, catalog["application_image"]),
        },
        {
            "role": "runner",
            "image": catalog["runner_image"],
            "status": image_status(engine, catalog["runner_image"]),
        },
    ]


def host_netns():
    """The network namespace of this process, where the recorder listens."""
    return os.readlink("/proc/self/ns/net")


def host_network_probe(engine, image):
    """Whether a --network=host container shares this machine's network.

    Compares readlink /proc/self/ns/net on this host with the same command
    inside a --network=host container of the pinned runner image. Engines
    whose daemon runs in another network namespace (a snap, a VM, an LXD
    container, a systemd unit with private networking) hand such containers
    a "host" network where this machine's listeners do not exist, so the
    scanner could never reach the recorder and the run would only fail
    inside Nuclei with no reason.
    """
    host = host_netns()
    probe_id = uuid.uuid4().hex
    name = "mockforge-cve-netns-" + probe_id
    try:
        result = command(
            [
                engine,
                "run",
                "--rm",
                "--name",
                name,
                "--label",
                f"{LABEL}={probe_id}",
                "--network=host",
                # Never pull: the image is verified present by the caller
                # (prepare or the doctor image listing) first.
                "--pull=never",
                "--cap-drop=ALL",
                "--security-opt=no-new-privileges",
                "--memory=64m",
                "--pids-limit=16",
                # The runner image's entrypoint is Nuclei; readlink needs no
                # capabilities and is a busybox applet on the image's busybox
                # base. Invoke it through /bin/busybox rather than assuming a
                # /bin/readlink path: in the pinned runner image busybox installs
                # the applet at /usr/bin/readlink, so a hardcoded /bin/readlink
                # entrypoint fails with "no such file or directory" and the probe
                # can never run.
                "--entrypoint",
                "/bin/busybox",
                image,
                "readlink",
                "/proc/self/ns/net",
            ],
            timeout=60,
            check=False,
        )
        if result.returncode:
            raise RuntimeError(
                "Cannot probe the engine's host network namespace: "
                f"{result.stderr[-1000:]}"
            )
        container = result.stdout.strip()
        return {
            "host": host,
            "container": container,
            "shared": host == container,
        }
    finally:
        owned_remove(engine, name, probe_id)


def host_network_refusal(recorder, probe):
    """The plain refusal for an engine whose --network=host containers get
    another network namespace from this machine's: the scanner could never
    reach the recorder, so no run can produce evidence."""
    return (
        "containers started with --network=host on this engine don't share "
        "this machine's network, so the scanner can't reach the recorder at "
        f"{recorder} (host network namespace {probe['host']}, --network=host "
        f"container namespace {probe['container']}); see the managed CVE labs "
        f"guide: {GUIDE_URL}"
    )


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


@contextmanager
def lab(engine, run_id, output, pack):
    app = pack["app"]
    cap_add = app.get("cap_add", ())
    if not isinstance(cap_add, (list, tuple)) or any(
        capability not in ALLOWED_APP_CAPABILITIES for capability in cap_add
    ):
        raise RuntimeError(
            "app.cap_add entries must be from the allowlist: SETUID, SETGID, CHOWN"
        )
    name = "mockforge-cve-" + run_id
    memory = memory_limit(app)
    budget = readiness_budget(app)
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
            f"--memory={memory}",
            "--pids-limit=256",
            "-p",
            f"127.0.0.1::{app['port']}",
        ]
        run_args.extend(f"--cap-add={capability}" for capability in cap_add)
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
            raise RuntimeError(
                "Application did not expose "
                + app["port"]
                + "; it failed to start (see application.log)"
            )
        backend = "http://" + ports[0]
        deadline = time.monotonic() + budget
        while time.monotonic() < deadline:
            try:
                ready = (
                    request(backend, app["readiness"]["path"], log_sent=False)[0]
                    == app["readiness"]["status"]
                )
            except (OSError, ValueError, http.client.HTTPException):
                ready = False
            if ready:
                break
            time.sleep(0.5)
        else:
            raise RuntimeError(
                f"Application did not become ready within {budget} seconds"
            )
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
