#!/usr/bin/env python3
"""Bounded, read-only paired diagnostic; Python standard library only."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time


THRESHOLD = 5.0
COMMAND = ["cargo", "+1.96.0", "bench", "--locked", "-p", "mockforge-core",
           "--bench", "core_benchmarks"]
# Inspected against core_benchmarks.rs. Fail closed if this suite changes.
EXPECTED = {
    "template_rendering/simple": 100,
    "template_rendering/complex": 100,
    "template_rendering/arrays": 100,
    "json_validation/simple": 100,
    "json_validation/complex": 100,
    "openapi_parsing/small_spec": 100,
    "openapi_parsing/medium_spec_10_paths": 100,
    "data_generation/generate_single_record": 100,
    "encryption/aes256_gcm": 100,
    "encryption/chacha20_poly1305": 100,
    "memory/large_spec_parsing": 10,
    "memory/deep_template_rendering": 10,
    "memory/large_data_validation": 10,
}
PLAN = [
    ("warmup-base", "base", 0), ("warmup-candidate", "candidate", 0),
    ("pair-1-base", "base", 1), ("pair-1-candidate", "candidate", 1),
    ("pair-2-candidate", "candidate", 2), ("pair-2-base", "base", 2),
    ("pair-3-base", "base", 3), ("pair-3-candidate", "candidate", 3),
]


class InvalidData(ValueError):
    """Missing, inconsistent, or malformed diagnostic evidence."""


def require(condition, message):
    if not condition:
        raise InvalidData(message)


def read_json(path):
    try:
        return json.loads(path.read_text())
    except (OSError, ValueError) as exc:
        raise InvalidData(f"Cannot read {path}: {exc}") from exc


def write_json(path, value):
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")
    temporary.replace(path)


def positive(value, context):
    require(type(value) in (float, int) and math.isfinite(value) and value > 0,
            f"Invalid positive finite number: {context}")
    return value


def parse_criterion(root):
    """Only parse fresh new/ results, never base/ or change/ estimates."""
    found = {}
    for estimate_file in sorted(root.glob("**/new/estimates.json")):
        directory = estimate_file.parent
        name = directory.parent.relative_to(root).as_posix()
        require(name in EXPECTED, f"Unexpected benchmark: {name}")
        require(name not in found, f"Duplicate benchmark: {name}")
        benchmark = read_json(directory / "benchmark.json")
        require(benchmark.get("full_id") == name, f"Mismatched benchmark identity: {name}")
        estimates = read_json(estimate_file)
        mean = estimates.get("mean", {})
        point = positive(mean.get("point_estimate"), f"{name} mean")
        interval = mean.get("confidence_interval", {})
        low = positive(interval.get("lower_bound"), f"{name} mean lower bound")
        high = positive(interval.get("upper_bound"), f"{name} mean upper bound")
        level = positive(interval.get("confidence_level"), f"{name} confidence level")
        require(low <= point <= high and level == 0.95, f"Invalid mean interval: {name}")
        samples = read_json(directory / "sample.json")
        iters, times = samples.get("iters"), samples.get("times")
        require(isinstance(iters, list) and isinstance(times, list), f"Missing raw samples: {name}")
        require(len(iters) == len(times) == EXPECTED[name], f"Incomplete sample count: {name}")
        require(samples.get("sampling_mode") in ("Linear", "Flat"), f"Invalid sampling mode: {name}")
        for value in iters + times:
            positive(value, f"{name} raw sample")
        found[name] = {"mean_ns": point, "mean_interval_ns": [low, high],
                       "confidence_level": level, "sample_count": len(iters)}
    require(set(found) == set(EXPECTED),
            f"Incomplete benchmark set; missing={sorted(set(EXPECTED) - set(found))}")
    return found


def classify(deltas):
    require(len(deltas) == 3 and all(type(d) in (int, float) and math.isfinite(d) for d in deltas),
            "Exactly three finite pair deltas are required")
    if all(d > THRESHOLD for d in deltas):
        return "reproducible_regression"
    if all(d < -THRESHOLD for d in deltas):
        return "reproducible_improvement"
    if all(-THRESHOLD <= d <= THRESHOLD for d in deltas):
        return "within_threshold"
    return "mixed"


def utc_now():
    return datetime.now(timezone.utc).isoformat()


def captured(source, *command):
    return subprocess.check_output(command, cwd=source, text=True, stderr=subprocess.STDOUT).strip()


def build_settings():
    # Never capture the full environment, which may contain runner credentials.
    names = {"RUSTUP_TOOLCHAIN", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTDOCFLAGS",
             "RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "CARGO_HOME",
             "CARGO_TARGET_DIR", "CARGO_BUILD_JOBS", "CARGO_INCREMENTAL", "CRITERION_HOME",
             "CC", "CXX", "CFLAGS", "CXXFLAGS", "LDFLAGS"}
    names.update(k for k in os.environ if k.startswith(("CARGO_PROFILE_", "CARGO_TARGET_")))
    return {name: os.environ.get(name) for name in sorted(names)}


def run(root, source):
    root.mkdir(parents=True, exist_ok=True)
    require(not (root / "runs.json").exists(), "Refusing to overwrite/retry an existing diagnostic")
    shas = {"base": os.environ.get("BASE_SHA", ""), "candidate": os.environ.get("CANDIDATE_SHA", "")}
    require(all(re.fullmatch(r"[0-9a-f]{40}", value) for value in shas.values()),
            "Replace BASE_SHA and CANDIDATE_SHA with full immutable commit SHAs")
    require(shas["base"] != shas["candidate"], "Base and candidate must be distinct commits")
    for sha in shas.values():
        require(captured(source, "git", "rev-parse", f"{sha}^{{commit}}") == sha, "Commit not available")
    require(captured(source, "git", "diff", "--name-only", shas["base"], shas["candidate"]) == "Cargo.lock",
            "Base and candidate must differ only in Cargo.lock")
    require(not captured(source, "git", "status", "--porcelain"), "Checkout must be clean")
    settings = build_settings()
    require(settings["RUSTUP_TOOLCHAIN"] == "1.96.0" and settings["CARGO_BUILD_JOBS"] == "2",
            "Use the fixed toolchain and two Cargo build jobs")
    require(settings["CARGO_TARGET_DIR"] and settings["CRITERION_HOME"], "Missing isolated output paths")
    criterion = Path(settings["CRITERION_HOME"]).resolve()
    target = Path(settings["CARGO_TARGET_DIR"]).resolve()
    # Only these dedicated temporary paths may be cleared; no source/build/cache deletion.
    require(criterion.name == "1154-paired-criterion", "Unexpected Criterion output directory")
    require(not root.is_relative_to(source) and not criterion.is_relative_to(source)
            and not target.is_relative_to(source), "Evidence and build outputs must be outside checkout")
    require(not criterion.is_relative_to(root) and not root.is_relative_to(criterion)
            and not criterion.is_relative_to(target) and not target.is_relative_to(criterion),
            "Criterion scratch directory must not overlap results or build output")
    compiler = captured(source, "rustc", "+1.96.0", "-Vv")
    require("release: 1.96.0" in compiler.splitlines(), "Unexpected compiler release")
    metadata = {"schema": 1, "started_utc": utc_now(), "shas": shas,
                "diagnostic_sha": captured(source, "git", "rev-parse", "HEAD"),
                "rustc": compiler, "cargo": captured(source, "cargo", "+1.96.0", "-Vv"),
                "host": captured(source, "uname", "-a"), "cpu": captured(source, "lscpu"),
                "os_release": Path("/etc/os-release").read_text(), "settings": settings,
                "command": COMMAND, "threshold_percent": THRESHOLD,
                "runner_image": {key: os.environ.get(key) for key in
                                 ("ImageOS", "ImageVersion", "RUNNER_OS", "RUNNER_ARCH")},
                "run_url": f"https://github.com/{os.environ.get('GITHUB_REPOSITORY', '')}/actions/runs/{os.environ.get('GITHUB_RUN_ID', '')}",
                "run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
                "expected_benchmarks": EXPECTED}
    write_json(root / "metadata.json", metadata)
    (root / "lockfile.diff").write_text(captured(source, "git", "diff", shas["base"], shas["candidate"], "--", "Cargo.lock") + "\n")
    for variant, sha in shas.items():
        (root / f"{variant}-Cargo.lock").write_bytes(
            subprocess.check_output(["git", "show", f"{sha}:Cargo.lock"], cwd=source)
        )
    records = []
    write_json(root / "runs.json", records)
    # Reserve time for report/upload inside the 45-minute workflow job budget.
    deadline = time.monotonic() + 35 * 60
    for ordinal, (run_id, variant, pair) in enumerate(PLAN, start=1):
        run_dir = root / "runs" / run_id
        run_dir.mkdir(parents=True, exist_ok=False)
        record = {"id": run_id, "ordinal": ordinal, "variant": variant, "pair": pair,
                  "measured": pair != 0, "sha": shas[variant], "started_utc": utc_now(),
                  "state": "running", "command": COMMAND, "settings": build_settings()}
        records.append(record)
        write_json(root / "runs.json", records)
        print(f"Starting {ordinal}/8: {run_id}", flush=True)
        started = time.monotonic()
        try:
            require(record["settings"] == settings, "Compiler/cache settings changed between runs")
            require(not captured(source, "git", "status", "--porcelain"), "Checkout changed between runs")
            captured(source, "git", "checkout", "--detach", shas[variant])
            require(captured(source, "git", "rev-parse", "HEAD") == shas[variant], "Wrong checkout SHA")
            record["lock_sha256"] = hashlib.sha256((source / "Cargo.lock").read_bytes()).hexdigest()
            # Fresh Criterion output on EVERY run avoids any stale new/base estimates.
            if criterion.exists():
                shutil.rmtree(criterion)
            criterion.mkdir()
            remaining = deadline - time.monotonic()
            require(remaining > 0, "Diagnostic time budget exhausted")
            with (run_dir / "cargo-bench.log").open("w") as log:
                result = subprocess.run(COMMAND, cwd=source, stdout=log, stderr=subprocess.STDOUT,
                                        timeout=remaining, check=False)
            record["exit_code"] = result.returncode
            require(result.returncode == 0, f"cargo bench failed: {run_id}, code {result.returncode}")
            require(not captured(source, "git", "status", "--porcelain"), "Benchmark changed tracked source")
            require(build_settings() == settings, "Build settings changed")
            parse_criterion(criterion)
            record["state"] = "complete"
        except (InvalidData, OSError, subprocess.SubprocessError) as exc:
            record["state"] = "failed"
            record["error"] = str(exc)
        finally:
            if criterion.exists():
                shutil.copytree(criterion, run_dir / "criterion")
            record["finished_utc"] = utc_now()
            record["elapsed_seconds"] = time.monotonic() - started
            write_json(root / "runs.json", records)
        if record["state"] != "complete":
            raise InvalidData(record["error"])
    print("All eight runs captured; the first two are excluded from comparisons.")


def report(root):
    metadata = read_json(root / "metadata.json")
    records = read_json(root / "runs.json")
    require(metadata.get("schema") == 1 and metadata.get("threshold_percent") == THRESHOLD,
            "Unexpected metadata schema or threshold")
    require(metadata.get("expected_benchmarks") == EXPECTED, "Unexpected benchmark inventory")
    require(metadata.get("command") == COMMAND, "Unexpected benchmark command")
    shas = metadata.get("shas", {})
    require(set(shas) == {"base", "candidate"} and all(re.fullmatch(r"[0-9a-f]{40}", sha) for sha in shas.values())
            and shas["base"] != shas["candidate"], "Invalid recorded SHAs")
    require(len(records) == len(PLAN), f"Incomplete run plan: {len(records)}/8 runs recorded")
    parsed = {}
    for ordinal, (record, (run_id, variant, pair)) in enumerate(zip(records, PLAN), start=1):
        require((record.get("id"), record.get("variant"), record.get("pair"), record.get("ordinal"))
                == (run_id, variant, pair, ordinal), f"Wrong run ordering: {run_id}")
        require(record.get("state") == "complete" and record.get("exit_code") == 0,
                f"Incomplete/failed run: {run_id}")
        require(record.get("sha") == shas[variant] and record.get("measured") == (pair != 0),
                f"Wrong run identity: {run_id}")
        require(record.get("settings") == metadata["settings"] and record.get("command") == COMMAND,
                f"Inconsistent build settings: {run_id}")
        expected_lock = hashlib.sha256((root / f"{variant}-Cargo.lock").read_bytes()).hexdigest()
        require(record.get("lock_sha256") == expected_lock, f"Wrong lockfile: {run_id}")
        parsed[run_id] = parse_criterion(root / "runs" / run_id / "criterion")
    lines = ["PR #1154 paired core benchmark diagnostic", "",
             f"Base: {shas['base']}", f"Candidate: {shas['candidate']}",
             "Scope: one GitHub-hosted ubuntu-latest runner, Rust 1.96.0, core_benchmarks only.",
             "One full base/candidate warm-up is excluded; three pairs run B/C, C/B, B/C.",
             "Metric: mean point estimate, ns/iteration; positive delta means candidate slower.",
             "Threshold: strictly greater than 5.0% (existing gate). No pair is discarded or averaged.",
             "Reproducible means all three point-estimate deltas cross the same threshold.",
             "Mixed means inconsistent threshold/direction; it remains unresolved, not a pass.",
             "Within-threshold means all three observed deltas lie in [-5%, +5%].",
             "Criterion mean 95% intervals are retained below; classification is not a statistical significance test.",
             "This diagnostic cannot establish all-platform or universal absence of regressions.", ""]
    results = {}
    failed = False
    for name in sorted(EXPECTED):
        pairs = []
        for pair in range(1, 4):
            base = parsed[f"pair-{pair}-base"][name]
            candidate = parsed[f"pair-{pair}-candidate"][name]
            delta = (candidate["mean_ns"] - base["mean_ns"]) / base["mean_ns"] * 100
            pairs.append({"pair": pair, "order": "candidate/base" if pair == 2 else "base/candidate",
                          "base": base, "candidate": candidate, "delta_percent": delta})
        classification = classify([pair["delta_percent"] for pair in pairs])
        failed |= classification in ("reproducible_regression", "mixed")
        results[name] = {"classification": classification, "pairs": pairs}
        lines.append(f"{name}: {classification}")
        for pair in pairs:
            base, candidate = pair["base"], pair["candidate"]
            lines.append(f"  Pair {pair['pair']} ({pair['order']}): base {base['mean_ns']:.6g} ns "
                         f"CI {base['mean_interval_ns']}; candidate {candidate['mean_ns']:.6g} ns "
                         f"CI {candidate['mean_interval_ns']}; delta {pair['delta_percent']:+.3f}%")
    status = "UNRESOLVED: regression or mixed result; review every pair." if failed else "WITHIN OBSERVED REGRESSION LIMIT: all benchmarks within-threshold or reproducibly faster."
    lines.extend(["", status, "", "Ordered run records:"])
    lines.extend(f"  {r['ordinal']}. {r['id']} {r['sha']} {r['started_utc']} to {r['finished_utc']} ({r['elapsed_seconds']:.1f}s)" for r in records)
    write_json(root / "report.json", {"status": status, "threshold_percent": THRESHOLD, "benchmarks": results})
    (root / "report.txt").write_text("\n".join(lines) + "\n")
    print("\n".join(lines))
    return 1 if failed else 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("run", "report"))
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--source", type=Path)
    args = parser.parse_args()
    root = args.root.resolve()
    try:
        if args.command == "run":
            require(args.source is not None, "run requires --source")
            run(root, args.source.resolve())
            return 0
        return report(root)
    except (InvalidData, OSError, KeyError, TypeError, subprocess.SubprocessError) as exc:
        root.mkdir(parents=True, exist_ok=True)
        message = f"INCOMPLETE: {exc}\nDo not infer performance results from incomplete evidence.\n"
        (root / "report.txt").write_text(message)
        write_json(root / "report.json", {"status": "incomplete", "error": str(exc)})
        print(message, file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
