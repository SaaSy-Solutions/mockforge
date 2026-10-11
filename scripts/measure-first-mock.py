#!/usr/bin/env python3
"""Measure the first HTTP mock using an isolated, uncached Docker daemon."""
import argparse
import json
import subprocess
import time
import uuid
from pathlib import Path

parser = argparse.ArgumentParser(description="Measure a public MockForge image with an empty Docker image cache.")
parser.add_argument("--image", default="ghcr.io/saasy-solutions/mockforge:latest")
parser.add_argument("--receipt", type=Path, default=Path("first-mock-receipt.json"))
args = parser.parse_args()
image = args.image
daemon = "mockforge-first-mock-" + uuid.uuid4().hex[:10]


def command(args, timeout=30, check=True):
    result = subprocess.run(args, text=True, capture_output=True, timeout=timeout)
    if check and result.returncode:
        raise RuntimeError("Quickstart verification command failed: " + args[0])
    return result


try:
    command([
        "docker", "run", "-d", "--rm", "--privileged", "--name", daemon,
        "--entrypoint", "dockerd", "docker:27-dind",
        "--host=unix:///var/run/docker.sock", "--storage-driver=overlay2",
    ], timeout=180)
    ready_deadline = time.monotonic() + 45
    while command(["docker", "exec", daemon, "docker", "info", "--format", "{{.ServerVersion}}"], check=False).returncode:
        if time.monotonic() >= ready_deadline:
            raise RuntimeError("Fresh Docker daemon did not become ready")
        time.sleep(0.5)

    empty_cache = command(["docker", "exec", daemon, "docker", "image", "ls", "-q"]).stdout.strip() == ""
    if not empty_cache:
        raise RuntimeError("Benchmark daemon unexpectedly contains cached images")
    started = time.monotonic()
    command(["docker", "exec", daemon, "docker", "pull", image], timeout=300)
    pulled = time.monotonic()
    command([
        "docker", "exec", daemon, "docker", "run", "-d", "--name", "first-mock",
        image, "serve", "--spec", "examples/openapi-demo.json", "--http-port", "3000",
    ])
    deadline = started + 360
    while True:
        response = command([
            "docker", "exec", daemon, "docker", "exec", "first-mock", "curl",
            "-fsS", "--max-time", "1", "http://127.0.0.1:3000/users",
        ], check=False)
        if response.returncode == 0:
            body = json.loads(response.stdout)
            if isinstance(body, list) and body and isinstance(body[0], dict) and "name" in body[0]:
                break
            raise RuntimeError("Demo endpoint did not return the documented users array")
        if time.monotonic() >= deadline:
            raise RuntimeError("First mock did not respond before the verification deadline")
        time.sleep(0.1)
    answered = time.monotonic()
    version = command(["docker", "exec", daemon, "docker", "exec", "first-mock", "mockforge", "--version"]).stdout.strip()
    receipt = {
        "environment": "Fresh nested Docker daemon and container; no prior images, Rust, or MockForge configuration. Host Docker and daemon preparation excluded.",
        "image": image,
        "version": version,
        "image_cache_initially_empty": empty_cache,
        "cold_image_pull_seconds": round(pulled - started, 3),
        "container_start_to_verified_response_seconds": round(answered - pulled, 3),
        "cold_pull_to_verified_first_mock_seconds": round(answered - started, 3),
        "target_seconds": 60,
        "under_target": answered - started < 60,
        "endpoint": "/users",
        "verified_users_array": True,
        "user_count": len(body),
    }
    args.receipt.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt, indent=2))
finally:
    command(["docker", "rm", "-fv", daemon], check=False)
