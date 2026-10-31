"""Per-CVE WAF/LB rule test from a declarative profile; Python 3.10+, stdlib.

Subcommands:
  new <CVE-ID>         write a profile skeleton to fill from the advisory
  server <profile>     print a MockForge mock of the guarded route (stdout)
  run <profile>        send the signature + benign controls through a supplied
                       load balancer to an owned recorder and assert the rule
                       blocked the signature and allowed the controls

The request set is bounded and inert: only GET, only the profile's marker
tokens (see cve_pack_profile), a correlation token, no bodies, no retries, no
redirects. It establishes rule behavior and bounded origin correlation, never
that a target is actually vulnerable or patched (that stays unverified).
"""

import argparse
import http.client
import json
import sys
import uuid
from functools import partial
from pathlib import Path

from cve_pack_evidence import observe, verdict
from cve_pack_profile import SCHEMA_VERSION, cases, load
from cve_pack_server import to_openapi
from security_rule_pack_transport import recorder, request, target_url

SKELETON = {
    "schema_version": SCHEMA_VERSION,
    "cve": "CVE-0000-00000",
    "title": "One-line description of the guarded weakness",
    "references": ["https://vendor.example/advisory"],
    "route": "/the/guarded/route",
    "signature": {
        "method": "GET",
        "query": {},
        "headers": {"x-example-signature-header": "marker"},
    },
    "expect": {"blocked_status": 403, "origin": "absent"},
    "controls": [
        {
            "id": "benign",
            "query": {},
            "headers": {},
            "expect": {"status": 200, "origin": "seen"},
        }
    ],
}


def run(profile, url, observations, lock, timeout=2, window=0.1):
    all_cases = list(cases(profile))
    beacon_path, beacon_headers = None, None
    for cid, path, headers, expect, kind in all_cases:
        if kind == "control" and expect["origin"] == "seen":
            beacon_path, beacon_headers = path, headers
            break
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
    results = []
    for cid, path, headers, expect, kind in all_cases:
        before = deliver(beacon_path, beacon_headers)
        status, error, records, valid, token = deliver(path, headers)
        after = deliver(beacon_path, beacon_headers)
        healthy = all(s == 200 and e is None and v for s, e, _, v, _ in (before, after))
        result = {
            "id": cid,
            "kind": kind,
            "expected": expect,
            "correlation_id": token,
            "status": status,
            "origin_seen": bool(records),
            "scenario_delivery": "origin-observed" if valid else "unverified",
            "runtime_patch_evidence": "unverified",
            "origin_observations": records,
        }
        result.update(verdict(expect, status, error, records, valid, healthy))
        results.append(result)
    return {
        "schema_version": SCHEMA_VERSION,
        "cve": profile["cve"],
        "run_id": run_id,
        "target": url.geturl(),
        "timeout_seconds": timeout,
        "observation_window_seconds": window,
        "request_count": 3 * len(all_cases),
        "scope": "synthetic transport/rule behavior only; no application protection or patch proof",
        "results": results,
    }


def _run(args):
    profile = load(args.profile)
    url = target_url(args.target, args.allow_external_target)
    if not 1 <= args.origin_port <= 65535 or not 0 < args.timeout <= 5:
        raise ValueError("origin port must be 1..65535; timeout must be >0 and <=5")
    if not 0.01 <= args.observation_window <= 1:
        raise ValueError("observation window must be 0.01..1 seconds")
    with recorder(args.origin_port) as (_, observations, lock):
        report = run(
            profile, url, observations, lock, args.timeout, args.observation_window
        )
    Path(args.report).write_text(json.dumps(report, indent=2) + "\n")
    for result in report["results"]:
        print(
            f"{result['id']} [{result['kind']}]: {result['rule_behavior']} ({result['reason']})"
        )
    return 0 if all(r["rule_behavior"] == "pass" for r in report["results"]) else 1


# Access-denied codes a WAF/CDN returns to block a request (vs 404/400/5xx,
# which are ambiguous: a missing route or a generic error, not a rule block).
DENY_CODES = {401, 403, 406, 429, 451, 501}


def _send(url, path, headers, timeout):
    try:
        return request(url, path, headers, timeout), None
    except (OSError, http.client.HTTPException) as exc:
        return None, type(exc).__name__


def probe(profile, url, timeout=5):
    """Send the signature + controls straight at a site and judge the signature
    RELATIVE to the benign controls. Use when you do NOT control the origin
    (a CDN/WAF in front of a third party), so recorder correlation is
    impossible: this reports what the front end did, not origin reachability.

    'blocked' means the signature got an access-denied status while a benign
    control was allowed. A 404/400/5xx is 'inconclusive', not a block, so a
    missing route is never mistaken for a rule firing."""
    results = []
    control_ok = False
    for cid, path, headers, expect, kind in cases(profile):
        if kind != "control":
            continue
        status, error = _send(url, path, headers, timeout)
        allowed = status is not None and status < 400
        control_ok = control_ok or allowed
        results.append(
            {
                "id": cid,
                "kind": "control",
                "status": status,
                "front_end": "allowed" if allowed else "unexpected",
                "reason": error or f"front end returned {status}",
            }
        )
    sid, spath, sheaders, _, _ = next(c for c in cases(profile) if c[4] == "signature")
    status, error = _send(url, spath, sheaders, timeout)
    if error:
        verdict, reason = "inconclusive", error
    elif not control_ok:
        verdict, reason = (
            "inconclusive",
            "no benign control was allowed; cannot baseline",
        )
    elif status in DENY_CODES:
        verdict, reason = "blocked", f"signature denied ({status}) while benign allowed"
    elif status < 400:
        verdict, reason = "not-blocked", f"signature passed ({status})"
    else:
        verdict, reason = "inconclusive", f"ambiguous status {status} (not a deny code)"
    results.insert(
        0,
        {
            "id": sid,
            "kind": "signature",
            "status": status,
            "front_end": verdict,
            "reason": reason,
        },
    )
    return {
        "schema_version": SCHEMA_VERSION,
        "cve": profile["cve"],
        "target": url.geturl(),
        "mode": "probe",
        "scope": "front-end response only; origin reachability not correlated, no patch proof",
        "results": results,
    }


