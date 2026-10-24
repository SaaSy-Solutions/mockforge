"""Bounded, harmless HTTP security-rule canaries; Python 3.10+, standard library."""

import argparse
import http.client
import ipaddress
import json
import threading
import time
import uuid
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlsplit


CASES = {
    "normal-control": ("/security-rule-probe", {}),
    "nextjs-benign": ("/security-rule-probe", {"X-Client-Note": "middleware"}),
    "nextjs-header": (
        "/security-rule-probe",
        {"x-middleware-subrequest": "middleware:" * 4 + "middleware"},
    ),
    "camel-benign": ("/security-rule-probe?beanMethodName=mockforgeCanary", {}),
    "camel-parameter": ("/security-rule-probe?cAmelBeanMethodName=mockforgeCanary", {}),
    "camel-header": (
        "/security-rule-probe",
        {"cAmelBeanMethodName": "mockforgeCanary"},
    ),
}
CONTROLS = {"normal-control", "nextjs-benign", "camel-benign"}


def load_pack(path):
    pack = json.loads(Path(path).read_text())
    if not isinstance(pack, dict) or set(pack) != {"schema_version", "cases"}:
        raise ValueError("pack requires only schema_version and cases")
    if type(pack["schema_version"]) is not int or pack["schema_version"] != 1:
        raise ValueError("unsupported schema_version")
    cases = pack["cases"]
    if not isinstance(cases, list) or not 5 <= len(cases) <= len(CASES):
        raise ValueError("pack requires five supported cases and optional camel-header")
    seen = set()
    for case in cases:
        if not isinstance(case, dict) or set(case) != {"id", "status", "origin"}:
            raise ValueError("case requires only id, status and origin")
        name, status, origin = case["id"], case["status"], case["origin"]
        if not isinstance(name, str) or name not in CASES or name in seen:
            raise ValueError("unknown or duplicate case id")
        seen.add(name)
        if type(status) is not int or not 200 <= status <= 599:
            raise ValueError("status must be an integer from 200 to 599")
        if origin not in ("seen", "absent"):
            raise ValueError("origin must be seen or absent")
        if name in CONTROLS and (status != 200 or origin != "seen"):
            raise ValueError("benign controls require 200 and origin seen")
        if origin == "absent" and status < 400:
            raise ValueError("rejection requires an explicit error status")
    if not set(CASES) - {"camel-header"} <= seen:
        raise ValueError("all supported probes and benign controls are required")
    return cases


def target_url(value, allow_external=False):
    url = urlsplit(value)
    if (
        url.scheme not in ("http", "https")
        or not url.hostname
        or url.username
        or url.password
        or url.path not in ("", "/")
        or url.query
        or url.fragment
    ):
        raise ValueError("target must be an HTTP(S) origin without credentials or path")
    # Loopback literals avoid DNS rebinding and accidental discovered-host probing.
    try:
        loopback = ipaddress.ip_address(url.hostname).is_loopback
    except ValueError:
        loopback = False
    if not loopback and not allow_external:
        raise ValueError("non-loopback targets require --allow-external-target")
    if url.port is not None and url.port == 0:
        raise ValueError("target port must be 1..65535")
    return url


@contextmanager
def recorder(port=0):
    """Own synchronous recorder; raw header pairs retain receiver representation."""
    observations = {}
    lock = threading.Lock()

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            query_token = parse_qs(urlsplit(self.path).query).get(
                "__mockforge_correlation", [None]
            )[0]
            header_token = self.headers.get("X-Mockforge-Correlation")
            # Observe either surviving token so removal of one cannot fake absence.
            for token in {query_token, header_token}:
                if token and len(token) <= 100:
                    with lock:
                        observations.setdefault(token, []).append(
                            {
                                "path": self.path,
                                "headers": list(self.headers.raw_items()),
                            }
                        )
            body = b'{"receiver":"mockforge-security-rule-recorder"}'
            self.send_response(200)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *args):
            pass

    class BoundedServer(HTTPServer):
        def get_request(self):
            connection, address = super().get_request()
            connection.settimeout(2)
            return connection, address

    server = BoundedServer(("127.0.0.1", port), Handler)
    server.timeout = 1
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield server.server_address[1], observations, lock
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2)


