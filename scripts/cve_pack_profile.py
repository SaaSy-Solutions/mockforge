"""Declarative CVE rule-test profile: strict schema that admits only inert
markers, so a profile can describe a WAF/LB rule's signature but can never
carry a working payload.

A profile names the route a rule guards and the signature tokens the rule
matches on (header and query-parameter *names*, with harmless marker
*values*), plus benign controls and the expected rule outcome. It is consumed
by cve_pack.py to drive the same canary -> supplied-LB -> owned-recorder flow
that security_rule_test_pack.py uses, and by cve_pack_server.py to emit a
MockForge mock of the guarded route.

Safety is structural, not advisory:
- values are constrained to `[A-Za-z0-9_.:-]`, so they cannot contain spaces,
  quotes, slashes, angle brackets, `;`, `|`, `&`, `$`, `{`, `}`, `(`, `)`,
  `=`, `%` or backslashes. Command/template/markup/traversal payloads cannot be
  expressed.
- there is no request-body field anywhere; the schema rejects unknown keys.
- only GET is allowed, mirroring the bounded transport.
"""

import json
import re
from pathlib import Path

SCHEMA_VERSION = 1
CVE_ID = re.compile(r"^CVE-\d{4}-\d{4,}$")
ROUTE = re.compile(r"^/[A-Za-z0-9/_.\-]{0,200}$")
HEADER_NAME = re.compile(r"^[A-Za-z0-9\-]{1,64}$")
QUERY_KEY = re.compile(r"^[A-Za-z0-9_.\-]{1,64}$")
CONTROL_ID = re.compile(r"^[a-z0-9][a-z0-9\-]{0,39}$")
# The one structural guarantee: marker values are inert tokens, never payloads.
MARKER = re.compile(r"^[A-Za-z0-9_.:\-]{1,128}$")
MAX_CONTROLS = 5
MAX_TOKENS = 8
CORRELATION_KEY = "__mockforge_correlation"
CORRELATION_HEADER = "X-Mockforge-Correlation"


class ProfileError(ValueError):
    """A profile that is malformed or would carry anything but inert markers."""


def _tokens(value, kind, keyre):
    if not isinstance(value, dict):
        raise ProfileError(f"{kind} must be an object of name -> marker value")
    if len(value) > MAX_TOKENS:
        raise ProfileError(f"{kind} allows at most {MAX_TOKENS} entries")
    out = {}
    for key, marker in value.items():
        if not isinstance(key, str) or not keyre.match(key):
            raise ProfileError(f"{kind} name {key!r} is not an allowed identifier")
        if key.lower() in (CORRELATION_KEY, CORRELATION_HEADER.lower()):
            raise ProfileError(f"{kind} name {key!r} is reserved for correlation")
        if not isinstance(marker, str) or not MARKER.match(marker):
            raise ProfileError(
                f"{kind} value for {key!r} must be an inert marker matching "
                f"[A-Za-z0-9_.:-]{{1,128}} (no payloads, spaces or metacharacters)"
            )
        out[key] = marker
    return out


def _signature(sig):
    if not isinstance(sig, dict) or set(sig) - {"method", "query", "headers"}:
        raise ProfileError("signature allows only method, query and headers")
    if sig.get("method", "GET") != "GET":
        raise ProfileError("only GET signatures are supported")
    query = _tokens(sig.get("query", {}), "signature.query", QUERY_KEY)
    headers = _tokens(sig.get("headers", {}), "signature.headers", HEADER_NAME)
    if not query and not headers:
        raise ProfileError("signature needs at least one query or header token")
    return {"method": "GET", "query": query, "headers": headers}


def _expect(expect, *, blocked):
    allowed = {"blocked_status", "origin"} if blocked else {"status", "origin"}
    if not isinstance(expect, dict) or set(expect) != allowed:
        raise ProfileError(f"expect requires exactly {sorted(allowed)}")
    status = expect["blocked_status"] if blocked else expect["status"]
    if type(status) is not int:
        raise ProfileError("expect status must be an integer")
    if blocked:
        if not 400 <= status <= 599:
            raise ProfileError("a blocked_status must be 400..599")
        if expect["origin"] != "absent":
            raise ProfileError("a blocked signature must expect origin 'absent'")
    else:
        if not 200 <= status <= 399:
            raise ProfileError("a control status must be 200..399")
        if expect["origin"] != "seen":
            raise ProfileError("a benign control must expect origin 'seen'")
    return {"status": status, "origin": expect["origin"]}


