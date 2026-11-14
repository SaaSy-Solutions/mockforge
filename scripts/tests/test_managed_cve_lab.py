"""Managed lab verdict and lifecycle contracts; no container engine required."""

import argparse
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import threading
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import cve_lab_catalog as catalog
import cve_lab_engine as engine
import cve_lab_nuclei as nuclei
import cve_lab_evidence as evidence
import cve_lab_transport as transport

SPEC = importlib.util.spec_from_file_location(
    "managed_cve_lab", Path(__file__).resolve().parents[1] / "cve_lab.py"
)
lab = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(lab)


def completed(stdout="", returncode=0):
    return subprocess.CompletedProcess([], returncode, stdout, "")


class LifecycleTests(unittest.TestCase):
    def test_cleanup_refuses_unowned_container(self):
        for labels in ({}, {catalog.LABEL: "another-run"}):
            with self.subTest(labels=labels):
                with patch.object(
                    engine,
                    "command",
                    return_value=completed(
                        json.dumps([{"Config": {"Labels": labels}}])
                    ),
                ) as command:
                    with self.assertRaisesRegex(RuntimeError, "unowned"):
                        engine.owned_remove("docker", "container", "our-run")
                    command.assert_called_once_with(
                        ["docker", "inspect", "container"], check=False
                    )

    def test_cleanup_removes_owned_container_only(self):
        with patch.object(engine, "doctor"), patch.object(
            engine,
            "command",
            side_effect=[
                completed(json.dumps([{"Config": {"Labels": {catalog.LABEL: "our-run"}}}])),
                completed(),
            ],
        ) as command:
            engine.owned_remove("docker", "container", "our-run")
        self.assertEqual(command.call_args.args[0], ["docker", "rm", "-f", "container"])

    def test_cleanup_already_absent_is_idempotent(self):
        with patch.object(
            engine, "command", side_effect=[completed(returncode=1), completed()]
        ):
            engine.owned_remove("docker", "container", "our-run")

    def test_cleanup_engine_failure_is_not_absence(self):
        with patch.object(engine, "command", return_value=completed(returncode=1)):
            with self.assertRaisesRegex(RuntimeError, "Cannot verify removal"):
                engine.owned_remove("docker", "container", "our-run")

    def test_cleanup_failed_inspect_of_existing_container_is_not_absence(self):
        with patch.object(engine, "doctor"), patch.object(
            engine,
            "command",
            side_effect=[completed(returncode=1), completed("container\n")],
        ):
            with self.assertRaisesRegex(RuntimeError, "Cannot verify removal"):
                engine.owned_remove("docker", "container", "our-run")

    def test_down_rejects_changed_engine_and_invalid_ownership(self):
        for ownership in (
            {"run_id": "a" * 32, "engine": "podman"},
            {"run_id": "../someone-elses-container", "engine": "docker"},
        ):
            with self.subTest(
                ownership=ownership
            ), tempfile.TemporaryDirectory() as directory:
                (Path(directory) / "ownership.json").write_text(json.dumps(ownership))
                with patch.object(
                    engine, "owned_remove"
                ) as remove, contextlib.redirect_stderr(io.StringIO()):
                    self.assertEqual(lab.main(["down", "--output-dir", directory]), 1)
                remove.assert_not_called()

    def test_existing_report_directory_is_never_overwritten(self):
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "report.json"
            report.write_text("existing evidence\n")
            with patch.object(lab, "prepare") as prepare, contextlib.redirect_stderr(
                io.StringIO()
            ):
                self.assertEqual(
                    lab.main(["run", lab.CVE, "--output-dir", directory]), 1
                )
            prepare.assert_not_called()
            self.assertEqual(report.read_text(), "existing evidence\n")
            self.assertEqual(list(Path(directory).iterdir()), [report])

    def test_lab_cleans_up_on_client_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            with patch.object(
                engine,
                "command",
                side_effect=[
                    completed(),
                    completed("127.0.0.1:12345\n"),
                    completed("application log"),
                ],
            ), patch.object(
                engine, "request", return_value=(200, [], b"login")
            ), patch.object(
                engine, "owned_remove"
            ) as remove:
                with self.assertRaisesRegex(RuntimeError, "client failed"):
                    with engine.lab("docker", "our-run", Path(directory), catalog.PACKS[catalog.CVE]):
                        raise RuntimeError("client failed")
            remove.assert_called_once_with("docker", "mockforge-cve-our-run", "our-run")
            self.assertEqual(
                (Path(directory) / "application.log").read_text(), "application log"
            )


