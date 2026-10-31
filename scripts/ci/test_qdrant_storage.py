#!/usr/bin/env python3
"""Real Qdrant acceptance with uniquely owned Docker resources and a restart."""
import argparse
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import tempfile
import uuid

from qdrant_fixture import FIXTURE_KEY, QdrantFixture, fixture_environment

ROOT = Path(__file__).resolve().parents[2]


def run_test(environment, name, phase=None):
    """Execute exactly one real test; terminate only its own group on interruption."""
    current = dict(environment)
    if phase:
        current["QDRANT_TEST_PHASE"] = phase
    process = subprocess.Popen([str(ROOT / "scripts/dev/cargo"), "test", "--locked", "-p", "mockforge-data",
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
    return {"test": name, "phase": phase, "exit_code": process.returncode, "one_test_executed": executed}


def terminated(_signal, _frame):
    raise KeyboardInterrupt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--image", default="qdrant/qdrant:v1.19.2")
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args()
    signal.signal(signal.SIGTERM, terminated)
    environment = fixture_environment()
    fixture = QdrantFixture(args.image, environment)
    phases = []

    def test(name, phase=None):
        result = run_test(environment, name, phase)
        phases.append(result)
        if result["exit_code"] or not result["one_test_executed"]:
            raise RuntimeError("Qdrant contract test failed or did not execute exactly one test")

    try:
        fixture.start()
        environment.update(QDRANT_TEST_URL=fixture.base, QDRANT_TEST_API_KEY=FIXTURE_KEY,
                           QDRANT_TEST_RESTART_COLLECTION="restart_" + uuid.uuid4().hex)
        with tempfile.TemporaryDirectory(prefix="mockforge-qdrant-contract-") as directory:
            environment["QDRANT_TEST_WORK_DIR"] = directory
            test("qdrant_lifecycle_and_snapshot")
            test("qdrant_metrics_and_vector_validation")
            test("qdrant_persistence_across_restart", "seed")
            fixture.restart()
            environment["QDRANT_TEST_URL"] = fixture.base
            test("qdrant_persistence_across_restart", "verify")
    finally:
        cleanup = fixture.close()
        proof = {"image": args.image, "phases": phases, "owned_resources_removed": cleanup,
                 "all_passed": cleanup and len(phases) == 4 and all(p["exit_code"] == 0 and p["one_test_executed"] for p in phases)}
        if args.receipt:
            args.receipt.write_text(json.dumps(proof, indent=2) + "\n")
        print(json.dumps(proof, indent=2), flush=True)
    if not cleanup:
        raise RuntimeError("owned Qdrant resources remain")


if __name__ == "__main__":
    main()
