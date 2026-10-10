#!/usr/bin/env python3
"""Measure immutable base/head sources together instead of mixing host loads."""

import argparse
import json
import math
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile


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


def run(root, base_sha, output):
    if not re.fullmatch(r"[0-9a-f]{40}", base_sha):
        raise ValueError("Base revision must be an immutable full Git SHA")
    resolved = subprocess.check_output(
        ["git", "rev-parse", base_sha + "^{commit}"], cwd=root, text=True
    ).strip()
    if resolved != base_sha:
        raise ValueError("Base revision did not resolve exactly")
    head = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=root, text=True
    ).strip()
    target = Path(os.environ.get("CARGO_TARGET_DIR", str(root / "target"))).resolve()
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="mockforge-benchmark-base-") as directory:
        base_root = Path(directory) / "source"
        subprocess.run(
            ["git", "worktree", "add", "--detach", str(base_root), base_sha],
            cwd=root,
            check=True,
        )
        try:
            # Cargo can reuse the same executable path for both revisions.
            # Preserve each build before the next build can overwrite it.
            base_binary = Path(directory) / "base"
            shutil.copy2(build(base_root, target), base_binary)
            head_binary = Path(directory) / "head"
            shutil.copy2(build(root, target), head_binary)
            base_names = names(base_binary, base_root)
            head_names = names(head_binary, root)
            common = sorted(set(base_names) & set(head_names))
            if not common:
                raise RuntimeError("No common benchmark cases to compare")
            if set(base_names) - set(head_names):
                raise RuntimeError("Candidate removed existing benchmark cases")
            baseline = {name: [] for name in common}
            current = {name: [] for name in head_names}
            for name in head_names:
                # ABBA pairs keep each case adjacent and balance ordering bias.
                for index, side in enumerate(("base", "head", "head", "base")):
                    if side == "base" and name not in baseline:
                        continue
                    binary, checkout, destination = (
                        (base_binary, base_root, baseline)
                        if side == "base"
                        else (head_binary, root, current)
                    )
                    destination[name].append(
                        measure(
                            binary,
                            checkout,
                            target,
                            name,
                            "paired-" + side + "-" + str(index),
                        )
                    )
            (output / "baseline.json").write_text(
                json.dumps(aggregate(baseline), indent=2) + "\n"
            )
            results = output / "criterion"
            for name, value in aggregate(current).items():
                estimates = results / name / "new" / "estimates.json"
                estimates.parent.mkdir(parents=True, exist_ok=True)
                timing = (
                    json.dumps({"mean": {"point_estimate": value["new"]["mean"]}})
                    + "\n"
                )
                estimates.write_text(timing)
                (estimates.parent / "benchmark.json").write_text(timing)
            (output / "sources.json").write_text(
                json.dumps(
                    {
                        "base_sha": base_sha,
                        "head_sha": head,
                        "order": "ABBA",
                        "base_samples_per_case": 2,
                        "head_samples_per_case": 2,
                        "load": os.getloadavg(),
                    },
                    indent=2,
                )
                + "\n"
            )
        finally:
            subprocess.run(
                ["git", "worktree", "remove", "--force", str(base_root)],
                cwd=root,
                check=True,
            )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-sha", required=True)
    parser.add_argument("--output", type=Path, default=Path("paired-benchmarks"))
    args = parser.parse_args()
    run(Path.cwd().resolve(), args.base_sha, args.output.resolve())
