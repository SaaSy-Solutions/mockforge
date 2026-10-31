#!/usr/bin/env python3
"""Exercise a real runner through an owned, disposable local Redis queue.

Build mockforge-test-runner with scripts/dev/cargo first. Pass its binary and
an isolated loopback Redis port; this harness creates and deletes only its
UUID-named queue. All callbacks and smoke traffic stay on an owned HTTP server.
"""

import argparse
import http.server
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import threading
import time
import uuid


def redis_command(port, *arguments):
    parts = [str(value).encode() for value in arguments]
    wire = f"*{len(parts)}\r\n".encode()
    for part in parts:
        wire += f"${len(part)}\r\n".encode() + part + b"\r\n"
    with socket.create_connection(("127.0.0.1", port), timeout=5) as connection:
        connection.sendall(wire)
        reply = connection.makefile("rb").readline()
    if not reply or reply.startswith(b"-"):
        raise RuntimeError(f"Redis rejected fixture command: {reply!r}")
    return reply


def run(runner, redis_port):
    token = uuid.uuid4().hex
    queue = f"runner-result-fixture:{uuid.uuid4()}"
    outcomes = {}
    failures = []
    probe_hits = []
    condition = threading.Condition()

    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, *_args):
            pass

        def respond(self, status, body):
            data = json.dumps(body).encode()
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

        def authorized(self):
            if self.headers.get("Authorization") == f"Bearer {token}":
                return True
            with condition:
                failures.append("internal callback omitted its fixture credential")
                condition.notify_all()
            self.respond(401, {})
            return False

        def do_GET(self):
            if self.path == "/probe":
                with condition:
                    probe_hits.append(self.path)
                self.respond(200, {"ok": True})
            elif self.path.startswith("/api/v1/internal/capture-sessions/") and self.path.endswith("/exchanges"):
                if self.authorized():
                    self.respond(200, [])
            else:
                self.respond(404, {})

        def do_POST(self):
            if not self.authorized():
                return
            body = json.loads(self.rfile.read(int(self.headers.get("Content-Length", "0"))))
            parts = self.path.strip("/").split("/")
            if len(parts) != 6 or parts[:4] != ["api", "v1", "internal", "test-runs"]:
                self.respond(404, {})
                return
            if parts[-1] == "finish":
                with condition:
                    outcomes[parts[-2]] = body
                    condition.notify_all()
            self.respond(200, {})

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    base = f"http://127.0.0.1:{server.server_port}"
    spec = {"openapi": "3.0.0", "info": {"title": "fixture", "version": "1"}, "paths": {}}
    valid_spec = dict(spec, paths={"/probe": {"get": {"responses": {"200": {"description": "ok"}}}}})
    cases = [
        ("unavailable executor", "snapshot", {}, "errored"),
        ("missing execution config", "test", {}, "errored"),
        ("empty replay", "replay", {"target_url": base, "session_id": str(uuid.uuid4())}, "errored"),
        ("empty smoke", "smoke", {"base_url": base, "spec": json.dumps(spec)}, "errored"),
        ("real smoke control", "smoke", {"base_url": base, "spec": json.dumps(valid_spec)}, "passed"),
    ]
    expected = {}
    environment = dict(os.environ)
    environment.update({
        "MOCKFORGE_RUNNER_REDIS_URL": f"redis://127.0.0.1:{redis_port}/0",
        "MOCKFORGE_RUNNER_QUEUE_KEY": queue,
        "MOCKFORGE_RUNNER_REGISTRY_INTERNAL_BASE_URL": base,
        "MOCKFORGE_RUNNER_REGISTRY_INTERNAL_TOKEN": token,
        "MOCKFORGE_RUNNER_MAX_CONCURRENT_JOBS": "1",
        "MOCKFORGE_RUNNER_POLL_TIMEOUT_SECS": "1",
        "MOCKFORGE_SSRF_ALLOW_LOOPBACK": "true",
    })
    process = None
    try:
        redis_command(redis_port, "PING")
        with tempfile.TemporaryFile() as log:
            process = subprocess.Popen([str(runner)], env=environment, stdout=log, stderr=log)
            for name, kind, payload, status in cases:
                run_id = str(uuid.uuid4())
                expected[run_id] = (name, status)
                redis_command(redis_port, "RPUSH", queue, json.dumps({
                    "run_id": run_id, "org_id": str(uuid.uuid4()),
                    "source_id": str(uuid.uuid4()), "kind": kind, "payload": payload,
                }))
            deadline = time.monotonic() + 45
            with condition:
                while len(outcomes) < len(expected) and not failures:
                    if process.poll() is not None:
                        raise AssertionError(f"runner exited unexpectedly: {process.returncode}")
                    remaining = deadline - time.monotonic()
                    if remaining <= 0:
                        raise AssertionError(f"missing terminal callbacks: {set(expected) - set(outcomes)}")
                    condition.wait(min(remaining, 1))
            assert not failures, failures
            assert set(outcomes) == set(expected), "unexpected terminal callback IDs"
            results = []
            for run_id, (name, status) in expected.items():
                outcome = outcomes[run_id]
                assert outcome["status"] == status, (name, outcome)
                if status == "errored":
                    assert outcome["runner_seconds"] == 0, (name, outcome)
                results.append({"case": name, "status": status, "runner_seconds": outcome["runner_seconds"]})
            assert len(probe_hits) == 1, f"expected one real successful probe, got {probe_hits}"
            return {"cases": results, "actual_probe_requests": len(probe_hits)}
    finally:
        if process is not None:
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)
        redis_command(redis_port, "DEL", queue)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runner", type=Path, required=True)
    parser.add_argument("--redis-port", type=int, required=True)
    args = parser.parse_args()
    assert 0 < args.redis_port < 65536, "invalid Redis fixture port"
    print(json.dumps(run(args.runner.resolve(strict=True), args.redis_port), indent=2))


if __name__ == "__main__":
    main()
