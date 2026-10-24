"""Real loopback proxy/recorder regressions; no vulnerable application or callbacks."""

import importlib.util
import json
import subprocess
import sys
import tempfile
import threading
import unittest
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/security_rule_test_pack.py"
spec = importlib.util.spec_from_file_location("pack", SCRIPT)
pack = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pack)
FIXTURE = ROOT / "examples/security-rule-packs/nextjs-camel.json"


@contextmanager
def proxy(origin, mode="filter"):
    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            suspicious = (
                self.headers.get("x-middleware-subrequest") or "cAmel" in self.path
            )
            blocked = mode == "block-all" or (mode == "filter" and suspicious)
            if mode == "drop":
                self.connection.close()
                return
            if not blocked:
                headers = dict(self.headers)
                if mode == "strip" or (mode == "selective-strip" and suspicious):
                    headers.pop("X-Mockforge-Correlation", None)
                pack.request(
                    pack.target_url(f"http://127.0.0.1:{origin}"), self.path, headers, 1
                )
            self.send_response(
                403
                if blocked
                or mode == "forward-deny"
                or (mode == "selective-strip" and suspicious)
                else 200
            )
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


class PackTests(unittest.TestCase):
    def exercise(self, mode, cases=None):
        with pack.recorder() as (port, observations, lock):
            with proxy(port, mode) as target:
                return pack.run(
                    cases or pack.load_pack(FIXTURE),
                    pack.target_url(target),
                    observations,
                    lock,
                    timeout=0.2,
                    window=0.01,
                )

    def test_allowed_denied_controls_and_wire_observations(self):
        report = self.exercise("filter")
        for result in report["results"]:
            expected = "unsupported" if result["id"] == "camel-header" else "pass"
            self.assertEqual(result["rule_behavior"], expected, result)
            self.assertEqual(result["runtime_patch_evidence"], "unverified")
        nextjs = report["results"][2]
        self.assertFalse(nextjs["origin_seen"])
        self.assertEqual(nextjs["status"], 403)
        self.assertEqual(report["results"][0]["scenario_delivery"], "origin-observed")

    def test_generic_2xx_is_not_a_pass(self):
        results = self.exercise("allow")["results"]
        for result in results:
            if (
                result["expected"]["origin"] == "absent"
                and result["id"] != "camel-header"
            ):
                self.assertEqual(result["rule_behavior"], "fail")

    def test_wrong_explicit_status_fails(self):
        cases = pack.load_pack(FIXTURE)
        cases[2]["status"] = 401
        self.assertEqual(
            self.exercise("filter", cases)["results"][2]["rule_behavior"], "fail"
        )

    def test_network_errors_missing_correlation_and_false_positives(self):
        for mode in ("drop", "strip", "block-all", "forward-deny"):
            with self.subTest(mode=mode):
                results = self.exercise(mode)["results"]
                self.assertFalse(any(r["rule_behavior"] == "pass" for r in results))

    def test_denied_status_with_origin_seen_fails(self):
        # Direct verdict regression with healthy flanking controls but forwarded probe.
        original = pack.request

        def forward_then_deny(url, path, headers, timeout):
            status = original(url, path, headers, timeout)
            return 403 if "x-middleware-subrequest" in headers else status

        pack.request = forward_then_deny
        try:
            with pack.recorder() as (port, observations, lock):
                result = pack.run(
                    pack.load_pack(FIXTURE),
                    pack.target_url(f"http://127.0.0.1:{port}"),
                    observations,
                    lock,
                    window=0.01,
                )["results"][2]
            self.assertEqual(result["rule_behavior"], "fail")
            self.assertTrue(result["origin_seen"])
        finally:
            pack.request = original

    def test_selective_correlation_loss_is_not_block_evidence(self):
        result = self.exercise("selective-strip")["results"][2]
        self.assertEqual(result["status"], 403)
        self.assertTrue(result["origin_seen"])
        self.assertEqual(result["rule_behavior"], "inconclusive")

    def test_duplicate_or_wrong_path_observations_are_inconclusive(self):
        original = pack.request

        def corrupt(url, path, headers, timeout):
            status = original(url, path, headers, timeout)
            token = headers["X-Mockforge-Correlation"]
            with lock:
                observations[token].append({"path": "/wrong", "headers": []})
            return status

        with pack.recorder() as (port, observations, lock):
            pack.request = corrupt
            try:
                report = pack.run(
                    pack.load_pack(FIXTURE),
                    pack.target_url(f"http://127.0.0.1:{port}"),
                    observations,
                    lock,
                    window=0.01,
                )
            finally:
                pack.request = original
        self.assertFalse(any(r["rule_behavior"] == "pass" for r in report["results"]))

    def test_strict_config_and_target_validation(self):
        baseline = json.loads(FIXTURE.read_text())
        mutations = [
            [],
            {},
            {**baseline, "extra": 1},
            {**baseline, "schema_version": True},
            {**baseline, "cases": baseline["cases"][1:]},
        ]
        for field, value in (
            ("status", True),
            ("status", 199),
            ("origin", "maybe"),
            ("id", "unknown"),
            ("headers", {}),
        ):
            changed = json.loads(FIXTURE.read_text())
            changed["cases"][0][field] = value
            mutations.append(changed)
        duplicate = json.loads(FIXTURE.read_text())
        duplicate["cases"][0] = duplicate["cases"][1]
        mutations.append(duplicate)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "pack.json"
            for invalid in mutations:
                path.write_text(json.dumps(invalid))
                with self.assertRaises(ValueError):
                    pack.load_pack(path)
        for target in (
            "http://example.com",
            "http://localhost",
            "http://user@127.0.0.1",
            "http://127.0.0.1/path",
            "ftp://127.0.0.1",
            "http://127.0.0.1:bad",
            "http://127.0.0.1:0",
        ):
            with self.assertRaises(ValueError):
                pack.target_url(target)
        pack.target_url("https://example.com", True)

    def test_cli_json_and_nonzero_exit(self):
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "report.json"
            # Reserve the origin port, then release before the child binds it.
            with pack.recorder() as (origin, _, _):
                pass
            with proxy(origin) as target:
                completed = subprocess.run(
                    [
                        sys.executable,
                        str(SCRIPT),
                        "--pack",
                        str(FIXTURE),
                        "--target",
                        target,
                        "--origin-port",
                        str(origin),
                        "--observation-window",
                        "0.01",
                        "--report",
                        str(report),
                    ],
                    capture_output=True,
                    text=True,
                    timeout=15,
                )
            self.assertEqual(completed.returncode, 1, completed.stderr)
            results = json.loads(report.read_text())["results"]
            self.assertEqual(sum(r["rule_behavior"] == "pass" for r in results), 5)
            self.assertEqual(results[-1]["rule_behavior"], "unsupported")
            supported = Path(directory) / "supported.json"
            config = json.loads(FIXTURE.read_text())
            config["cases"] = config["cases"][:-1]
            supported.write_text(json.dumps(config))
            with proxy(origin) as target:
                completed = subprocess.run(
                    [
                        sys.executable,
                        str(SCRIPT),
                        "--pack",
                        str(supported),
                        "--target",
                        target,
                        "--origin-port",
                        str(origin),
                        "--observation-window",
                        "0.01",
                        "--report",
                        str(report),
                    ],
                    capture_output=True,
                    text=True,
                    timeout=15,
                )
            self.assertEqual(completed.returncode, 0, completed.stderr)


if __name__ == "__main__":
    unittest.main()