def request(url, path, headers, timeout):
    cls = (
        http.client.HTTPSConnection
        if url.scheme == "https"
        else http.client.HTTPConnection
    )
    connection = cls(url.hostname, url.port, timeout=timeout)
    try:
        connection.request("GET", path, headers=headers)
        response = connection.getresponse()
        # Bound response consumption; no retries or redirects.
        response.read(4096)
        return response.status
    finally:
        connection.close()


def run(cases, url, observations, lock, timeout=2, window=0.1):
    results = []
    run_id = uuid.uuid4().hex

    def deliver(name):
        token = run_id + "-" + uuid.uuid4().hex
        path, headers = CASES[name]
        path += ("&" if "?" in path else "?") + "__mockforge_correlation=" + token
        headers = {**headers, "X-Mockforge-Correlation": token}
        try:
            status = request(url, path, headers, timeout)
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
                [
                    value
                    for key, value in records[0]["headers"]
                    if key.lower() == k.lower()
                ]
                == [v]
                for k, v in headers.items()
            )
        return status, error, records, valid, token

    for case in cases:
        name = case["id"]
        result = {
            "id": name,
            "expected": case,
            "status": None,
            "origin_seen": None,
            "scenario_delivery": "unverified",
            "rule_behavior": "inconclusive",
            "runtime_patch_evidence": "unverified",
            "origin_observations": [],
        }
        if name == "camel-header":
            result.update(
                rule_behavior="unsupported",
                reason="mixed-case wire fidelity not verified",
            )
            results.append(result)
            continue
        # Corroborate origin reachability on both sides of every case.
        before = deliver("normal-control")
        status, error, records, valid, token = deliver(name)
        after = deliver("normal-control")
        healthy = all(s == 200 and e is None and v for s, e, _, v, _ in (before, after))
        result.update(
            correlation_id=token,
            status=status,
            origin_seen=bool(records),
            origin_observations=records,
            scenario_delivery="origin-observed" if valid else "unverified",
            control_before={
                "status": before[0],
                "origin_valid": before[3],
                "correlation_id": before[4],
                "observations": before[2],
            },
            control_after={
                "status": after[0],
                "origin_valid": after[3],
                "correlation_id": after[4],
                "observations": after[2],
            },
        )
        if error or (records and not valid):
            result["reason"] = (
                error or "origin correlation or flanking control unavailable"
            )
        elif status != case["status"]:
            result.update(
                rule_behavior="fail", reason="explicit status assertion mismatch"
            )
        elif not healthy:
            result["reason"] = (
                "flanking controls could not corroborate origin reachability"
            )
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
        results.append(result)
    return {
        "schema_version": 1,
        "run_id": run_id,
        "target": url.geturl(),
        "timeout_seconds": timeout,
        "request_count": 3 * sum(c["id"] != "camel-header" for c in cases),
        "observation_window_seconds": window,
        "scope": "synthetic transport/rule behavior only; no application protection or patch proof",
        "results": results,
    }


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pack", required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--report", required=True)
    parser.add_argument(
        "--allow-external-target",
        action="store_true",
        help="affirm explicit authorization for this exact supplied target",
    )
    parser.add_argument("--origin-port", type=int, default=18081)
    parser.add_argument("--timeout", type=float, default=2)
    parser.add_argument("--observation-window", type=float, default=0.1)
    args = parser.parse_args(argv)
    try:
        cases = load_pack(args.pack)
        url = target_url(args.target, args.allow_external_target)
        if not 1 <= args.origin_port <= 65535 or not 0 < args.timeout <= 5:
            raise ValueError("origin port must be 1..65535; timeout must be >0 and <=5")
        if not 0.01 <= args.observation_window <= 1:
            raise ValueError("observation window must be 0.01..1 seconds")
        with recorder(args.origin_port) as (_, observations, lock):
            report = run(
                cases, url, observations, lock, args.timeout, args.observation_window
            )
        Path(args.report).write_text(json.dumps(report, indent=2) + "\n")
    except (ValueError, OSError) as exc:
        parser.error(str(exc))
    for result in report["results"]:
        print(f"{result['id']}: {result['rule_behavior']} ({result['reason']})")
    return 0 if all(r["rule_behavior"] == "pass" for r in report["results"]) else 1


if __name__ == "__main__":
    raise SystemExit(main())
