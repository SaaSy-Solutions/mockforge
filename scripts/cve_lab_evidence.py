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
        baseline_control["root_status"] != pack["controls"]["ordinary_status"]
        or baseline["status"] != 200
        or not baseline["marker"]
    ):
        raise RuntimeError(
            "Direct baseline did not demonstrate authentication bypass"
        )


def verify_cgi_injection_baseline(pack, baseline_control, baseline):
    """Direct-run gate for the CGI argument-injection pack: both controls must
    match the application's non-vulnerable behavior and the signature request
    must execute the injected PHP."""
    controls = pack["controls"]
    if (
        baseline_control["status"] != controls["benign_status"]
        or baseline_control["root_status"] != controls["ordinary_status"]
        or baseline["status"] != 200
        or not baseline["marker"]
    ):
        raise RuntimeError(
            "Direct baseline did not demonstrate PHP code execution"
        )


def verify_modphp_segfault_baseline(pack, baseline_control, baseline):
    """Direct-run gate for the mod_php packs: both controls must match the
    application's healthy behavior and the pinned runner must have observed
    it. The corruption itself is asserted by the pack's exploit step, which
    owns the segmentation-fault log check."""
    controls = pack["controls"]
    if (
        baseline_control.get("status") != controls["benign_status"]
        or baseline_control.get("root_status") != controls["ordinary_status"]
        or baseline["status"] != 200
        or not baseline["marker"]
    ):
        raise RuntimeError(
            "Direct baseline did not demonstrate the request-lifecycle corruption"
        )


BASELINE_GATES = {
    "middleware-bypass": verify_middleware_bypass_baseline,
    "cgi-argument-injection": verify_cgi_injection_baseline,
    "modphp-segfault": verify_modphp_segfault_baseline,
}


def extra_case_evidence(pack, outcome, records):
    """Verify one control phase's data-driven extra cases.

    A pack may declare ``extra_cases`` - additional reviewed requests, each
    with its own expected origin status. This helper checks that every
    declared case was answered with that status and that the recorder saw
    exactly one matching request for its token. With no extras declared the
    result is vacuously ok and adds no checks, so packs that do not use the
    mechanism are unchanged. Returns (check_entries, all_ok, tokens).
    """
    entries = []
    all_ok = True
    seen_extras = (outcome or {}).get("extras", {})
    for case in (pack or {}).get("extra_cases", ()):
        case_id = case["id"]
        expect = case["expect_status"]
        seen = seen_extras.get(case_id) or {}
        token = seen.get("token")
        at_origin = observations(records, token) if token else []
        ok = (
            seen.get("status") == expect
            and len(at_origin) == 1
            and at_origin[0].get("status") == expect
        )
        if ok:
            detail = "ok"
        elif seen.get("status") is not None and seen.get("status") != expect:
            detail = f"not satisfied, origin status {seen.get('status')}"
        elif not at_origin:
            detail = (
                "not satisfied, no usable response "
                f"({seen.get('error') or 'no origin observation'})"
            )
        else:
            detail = (
                "not satisfied, origin observation status "
                f"{at_origin[0].get('status')}"
            )
        entries.append(
            {"check": f"extra case {case_id} answered {expect}", "outcome": detail}
        )
        all_ok = all_ok and ok
    tokens = {
        seen["token"] for seen in seen_extras.values() if seen.get("token")
    }
    return entries, all_ok, tokens


def _extras_phase(pack, outcome, records, checks):
    """Merge one phase's extra-case checks; returns (all_ok, tokens)."""
    entries, ok, tokens = extra_case_evidence(pack, outcome, records)
    checks.extend(entries)
    return ok, tokens


def verify_data_driven_baseline(pack, baseline_control, baseline):
    """Direct-run gate for packs using the generic data-driven evidence path.

    Both controls must match the statuses the pack declares and the pinned
    runner must have observed the marker. Everything else (extra cases, the
    signature's expected status and marker) is pack data checked generically.
    """
    controls = pack["controls"]
    if (
        baseline_control.get("status") != controls["benign_status"]
        or baseline_control.get("root_status") != controls["ordinary_status"]
        or baseline["status"] != 200
        or not baseline["marker"]
    ):
        raise RuntimeError(
            "Direct baseline did not match the pack's expected behavior"
        )


BASELINE_GATES["data-driven"] = verify_data_driven_baseline


def _classify_middleware_bypass(pack, baseline, target, before, after, records, block_status):
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

    if baseline["status"] != 200 or not baseline["marker"]:
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
    extras_ok_before, extras_tokens_before = _extras_phase(pack, before, records, checks)
    extras_ok_after, extras_tokens_after = _extras_phase(pack, after, records, checks)
    if not (extras_ok_before and extras_ok_after):
        return (
            "inconclusive",
            "a data-driven extra case did not match its expected status",
            checks,
        )
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
    expected_tokens = (
        {target["token"]}
        | extras_tokens_before
        | extras_tokens_after
        | {
            c[key] for c in (before, after) for key in ("token", "root_token")
        }
    )
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


