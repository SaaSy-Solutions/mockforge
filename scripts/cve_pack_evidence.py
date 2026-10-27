"""Send one profile case through the supplied load balancer and correlate it
at the owned recorder. Reuses the vetted transport and classifier from the
Security Rule Test Packs (#1404); the only generalization is that the case's
path and headers come from a declarative profile instead of a fixed table.
"""

import http.client
import time
import uuid

from cve_pack_profile import CORRELATION_HEADER, CORRELATION_KEY
from security_rule_pack_evidence import classify  # reused unchanged
from security_rule_pack_transport import request  # bounded GET, no retries


def observe(
    path, headers, *, run_id, url, observations, lock, timeout, window, send=request
):
    """Send `path`+`headers` (plus a one-shot correlation token) through `url`
    and report (status, error, records, valid, token). `valid` means exactly
    one correlated observation arrived with the intended path and markers."""
    token = run_id + "-" + uuid.uuid4().hex
    sep = "&" if "?" in path else "?"
    sent_path = path + sep + CORRELATION_KEY + "=" + token
    sent_headers = {**headers, CORRELATION_HEADER: token}
    try:
        status = send(url, sent_path, sent_headers, timeout)
        error = None
    except (OSError, http.client.HTTPException) as exc:
        status, error = None, type(exc).__name__
    time.sleep(window)
    with lock:
        records = list(observations.get(token, []))
    valid = len(records) == 1 and records[0]["path"] == sent_path
    if valid:
        valid = all(
            [v for k, v in records[0]["headers"] if k.lower() == name.lower()]
            == [value]
            for name, value in sent_headers.items()
        )
    return status, error, records, valid, token


def verdict(expect, status, error, records, valid, healthy):
    """Thin adapter: classify() expects a case dict of {status, origin}."""
    return classify(
        {"status": expect["status"], "origin": expect["origin"]},
        status,
        error,
        records,
        valid,
        healthy,
    )
