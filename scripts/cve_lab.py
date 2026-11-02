#!/usr/bin/env python3
"""Managed, pinned CVE labs. Python 3.11+ and a local Linux Docker/Podman engine."""

import argparse
from contextlib import contextmanager
import hashlib
import http.client
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import ipaddress
import json
import os
from pathlib import Path
import platform
import re
import shutil
import signal
import subprocess
import sys
import threading
import time
from urllib.parse import parse_qs, urlsplit
import uuid

CVE = "CVE-2025-29927"
CATALOG = {
    "schema_version": 1,
    "pack_revision": 1,
    "cve": CVE,
    "title": "Next.js middleware authorization bypass",
    "platform": "Linux, local Docker/Podman engine, amd64",
    "application_version": "Next.js 15.2.2 (Vulhub)",
    "application_image": "docker.io/vulhub/nextjs@sha256:f01608b225bcb7952f5e558c445b9bd76a3bfb385e11521d1a06ad4cbe08b4a8",
    "runner_version": "Nuclei 3.11.1",
    "runner_image": "docker.io/projectdiscovery/nuclei@sha256:582d5546902e67052097cb2d07296c642d50a1afc5e44623cb038845df9a32eb",
    "references": [
        "https://github.com/vulhub/vulhub/tree/master/next.js/CVE-2025-29927",
        "https://github.com/advisories/GHSA-f82v-jwr5-mffw",
    ],
    "coverage": "One reviewed authorization-bypass case and benign controls; not all variants",
}
SIGNATURE = "middleware:middleware:middleware:middleware:middleware"
LABEL = "io.mockforge.cve.run"


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


def prepare(engine):
    doctor(engine)
    for image in (CATALOG["application_image"], CATALOG["runner_image"]):
        if command([engine, "image", "inspect", image], check=False).returncode:
            print(f"Downloading pinned image {image}", flush=True)
            command([engine, "pull", image], timeout=900)
        # An inspect by digest ensures the exact requested image exists locally.
        command([engine, "image", "inspect", image])


def target_url(value):
    url = urlsplit(value)
    if (
        url.scheme not in ("http", "https")
        or not url.hostname
        or url.username
        or url.password
        or url.path not in ("", "/")
        or url.query
        or url.fragment
    ):
        raise ValueError(
            "target must be an HTTP(S) origin without credentials, path or query"
        )
    if url.port == 0:
        raise ValueError("target port must be 1..65535")
    return url


def request(base, path, headers=None):
    url = target_url(base)
    cls = (
        http.client.HTTPSConnection
        if url.scheme == "https"
        else http.client.HTTPConnection
    )
    conn = cls(url.hostname, url.port, timeout=5)
    try:
        conn.request("GET", path, headers=headers or {})
        response = conn.getresponse()
        body = response.read(2 * 1024 * 1024 + 1)
        if len(body) > 2 * 1024 * 1024:
            raise ValueError("response exceeds 2 MiB")
        return response.status, response.getheaders(), body
    finally:
        conn.close()


@contextmanager
def receiver(port, address, backend):
    records = []
    lock = threading.Lock()

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            token = parse_qs(urlsplit(self.path).query).get("__mockforge_cve", [None])[
                0
            ]
            header = self.headers.get("X-Mockforge-Cve")
            observation = {
                "path": self.path,
                "tokens": [token, header],
                "signature": self.headers.get("x-middleware-subrequest"),
            }
            with lock:
                records.append(observation)
            try:
                headers = {
                    k: v
                    for k, v in self.headers.items()
                    if k.lower() not in ("host", "connection", "accept-encoding")
                }
                status, response_headers, body = request(backend, self.path, headers)
                observation["status"] = status
                self.send_response(status)
                for name, value in response_headers:
                    if name.lower() not in (
                        "transfer-encoding",
                        "connection",
                        "content-length",
                    ):
                        self.send_header(name, value)
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
            except (OSError, ValueError, http.client.HTTPException) as exc:
                observation["error"] = str(exc)
                self.send_error(502)

        def log_message(self, *_args):
            pass

    class Server(ThreadingHTTPServer):
        daemon_threads = True

        def get_request(self):
            sock, addr = super().get_request()
            sock.settimeout(5)
            return sock, addr

    server = Server((address, port), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield records
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=6)


def owned_remove(engine, name, run_id):
    result = command([engine, "inspect", name], check=False)
    if result.returncode:
        return
    item = json.loads(result.stdout)[0]
    if item.get("Config", {}).get("Labels", {}).get(LABEL) != run_id:
        raise RuntimeError(f"Refusing to remove unowned container {name}")
    command([engine, "rm", "-f", name])


@contextmanager
def lab(engine, run_id, output):
    name = "mockforge-cve-" + run_id
    try:
        command(
            [
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
                "127.0.0.1::3000",
                "-e",
                "NODE_ENV=production",
                CATALOG["application_image"],
            ]
        )
        ports = command([engine, "port", name, "3000/tcp"]).stdout.strip().splitlines()
        backend = "http://" + ports[0]
        deadline = time.monotonic() + 90
        while time.monotonic() < deadline:
            try:
                ready = request(backend, "/login")[0] == 200
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


