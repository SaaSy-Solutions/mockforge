"""Managed CVE lab evidence operations."""

from cve_lab_transport import observations


def classify(baseline, target, before, after, records, block_status):
    """Return (verdict, reason, checks) for one target run.

    The verdict semantics are unchanged: pass requires a reproduced direct
    baseline, healthy benign controls, fully correlated origin traffic and a
    blocked signature request; any origin hit for the signature request fails
    the run. `checks` records each assertion in evaluation order so reports
    can show exactly which check produced the verdict.
    """
    checks = []

    def record(check, ok):
        checks.append({"check": check, "outcome": "ok" if ok else "not satisfied"})
        return ok

    if baseline["status"] != 200 or not baseline["dashboard"]:
        record("direct baseline reproduces the vulnerability", False)
        return "inconclusive", "direct application vulnerability was not reproduced", checks
    record("direct baseline reproduces the vulnerability", True)
    if not all(
        c["status"] == 200
        and len(observations(records, c["token"])) == 1
        and observations(records, c["token"])[0].get("status") == 200
        for c in (before, after)
    ):
        record("benign controls corroborate origin reachability", False)
        return "inconclusive", "benign controls did not corroborate origin reachability", checks
    record("benign controls corroborate origin reachability", True)
    if not all(
        c["root_status"] == 307
        and len(observations(records, c["root_token"])) == 1
        and observations(records, c["root_token"])[0].get("status") == 307
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
        c[key] for c in (before, after) for key in ("token", "root_token")
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
                "outcome": "not satisfied — reached the origin with status "
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
