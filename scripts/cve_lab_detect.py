"""Passive, detection-only CVE version check (``mockforge cve detect``).

Sends one ordinary read-only ``GET /`` to the target and reads the version
the target announces in its response headers. It never sends anything that
could trigger a vulnerability. The result is one of ``likely-affected``,
``not-affected`` or ``unknown``, written to ``report.json`` and logged in
``sent_requests.jsonl`` like the other packs.
"""

import json
import uuid
from pathlib import Path

import cve_lab_sent as sent
from cve_lab_authz import require_authorization
from cve_lab_catalog import DETECT_PACKS
from cve_lab_evidence import CLASSIFIERS
from cve_lab_transport import target_url, unverified_context, version_probe


def detect(args):
    pack = DETECT_PACKS[args.cve]
    target_url(args.target)
    authorization = require_authorization(args.target, assume_yes=args.i_am_authorized)
    output = Path(args.output_dir).resolve()
    output.mkdir(parents=True, exist_ok=False)
    run_id = uuid.uuid4().hex
    sent.open_sink(output / "sent_requests.jsonl", target=args.target)
    report = {
        "schema_version": 1,
        "run_id": run_id,
        "mode": "detect-only",
        "pack": pack["catalog"],
        "target": args.target,
        "authorization": authorization,
        "verdict": "unknown",
        "sent_requests": "sent_requests.jsonl",
        "scope": "Passive version fingerprint from response headers; the vulnerability is not tested",
    }
    try:
        context = unverified_context() if args.skip_tls_verify else None
        observation = version_probe(args.target.rstrip("/"), run_id, context=context)
        verdict, reason, checks = CLASSIFIERS[pack["classifier"]](pack, observation)
        report.update(
            verdict=verdict,
            reason=reason,
            evidence_checks=checks,
            observation={k: v for k, v in observation.items() if k != "headers"},
            announced_headers={
                name: value
                for name, value in observation.get("headers", {}).items()
                if name in ("server", "x-powered-by", "x-node-version")
            },
        )
    finally:
        try:
            (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        finally:
            sent.close_sink()
    cve = pack["catalog"]["cve"]
    print(f"{report['verdict']}: {cve} (passive version check of {args.target})")
    print(f"Reason: {report['reason']}")
    for check in report.get("evidence_checks", []):
        print(f"  - {check['check']}: {check['outcome']}")
    print(f"Report: {output / 'report.json'}")
    print(f"Sent requests: {output / 'sent_requests.jsonl'}")
    return 1 if report["verdict"] == "likely-affected" else 0