def template(token):
    # This template emits a transport observation for any HTTP response. Only
    # classify() decides protection; a Nuclei match here is NOT a vulnerability.
    return f"""id: mockforge-nextjs-29927-observation
info:
  name: MockForge Next.js controlled lab observation
  author: mockforge
  severity: info
http:
  - method: GET
    path:
      - "{{{{BaseURL}}}}/?__mockforge_cve={token}"
    headers:
      X-Mockforge-Cve: "{token}"
      x-middleware-subrequest: "{SIGNATURE}"
    redirects: false
    matchers:
      - type: dsl
        dsl:
          - "status_code >= 100 && status_code <= 599"
"""


def probe(engine, base, output, phase, run_id):
    token = run_id + "-" + phase
    path = output / f"{phase}.yaml"
    path.write_text(template(token))
    name = "mockforge-cve-client-" + token
    try:
        result = command(
            [
                engine,
                "run",
                "--rm",
                "--name",
                name,
                "--label",
                f"{LABEL}={run_id}",
                "--network=host",
                "--cap-drop=ALL",
                "--security-opt=no-new-privileges",
                "--memory=512m",
                "--pids-limit=128",
                "-v",
                f"{output.resolve()}:/work:ro",
                CATALOG["runner_image"],
                "-u",
                base,
                "-t",
                f"/work/{phase}.yaml",
                "-duc",
                "-ni",
                "-no-stdin",
                "-silent",
                "-jsonl",
                "-timeout",
                "5",
                "-retries",
                "0",
                "-rl",
                "1",
                "-c",
                "1",
            ],
            timeout=90,
        )
        (output / f"{phase}.jsonl").write_text(result.stdout)
        (output / f"{phase}.log").write_text(result.stderr)
        findings = [
            json.loads(line)
            for line in result.stdout.splitlines()
            if line.startswith("{")
        ]
        if (
            len(findings) != 1
            or findings[0].get("template-id") != "mockforge-nextjs-29927-observation"
        ):
            raise RuntimeError(
                f"{phase}: Nuclei did not produce exactly one HTTP observation"
            )
        raw = findings[0].get("response", "")
        match = re.match(r"HTTP/\S+ (\d{3})", raw)
        if not match:
            raise RuntimeError(f"{phase}: Nuclei response evidence is missing")
        return {
            "token": token,
            "status": int(match[1]),
            "dashboard": "Dashboard" in raw,
            "response_sha256": hashlib.sha256(raw.encode()).hexdigest(),
        }
    finally:
        owned_remove(engine, name, run_id)


def control(base, run_id, phase):
    token = f"{run_id}-{phase}-{uuid.uuid4().hex}"
    status, _headers, body = request(
        base, f"/login?__mockforge_cve={token}", {"X-Mockforge-Cve": token}
    )
    root_token = token + "-root"
    root_status = request(
        base, f"/?__mockforge_cve={root_token}", {"X-Mockforge-Cve": root_token}
    )[0]
    return {
        "token": token,
        "status": status,
        "body_sha256": hashlib.sha256(body).hexdigest(),
        "root_token": root_token,
        "root_status": root_status,
    }


def observations(records, token):
    return [r for r in records if token in r["tokens"]]


def classify(baseline, target, before, after, records, block_status):
    if baseline["status"] != 200 or not baseline["dashboard"]:
        return "inconclusive", "direct application vulnerability was not reproduced"
    if not all(
        c["status"] == 200
        and len(observations(records, c["token"])) == 1
        and observations(records, c["token"])[0].get("status") == 200
        for c in (before, after)
    ):
        return "inconclusive", "benign controls did not corroborate origin reachability"
    if not all(
        c["root_status"] == 307
        and len(observations(records, c["root_token"])) == 1
        and observations(records, c["root_token"])[0].get("status") == 307
        for c in (before, after)
    ):
        return (
            "inconclusive",
            "ordinary requests to the protected route did not match the application redirect",
        )
    if any(not any(r["tokens"]) for r in records):
        return (
            "inconclusive",
            "uncorrelated origin traffic prevents a blocking assertion",
        )
    seen = observations(records, target["token"])
    if seen:
        return "fail", "signature request reached the origin; blocking assertion failed"
    if target["status"] != block_status:
        return "inconclusive", "response did not match the configured block status"
    return (
        "pass",
        "expected rejection with no correlated origin request and healthy controls",
    )


