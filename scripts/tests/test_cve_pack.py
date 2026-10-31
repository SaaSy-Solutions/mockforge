"""cve_pack: profile safety schema, server mock shape, and rule-test verdicts
through a simulated load balancer. No vulnerable application or callbacks; the
'suspicious' request carries only an inert marker header.
"""

import importlib.util
import json
import sys
import tempfile
import threading
import unittest
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
spec = importlib.util.spec_from_file_location("cve_pack", ROOT / "scripts/cve_pack.py")
cve_pack = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cve_pack)
import cve_pack_profile as cpp  # noqa: E402
import cve_pack_server as cps  # noqa: E402

EXAMPLE = ROOT / "examples/cve-packs/CVE-2025-29927/profile.json"
SIG_HEADER = "x-middleware-subrequest"


def good():
    return json.loads(EXAMPLE.read_text())


@contextmanager
def proxy(origin, mode="filter"):
    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            suspicious = self.headers.get(SIG_HEADER) is not None
            blocked = mode == "block-all" or (mode == "filter" and suspicious)
            if mode == "drop":
                self.connection.close()
                return
            if not blocked:
                cve_pack.request(
                    cve_pack.target_url(f"http://127.0.0.1:{origin}"),
                    self.path,
                    dict(self.headers),
                    1,
                )
            self.send_response(403 if blocked or mode == "forward-deny" else 200)
            self.send_header("Content-Length", "0")
            self.end_headers()

        def log_message(self, *args):
            pass

    server = HTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_port}"
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


class ProfileSafetyTests(unittest.TestCase):
    def test_example_is_valid_and_normalizes(self):
        p = cpp.validate(good())
        self.assertEqual(p["cve"], "CVE-2025-29927")
        self.assertEqual(p["expect"], {"status": 403, "origin": "absent"})
        self.assertEqual(p["signature"]["headers"][SIG_HEADER].count("middleware"), 5)

    def test_marker_values_reject_every_payload_shape(self):
        # Each entry carries a metacharacter the marker charset forbids, so no
        # command, template, markup, traversal or quoted payload can be encoded.
        payloads = [
            "a b",  # space
            "a;b",  # command separator
            "a<b>c",  # markup angle brackets
            "a${b}c",  # template expression
            "a/b",  # slash (path/traversal)
            "a$(b)c",  # command substitution
            "a|b",  # pipe
            "a&b=c",  # ampersand / query joiner
            "a%2fb",  # percent-encoding
            'a"b',  # double quote
            "a\\b",  # backslash
            "x" * 129,  # too long
        ]
        for bad in payloads:
            prof = good()
            prof["signature"]["headers"] = {SIG_HEADER: bad}
            with self.subTest(payload=bad):
                with self.assertRaises(cpp.ProfileError):
                    cpp.validate(prof)

    def test_no_body_or_extra_keys_anywhere(self):
        prof = good()
        prof["signature"]["body"] = "anything"
        with self.assertRaises(cpp.ProfileError):
            cpp.validate(prof)
        prof = good()
        prof["payload"] = "x"
        with self.assertRaises(cpp.ProfileError):
            cpp.validate(prof)

    def test_method_and_status_and_origin_bounds(self):
        prof = good()
        prof["signature"]["method"] = "POST"
        with self.assertRaises(cpp.ProfileError):
            cpp.validate(prof)
        prof = good()
        prof["expect"] = {"blocked_status": 200, "origin": "absent"}
        with self.assertRaises(cpp.ProfileError):
            cpp.validate(prof)
        prof = good()
        prof["expect"] = {"blocked_status": 403, "origin": "seen"}
        with self.assertRaises(cpp.ProfileError):
            cpp.validate(prof)
        prof = good()
        prof["controls"][0]["expect"] = {"status": 403, "origin": "seen"}
        with self.assertRaises(cpp.ProfileError):
            cpp.validate(prof)

    def test_route_and_reserved_names(self):
        for route in ["dashboard", "/a/../b", "/a b", "/a?x=1", "http://x/y"]:
            prof = good()
            prof["route"] = route
            with self.subTest(route=route):
                with self.assertRaises(cpp.ProfileError):
                    cpp.validate(prof)
        prof = good()
        prof["signature"]["headers"] = {cpp.CORRELATION_HEADER: "marker"}
        with self.assertRaises(cpp.ProfileError):
            cpp.validate(prof)

    def test_signature_needs_a_token(self):
        prof = good()
        prof["signature"] = {"method": "GET", "query": {}, "headers": {}}
        with self.assertRaises(cpp.ProfileError):
            cpp.validate(prof)


class ServerMockTests(unittest.TestCase):
    def test_openapi_mock_has_route_and_no_payload(self):
        doc = cps.to_openapi(good())
        self.assertIn("/dashboard", doc["paths"])
        self.assertIn("get", doc["paths"]["/dashboard"])
        text = json.dumps(doc)
        self.assertNotIn("middleware:middleware", text)
        self.assertIn("no vulnerable behavior", doc["info"]["description"])


