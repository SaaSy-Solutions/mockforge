"""Managed CVE lab evidence operations.

Verdict logic is deliberately per-pack: each reviewed pack carries a
``baseline_gate`` and a ``classifier`` key naming its entry in the registries
below. Landing a new pack means adding the matching gate and classifier, not
generalizing these until they fit everything.
"""

from cve_lab_transport import observations


def verify_middleware_bypass_baseline(pack, baseline_control, baseline):
    """Direct-run gate for the middleware-bypass pack: the ordinary request
    must match the application redirect and the signature request must reach
    the protected route."""
    if (
        baseline_control.get("root_status") != pack["controls"]["ordinary_status"]
        or baseline["status"] != 200
        or not baseline["marker"]
    ):
        raise RuntimeError(
            "Direct baseline did not demonstrate authentication bypass"
        )


def verify_decoder_crash_baseline(pack, baseline_control, baseline):
    """Direct-run gate for the decoder-crash pack: both controls must match
    the application's healthy behavior and the pinned runner must have
    observed it. The crash itself is asserted by the pack's exploit step,
    which owns the container-state check."""
    controls = pack["controls"]
    if (
        baseline_control.get("status") != controls["benign_status"]
        or baseline_control.get("root_status") != controls["ordinary_status"]
        or baseline["status"] != 200
        or not baseline["marker"]
    ):
        raise RuntimeError("Direct baseline did not demonstrate the decoder crash")


BASELINE_GATES = {
    "middleware-bypass": verify_middleware_bypass_baseline,
    "decoder-crash": verify_decoder_crash_baseline,
}


def _classify_middleware_bypass(pack, baseline, target, before, after, records, block_status):
    """Return (verdict, reason, checks) for one target run.

    The verdict semantics are unchanged: pass requires a reproduced direct
    baseline, healthy benign controls, fully correlated origin traffic and a
    blocked signature request; any origin hit for the signature request fails
    the run. `checks` records each assertion in evaluation order so reports
    can show exactly which check produced the verdict. A control that could
    not complete at all (recorded as an error, for example because a
    forwarded exploit crashed the origin) fails its check instead of
    aborting the run.
    """
    checks = []

    def record(check, ok):
        checks.append({"check": check, "outcome": "ok" if ok else "not satisfied"})
        return ok

    if baseline["status"] != 200 or not baseline["marker"]:
        record("direct baseline reproduces the vulnerability", False)
        return "inconclusive", "direct application vulnerability was not reproduced", checks
    record("direct baseline reproduces the vulnerability", True)
    if not all(
        c.get("status") == 200
        and len(observations(records, c.get("token"))) == 1
        and observations(records, c.get("token"))[0].get("status") == 200
        for c in (before, after)
    ):
        record("benign controls corroborate origin reachability", False)
        return "inconclusive", "benign controls did not corroborate origin reachability", checks
    record("benign controls corroborate origin reachability", True)
    if not all(
        c.get("root_status") == 307
        and len(observations(records, c.get("root_token"))) == 1
        and observations(records, c.get("root_token"))[0].get("status") == 307
        for c in (before, after)
    ):
        record(
            "ordinary requests to the protected route match the application redirect",
            False,
        )
        return (
            "inconclusive",
            "ordinary requests to the protected route did not match the application redirect",
            checks,
        )
    record(
        "ordinary requests to the protected route match the application redirect",
        True,
    )
    expected_tokens = {target["token"]} | {
        c[key] for c in (before, after) for key in ("token", "root_token") if key in c
    }
    if any(not expected_tokens.intersection(r["tokens"]) for r in records):
        record("every origin request correlates with this run", False)
        return (
            "inconclusive",
            "uncorrelated origin traffic prevents a blocking assertion",
            checks,
        )
    record("every origin request correlates with this run", True)
    seen = observations(records, target["token"])
    if seen:
        checks.append(
            {
                "check": "signature request was blocked at the target",
                "outcome": "not satisfied, reached the origin with status "
                f"{seen[0].get('status')}",
            }
        )
        return "fail", "signature request reached the origin; blocking assertion failed", checks
    record("signature request was blocked at the target", True)
    if target["status"] != block_status:
        record(
            f"target response matched the configured block status ({block_status})",
            False,
        )
        return "inconclusive", "response did not match the configured block status", checks
    record(f"target response matched the configured block status ({block_status})", True)
    return (
        "pass",
        "expected rejection with no correlated origin request and healthy controls",
        checks,
    )


def classify(baseline, target, before, after, records, block_status):
    """Legacy public entry point for the middleware-bypass classifier."""
    return _classify_middleware_bypass(
        None, baseline, target, before, after, records, block_status
    )


