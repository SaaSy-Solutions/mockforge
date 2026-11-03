"""Managed CVE lab evidence operations."""

from cve_lab_transport import observations


def classify(baseline, target, before, after, records, block_status):
    if baseline["status"] != 200 or not baseline["dashboard"]:
        return "inconclusive", "direct application vulnerability was not reproduced"
    if not all(
        c["status"] == 200
        and len(observations(records, c["token"])) == 1
        and observations(records, c["token"])[0].get("status") == 200
        for c in (before, after)
    ):
        return "inconclusive", "benign controls did not corroborate origin reachability"
    if not all(
        c["root_status"] == 307
        and len(observations(records, c["root_token"])) == 1
        and observations(records, c["root_token"])[0].get("status") == 307
        for c in (before, after)
    ):
        return (
            "inconclusive",
            "ordinary requests to the protected route did not match the application redirect",
        )
    expected_tokens = {target["token"]} | {
        c[key] for c in (before, after) for key in ("token", "root_token")
    }
    if any(not expected_tokens.intersection(r["tokens"]) for r in records):
        return (
            "inconclusive",
            "uncorrelated origin traffic prevents a blocking assertion",
        )
    seen = observations(records, target["token"])
    if seen:
        return "fail", "signature request reached the origin; blocking assertion failed"
    if target["status"] != block_status:
        return "inconclusive", "response did not match the configured block status"
    return (
        "pass",
        "expected rejection with no correlated origin request and healthy controls",
    )
