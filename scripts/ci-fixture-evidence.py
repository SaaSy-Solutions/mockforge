#!/usr/bin/env python3
"""Read-only, own-project fixture evidence. Never enumerates a whole daemon."""

import argparse
import ctypes
from datetime import datetime, timezone
import importlib.util
import json
import math
import os
from pathlib import Path
import re
import signal
import subprocess
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("owned", ROOT / "scripts/ci-owned-process.py")
owned = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(owned)
STOPPING = False


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise RuntimeError("fixture health redirects are forbidden")


LOCAL_HTTP = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())


class Stopping(Exception):
    pass


def docker(*args):
    if STOPPING:
        raise Stopping()
    parent = os.getpid()

    def bind_parent_lifetime():
        # Bind the newly owned timeout supervisor to this observer. ALRM maps
        # to timeout's configured KILL signal for its own command/plugin group.
        # No grace window can lose a plugin after its direct client exits.
        # No preexisting process group is adopted.
        # Close the parent-death-before-prctl race before any client is spawned.
        libc = ctypes.CDLL(None, use_errno=True)
        if libc.prctl(1, signal.SIGALRM, 0, 0, 0) != 0:  # PR_SET_PDEATHSIG
            os._exit(125)
        if os.getppid() != parent:
            os._exit(125)

    # Even builtin Docker commands can launch CLI-plugin metadata children.
    # GNU timeout creates a fresh owned process group for the whole invocation.
    result = subprocess.run(["timeout", "--signal=KILL", "5s", "docker", *args],
                            text=True, capture_output=True, timeout=8,
                            preexec_fn=bind_parent_lifetime)
    if result.returncode:
        # Do not echo Docker stderr, configuration, environment, or endpoints.
        raise RuntimeError("owned fixture Docker metadata check failed")
    return result.stdout.strip()


def validate_identity(args):
    if not re.fullmatch(r"[a-z0-9][a-z0-9_-]{1,180}", args.project):
        raise ValueError("invalid project identity")
    if args.compose not in {"docker-compose.e2e.yml", "docker-compose.rls-gate.yml"}:
        raise ValueError("unexpected fixture Compose file")
    if not re.fullmatch(r"[0-9]+", args.run_id):
        raise ValueError("invalid run identity")


def emit(args, phase, **data):
    event = {"at": datetime.now(timezone.utc).isoformat(), "phase": phase,
             "run_id": args.run_id, "project": args.project, "compose": args.compose, **data}
    with args.output.open("x" if phase == "started" else "a") as stream:
        stream.write(json.dumps(event, sort_keys=True) + "\n")
        stream.flush()


def sample(args, phase="healthy"):
    validate_identity(args)
    daemon = docker("info", "--format", "{{.ID}}")
    if not re.fullmatch(r"[a-zA-Z0-9:-]{8,128}", daemon):
        raise ValueError("missing Docker daemon identity")
    services = {}
    ports = {}
    for service, cid in (("db", args.db_cid), ("minio", args.minio_cid)):
        if not re.fullmatch(r"[a-f0-9]{12,64}", cid):
            raise ValueError("fixture service identity is absent or ambiguous")
        template = ('{"project":{{json (index .Config.Labels "com.docker.compose.project")}},'
                    '"healthy":{{json .State.Health.Status}},"running":{{json .State.Running}},'
                    '"image":{{json .Image}},"ports":{{json .NetworkSettings.Ports}}}')
        state = json.loads(docker("inspect", "--format", template, cid))
        if state["project"] != args.project:
            raise ValueError("refusing to inspect a foreign fixture project")
        if state["healthy"] != "healthy" or state["running"] is not True:
            raise RuntimeError("owned fixture service is not healthy")
        image_id = state["image"]
        if not re.fullmatch(r"sha256:[a-f0-9]{64}", image_id):
            raise ValueError("missing immutable fixture image identity")
        digests = json.loads(docker("image", "inspect", "--format", "{{json .RepoDigests}}", image_id))
        if not isinstance(digests, list) or not digests or not all(
                isinstance(value, str) and re.fullmatch(r"[a-zA-Z0-9./:_-]+@sha256:[a-f0-9]{64}", value)
                for value in digests):
            raise ValueError("fixture image RepoDigest is missing")
        services[service] = {"container": cid, "image_id": image_id, "repo_digests": digests}
        expected = (("postgres", "5432/tcp"),) if service == "db" else (("minio", "9000/tcp"), ("minio_console", "9001/tcp"))
        for name, port in expected:
            bindings = state["ports"].get(port)
            if not isinstance(bindings, list) or len(bindings) != 1:
                raise ValueError("fixture endpoint is absent or ambiguous")
            binding = bindings[0]
            if binding.get("HostIp") != "127.0.0.1" or not re.fullmatch(r"[0-9]{1,5}", binding.get("HostPort", "")) or not 0 < int(binding["HostPort"]) < 65536:
                raise ValueError("fixture endpoint is not a valid loopback binding")
            ports[name] = "127.0.0.1:" + binding["HostPort"]
    ports["registry"] = f"127.0.0.1:{args.port}"
    identity = json.loads(args.receipt.read_text())
    if identity.get("context") != owned.process_context() or owned.start_time(identity["pid"]) != identity["start_time"]:
        raise ValueError("fixture server ownership identity changed")
    if not 0 < args.port < 65536:
        raise ValueError("invalid fixture port")
    if STOPPING:
        raise Stopping()
    with LOCAL_HTTP.open(f"http://127.0.0.1:{args.port}/health", timeout=2) as response:
        if response.status != 200:
            raise RuntimeError("owned registry is not healthy")
    emit(args, phase, daemon=daemon, services=services, ports=ports, registry_healthy=True)