class InputTests(unittest.TestCase):
    def test_unsupported_cve_is_rejected_before_download(self):
        for action in ("prepare", "run"):
            with self.subTest(action=action), patch.object(
                lab, "prepare"
            ) as prepare, contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit) as error:
                    lab.main([action, "CVE-2099-99999"])
                self.assertEqual(error.exception.code, 2)
                prepare.assert_not_called()

    def test_run_requires_output_directory(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(
            SystemExit
        ) as error:
            lab.main(["run", lab.CVE])
        self.assertEqual(error.exception.code, 2)

    def test_accepts_http_origins(self):
        for url in (
            "http://localhost:8080",
            "https://example.test/",
            "http://[::1]:8080",
        ):
            with self.subTest(url=url):
                self.assertIsNotNone(transport.target_url(url).hostname)

    def test_rejects_non_origin_targets(self):
        for url in (
            "file:///tmp/lab",
            "http:///",
            "http://user:password@host",
            "http://host/app",
            "http://host/?query=1",
            "http://host/#fragment",
            "http://host:0",
            "http://host:65536",
            "http://host:abc",
        ):
            with self.subTest(url=url), self.assertRaises(ValueError):
                transport.target_url(url)


class VerdictTests(unittest.TestCase):
    def setUp(self):
        self.baseline = {"status": 200, "marker": True}
        self.target = {"token": "attack", "status": 403, "marker": False}
        self.before = {
            "token": "before",
            "status": 200,
            "root_token": "before-root",
            "root_status": 307,
        }
        self.after = {
            "token": "after",
            "status": 200,
            "root_token": "after-root",
            "root_status": 307,
        }
        self.records = [
            {"tokens": [control[key]], "status": status}
            for control in (self.before, self.after)
            for key, status in (("token", 200), ("root_token", 307))
        ]

    def verdict(self):
        return self.outcome()[0]

    def outcome(self):
        return evidence.classify(
            self.baseline, self.target, self.before, self.after, self.records, 403
        )

    def test_correlated_controls_and_blocked_signature_pass(self):
        self.assertEqual(self.verdict(), "pass")

    def test_baseline_must_demonstrate_vulnerability(self):
        for baseline in (
            {"status": 403, "marker": True},
            {"status": 200, "marker": False},
        ):
            with self.subTest(baseline=baseline):
                self.baseline = baseline
                self.assertEqual(self.verdict(), "inconclusive")

    def test_missing_login_origin_observation_is_inconclusive(self):
        self.records = self.records[1:]
        self.assertEqual(self.verdict(), "inconclusive")

    def test_failed_login_control_is_inconclusive(self):
        self.after["status"] = 503
        self.assertEqual(self.verdict(), "inconclusive")

    def test_duplicate_control_evidence_is_inconclusive(self):
        self.records.append(self.records[0])
        self.assertEqual(self.verdict(), "inconclusive")

    def test_signature_reaching_origin_fails_even_if_response_is_block_status(self):
        self.records.append({"tokens": ["attack"], "status": 200})
        self.assertEqual(self.verdict(), "fail")

    def test_unexpected_rejection_is_inconclusive(self):
        self.target["status"] = 502
        self.assertEqual(self.verdict(), "inconclusive")

    def test_uncorrelated_origin_request_cannot_pass(self):
        self.records.append({"tokens": [None, None], "status": 200})
        self.assertEqual(self.verdict(), "inconclusive")

    def test_rewritten_origin_correlation_cannot_pass(self):
        self.records.append({"tokens": ["rewritten", "rewritten"], "status": 200})
        self.assertEqual(self.verdict(), "inconclusive")

    def test_blanket_block_of_root_path_cannot_pass(self):
        self.before["root_status"] = 403
        self.after["root_status"] = 403
        self.assertEqual(self.verdict(), "inconclusive")

    def test_missing_root_origin_evidence_is_inconclusive(self):
        self.records = [
            record for record in self.records if "after-root" not in record["tokens"]
        ]
        self.assertEqual(self.verdict(), "inconclusive")

    def test_origin_error_on_root_control_is_inconclusive(self):
        self.records[1]["status"] = 502
        self.assertEqual(self.verdict(), "inconclusive")

class PackRegistryTests(unittest.TestCase):
    """Pack data drives the runner; the reviewed pack stays byte-compatible."""

    def test_legacy_exports_match_the_shipped_pack(self):
        self.assertEqual(catalog.CVE, "CVE-2025-29927")
        self.assertEqual(catalog.CATALOG, catalog.PACKS[catalog.CVE]["catalog"])
        self.assertEqual(
            catalog.SIGNATURE,
            catalog.PACKS[catalog.CVE]["probe"]["headers"]["x-middleware-subrequest"],
        )

    def test_shipped_pack_carries_runner_config(self):
        pack = catalog.PACKS["CVE-2025-29927"]
        for key in ("catalog", "probe", "app"):
            self.assertIn(key, pack)
        self.assertEqual(pack["app"]["port"], "3000/tcp")
        self.assertEqual(pack["app"]["readiness"], {"path": "/login", "status": 200})

    def test_template_renders_the_signature_request(self):
        import cve_lab_nuclei as nuclei

        text = nuclei.template(catalog.PACKS["CVE-2025-29927"], "run1-baseline")
        self.assertIn("id: mockforge-nextjs-29927-observation", text)
        self.assertIn("method: GET", text)
        self.assertIn('X-Mockforge-Cve: "run1-baseline"', text)
        self.assertIn("x-middleware-subrequest: \"middleware:", text)
        self.assertIn("__mockforge_cve=run1-baseline", text)

    def test_template_renders_post_bodies_for_body_packs(self):
        import copy

        import cve_lab_nuclei as nuclei

        synthetic = copy.deepcopy(catalog.PACKS["CVE-2025-29927"])
        synthetic["probe"].update(
            {
                "template_id": "mockforge-synthetic-observation",
                "template_name": "MockForge synthetic observation",
                "method": "POST",
                "body": "<?php echo '{token}';?>",
            }
        )
        text = nuclei.template(synthetic, "tok")
        self.assertIn("method: POST", text)
        self.assertIn("<?php echo 'tok';?>", text)
        self.assertIn("id: mockforge-synthetic-observation", text)

    def test_prepare_pulls_only_the_packs_pinned_images(self):
        pack = catalog.PACKS["CVE-2025-29927"]
        images = (
            pack["catalog"]["application_image"],
            pack["catalog"]["runner_image"],
        )
        with patch.object(engine, "doctor"), patch.object(
            engine,
            "command",
            side_effect=[
                completed(returncode=1),
                completed(),
                completed(),
                completed(returncode=1),
                completed(),
                completed(),
            ],
        ) as command:
            engine.prepare("docker", pack)
        pulled = [
            call.args[0] for call in command.call_args_list if call.args[0][1] == "pull"
        ]
        self.assertEqual(pulled, [["docker", "pull", images[0]], ["docker", "pull", images[1]]])


class Cve20244577PackTests(unittest.TestCase):
    """The CVE-2024-4577 pack is complete except for its pending reviewed image."""

    def setUp(self):
        self.pack = catalog.PACKS["CVE-2024-4577"]

    def test_pack_carries_runner_config(self):
        for key in ("catalog", "probe", "app", "controls"):
            self.assertIn(key, self.pack)
        self.assertEqual(self.pack["app"]["port"], "8080/tcp")
        self.assertEqual(
            self.pack["app"]["readiness"],
            {"path": "/php-cgi/php-cgi.exe", "status": 404},
        )
        self.assertEqual(
            self.pack["controls"]["benign_status"], self.pack["app"]["readiness"]["status"]
        )

    def test_pack_declares_its_image_unreviewed(self):
        self.assertEqual(
            self.pack["catalog"]["image_status"], "pending-lab-host-review"
        )
        self.assertNotIn("@sha256:", self.pack["catalog"]["application_image"])

    def test_probe_keeps_the_exploit_query_whole(self):
        probe = self.pack["probe"]
        self.assertEqual(probe["method"], "POST")
        # php-cgi parses the query string argv-style; a correlated query
        # parameter would fold into the injected switches, so the token only
        # rides the header.
        self.assertNotIn("__mockforge_cve", probe["path"])
        self.assertIn("%ADd+allow_url_include%3D1", probe["path"])
        self.assertIn("auto_prepend_file%3Dphp%3A%2F%2Finput", probe["path"])
        self.assertIsNone(probe["signature_header"])
        self.assertIn(probe["marker"], probe["body"])

    def test_prepare_refuses_the_unreviewed_image_before_any_engine_call(self):
        with patch.object(engine, "command") as command:
            with self.assertRaisesRegex(RuntimeError, "not pinned by digest"):
                engine.prepare("docker", self.pack)
        command.assert_not_called()

    def test_run_of_the_pending_pack_refuses_and_records_why(self):
        stdout = io.StringIO()
        stderr = io.StringIO()
        with tempfile.TemporaryDirectory() as directory, patch.object(
            engine, "command"
        ) as command, contextlib.redirect_stdout(
            stdout
        ), contextlib.redirect_stderr(stderr):
            output = Path(directory) / "out"
            self.assertEqual(
                lab.main(["run", "CVE-2024-4577", "--output-dir", str(output)]), 1
            )
            command.assert_not_called()
            report = json.loads((output / "report.json").read_text())
        self.assertIn("Lab image pending review", stdout.getvalue())
        self.assertEqual(report["verdict"], "inconclusive")
        self.assertIn("not pinned by digest", report["reason"])

    def test_template_renders_the_cgi_injection_observation(self):
        text = nuclei.template(self.pack, "run1-baseline")
        self.assertIn("id: mockforge-phpcgi-4577-observation", text)
        self.assertIn("method: POST", text)
        self.assertIn('X-Mockforge-Cve: "run1-baseline"', text)
        self.assertIn("%ADd+allow_url_include%3D1", text)
        self.assertIn("<?php echo 'MOCKFORGE-CVE-2024-4577'; ?>", text)
        self.assertNotIn("__mockforge_cve", text)


class Cve20244577VerdictTests(unittest.TestCase):
    """The CGI classifier mirrors the blocking semantics for the PHP pack."""

    def setUp(self):
        self.pack = catalog.PACKS["CVE-2024-4577"]
        self.baseline = {"status": 200, "marker": True}
        self.target = {"token": "attack", "status": 403, "marker": False}
        self.before = {
            "token": "before",
            "status": 404,
            "root_token": "before-root",
            "root_status": 200,
        }
        self.after = {
            "token": "after",
            "status": 404,
            "root_token": "after-root",
            "root_status": 200,
        }
        self.records = [
            {"tokens": [control[key]], "status": status}
            for control in (self.before, self.after)
            for key, status in (("token", 404), ("root_token", 200))
        ]

    def outcome(self):
        return evidence.CLASSIFIERS["cgi-argument-injection"](
            self.pack, self.baseline, self.target, self.before, self.after, self.records, 403
        )

    def verdict(self):
        return self.outcome()[0]

    def test_correlated_controls_and_blocked_exploit_pass(self):
        self.assertEqual(self.verdict(), "pass")

    def test_baseline_must_execute_the_payload(self):
        for baseline in (
            {"status": 404, "marker": True},
            {"status": 200, "marker": False},
        ):
            with self.subTest(baseline=baseline):
                self.baseline = baseline
                self.assertEqual(self.verdict(), "inconclusive")

    def test_failed_benign_control_is_inconclusive(self):
        self.after["status"] = 200
        self.assertEqual(self.verdict(), "inconclusive")

    def test_blanket_block_of_the_ordinary_path_is_inconclusive(self):
        self.before["root_status"] = 403
        self.after["root_status"] = 403
        self.assertEqual(self.verdict(), "inconclusive")

    def test_exploit_reaching_origin_fails_even_if_response_is_block_status(self):
        self.records.append({"tokens": ["attack"], "status": 200})
        self.assertEqual(self.verdict(), "fail")

    def test_unexpected_rejection_is_inconclusive(self):
        self.target["status"] = 502
        self.assertEqual(self.verdict(), "inconclusive")

    def test_uncorrelated_origin_request_cannot_pass(self):
        self.records.append({"tokens": [None, None], "status": 200})
        self.assertEqual(self.verdict(), "inconclusive")

    def test_gate_accepts_a_reproduced_direct_run(self):
        baseline_control = {"status": 404, "root_status": 200}
        baseline = {"status": 200, "marker": True}
        evidence.BASELINE_GATES["cgi-argument-injection"](
            self.pack, baseline_control, baseline
        )

    def test_gate_rejects_mismatched_direct_runs(self):
        for baseline_control, baseline in (
            ({"status": 200, "root_status": 200}, {"status": 200, "marker": True}),
            ({"status": 404, "root_status": 404}, {"status": 200, "marker": True}),
            ({"status": 404, "root_status": 200}, {"status": 200, "marker": False}),
        ):
            with self.subTest(baseline_control=baseline_control):
                with self.assertRaisesRegex(RuntimeError, "PHP code execution"):
                    evidence.BASELINE_GATES["cgi-argument-injection"](
                        self.pack, baseline_control, baseline
                    )


class TransportForwardingTests(unittest.TestCase):
    """The recorder forwards GET and POST with bodies on owned loopback ports."""

    def setUp(self):
        from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

        seen = []

        class Backend(BaseHTTPRequestHandler):
            def _reply(self):
                length = int(self.headers.get("Content-Length") or 0)
                body = self.rfile.read(length) if length else b""
                seen.append((self.command, self.path, body))
                self.send_response(201)
                self.send_header("Content-Length", "2")
                self.end_headers()
                self.wfile.write(b"ok")

            do_GET = _reply
            do_POST = _reply

            def log_message(self, *_args):
                pass

        self.backend = ThreadingHTTPServer(("127.0.0.1", 0), Backend)
        threading.Thread(target=self.backend.serve_forever, daemon=True).start()
        self.backend_port = self.backend.server_address[1]
        self.seen = seen

    def tearDown(self):
        self.backend.shutdown()
        self.backend.server_close()

    @contextlib.contextmanager
    def recorder(self, signature_header=None):
        with socket.socket() as reservation:
            reservation.bind(("127.0.0.1", 0))
            port = reservation.getsockname()[1]
        records = []
        with transport.receiver(
            port,
            "127.0.0.1",
            f"http://127.0.0.1:{self.backend_port}",
            signature_header=signature_header,
        ) as records:
            yield f"http://127.0.0.1:{port}", records

    def test_get_requests_are_forwarded_and_observed(self):
        with self.recorder() as (base, records):
            status, _headers, body = transport.request(
                base,
                "/login?__mockforge_cve=tok1",
                {"X-Mockforge-Cve": "tok1"},
            )
        self.assertEqual(status, 201)
        self.assertEqual(body, b"ok")
        self.assertEqual(self.seen[0][0], "GET")
        self.assertEqual(records[0]["method"], "GET")
        self.assertIn("tok1", records[0]["tokens"])

    def test_post_bodies_are_forwarded_and_observed(self):
        with self.recorder() as (base, records):
            status, _headers, body = transport.request(
                base,
                "/?%ADd+auto_prepend_file%3Dphp://input&__mockforge_cve=tok2",
                {"X-MockForge-Cve": "tok2"},
                method="POST",
                body=b"<?php echo 'tok2';?>",
            )
        self.assertEqual(status, 201)
        self.assertEqual(body, b"ok")
        self.assertEqual(self.seen[0][0], "POST")
        self.assertEqual(self.seen[0][2], b"<?php echo 'tok2';?>")
        self.assertEqual(records[0]["method"], "POST")
        self.assertIn("tok2", records[0]["tokens"])

    def test_signature_header_is_recorded_only_when_configured(self):
        with self.recorder(signature_header="x-middleware-subrequest") as (
            base,
            records,
        ):
            transport.request(
                base,
                "/?__mockforge_cve=tok3",
                {"X-Mockforge-Cve": "tok3", "x-middleware-subrequest": "middleware:chain"},
            )
        self.assertEqual(records[0]["signature"], "middleware:chain")
        with self.recorder() as (base, records):
            transport.request(
                base,
                "/php-cgi/php-cgi.exe?%ADd+allow_url_include%3D1",
                {"X-Mockforge-Cve": "tok4"},
                method="POST",
            )
        self.assertIsNone(records[0]["signature"])
        self.assertIn("tok4", records[0]["tokens"])

    def test_control_requests_follow_the_pack_paths(self):
        pack = {
            "controls": {
                "benign_path": "/benign?__mockforge_cve={token}",
                "benign_status": 404,
                "ordinary_path": "/ordinary?__mockforge_cve={token}",
                "ordinary_status": 200,
            }
        }
        with self.recorder() as (base, records):
            outcome = transport.control(pack, base, "run1", "phase")
        paths = [item[1] for item in self.seen]
        self.assertEqual(len(paths), 2)
        self.assertTrue(paths[0].startswith("/benign?__mockforge_cve=run1-phase-"))
        ordinary_token = paths[1].split("__mockforge_cve=")[1]
        self.assertTrue(ordinary_token.startswith("run1-phase-"))
        self.assertTrue(ordinary_token.endswith("-root"))
        self.assertEqual(outcome["root_token"], ordinary_token)
        self.assertEqual(outcome["root_token"], outcome["token"] + "-root")
        self.assertEqual(len(records), 2)

    def test_oversized_post_bodies_are_rejected(self):
        import http.client as http_client

        with patch.object(transport, "MAX_FORWARDED_BODY", 8), self.recorder() as (
            base,
            _records,
        ):
            url = transport.target_url(base)
            connection = http_client.HTTPConnection(url.hostname, url.port, timeout=5)
            try:
                connection.putrequest("POST", "/?__mockforge_cve=tok3")
                connection.putheader("Content-Length", "16")
                connection.endheaders(message_body=b"0123456789abcdef")
                response = connection.getresponse()
                self.assertEqual(response.status, 413)
            finally:
                connection.close()
        # The recorder refused before forwarding anything to the backend.
        self.assertEqual(self.seen, [])



class VerdictDescriptionTests(unittest.TestCase):
    """Reports state exposed / not exposed plainly, with per-check detail."""

    def setUp(self):
        super().setUp()
        self.base_report = {
            "verdict": "pass",
            "reason": "expected rejection with no correlated origin request and healthy controls",
            "target": "https://lb.example",
            "pack": {"cve": "CVE-2025-29927"},
            "evidence_checks": [
                {"check": "direct baseline reproduces the vulnerability", "outcome": "ok"},
                {"check": "signature request was blocked at the target", "outcome": "ok"},
            ],
        }

    def test_pass_says_not_exposed_with_target_and_cve(self):
        text = evidence.describe(self.base_report)
        self.assertIn("https://lb.example is NOT exposed to CVE-2025-29927", text)
        self.assertIn("pass: ", text)
        self.assertIn("Reason: expected rejection", text)

    def test_fail_says_exposed_with_reason_and_details(self):
        self.base_report.update(
            verdict="fail",
            reason="signature request reached the origin; blocking assertion failed",
            evidence_checks=[
                {"check": "direct baseline reproduces the vulnerability", "outcome": "ok"},
                {
                    "check": "signature request was blocked at the target",
                    "outcome": "not satisfied, reached the origin with status 200",
                },
            ],
        )
        text = evidence.describe(self.base_report)
        self.assertIn("https://lb.example is EXPOSED to CVE-2025-29927", text)
        self.assertIn("attack traffic reached your origin", text)
        self.assertIn("reached the origin with status 200", text)

    def test_inconclusive_defers_the_exposure_judgment(self):
        self.base_report.update(verdict="inconclusive", reason="benign controls did not corroborate origin reachability")
        text = evidence.describe(self.base_report)
        self.assertIn("no exposure determination could be made for https://lb.example", text)

    def test_baseline_confirmed_mentions_no_lb_was_tested(self):
        report = {"verdict": "baseline-confirmed", "reason": "Direct lab reproduction only; no LB tested", "pack": {"cve": "CVE-2025-29927"}}
        text = evidence.describe(report)
        self.assertIn("no LB/WAAP was tested", text)
        self.assertIn("no exposure determination was made", text)

    def test_direct_runs_use_the_lab_application_placeholder(self):
        report = {"verdict": "pass", "reason": "r", "pack": {"cve": "CVE-2025-29927"}}
        self.assertIn("the direct lab application is NOT exposed", evidence.describe(report))


class EvidenceCheckTests(unittest.TestCase):
    """classify() records each assertion so reports can show the deciding check."""

    def setUp(self):
        self.baseline = {"status": 200, "marker": True}
        self.target = {"token": "attack", "status": 403, "marker": False}
        self.before = {
            "token": "before",
            "status": 200,
            "root_token": "before-root",
            "root_status": 307,
        }
        self.after = {
            "token": "after",
            "status": 200,
            "root_token": "after-root",
            "root_status": 307,
        }
        self.records = [
            {"tokens": [control[key]], "status": status}
            for control in (self.before, self.after)
            for key, status in (("token", 200), ("root_token", 307))
        ]

    def outcome(self):
        return evidence.classify(
            self.baseline, self.target, self.before, self.after, self.records, 403
        )

    def test_pass_records_every_check_ok(self):
        _verdict, _reason, checks = self.outcome()
        self.assertTrue(all(check["outcome"] == "ok" for check in checks))
        names = [check["check"] for check in checks]
        self.assertIn("direct baseline reproduces the vulnerability", names)
        self.assertIn("signature request was blocked at the target", names)

    def test_fail_names_the_origin_reach_with_status(self):
        self.records.append({"tokens": ["attack"], "status": 200})
        _verdict, _reason, checks = self.outcome()
        blocked = [check for check in checks if check["check"] == "signature request was blocked at the target"]
        self.assertEqual(len(blocked), 1)
        self.assertIn("status 200", blocked[0]["outcome"])

    def test_short_circuit_stops_at_the_first_failed_check(self):
        self.baseline = {"status": 403, "marker": True}
        _verdict, _reason, checks = self.outcome()
        self.assertEqual(len(checks), 1)



class RunPlanTests(unittest.TestCase):
    """The run plan states the recorder endpoint and listen-address needs upfront."""

    def args(self, **overrides):
        defaults = {
            "cve": "CVE-2025-29927",
            "target": "https://lb.example",
            "listen_address": "127.0.0.1",
            "origin_port": 18082,
        }
        defaults.update(overrides)
        return argparse.Namespace(**defaults)

    def test_plan_states_recorder_endpoint_and_target(self):
        lines = lab.run_plan_lines(self.args())
        text = "\n".join(lines)
        self.assertIn("Recorder endpoint: 127.0.0.1:18082", text)
        self.assertIn("Target under test: https://lb.example", text)

    def test_default_listen_address_warns_about_remote_targets(self):
        text = "\n".join(lab.run_plan_lines(self.args()))
        self.assertIn("--listen-address <an isolated lab interface IP>", text)

    def test_explicit_listen_address_does_not_warn(self):
        text = "\n".join(
            lab.run_plan_lines(self.args(listen_address="192.168.2.10"))
        )
        self.assertNotIn("--listen-address", text.replace("Recorder endpoint:", ""))

    def test_direct_runs_say_no_lb_is_tested(self):
        text = "\n".join(lab.run_plan_lines(self.args(target=None)))
        self.assertIn("No --target", text)
        self.assertIn("no LB/WAAP is tested", text)

    def test_list_advertises_every_pack_with_origin_port_and_guidance(self):
        stdout = io.StringIO()
        with contextlib.redirect_stdout(stdout):
            self.assertEqual(lab.main(["list"]), 0)
        packs = json.loads(stdout.getvalue())
        self.assertEqual(set(packs), set(catalog.PACKS))
        for cve, entry in packs.items():
            with self.subTest(cve=cve):
                self.assertEqual(entry["origin_port"], 18082)
                self.assertIn("--listen-address", entry["listen_address_guidance"])
        self.assertIn(
            "@sha256:", packs["CVE-2025-29927"]["application_image"]
        )
        self.assertEqual(
            packs["CVE-2024-4577"]["image_status"], "pending-lab-host-review"
        )

    def test_pending_pack_plan_states_the_image_is_under_review(self):
        lines = lab.run_plan_lines(self.args(cve="CVE-2024-4577", target=None))
        self.assertIn("Lab image pending review", "\n".join(lines))

    def test_reviewed_pack_plan_does_not_mention_pending_review(self):
        lines = lab.run_plan_lines(self.args(target=None))
        self.assertNotIn("pending", "\n".join(lines))



class TlsVerifyTests(unittest.TestCase):
    """--skip-tls-verify is an explicit opt-in; the default path verifies."""

    def test_unverified_context_disables_checks(self):
        import ssl

        context = transport.unverified_context()
        self.assertFalse(context.check_hostname)
        self.assertEqual(context.verify_mode, ssl.CERT_NONE)

    def test_request_verifies_by_default_and_passes_context_when_given(self):
        with patch.object(transport.http.client, "HTTPSConnection") as connection:
            transport.request("https://lab.example", "/login")
        self.assertIsNone(connection.call_args.kwargs.get("context"))

        sentinel = object()
        with patch.object(transport.http.client, "HTTPSConnection") as connection:
            transport.request("https://lab.example", "/login", context=sentinel)
        self.assertIs(connection.call_args.kwargs.get("context"), sentinel)

    def test_plain_http_connections_never_receive_a_context(self):
        with patch.object(transport.http.client, "HTTPConnection") as connection:
            transport.request("http://lab.example", "/login")
        self.assertNotIn("context", connection.call_args.kwargs)

    def test_run_records_tls_verify_state_and_warns_when_disabled(self):
        stdout = io.StringIO()
        with tempfile.TemporaryDirectory() as directory, patch.object(
            lab, "prepare"
        ) as prepare, contextlib.redirect_stdout(stdout):
            output = Path(directory) / "out"
            self.assertEqual(
                lab.main(
                    [
                        "run",
                        lab.CVE,
                        "--target",
                        "https://lab.example",
                        "--output-dir",
                        str(output),
                        "--skip-tls-verify",
                    ]
                ),
                1,
            )
            prepare.assert_called_once()
            self.assertIn("TLS VERIFICATION DISABLED", stdout.getvalue())
            report = json.loads((output / "report.json").read_text())
            self.assertFalse(report["tls_verify"])

    def test_run_keeps_verification_on_by_default(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(
            lab, "prepare"
        ) as prepare, contextlib.redirect_stdout(io.StringIO()):
            output = Path(directory) / "out"
            self.assertEqual(
                lab.main(
                    ["run", lab.CVE, "--target", "https://lab.example", "--output-dir", str(output)]
                ),
                1,
            )
            prepare.assert_called_once()
            report = json.loads((output / "report.json").read_text())
            self.assertTrue(report["tls_verify"])

    def test_probe_adds_insecure_only_when_opted_in(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            pack = catalog.PACKS[catalog.CVE]
            findings = json.dumps(
                {
                    "template-id": "mockforge-nextjs-29927-observation",
                    "response": "HTTP/1.1 200 OK\r\n",
                }
            )
            with patch.object(nuclei, "owned_remove"), patch.object(
                nuclei,
                "command",
                return_value=completed(stdout=f"{findings}\n"),
            ) as command:
                nuclei.probe(
                    "docker", "http://127.0.0.1:1", output, "baseline", "run1", pack
                )
                args = command.call_args.args[0]
                self.assertNotIn("-insecure", args)

                command.reset_mock()
                nuclei.probe(
                    "docker",
                    "http://127.0.0.1:1",
                    output,
                    "baseline",
                    "run1",
                    pack,
                    insecure=True,
                )
                self.assertIn("-insecure", command.call_args.args[0])


class ImageVerificationTests(unittest.TestCase):
    """Pinned-image presence is reported from catalog data via by-digest
    engine inspects - never by parsing `docker images` output."""

    def test_image_status_uses_a_by_digest_inspect(self):
        with patch.object(
            engine, "command", return_value=completed()
        ) as command:
            self.assertEqual(engine.image_status("docker", "some@sha256:abc"), "present")
        command.assert_called_once_with(
            ["docker", "image", "inspect", "some@sha256:abc"], check=False
        )
        with patch.object(
            engine, "command", return_value=completed(returncode=1)
        ):
            self.assertEqual(engine.image_status("docker", "some@sha256:abc"), "missing")

    def test_image_status_reports_a_tag_as_unpinned_without_inspecting(self):
        with patch.object(engine, "command") as command:
            self.assertEqual(
                engine.image_status("docker", "some/image:pending-review"), "unpinned"
            )
        command.assert_not_called()

    def test_images_command_does_not_fail_on_unpinned_images(self):
        stdout = io.StringIO()
        with patch.object(lab, "doctor"), patch.object(
            lab,
            "verify_images",
            return_value=[
                {"role": "application", "image": "app:pending", "status": "unpinned"},
                {"role": "runner", "image": "runner@sha256:b", "status": "present"},
            ],
        ), contextlib.redirect_stdout(stdout):
            self.assertEqual(lab.main(["images"]), 0)

    def test_verify_images_reports_both_pinned_images_with_roles(self):
        pack = catalog.PACKS["CVE-2025-29927"]
        statuses = iter(("present", "missing"))
        with patch.object(
            engine, "command", side_effect=lambda *_a, **_k: completed(
                returncode=0 if next(statuses) == "present" else 1
            )
        ):
            listing = engine.verify_images("docker", pack)
        self.assertEqual(
            listing,
            [
                {
                    "role": "application",
                    "image": pack["catalog"]["application_image"],
                    "status": "present",
                },
                {
                    "role": "runner",
                    "image": pack["catalog"]["runner_image"],
                    "status": "missing",
                },
            ],
        )

    def test_images_command_lists_every_pack_and_exits_clean_when_present(self):
        stdout = io.StringIO()
        with patch.object(lab, "doctor"), patch.object(
            lab, "verify_images", return_value=[{"role": "application", "image": "app@sha256:a", "status": "present"}, {"role": "runner", "image": "runner@sha256:b", "status": "present"}]
        ), contextlib.redirect_stdout(stdout):
            self.assertEqual(lab.main(["images"]), 0)
        listing = json.loads(stdout.getvalue())
        self.assertEqual(set(listing), set(catalog.PACKS))
        for cve, entries in listing.items():
            with self.subTest(cve=cve):
                self.assertEqual(
                    [entry["role"] for entry in entries],
                    ["application", "runner"],
                )
                self.assertTrue(
                    all(entry["status"] == "present" for entry in entries)
                )
                for entry in entries:
                    self.assertIn("@sha256:", entry["image"])

    def test_images_command_exits_nonzero_when_an_image_is_missing(self):
        stdout = io.StringIO()
        stderr = io.StringIO()

        def verify(engine_arg, pack):
            if pack["catalog"]["cve"] == "CVE-2024-4577":
                return [
                    {"role": "application", "image": "app@sha256:a", "status": "present"},
                    {"role": "runner", "image": "runner@sha256:b", "status": "missing"},
                ]
            return [
                {"role": "application", "image": "app@sha256:a", "status": "present"},
                {"role": "runner", "image": "runner@sha256:b", "status": "present"},
            ]

        with patch.object(lab, "doctor"), patch.object(
            lab, "verify_images", side_effect=verify
        ), contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            self.assertEqual(lab.main(["images"]), 1)
        listing = json.loads(stdout.getvalue())
        statuses = [
            (cve, entry["role"], entry["status"])
            for cve, entries in listing.items()
            for entry in entries
        ]
        self.assertIn(("CVE-2024-4577", "runner", "missing"), statuses)
        self.assertIn("missing on this machine", stderr.getvalue())

    def test_images_command_filters_to_one_pack(self):
        stdout = io.StringIO()
        entries = [
            {"role": "application", "image": "app@sha256:a", "status": "present"},
            {"role": "runner", "image": "runner@sha256:b", "status": "present"},
        ]
        with patch.object(lab, "doctor"), patch.object(
            lab, "verify_images", return_value=entries
        ), contextlib.redirect_stdout(stdout):
            self.assertEqual(lab.main(["images", "CVE-2025-29927"]), 0)
        listing = json.loads(stdout.getvalue())
        self.assertEqual(set(listing), {"CVE-2025-29927"})

    def test_run_report_records_the_verified_images(self):
        entries = [
            {"role": "application", "image": "app@sha256:a", "status": "present"},
            {"role": "runner", "image": "runner@sha256:b", "status": "present"},
        ]
        stdout = io.StringIO()
        with tempfile.TemporaryDirectory() as directory, patch.object(
            lab, "prepare"
        ) as prepare, patch.object(
            lab, "verify_images", return_value=entries
        ) as verify, contextlib.redirect_stdout(stdout):
            output = Path(directory) / "out"
            self.assertEqual(
                lab.main(["run", lab.CVE, "--output-dir", str(output)]), 1
            )
            prepare.assert_called_once()
            verify.assert_called_once()
            report = json.loads((output / "report.json").read_text())
        self.assertEqual(report["image_verification"], entries)
        self.assertIn("Images verified on this machine: 2/2", stdout.getvalue())

    def test_run_output_prints_the_verification_line_even_on_failure(self):
        # prepare succeeds (mocked), the engine step later fails on this
        # host - the verification line and report entry are still written
        entries = [
            {"role": "application", "image": "app@sha256:a", "status": "present"},
            {"role": "runner", "image": "runner@sha256:b", "status": "present"},
        ]
        with tempfile.TemporaryDirectory() as directory, patch.object(
            lab, "prepare"
        ), patch.object(
            lab, "verify_images", return_value=entries
        ), contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(
            io.StringIO()
        ):
            output = Path(directory) / "out"
            self.assertEqual(
                lab.main(["run", lab.CVE, "--output-dir", str(output)]), 1
            )
            report = json.loads((output / "report.json").read_text())
        self.assertEqual(report["image_verification"], entries)


if __name__ == "__main__":
    unittest.main()