def run(args):
    target_url(args.target) if args.target else None
    ipaddress.IPv4Address(args.listen_address)
    if not 1 <= args.origin_port <= 65535 or not 400 <= args.block_status <= 599:
        raise ValueError("origin-port must be 1..65535; block-status must be 400..599")
    output = Path(args.output_dir).resolve()
    output.mkdir(parents=True, exist_ok=False)
    run_id = uuid.uuid4().hex
    report = {
        "schema_version": 1,
        "run_id": run_id,
        "pack": CATALOG,
        "target": args.target,
        "verdict": "inconclusive",
        "runtime_patch_evidence": "unverified",
        "scope": "Selected CVE lab reproduction and request-blocking regression, not comprehensive CVE coverage",
    }
    (output / "ownership.json").write_text(
        json.dumps({"run_id": run_id, "engine": args.engine})
    )
    try:
        prepare(args.engine)
        with lab(args.engine, run_id, output) as backend:
            with receiver(args.origin_port, args.listen_address, backend) as records:
                direct_host = (
                    "127.0.0.1"
                    if args.listen_address == "0.0.0.0"
                    else args.listen_address
                )
                origin = f"http://{direct_host}:{args.origin_port}"
                print(
                    f"Lab ready. LB upstream: {args.listen_address}:{args.origin_port}",
                    flush=True,
                )
                baseline_control = control(origin, run_id, "baseline-control")
                ordinary_status = baseline_control["root_status"]
                baseline = probe(args.engine, origin, output, "baseline", run_id)
                report.update(
                    baseline=baseline,
                    baseline_control=baseline_control,
                    unauthenticated_status=ordinary_status,
                )
                if (
                    ordinary_status != 307
                    or baseline["status"] != 200
                    or not baseline["dashboard"]
                ):
                    raise RuntimeError(
                        "Direct baseline did not demonstrate authentication bypass"
                    )
                if args.target:
                    before = control(args.target, run_id, "before")
                    tested = probe(
                        args.engine, args.target.rstrip("/"), output, "target", run_id
                    )
                    after = control(args.target, run_id, "after")
                    time.sleep(0.5)
                    verdict, reason = classify(
                        baseline, tested, before, after, records, args.block_status
                    )
                    if (
                        before["body_sha256"] != baseline_control["body_sha256"]
                        or after["body_sha256"] != baseline_control["body_sha256"]
                    ):
                        verdict, reason = (
                            "inconclusive",
                            "LB controls did not match this application's login page",
                        )
                    report.update(
                        verdict=verdict,
                        reason=reason,
                        target_probe=tested,
                        control_before=before,
                        control_after=after,
                    )
                else:
                    report.update(
                        verdict="baseline-confirmed",
                        reason="Direct lab reproduction only; no LB tested",
                    )
                report["origin_observations"] = records
    except (
        RuntimeError,
        OSError,
        ValueError,
        http.client.HTTPException,
        subprocess.TimeoutExpired,
    ) as exc:
        report.update(verdict="inconclusive", reason=str(exc))
    finally:
        (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"{report['verdict']}: {report['reason']}\nReport: {output / 'report.json'}")
    return 0 if report["verdict"] in ("pass", "baseline-confirmed") else 1


def cleanup(args):
    data = json.loads((Path(args.output_dir) / "ownership.json").read_text())
    run_id = data["run_id"]
    if not re.fullmatch("[0-9a-f]{32}", run_id):
        raise ValueError("invalid ownership record")
    if data["engine"] != args.engine:
        raise ValueError("use the engine recorded in ownership.json")
    for name in (
        "mockforge-cve-" + run_id,
        "mockforge-cve-client-" + run_id + "-baseline",
        "mockforge-cve-client-" + run_id + "-target",
    ):
        owned_remove(args.engine, name, run_id)
    print("Owned containers removed; reports and image cache retained")


def main(argv=None):
    parser = argparse.ArgumentParser(prog="mockforge cve", description=__doc__)
    parser.add_argument("--engine", choices=("docker", "podman"), default="docker")
    commands = parser.add_subparsers(dest="action", required=True)
    commands.add_parser("doctor", help="Check local prerequisites without downloading")
    commands.add_parser("list", help="Show reviewed pack and pinned versions")
    prepare_parser = commands.add_parser(
        "prepare", help="Download and cache the reviewed images"
    )
    prepare_parser.add_argument("cve", choices=(CVE,))
    runner = commands.add_parser(
        "run",
        help="Auto-prepare, reproduce directly, optionally compare through your LB, then stop",
    )
    runner.add_argument("cve", choices=(CVE,))
    runner.add_argument(
        "--target",
        help="Your authorized LB HTTP(S) origin, configured to forward to origin-port",
    )
    runner.add_argument("--origin-port", type=int, default=18082)
    runner.add_argument(
        "--listen-address",
        default="127.0.0.1",
        help="Recorder bind IPv4 address; choose an isolated lab interface for a remote LB",
    )
    runner.add_argument("--block-status", type=int, default=403)
    runner.add_argument(
        "--output-dir", required=True, help="New report directory, never overwritten"
    )
    down = commands.add_parser(
        "down", help="Recover owned containers after an interrupted run"
    )
    down.add_argument("--output-dir", required=True)
    args = parser.parse_args(argv)
    try:
        if args.action == "doctor":
            print(json.dumps(doctor(args.engine), indent=2))
        elif args.action == "list":
            print(json.dumps(CATALOG, indent=2))
        elif args.action == "prepare":
            prepare(args.engine)
            print(
                "Pinned lab and Nuclei images ready; updates ship with reviewed MockForge packs"
            )
        elif args.action == "run":
            return run(args)
        elif args.action == "down":
            cleanup(args)
    except (RuntimeError, ValueError, OSError, subprocess.TimeoutExpired) as exc:
        print(str(exc), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(143))
    raise SystemExit(main())