def monitor(args):
    global STOPPING
    if not math.isfinite(args.interval) or not 1 <= args.interval <= 30:
        raise ValueError("evidence interval must be between 1 and 30 seconds")
    if not math.isfinite(args.max_seconds) or not 1 <= args.max_seconds <= 2700:
        raise ValueError("evidence duration must be between 1 and 2700 seconds")

    def stop(signum, frame):
        global STOPPING
        STOPPING = True

    signal.signal(signal.SIGINT, stop)
    signal.signal(signal.SIGTERM, stop)
    deadline = time.monotonic() + args.max_seconds
    while not STOPPING and time.monotonic() < deadline:
        try:
            sample(args)
        except Stopping:
            return
        except Exception as error:
            emit(args, "health_error", error=type(error).__name__)
            raise
        # Respond to TERM promptly; an in-flight command/group has a 5s hard deadline.
        end = min(time.monotonic() + args.interval, deadline)
        while not STOPPING and time.monotonic() < end:
            time.sleep(min(.2, max(0, end - time.monotonic())))
    if not STOPPING:
        emit(args, "health_error", error="monitor_deadline_exceeded")
        raise RuntimeError("fixture evidence monitor expired before owned cleanup")


def cleaned(args):
    validate_identity(args)
    # Built-in Docker command with a daemon-side exact-project label filter.
    # No Compose plugin descendant, and no unrelated container enumeration.
    if docker("container", "ls", "--all", "--filter",
              "label=com.docker.compose.project=" + args.project, "--format", "{{.ID}}"):
        emit(args, "cleanup_error", error="owned_project_not_empty")
        raise RuntimeError("owned project still has containers")
    daemon = docker("info", "--format", "{{.ID}}")
    if not re.fullmatch(r"[a-zA-Z0-9:-]{8,128}", daemon):
        raise ValueError("missing Docker daemon identity")
    emit(args, "cleaned", daemon=daemon)


def events(path):
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def verify(path, project):
    data = events(path)
    if not data or any(item["project"] != project for item in data):
        raise ValueError("mixed or absent owned evidence")
    if len({item["run_id"] for item in data}) != 1 or len({item["compose"] for item in data}) != 1:
        raise ValueError("mixed run or lane identity")
    if any(item["phase"].endswith("error") for item in data):
        raise RuntimeError("fixture evidence contains a failure")
    if data[0]["phase"] != "started" or data[-1]["phase"] != "cleaned" or not any(item["phase"] == "healthy" for item in data):
        raise RuntimeError("fixture evidence is incomplete")
    if len({item["daemon"] for item in data}) != 1:
        raise ValueError("Docker daemon identity changed")
    moments = [datetime.fromisoformat(item["at"]) for item in data]
    if any(moment.tzinfo is None for moment in moments) or moments != sorted(moments):
        raise ValueError("fixture timestamps are ambiguous or unordered")
    services = [item["services"] for item in data if item["phase"] in {"started", "healthy"}]
    if not services or any(value != services[0] for value in services):
        raise ValueError("fixture service identity changed")
    ports = [item["ports"] for item in data if item["phase"] in {"started", "healthy"}]
    if any(value != ports[0] for value in ports):
        raise ValueError("fixture port bindings changed")
    return data


