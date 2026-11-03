"""Build benchmark executables and require complete, fresh timing samples."""

import json
import math
import os
from pathlib import Path
import re
import subprocess


def refresh_rust_sources(root):
    """Invalidate mtime-based fingerprints inherited from another checkout."""
    root = root.resolve()
    tracked = subprocess.check_output(
        ["git", "ls-files", "-z", "--", "*.rs"], cwd=root
    )
    sources = [root / os.fsdecode(path) for path in tracked.split(b"\0") if path]
    if not sources:
        raise RuntimeError("No tracked Rust sources in benchmark checkout")
    for source in sources:
        if not source.resolve(strict=True).is_relative_to(root):
            raise RuntimeError("Rust source escapes benchmark checkout")
    # Cargo dep-info uses workspace-relative paths. With a shared target,
    # older source mtimes can make the next checkout reuse the prior library
    # and executable. Refresh libraries, build scripts and bench entry points;
    # dependency registry artifacts remain cached, and source bytes stay intact.
    for source in sources:
        os.utime(source, None)


def build(root, target):
    env = {
        **os.environ,
        "CARGO_TARGET_DIR": str(target),
        "CARGO_BUILD_JOBS": "2",
        "CARGO_INCREMENTAL": "0",
    }
    refresh_rust_sources(root)
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
        a
        for a in artifacts
        if a.get("reason") == "compiler-artifact"
        and a.get("target", {}).get("name") == "core_benchmarks"
        and a.get("executable")
    ]
    if len(executables) != 1:
        raise RuntimeError("Expected one compiled core benchmark executable")
    artifact = executables[0]
    if artifact.get("fresh") is not False:
        raise RuntimeError("Benchmark executable was reused instead of rebuilt")
    return Path(artifact["executable"])


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
        name: {
            "new": {
                "mean": sum(values) / len(values),
                # Kept so the comparison can require base and head to separate,
                # not just their means (#1309).
                "samples": list(values),
                "unit": "ns",
            }
        }
        for name, values in samples.items()
    }