class RunTests(unittest.TestCase):
    def exercise(self, mode, profile=None):
        prof = cpp.validate(profile or good())
        with cve_pack.recorder() as (port, observations, lock):
            with proxy(port, mode) as target:
                return cve_pack.run(
                    prof,
                    cve_pack.target_url(target),
                    observations,
                    lock,
                    timeout=0.2,
                    window=0.01,
                )

    def test_filter_blocks_signature_allows_controls(self):
        report = self.exercise("filter")
        by_id = {r["id"]: r for r in report["results"]}
        self.assertEqual(by_id["CVE-2025-29927"]["rule_behavior"], "pass")
        self.assertFalse(by_id["CVE-2025-29927"]["origin_seen"])
        self.assertEqual(by_id["CVE-2025-29927"]["status"], 403)
        self.assertEqual(by_id["benign"]["rule_behavior"], "pass")
        self.assertEqual(by_id["benign-note"]["rule_behavior"], "pass")
        self.assertTrue(
            all(r["runtime_patch_evidence"] == "unverified" for r in report["results"])
        )

    def test_allow_all_forward_fails_the_signature(self):
        report = self.exercise("allow")
        sig = next(r for r in report["results"] if r["kind"] == "signature")
        self.assertEqual(sig["rule_behavior"], "fail")
        self.assertTrue(sig["origin_seen"])

    def test_denied_but_forwarded_is_a_fail(self):
        # Rule returns the blocked status yet still forwarded to origin: the
        # verdict must fail even with healthy flanking controls.
        from cve_pack_evidence import verdict

        v = verdict(
            {"status": 403, "origin": "absent"},
            403,
            None,
            [{"path": "/dashboard?x", "headers": []}],
            True,
            True,
        )
        self.assertEqual(v["rule_behavior"], "fail")

    def test_transport_errors_and_blanket_deny_never_pass(self):
        for mode in ("drop", "block-all", "forward-deny"):
            with self.subTest(mode=mode):
                report = self.exercise(mode)
                self.assertFalse(
                    any(r["rule_behavior"] == "pass" for r in report["results"])
                )


class CliTests(unittest.TestCase):
    def test_unfinished_template_rejected_by_server_run_probe(self):
        with tempfile.TemporaryDirectory() as d:
            out = Path(d) / "profile.json"
            cve_pack.main(["new", "CVE-2026-109618", "--output", str(out)])
            for command in ("server", "run", "probe"):
                args = [command, str(out)]
                if command != "server":
                    args += [
                        "--target",
                        "http://127.0.0.1:1",
                        "--report",
                        str(Path(d) / "report.json"),
                    ]
                with self.assertRaises(SystemExit) as raised:
                    cve_pack.main(args)
                self.assertEqual(raised.exception.code, 2)
            self.assertFalse((Path(d) / "report.json").exists())

    def test_placeholder_signature_cannot_be_used_with_completed_metadata(self):
        profile = dict(
            cve_pack.SKELETON,
            cve="CVE-2026-109618",
            title="Reviewed title",
            references=["https://example.com/advisory"],
            route="/test",
        )
        with self.assertRaisesRegex(cpp.ProfileError, "unfinished template"):
            cpp.validate(profile)

    def test_new_writes_skeleton(self):
        with tempfile.TemporaryDirectory() as d:
            out = Path(d) / "p.json"
            self.assertEqual(
                cve_pack.main(["new", "CVE-2026-12345", "--output", str(out)]), 0
            )
            skeleton = json.loads(out.read_text())
            self.assertEqual(skeleton["cve"], "CVE-2026-12345")
            self.assertEqual(skeleton["schema_version"], cpp.SCHEMA_VERSION)

    def test_server_prints_mock(self):
        self.assertEqual(cve_pack.main(["server", str(EXAMPLE)]), 0)

    def test_run_exit_codes(self):
        # main() opens its own recorder on --origin-port; point the proxy there.
        origin = 18099
        with tempfile.TemporaryDirectory() as d:
            report = Path(d) / "r.json"
            with proxy(origin, "filter") as target:
                rc = cve_pack.main(
                    [
                        "run",
                        str(EXAMPLE),
                        "--target",
                        target,
                        "--report",
                        str(report),
                        "--origin-port",
                        str(origin),
                        "--timeout",
                        "0.2",
                        "--observation-window",
                        "0.01",
                    ]
                )
            self.assertEqual(rc, 0)
            self.assertTrue(json.loads(report.read_text())["results"])


@contextmanager
def waf(blocks_signature, always=None):
    """A front end that 403s the signature header when blocks_signature, else
    200 — or always returns `always`. No origin forwarding."""

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            if always is not None:
                code = always
            else:
                suspicious = self.headers.get(SIG_HEADER) is not None
                code = 403 if (blocks_signature and suspicious) else 200
            self.send_response(code)
            self.send_header("Content-Length", "0")
            self.end_headers()

        def log_message(self, *args):
            pass

    server = HTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_port}"
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


class ProbeTests(unittest.TestCase):
    def test_probe_reports_blocked_without_origin_correlation(self):
        prof = cpp.validate(good())
        with waf(blocks_signature=True) as target:
            report = cve_pack.probe(prof, cve_pack.target_url(target), timeout=1)
        self.assertEqual(report["mode"], "probe")
        sig = next(r for r in report["results"] if r["kind"] == "signature")
        self.assertEqual(sig["front_end"], "blocked")
        self.assertTrue(
            all(
                r["front_end"] == "allowed"
                for r in report["results"]
                if r["kind"] == "control"
            )
        )

    def test_probe_not_blocked_when_front_end_allows(self):
        prof = cpp.validate(good())
        with waf(blocks_signature=False) as target:
            report = cve_pack.probe(prof, cve_pack.target_url(target), timeout=1)
        sig = next(r for r in report["results"] if r["kind"] == "signature")
        self.assertEqual(sig["front_end"], "not-blocked")

    def test_probe_404_is_not_mistaken_for_a_block(self):
        prof = cpp.validate(good())
        with waf(blocks_signature=False, always=404) as target:
            report = cve_pack.probe(prof, cve_pack.target_url(target), timeout=1)
        sig = next(r for r in report["results"] if r["kind"] == "signature")
        # benign also 404 -> no baseline -> inconclusive, never "blocked"
        self.assertEqual(sig["front_end"], "inconclusive")


if __name__ == "__main__":
    unittest.main(verbosity=1)