def emit_verified(data):
    """Also preserve validated, allowlisted evidence in ordinary job logs."""
    for item in data:
        safe = {key: item[key] for key in ("at", "phase", "run_id", "project", "compose", "daemon")}
        if item["phase"] in {"started", "healthy"}:
            safe["services"] = {name: {key: item["services"][name][key]
                for key in ("container", "image_id", "repo_digests")} for name in ("db", "minio")}
            safe["ports"] = {name: item["ports"][name]
                for name in ("postgres", "minio", "minio_console", "registry")}
            safe["registry_healthy"] = item["registry_healthy"]
        print("FIXTURE_EVIDENCE " + json.dumps(safe, sort_keys=True))


def compare(paths):
    data = []
    projects = set()
    for path in paths:
        entries = events(path)
        if not entries or entries[0]["project"] in projects:
            raise ValueError("missing or duplicate project evidence")
        projects.add(entries[0]["project"])
        data.extend(verify(path, entries[0]["project"]))
    for first in (item for item in data if item["phase"] == "cleaned"):
        cleanup_time = datetime.fromisoformat(first["at"])
        peers = [item for item in data if item["phase"] == "healthy"
                 and item["run_id"] != first["run_id"] and item["project"] != first["project"]
                 and item.get("daemon") == first.get("daemon")]
        for project in {item["project"] for item in peers}:
            own = [datetime.fromisoformat(item["at"]) for item in peers if item["project"] == project]
            # Require actual observed liveness shortly before and after owned
            # cleanup. No peer communication, mutation, or timing extension.
            before = any(0 <= (cleanup_time - moment).total_seconds() <= 30 for moment in own)
            after = any(5 <= (moment - cleanup_time).total_seconds() <= 30 for moment in own)
            first_ports = next(item["ports"] for item in data if item["project"] == first["project"] and item["phase"] == "started")
            peer_ports = next(item["ports"] for item in peers if item["project"] == project)
            if before and after:
                if set(first_ports.values()) & set(peer_ports.values()):
                    raise ValueError("overlapping projects unexpectedly share a fixture endpoint")
                print(json.dumps({"status": "overlap_proven", "cleaned_project": first["project"],
                                  "surviving_project": project, "daemon": first["daemon"],
                                  "cleanup_at": first["at"]}, sort_keys=True))
                return
    raise RuntimeError("no same-daemon cross-run survivor interval was observed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    for name in ("sample", "monitor", "cleaned"):
        command = commands.add_parser(name)
        command.add_argument("--project", required=True)
        command.add_argument("--compose", required=True)
        command.add_argument("--run-id", required=True)
        command.add_argument("--output", type=Path, required=True)
        if name != "cleaned":
            command.add_argument("--port", type=int, required=True)
            command.add_argument("--receipt", type=Path, required=True)
            command.add_argument("--db-cid", required=True)
            command.add_argument("--minio-cid", required=True)
        if name == "monitor":
            command.add_argument("--interval", type=float, default=5)
            command.add_argument("--max-seconds", type=float, default=2700)
    command = commands.add_parser("verify")
    command.add_argument("--output", type=Path, required=True)
    command.add_argument("--project", required=True)
    command = commands.add_parser("compare")
    command.add_argument("paths", type=Path, nargs="+")
    args = parser.parse_args()
    if args.command == "sample": sample(args, "started")
    elif args.command == "monitor": monitor(args)
    elif args.command == "cleaned": cleaned(args)
    elif args.command == "verify": emit_verified(verify(args.output, args.project))
    else: compare(args.paths)


if __name__ == "__main__":
    main()
