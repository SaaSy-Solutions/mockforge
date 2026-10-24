"""Bounded, harmless HTTP security-rule canaries; Python 3.10+, standard library."""

import argparse
from functools import partial
from security_rule_pack_evidence import observe, classify
from security_rule_pack_config import load_pack
from security_rule_pack_transport import recorder, request, target_url
import json
import uuid
from pathlib import Path


def run(cases, url, observations, lock, timeout=2, window=0.1):
    results = []
    run_id = uuid.uuid4().hex

    deliver = partial(
        observe,
        run_id=run_id,
        url=url,
        observations=observations,
        lock=lock,
        timeout=timeout,
        window=window,
        send=request,
    )

    for case in cases:
        name = case["id"]
        result = {
            "id": name,
            "expected": case,
            "status": None,
            "origin_seen": None,
            "scenario_delivery": "unverified",
            "rule_behavior": "inconclusive",
            "runtime_patch_evidence": "unverified",
            "origin_observations": [],
        }
        if name == "camel-header":
            result.update(
                rule_behavior="unsupported",
                reason="mixed-case wire fidelity not verified",
            )
            results.append(result)
            continue
        # Corroborate origin reachability on both sides of every case.
        before = deliver("normal-control")
        status, error, records, valid, token = deliver(name)
        after = deliver("normal-control")
        healthy = all(s == 200 and e is None and v for s, e, _, v, _ in (before, after))
        result.update(
            correlation_id=token,
            status=status,
            origin_seen=bool(records),
            origin_observations=records,
            scenario_delivery="origin-observed" if valid else "unverified",
            control_before={
                "status": before[0],
                "origin_valid": before[3],
                "correlation_id": before[4],
                "observations": before[2],
            },
            control_after={
                "status": after[0],
                "origin_valid": after[3],
                "correlation_id": after[4],
                "observations": after[2],
            },
        )
        result.update(classify(case, status, error, records, valid, healthy))
        results.append(result)
    return {
        "schema_version": 1,
        "run_id": run_id,
        "target": url.geturl(),
        "timeout_seconds": timeout,
        "request_count": 3 * sum(c["id"] != "camel-header" for c in cases),
        "observation_window_seconds": window,
        "scope": "synthetic transport/rule behavior only; no application protection or patch proof",
        "results": results,
    }


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pack", required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--report", required=True)
    parser.add_argument(
        "--allow-external-target",
        action="store_true",
        help="affirm explicit authorization for this exact supplied target",
    )
    parser.add_argument("--origin-port", type=int, default=18081)
    parser.add_argument("--timeout", type=float, default=2)
    parser.add_argument("--observation-window", type=float, default=0.1)
    args = parser.parse_args(argv)
    try:
        cases = load_pack(args.pack)
        url = target_url(args.target, args.allow_external_target)
        if not 1 <= args.origin_port <= 65535 or not 0 < args.timeout <= 5:
            raise ValueError("origin port must be 1..65535; timeout must be >0 and <=5")
        if not 0.01 <= args.observation_window <= 1:
            raise ValueError("observation window must be 0.01..1 seconds")
        with recorder(args.origin_port) as (_, observations, lock):
            report = run(
                cases, url, observations, lock, args.timeout, args.observation_window
            )
        Path(args.report).write_text(json.dumps(report, indent=2) + "\n")
    except (ValueError, OSError) as exc:
        parser.error(str(exc))
    for result in report["results"]:
        print(f"{result['id']}: {result['rule_behavior']} ({result['reason']})")
    return 0 if all(r["rule_behavior"] == "pass" for r in report["results"]) else 1


if __name__ == "__main__":
    raise SystemExit(main())
