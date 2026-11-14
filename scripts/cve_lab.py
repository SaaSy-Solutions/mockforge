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
from cve_lab_catalog import CATALOG, CVE, PACKS
from cve_lab_engine import doctor, prepare, lab, cleanup, verify_images
from cve_lab_transport import target_url, receiver, control, unverified_context
from cve_lab_nuclei import probe
from cve_lab_evidence import BASELINE_GATES, CLASSIFIERS, describe


def run_plan_lines(args):
    """Human-readable plan printed before anything downloads or starts."""
    pack_catalog = PACKS[args.cve]["catalog"]
    lines = [
        f"Recorder endpoint: {args.listen_address}:{args.origin_port}",
    ]
    if args.target:
        lines.append(
            f"Target under test: {args.target}, configure its LB/WAAP upstream to "
            "forward to the recorder endpoint above before requests flow"
        )
        if args.listen_address == "127.0.0.1":
            lines.append(
                "Note: the recorder only accepts connections on 127.0.0.1. If your "
                "LB/WAAP runs on another machine, pass --listen-address <an isolated "
                "lab interface IP> so it can reach the recorder."
            )
    else:
        lines.append(
            "No --target: the lab reproduces the CVE directly and no LB/WAAP is tested"
        )
    lines.append(
        f"Images: pinned {pack_catalog['application_version']} lab and "
        f"{pack_catalog['runner_version']} runner (see `mockforge cve list`)"
    )
    if pack_catalog.get("image_status") == "pending-lab-host-review":
        lines.append(
            "Lab image pending review: this pack refuses to run until the "
            "reviewed image digest is pinned (see `mockforge cve list`)"
        )
    return lines


def run(args):
    pack = PACKS[args.cve]
    target_url(args.target) if args.target else None
    ipaddress.IPv4Address(args.listen_address)
    if not 1 <= args.origin_port <= 65535 or not 400 <= args.block_status <= 599:
        raise ValueError("origin-port must be 1..65535; block-status must be 400..599")
    for line in run_plan_lines(args):
        print(line, flush=True)
    output = Path(args.output_dir).resolve()
    output.mkdir(parents=True, exist_ok=False)
    run_id = uuid.uuid4().hex
    report = {
        "schema_version": 1,
        "run_id": run_id,
        "pack": pack["catalog"],
        "target": args.target,
        "tls_verify": not args.skip_tls_verify,
        "verdict": "inconclusive",
        "runtime_patch_evidence": "unverified",
        "scope": "Selected CVE lab reproduction and request-blocking regression, not comprehensive CVE coverage",
    }
    (output / "ownership.json").write_text(
        json.dumps({"run_id": run_id, "engine": args.engine})
    )
    tls_context = unverified_context() if args.skip_tls_verify else None
    if args.skip_tls_verify:
        print(
            "TLS VERIFICATION DISABLED: certificate errors on the target are "
            "ignored for this run (self-signed test certificates only). Never "
            "use this against an untrusted network.",
            flush=True,
        )
    try:
        prepare(args.engine, pack)
        image_verification = verify_images(args.engine, pack)
        report["image_verification"] = image_verification
        present = sum(1 for entry in image_verification if entry["status"] == "present")
        print(
            f"Images verified on this machine: {present}/{len(image_verification)} "
            "pinned images present (details in report.json)",
            flush=True,
        )
        with lab(args.engine, run_id, output, pack) as backend:
            with receiver(
                args.origin_port,
                args.listen_address,
                backend,
                signature_header=pack["probe"].get("signature_header"),
            ) as records:
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
                baseline_control = control(
                    pack, origin, run_id, "baseline-control", context=tls_context
                )
                baseline = probe(
                    args.engine, origin, output, "baseline", run_id, pack,
                    insecure=args.skip_tls_verify,
                )
                report.update(
                    baseline=baseline,
                    baseline_control=baseline_control,
                    unauthenticated_status=baseline_control["root_status"],
                )
                BASELINE_GATES[pack["baseline_gate"]](pack, baseline_control, baseline)
                if args.target:
                    target_start = len(records)
                    before = control(pack, args.target, run_id, "before", context=tls_context)
                    tested = probe(
                        args.engine,
                        args.target.rstrip("/"),
                        output,
                        "target",
                        run_id,
                        pack,
                        insecure=args.skip_tls_verify,
                    )
                    after = control(pack, args.target, run_id, "after", context=tls_context)
                    time.sleep(0.5)
                    verdict, reason, checks = CLASSIFIERS[pack["classifier"]](
                        pack,
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
                            "LB controls did not match this application's benign control responses",
                        )
                        checks = [
                            {
                                "check": "benign control responses match this application",
                                "outcome": "not satisfied",
                            }
                        ]
                    report.update(
                        verdict=verdict,
                        reason=reason,
                        evidence_checks=checks,
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
    print(describe(report))
    print(f"Report: {output / 'report.json'}")
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
    runner.add_argument(
        "--origin-port",
        type=int,
        default=CATALOG["origin_port"],
        help="Port the recorder listens on; point your LB/WAAP upstream here",
    )
    runner.add_argument(
        "--listen-address",
        default="127.0.0.1",
        help="Recorder bind IPv4 address; 127.0.0.1 only accepts local connections, "
        "so pass an isolated lab interface IP when your LB/WAAP runs on another machine",
    )
    runner.add_argument("--block-status", type=int, default=403)
    runner.add_argument(
        "--skip-tls-verify",
        action="store_true",
        help="Accept self-signed or otherwise unverified certificates on --target "
        "(explicit opt-in for lab test targets; verification is on by default)",
    )
    runner.add_argument(
        "--output-dir", required=True, help="New report directory, never overwritten"
    )
    down = commands.add_parser(
        "down", help="Recover owned containers after an interrupted run"
    )
    down.add_argument("--output-dir", required=True)
    images_parser = commands.add_parser(
        "images",
        help="Show each pinned pack image and whether it is present on this machine",
    )
    images_parser.add_argument(
        "cve",
        nargs="?",
        choices=tuple(PACKS),
        default=None,
        help="One pack, or omit for every reviewed pack",
    )
    args = parser.parse_args(argv)
    try:
        if args.action == "doctor":
            print(json.dumps(doctor(args.engine), indent=2))
        elif args.action == "list":
            print(json.dumps({cve: PACKS[cve]["catalog"] for cve in PACKS}, indent=2))
        elif args.action == "prepare":
            prepare(args.engine, PACKS[args.cve])
            print(
                "Pinned lab and Nuclei images ready; updates ship with reviewed MockForge packs"
            )
        elif args.action == "run":
            return run(args)
        elif args.action == "down":
            cleanup(args)
        elif args.action == "images":
            doctor(args.engine)
            targets = [args.cve] if args.cve else list(PACKS)
            listing = {
                cve: verify_images(args.engine, PACKS[cve]) for cve in targets
            }
            print(json.dumps(listing, indent=2))
            missing = [
                entry
                for entries in listing.values()
                for entry in entries
                # Unpinned (pending-review) images are listed, not counted.
                if entry["status"] == "missing"
            ]
            if missing:
                print(
                    f"{len(missing)} pinned image(s) missing on this machine; "
                    "run `mockforge cve prepare <CVE>` or load the exact digests "
                    "(see the managed CVE labs guide)",
                    file=sys.stderr,
                )
                return 1
    except (RuntimeError, ValueError, OSError, subprocess.TimeoutExpired) as exc:
        print(str(exc), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(143))
    raise SystemExit(main())
