"""Correlate receiver observations and conservatively classify rule evidence."""

import http.client
import time
import uuid
from security_rule_pack_config import CASES


def observe(name, *, run_id, url, observations, lock, timeout, window, send):
    token = run_id + "-" + uuid.uuid4().hex
    path, headers = CASES[name]
    path += ("&" if "?" in path else "?") + "__mockforge_correlation=" + token
    headers = {**headers, "X-Mockforge-Correlation": token}
    try:
        status = send(url, path, headers, timeout)
        error = None
    except (OSError, http.client.HTTPException) as exc:
        status, error = None, type(exc).__name__
    time.sleep(window)
    with lock:
        records = list(observations.get(token, []))
    # Require exact request path, one observation, and matching probe headers.
    valid = len(records) == 1 and records[0]["path"] == path
    if valid:
        valid = all(
            [value for key, value in records[0]["headers"] if key.lower() == k.lower()]
            == [v]
            for k, v in headers.items()
        )
    return status, error, records, valid, token


def classify(case, status, error, records, valid, healthy):
    """Decide only the evidence supported verdict; absent data never proves blocking."""
    result = {"rule_behavior": "inconclusive"}
    if error or (records and not valid):
        result["reason"] = error or "origin correlation unavailable"
    elif status != case["status"]:
        result.update(rule_behavior="fail", reason="explicit status assertion mismatch")
    elif not healthy:
        result["reason"] = "flanking controls could not corroborate origin reachability"
    elif case["origin"] == "seen" and not records:
        result["reason"] = "expected origin observation missing"
    elif case["origin"] == "absent" and records:
        result.update(
            rule_behavior="fail", reason="rejected response still reached origin"
        )
    else:
        result.update(
            rule_behavior="pass", reason="status and bounded origin assertion agree"
        )
    return result