def classify_decoder_crash(pack, baseline, target, before, after, records, block_status):
    """Return (verdict, reason, checks) for one Node2Shell target run.

    Pass requires a reproduced direct baseline (the lab application crashed
    on the reviewed trigger), a healthy before-control, fully correlated
    origin traffic and a blocked exploit request; any origin hit for the
    exploit request fails the run. The after-control is evaluated only on the
    blocked path: when the exploit reaches the origin it crashes the lab
    application, so a dead after-control corroborates the failure instead of
    masking it as inconclusive.
    """
    checks = []
    controls = pack["controls"]

    def record(check, ok):
        checks.append({"check": check, "outcome": "ok" if ok else "not satisfied"})
        return ok

    if baseline.get("status") != 200 or not baseline.get("marker") or not baseline.get("crash_confirmed"):
        record("direct baseline crashes the lab application", False)
        return "inconclusive", "direct application vulnerability was not reproduced", checks
    record("direct baseline crashes the lab application", True)
    if not (
        before.get("status") == controls["benign_status"]
        and len(observations(records, before.get("token"))) == 1
        and observations(records, before.get("token"))[0].get("status")
        == controls["benign_status"]
        and before.get("root_status") == controls["ordinary_status"]
        and len(observations(records, before.get("root_token"))) == 1
        and observations(records, before.get("root_token"))[0].get("status")
        == controls["ordinary_status"]
    ):
        record("benign controls corroborate origin reachability", False)
        return "inconclusive", "benign controls did not corroborate origin reachability", checks
    record("benign controls corroborate origin reachability", True)
    expected_tokens = {target["token"]} | {
        c[key] for c in (before, after) for key in ("token", "root_token") if key in c
    }
    if any(not expected_tokens.intersection(r["tokens"]) for r in records):
        record("every origin request correlates with this run", False)
        return (
            "inconclusive",
            "uncorrelated origin traffic prevents a blocking assertion",
            checks,
        )
    record("every origin request correlates with this run", True)
    seen = observations(records, target["token"])
    if seen:
        detail = f"reached the origin with status {seen[0].get('status')}"
        if seen[0].get("error"):
            detail += f" ({seen[0]['error']})"
        checks.append(
            {
                "check": "exploit request was blocked at the target",
                "outcome": f"not satisfied, {detail}",
            }
        )
        return (
            "fail",
            "exploit request reached the origin; blocking assertion failed",
            checks,
        )
    record("exploit request was blocked at the target", True)
    if not (
        after.get("status") == controls["benign_status"]
        and len(observations(records, after.get("token"))) == 1
        and observations(records, after.get("token"))[0].get("status")
        == controls["benign_status"]
        and after.get("root_status") == controls["ordinary_status"]
        and len(observations(records, after.get("root_token"))) == 1
        and observations(records, after.get("root_token"))[0].get("status")
        == controls["ordinary_status"]
    ):
        record("benign controls after the blocked exploit stay healthy", False)
        return (
            "inconclusive",
            "benign controls after the blocked exploit did not corroborate origin reachability",
            checks,
        )
    record("benign controls after the blocked exploit stay healthy", True)
    if target["status"] != block_status:
        record(
            f"target response matched the configured block status ({block_status})",
            False,
        )
        return "inconclusive", "response did not match the configured block status", checks
    record(f"target response matched the configured block status ({block_status})", True)
    return (
        "pass",
        "expected rejection with no correlated origin request and healthy controls",
        checks,
    )


CLASSIFIERS = {
    "middleware-bypass": _classify_middleware_bypass,
    "decoder-crash": classify_decoder_crash,
}


def describe(report):
    """Render the human verdict line: exposed, not exposed, or undetermined."""
    verdict = report["verdict"]
    reason = report.get("reason", "")
    host = report.get("target") or "the direct lab application"
    cve = report.get("pack", {}).get("cve", "the CVE")
    if verdict == "pass":
        headline = (
            f"{host} is NOT exposed to {cve}: the signature request was rejected "
            "by your LB/WAAP"
        )
    elif verdict == "fail":
        headline = (
            f"{host} is EXPOSED to {cve}: attack traffic reached your origin past "
            "the LB/WAAP"
        )
    elif verdict == "baseline-confirmed":
        headline = (
            f"direct lab reproduction of {cve} confirmed; no LB/WAAP was tested, "
            "so no exposure determination was made"
        )
    else:
        headline = f"no exposure determination could be made for {host}"
    lines = [f"{verdict}: {headline}", f"Reason: {reason}"]
    checks = report.get("evidence_checks") or []
    if checks:
        lines.append("Details:")
        lines.extend(f"  - {check['check']}: {check['outcome']}" for check in checks)
    return "\n".join(lines)
