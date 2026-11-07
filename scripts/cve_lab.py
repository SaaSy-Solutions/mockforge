#!/usr/bin/env python3
"""Managed, pinned CVE labs. Python 3.11+ and a local Linux Docker/Podman engine."""

import argparse
import http.client
import ipaddress
import json
from pathlib import Path
import signal
import subprocess
import sys
import time
import uuid
from cve_lab_catalog import CVE, PACKS
from cve_lab_engine import doctor, prepare, lab, cleanup
from cve_lab_transport import target_url, receiver, control
from cve_lab_nuclei import probe
from cve_lab_evidence import classify


def run(args):
    pack = PACKS[args.cve]
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
        "pack": pack["catalog"],
        "target": args.target,
        "verdict": "inconclusive",
        "runtime_patch_evidence": "unverified",
        "scope": "Selected CVE lab reproduction and request-blocking regression, not comprehensive CVE coverage",
    }
    (output / "ownership.json").write_text(
        json.dumps({"run_id": run_id, "engine": args.engine})
    )
    try:
        prepare(args.engine, pack)
        with lab(args.engine, run_id, output, pack) as backend:
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
                baseline = probe(args.engine, origin, output, "baseline", run_id, pack)
                report.update(
                    baseline=baseline,
                    baseline_control=baseline_control,
                    unauthenticated_status=ordinary_status,
                )
                if (
                    ordinary_status != 307
                    or baseline["status"] != 200
                    or not baseline["marker"]
                ):
                    raise RuntimeError(
                        "Direct baseline did not demonstrate authentication bypass"
                    )
                if args.target:
                    target_start = len(records)
                    before = control(args.target, run_id, "before")
                    tested = probe(
                        args.engine, args.target.rstrip("/"), output, "target", run_id, pack
                    )
                    after = control(args.target, run_id, "after")
                    time.sleep(0.5)
                    verdict, reason = classify(
                        baseline,
                        tested,
                        before,
                        after,
                        records[target_start:],
                        args.block_status,
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


def main(argv=None):
    parser = argparse.ArgumentParser(prog="mockforge cve", description=__doc__)
    parser.add_argument("--engine", choices=("docker", "podman"), default="docker")
    commands = parser.add_subparsers(dest="action", required=True)
    commands.add_parser("doctor", help="Check local prerequisites without downloading")
    commands.add_parser("list", help="Show reviewed pack and pinned versions")
    prepare_parser = commands.add_parser(
        "prepare", help="Download and cache the reviewed images"
    )
    prepare_parser.add_argument("cve", choices=tuple(PACKS))
    runner = commands.add_parser(
        "run",
        help="Auto-prepare, reproduce directly, optionally compare through your LB, then stop",
    )
    runner.add_argument("cve", choices=tuple(PACKS))
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
            prepare(args.engine, PACKS[args.cve])
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
