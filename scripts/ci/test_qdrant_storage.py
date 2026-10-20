#!/usr/bin/env python3
"""Real Qdrant acceptance with uniquely owned Docker resources and a restart."""
import argparse
import json
import os
import re
import signal
from pathlib import Path
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parents[2]
FIXTURE_KEY = "mockforge-qdrant-fixture-not-for-production"
LABEL = "dev.mockforge.qdrant-fixture"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--image", default="qdrant/qdrant:v1.19.2")
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args()
    def terminated(_signal, _frame):
        raise KeyboardInterrupt
    signal.signal(signal.SIGTERM, terminated)
    owner = "mockforge-qdrant-" + uuid.uuid4().hex
    volume = owner + "-data"
    phases = []
    cleanup = False
    # Do not inherit provider credentials into data/LLM tests.
    names = ("PATH", "HOME", "CARGO_HOME", "RUSTUP_HOME", "RUSTUP_TOOLCHAIN",
             "LD_LIBRARY_PATH", "LIBRARY_PATH", "PKG_CONFIG_PATH", "CC", "CXX", "AR",
             "CFLAGS", "CXXFLAGS", "CPPFLAGS", "LDFLAGS", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS",
             "RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER",
             "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER", "CARGO_TARGET_DIR",
             "CARGO_BUILD_JOBS", "CARGO_INCREMENTAL", "CARGO_PROFILE_DEV_DEBUG",
             "CARGO_PROFILE_TEST_DEBUG", "TMPDIR")
    env = {key: os.environ[key] for key in names if key in os.environ}

    def docker(*command):
        return subprocess.check_output(["docker", *command], env=env, text=True).strip()

    context = json.loads(docker("context", "inspect"))
    if not context[0]["Endpoints"]["docker"]["Host"].startswith("unix://"):
        raise RuntimeError("Qdrant fixture requires a local Unix-socket Docker context")

    def assert_owner(resource, name):
        values = json.loads(docker(resource, "inspect", name))
        labels = values[0].get("Labels", {}) if resource == "volume" else values[0]["Config"].get("Labels", {})
        if labels.get(LABEL) != owner:
            raise RuntimeError("refusing to mutate a fixture with different ownership")

    def wait_ready(base):
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            request = urllib.request.Request(base + "/readyz", headers={"api-key": FIXTURE_KEY})
            try:
                with urllib.request.urlopen(request, timeout=2) as response:
                    if response.status == 200:
                        return
            except (urllib.error.URLError, TimeoutError, ConnectionError):
                time.sleep(.2)
        raise RuntimeError("owned Qdrant fixture did not become ready")

    def test(name, phase=None):
        current = dict(env)
        if phase:
            current["QDRANT_TEST_PHASE"] = phase
        process = subprocess.Popen(["cargo", "test", "--locked", "-p", "mockforge-data",
                                    "--features", "qdrant", "--test", "qdrant_storage", name,
                                    "--", "--exact", "--ignored", "--test-threads=1"], cwd=ROOT, env=current,
                                   start_new_session=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        try:
            output, _ = process.communicate()
        except BaseException:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGTERM)
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=5)
            raise
        print(output, end="", flush=True)
        executed = bool(re.search(r"test result: ok\. 1 passed; 0 failed; 0 ignored;", output))
        phases.append({"test": name, "phase": phase, "exit_code": process.returncode, "one_test_executed": executed})
        if process.returncode or not executed:
            raise RuntimeError("Qdrant contract test failed or did not execute exactly one test")

    try:
        docker("pull", args.image)
        docker("volume", "create", "--label", LABEL + "=" + owner, volume)
        server_env = dict(env, QDRANT__SERVICE__API_KEY=FIXTURE_KEY)
        subprocess.run(["docker", "run", "--detach", "--name", owner, "--label", LABEL + "=" + owner,
                        "--publish", "127.0.0.1::6333", "--mount", "type=volume,source=" + volume + ",target=/qdrant/storage",
                        "--env", "QDRANT__SERVICE__API_KEY", args.image], env=server_env, check=True,
                       stdout=subprocess.DEVNULL)
        address = docker("port", owner, "6333/tcp")
        if not address.startswith("127.0.0.1:") or "\n" in address:
            raise RuntimeError("fixture must publish exactly one loopback port")
        base = "http://" + address
        wait_ready(base)
        env.update(QDRANT_TEST_URL=base, QDRANT_TEST_API_KEY=FIXTURE_KEY,
                   QDRANT_TEST_RESTART_COLLECTION="restart_" + uuid.uuid4().hex)
        with tempfile.TemporaryDirectory(prefix="mockforge-qdrant-contract-") as directory:
            env["QDRANT_TEST_WORK_DIR"] = directory
            test("qdrant_lifecycle_and_snapshot")
            test("qdrant_metrics_and_vector_validation")
            test("qdrant_persistence_across_restart", "seed")
            assert_owner("container", owner)
            docker("restart", owner)
            # Docker may allocate a new ephemeral host port after a restart.
            address = docker("port", owner, "6333/tcp")
            if not address.startswith("127.0.0.1:") or "\n" in address:
                raise RuntimeError("restarted fixture has unexpected port bindings")
            base = "http://" + address
            env["QDRANT_TEST_URL"] = base
            wait_ready(base)
            test("qdrant_persistence_across_restart", "verify")
    finally:
        owned_containers = docker("ps", "-aq", "--filter", "label=" + LABEL + "=" + owner).split()
        for container in owned_containers:
            assert_owner("container", container)
            docker("rm", "--force", container)
        owned_volumes = docker("volume", "ls", "-q", "--filter", "label=" + LABEL + "=" + owner).split()
        for owned_volume in owned_volumes:
            assert_owner("volume", owned_volume)
            docker("volume", "rm", owned_volume)
        containers = docker("ps", "-aq", "--filter", "label=" + LABEL + "=" + owner)
        volumes = docker("volume", "ls", "-q", "--filter", "label=" + LABEL + "=" + owner)
        cleanup = not containers and not volumes
        proof = {"image": args.image, "phases": phases, "owned_resources_removed": cleanup,
                 "all_passed": cleanup and len(phases) == 4 and all(p["exit_code"] == 0 and p["one_test_executed"] for p in phases)}
        if args.receipt:
            args.receipt.write_text(json.dumps(proof, indent=2) + "\n")
        print(json.dumps(proof, indent=2), flush=True)
    if not cleanup:
        raise RuntimeError("owned Qdrant resources remain")


if __name__ == "__main__":
    main()
