"""Per-run log of every request the managed CVE lab sends.

``run()`` opens ``<output>/sent_requests.jsonl`` as soon as the output
directory exists and closes it when the run ends. Every lab send site then
appends one JSON line per request it puts on the wire: the direct baseline
against the lab's own origin and the ``--target`` phase through the LB under
test. Lab internals - the recorder forwarding what arrived to the lab
application, and engine readiness polls - are not sent requests and are not
logged (they call the transport with ``log_sent=False``). The log pairs
with ``report.json``'s ``origin_observations`` (what arrived at the
origin): joining the two on ``marker`` shows which sent requests were
blocked and which got through.

When no sink is open - unit tests and the other subcommands - recording is
a no-op, and a write failure never breaks a run.
"""

import hashlib
import json
import threading
from datetime import datetime, timezone
from urllib.parse import parse_qs, urlsplit

_lock = threading.Lock()
_sink = None
_seq = 0
_origin = None
_target = None

REDACTED = "[redacted]"
_REDACT_HEADERS = frozenset(
    ("authorization", "proxy-authorization", "cookie", "set-cookie")
)
_DEFAULT_PORTS = {"http": 80, "https": 443}


def base_url(scheme, host, port):
    """``scheme://host:port`` for a send site that holds the parts."""
    return f"{scheme}://{host}:{port}" if port is not None else f"{scheme}://{host}"


def _endpoint(base):
    """(scheme, host, port) for destination matching; an absent port means
    the scheme's default."""
    url = urlsplit(base)
    return (
        url.scheme,
        (url.hostname or "").lower(),
        url.port if url.port is not None else _DEFAULT_PORTS.get(url.scheme),
    )


def _close_locked():
    global _sink
    if _sink is not None:
        try:
            _sink.close()
        except OSError:
            pass
        _sink = None


def open_sink(path, origin=None, target=None):
    """Open the run's sent-request log and configure destination matching.

    ``origin`` is the recorder endpoint the direct baseline is sent to
    (``http://<direct-host>:<origin-port>``); ``target`` is ``--target``.
    A request whose scheme/host/port matches neither logs ``other``. An
    open failure leaves the sink closed: a run without a log must still run.
    """
    global _sink, _seq, _origin, _target
    with _lock:
        _close_locked()
        _seq = 0
        _origin = _endpoint(origin) if origin else None
        _target = _endpoint(target) if target else None
        try:
            _sink = open(path, "w", encoding="utf-8")
        except OSError:
            _sink = None


def close_sink():
    with _lock:
        _close_locked()


def _destination(base):
    try:
        endpoint = _endpoint(base)
    except ValueError:
        return "other"
    if _origin is not None and endpoint == _origin:
        return "origin"
    if _target is not None and endpoint == _target:
        return "target"
    return "other"


def _marker(path, headers):
    """The run's correlation token: the ``__mockforge_cve`` query value or
    the ``X-Mockforge-Cve`` header value, whichever is present."""
    token = parse_qs(urlsplit(path).query).get("__mockforge_cve", [None])[0]
    if token:
        return token
    for name, value in headers.items():
        if name.lower() == "x-mockforge-cve":
            return value
    return None


def record(
    *,
    base,
    method,
    path,
    headers=None,
    body=None,
    status=None,
    error=None,
    elapsed_ms=None,
    client="python-http1",
):
    """Append one sent-request line; a no-op when no sink is open."""
    global _seq
    with _lock:
        if _sink is None:
            return
    if isinstance(body, str):
        body = body.encode("utf-8")
    header_map = {
        name: REDACTED if name.lower() in _REDACT_HEADERS else value
        for name, value in dict(headers or {}).items()
    }
    entry = {
        "ts": datetime.now(timezone.utc).isoformat(timespec="milliseconds"),
        "destination": _destination(base),
        "base": base,
        "method": method,
        "path": path,
        "headers": header_map,
        "marker": _marker(path, header_map),
        "body_bytes": len(body) if body else 0,
        "body_sha256": hashlib.sha256(body).hexdigest() if body else None,
        "body_preview": body[:256].decode("utf-8", errors="replace") if body else "",
        "status": status,
        "error": error,
        "elapsed_ms": round(elapsed_ms, 3) if elapsed_ms is not None else None,
        "client": client,
    }
    with _lock:
        if _sink is None:
            return
        _seq += 1
        entry["seq"] = _seq
        try:
            _sink.write(json.dumps(entry, ensure_ascii=False) + "\n")
            _sink.flush()
        except (OSError, ValueError):
            pass