def classify_php_cgi_argument_injection(
    pack, baseline, target, before, after, records, block_status
):
    """Return (verdict, reason, checks) for one PHP-CGI target run.

    Pass requires a reproduced direct baseline (the injected PHP executed),
    benign and ordinary controls that match the application's non-vulnerable
    behavior, fully correlated origin traffic, and a blocked exploit request;
    any origin hit for the exploit request fails the run.
    """
    checks = []
    controls = pack["controls"]

    def record(check, ok):
        checks.append({"check": check, "outcome": "ok" if ok else "not satisfied"})
        return ok

    if baseline["status"] != 200 or not baseline["marker"]:
        record("direct baseline executes the injected PHP payload", False)
        return "inconclusive", "direct application vulnerability was not reproduced", checks
    record("direct baseline executes the injected PHP payload", True)
    if not all(
        c["status"] == controls["benign_status"]
        and len(observations(records, c["token"])) == 1
        and observations(records, c["token"])[0].get("status")
        == controls["benign_status"]
        for c in (before, after)
    ):
        record("benign controls corroborate origin reachability", False)
        return "inconclusive", "benign controls did not corroborate origin reachability", checks
    record("benign controls corroborate origin reachability", True)
    extras_ok_before, extras_tokens_before = _extras_phase(pack, before, records, checks)
    extras_ok_after, extras_tokens_after = _extras_phase(pack, after, records, checks)
    if not (extras_ok_before and extras_ok_after):
        return (
            "inconclusive",
            "a data-driven extra case did not match its expected status",
            checks,
        )
    if not all(
        c["root_status"] == controls["ordinary_status"]
        and len(observations(records, c["root_token"])) == 1
        and observations(records, c["root_token"])[0].get("status")
        == controls["ordinary_status"]
        for c in (before, after)
    ):
        record(
            "ordinary requests match the application response",
            False,
        )
        return (
            "inconclusive",
            "ordinary requests did not match the application response",
            checks,
        )
    record(
        "ordinary requests match the application response",
        True,
    )
    expected_tokens = (
        {target["token"]}
        | extras_tokens_before
        | extras_tokens_after
        | {
            c[key] for c in (before, after) for key in ("token", "root_token")
        }
    )
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
                "check": "exploit request was blocked at the target",
                "outcome": "not satisfied, reached the origin with status "
                f"{seen[0].get('status')}",
            }
        )
        return "fail", "exploit request reached the origin; blocking assertion failed", checks
    record("exploit request was blocked at the target", True)
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


def classify_modphp_segfault(
    pack, baseline, target, before, after, records, block_status
):
    """Return (verdict, reason, checks) for one mod_php target run.

    Serves both PHP2Shell 2026 packs (single-request OOM and same-worker
    sequence): pass requires a reproduced direct baseline (a new
    segmentation-fault log entry after the reviewed trigger), a healthy
    before-control, fully correlated origin traffic, and a blocked exploit
    request; any origin hit for the exploit request fails the run. The
    after-control is evaluated only on the blocked path: when the exploit
    reaches the origin it crashes an Apache child, so a dead control after
    the exploit corroborates the failure instead of masking it.
    """
    checks = []
    controls = pack["controls"]

    def record(check, ok):
        checks.append({"check": check, "outcome": "ok" if ok else "not satisfied"})
        return ok

    if baseline.get("status") != 200 or not baseline.get("marker") or not baseline.get("crash_confirmed"):
        record("direct baseline corrupts request lifecycle state", False)
        return "inconclusive", "direct application vulnerability was not reproduced", checks
    record("direct baseline corrupts request lifecycle state", True)
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



def classify_data_driven(pack, baseline, target, before, after, records, block_status):
    """Return (verdict, reason, checks) using only pack-declared data.

    This is the generic evidence path for packs whose outcome is a plain
    response status: the signature request must be blocked at the target
    while every control (the two built-in ones and any declared extra cases)
    is answered with the status the pack declares. A pack whose evidence
    model fits this shape needs no classifier code: it names
    ``data-driven`` for both its ``baseline_gate`` and ``classifier``.
    """
    checks = []
    controls = pack["controls"]

    def record(check, ok):
        checks.append({"check": check, "outcome": "ok" if ok else "not satisfied"})
        return ok

    if baseline.get("status") != 200 or not baseline.get("marker"):
        record("direct baseline reproduces the vulnerability", False)
        return "inconclusive", "direct application vulnerability was not reproduced", checks
    record("direct baseline reproduces the vulnerability", True)
    if not all(
        c.get("status") == controls["benign_status"]
        and len(observations(records, c.get("token"))) == 1
        and observations(records, c.get("token"))[0].get("status") == controls["benign_status"]
        and c.get("root_status") == controls["ordinary_status"]
        and len(observations(records, c.get("root_token"))) == 1
        and observations(records, c.get("root_token"))[0].get("status") == controls["ordinary_status"]
        for c in (before,)
    ):
        record("benign controls corroborate origin reachability", False)
        return "inconclusive", "benign controls did not corroborate origin reachability", checks
    record("benign controls corroborate origin reachability", True)
    extras_ok_before, extras_tokens = _extras_phase(pack, before, records, checks)
    if not extras_ok_before:
        return "inconclusive", "a data-driven extra case did not match its expected status", checks
    phase_tokens = set()
    for phase in (before, after):
        phase_tokens |= {phase[key] for key in ("token", "root_token") if key in phase}
        for seen_case in (phase.get("extras") or {}).values():
            if seen_case.get("token"):
                phase_tokens.add(seen_case["token"])
    expected_tokens = {target["token"]} | extras_tokens | phase_tokens
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
    extras_ok_after, _tokens = _extras_phase(pack, after, records, checks)
    if not extras_ok_after:
        return "inconclusive", "a data-driven extra case did not match its expected status", checks
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
    "cgi-argument-injection": classify_php_cgi_argument_injection,
    "modphp-segfault": classify_modphp_segfault,
    "data-driven": classify_data_driven,
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
