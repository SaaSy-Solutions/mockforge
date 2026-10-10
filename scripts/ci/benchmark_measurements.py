"""Build benchmark executables and require complete, fresh timing samples."""

import json
import math
import os
from pathlib import Path
import re
import subprocess


def build(root, target):
    env = {
        **os.environ,
        "CARGO_TARGET_DIR": str(target),
        "CARGO_BUILD_JOBS": "2",
        "CARGO_INCREMENTAL": "0",
    }
    result = subprocess.run(
        [
            "cargo",
            "bench",
            "--locked",
            "-p",
            "mockforge-core",
            "--bench",
            "core_benchmarks",
            "--no-run",
            "--message-format=json",
        ],
        cwd=root,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        check=True,
    )
    artifacts = [
        json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")
    ]
    executables = [
        a["executable"]
        for a in artifacts
        if a.get("reason") == "compiler-artifact"
        and a.get("target", {}).get("name") == "core_benchmarks"
        and a.get("executable")
    ]
    if len(executables) != 1:
        raise RuntimeError("Expected one compiled core benchmark executable")
    return Path(executables[0])


def names(binary, root):
    result = subprocess.run(
        [str(binary), "--list"], cwd=root, text=True, stdout=subprocess.PIPE, check=True
    )
    found = sorted(
        line.removesuffix(": benchmark")
        for line in result.stdout.splitlines()
        if line.endswith(": benchmark")
    )
    if not found:
        raise RuntimeError("No benchmark cases discovered")
    return found


def measure(binary, root, target, name, label):
    estimates = target / "criterion" / name / label / "estimates.json"
    # A cache restored by CI must never stand in for a measurement that did
    # not execute (for example, a filter that accidentally selected no case).
    estimates.unlink(missing_ok=True)
    subprocess.run(
        [
            str(binary),
            "--bench",
            "^" + re.escape(name) + "$",
            "--save-baseline",
            label,
            "--noplot",
        ],
        cwd=root,
        env={**os.environ, "CARGO_TARGET_DIR": str(target)},
        check=True,
    )
    value = json.loads(estimates.read_text())["mean"]["point_estimate"]
    if (
        isinstance(value, bool)
        or not isinstance(value, (int, float))
        or not math.isfinite(value)
        or value <= 0
    ):
        raise RuntimeError("Benchmark produced an invalid timing")
    return value


def aggregate(samples):
    if not samples or any(len(values) != 2 for values in samples.values()):
        raise ValueError("Every paired benchmark needs both measurements")
    return {
        name: {"new": {"mean": sum(values) / len(values), "unit": "ns"}}
        for name, values in samples.items()
    }