def _probe(args):
    profile = load(args.profile)
    url = target_url(args.target, args.allow_external_target)
    if not 0 < args.timeout <= 10:
        raise ValueError("timeout must be >0 and <=10")
    report = probe(profile, url, args.timeout)
    Path(args.report).write_text(json.dumps(report, indent=2) + "\n")
    for r in report["results"]:
        print(f"{r['id']} [{r['kind']}]: {r['front_end']} ({r['reason']})")
    sig = next(r for r in report["results"] if r["kind"] == "signature")
    controls_ok = all(
        r["front_end"] == "allowed" for r in report["results"] if r["kind"] == "control"
    )
    return 0 if sig["front_end"] == "blocked" and controls_ok else 1


def _server(args):
    # Pass the raw profile; to_openapi validates it (load() would normalize first).
    print(json.dumps(to_openapi(json.loads(Path(args.profile).read_text())), indent=2))
    return 0


def _new(args):
    if not cve_ok(args.cve):
        raise ValueError("cve must look like CVE-YYYY-NNNN")
    out = Path(args.output) if args.output else Path(f"{args.cve}.profile.json")
    if out.exists() and not args.force:
        raise ValueError(f"{out} exists; pass --force to overwrite")
    skeleton = dict(SKELETON, cve=args.cve)
    out.write_text(json.dumps(skeleton, indent=2) + "\n")
    print(
        f"Wrote unfinished template to {out}. No CVE lookup or simulation was performed. "
        "Replace the placeholder title, reference, route and signature using the advisory before server/run/probe."
    )
    return 0


def cve_ok(value):
    from cve_pack_profile import CVE_ID

    return bool(CVE_ID.match(value))


def _tips(args):
    # Lazy import so the core run/new/server path needs no feed access.
    from cve_pack_enrich import tips
    from cve_pack_feeds import Fetcher

    if not cve_ok(args.cve):
        raise ValueError("cve must look like CVE-YYYY-NNNN")
    if not 0 < args.timeout <= 120:
        raise ValueError("timeout must be >0 and <=120")
    report = tips(args.cve, fetcher=Fetcher(timeout=args.timeout))
    print(json.dumps(report, indent=2))
    if not report["complete"]:
        print(
            "Some feeds are unavailable; partial results are above. Retry later or check access to the URLs in feed_errors.",
            file=sys.stderr,
        )
    return 0 if report["complete"] else 1


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_new = sub.add_parser(
        "new", help="write an unfinished template; does not look up or simulate a CVE"
    )
    p_new.add_argument("cve")
    p_new.add_argument("--output")
    p_new.add_argument("--force", action="store_true")
    p_new.set_defaults(func=_new)

    p_srv = sub.add_parser("server", help="print a MockForge mock of the route")
    p_srv.add_argument("profile")
    p_srv.set_defaults(func=_server)

    p_run = sub.add_parser("run", help="test a supplied LB against the profile")
    p_run.add_argument("profile")
    p_run.add_argument("--target", required=True, help="the load balancer origin")
    p_run.add_argument("--report", required=True)
    p_run.add_argument("--allow-external-target", action="store_true")
    p_run.add_argument("--origin-port", type=int, default=18082)
    p_run.add_argument("--timeout", type=float, default=2)
    p_run.add_argument("--observation-window", type=float, default=0.1)
    p_run.set_defaults(func=_run)

    p_probe = sub.add_parser(
        "probe",
        help="send the signature at an authorized target without an origin recorder and "
        "report the front end's response; no origin correlation",
    )
    p_probe.add_argument("profile")
    p_probe.add_argument("--target", required=True, help="the site/CDN/WAF URL")
    p_probe.add_argument("--report", required=True)
    p_probe.add_argument("--allow-external-target", action="store_true")
    p_probe.add_argument("--timeout", type=float, default=5)
    p_probe.set_defaults(func=_probe)

    p_tips = sub.add_parser(
        "tips", help="print remediation tips for a CVE (upgrade target, urgency)"
    )
    p_tips.add_argument("cve")
    p_tips.add_argument("--timeout", type=float, default=20)
    p_tips.set_defaults(func=_tips)

    args = parser.parse_args(argv)
    try:
        return args.func(args)
    except (ValueError, OSError) as exc:
        parser.error(str(exc))


if __name__ == "__main__":
    raise SystemExit(main())