def _control(control):
    if not isinstance(control, dict) or set(control) - {
        "id",
        "query",
        "headers",
        "expect",
    }:
        raise ProfileError("control allows only id, query, headers and expect")
    cid = control.get("id")
    if not isinstance(cid, str) or not CONTROL_ID.match(cid):
        raise ProfileError("control id must be a short lower-case slug")
    return {
        "id": cid,
        "query": _tokens(control.get("query", {}), "control.query", QUERY_KEY),
        "headers": _tokens(control.get("headers", {}), "control.headers", HEADER_NAME),
        "expect": _expect(control.get("expect"), blocked=False),
    }


def validate(profile):
    """Return a normalized profile or raise ProfileError. Pure, no I/O."""
    if not isinstance(profile, dict):
        raise ProfileError("profile must be a JSON object")
    allowed = {
        "schema_version",
        "cve",
        "title",
        "references",
        "route",
        "signature",
        "expect",
        "controls",
    }
    if set(profile) != allowed:
        raise ProfileError(f"profile requires exactly {sorted(allowed)}")
    if profile["schema_version"] != SCHEMA_VERSION:
        raise ProfileError("unsupported schema_version")
    if not isinstance(profile["cve"], str) or not CVE_ID.match(profile["cve"]):
        raise ProfileError("cve must look like CVE-YYYY-NNNN")
    if not isinstance(profile["title"], str) or not 1 <= len(profile["title"]) <= 200:
        raise ProfileError("title must be 1..200 characters")
    refs = profile["references"]
    if not isinstance(refs, list) or not 1 <= len(refs) <= 10:
        raise ProfileError("references must list 1..10 vendor/advisory URLs")
    for ref in refs:
        if not isinstance(ref, str) or not ref.startswith("https://"):
            raise ProfileError("each reference must be an https URL")
    if not isinstance(profile["route"], str) or not ROUTE.match(profile["route"]):
        raise ProfileError("route must be an absolute path with no query or '..'")
    if ".." in profile["route"]:
        raise ProfileError("route must not contain '..'")
    signature = _signature(profile["signature"])
    if (
        profile["title"] == "One-line description of the guarded weakness"
        or "https://vendor.example/advisory" in refs
        or profile["route"] == "/the/guarded/route"
        or "x-example-signature-header" in signature["headers"]
    ):
        raise ProfileError(
            "unfinished template: replace title, references, route and signature from the advisory; new does not simulate a CVE"
        )
    controls = profile["controls"]
    if not isinstance(controls, list) or not 1 <= len(controls) <= MAX_CONTROLS:
        raise ProfileError(f"controls must list 1..{MAX_CONTROLS} benign cases")
    seen = set()
    norm_controls = []
    for control in controls:
        c = _control(control)
        if c["id"] in seen:
            raise ProfileError("duplicate control id")
        seen.add(c["id"])
        norm_controls.append(c)
    return {
        "schema_version": SCHEMA_VERSION,
        "cve": profile["cve"],
        "title": profile["title"],
        "references": list(refs),
        "route": profile["route"],
        "signature": signature,
        "expect": _expect(profile["expect"], blocked=True),
        "controls": norm_controls,
    }


def load(path):
    return validate(json.loads(Path(path).read_text()))


def _case_path(route, query):
    if not query:
        return route
    pairs = "&".join(f"{k}={v}" for k, v in query.items())
    return route + ("&" if "?" in route else "?") + pairs


def cases(profile):
    """Yield (id, path, headers, expect, kind) for the signature then controls.

    kind is 'signature' or 'control'. Paths and headers hold only the profile's
    inert markers; cve_pack.py adds the correlation token at send time.
    """
    sig = profile["signature"]
    yield (
        profile["cve"],
        _case_path(profile["route"], sig["query"]),
        dict(sig["headers"]),
        profile["expect"],
        "signature",
    )
    for control in profile["controls"]:
        yield (
            control["id"],
            _case_path(profile["route"], control["query"]),
            dict(control["headers"]),
            control["expect"],
            "control",
        )
