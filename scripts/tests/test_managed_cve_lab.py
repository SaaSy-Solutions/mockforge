"""Managed lab verdict and lifecycle contracts; no container engine required."""

import argparse
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import shutil
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
import cve_lab_http2 as http2_module
import cve_lab_payloads as payloads
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
    def recorder(self):
        with socket.socket() as reservation:
            reservation.bind(("127.0.0.1", 0))
            port = reservation.getsockname()[1]
        records = []
        with transport.receiver(
            port, "127.0.0.1", f"http://127.0.0.1:{self.backend_port}"
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
        self.assertIn("@sha256:", packs["CVE-2025-29927"]["application_image"])
        self.assertIn("@sha256:", packs["CVE-2026-111497"]["application_image"])



class Node2ShellPackTests(unittest.TestCase):
    """The Node2Shell pack data, transport and verdict contracts."""

    def setUp(self):
        self.pack = catalog.PACKS["CVE-2026-111497"]

    def test_pack_carries_runner_config(self):
        for key in ("catalog", "probe", "app", "controls", "exploit", "application_script"):
            self.assertIn(key, self.pack)
        self.assertEqual(self.pack["app"]["port"], "3000/tcp")
        self.assertEqual(
            self.pack["app"]["readiness"], {"path": "/health", "status": 200}
        )
        self.assertEqual(self.pack["app"]["mounts"][0]["container"], "/app/server.js")

    def test_pack_images_are_digest_pinned(self):
        self.assertIn("@sha256:", self.pack["catalog"]["application_image"])
        self.assertIn("@sha256:", self.pack["catalog"]["runner_image"])

    def test_application_script_is_the_reviewed_lab_app(self):
        script = self.pack["application_script"]
        self.assertIn("require('node:http')", script)
        self.assertIn("createBrotliDecompress", script)
        self.assertIn("PHASE_ONE_BYTES = 23068693", script)
        self.assertIn("mockforge-node2shell-lab", script)
        # No restart policy and no respawn: a crash must be observable.
        self.assertNotIn("fork", script)
        self.assertNotIn("cluster", script)

    def test_exploit_rides_the_raw_h2_client(self):
        self.assertEqual(self.pack["baseline_exploit"], "http2-crash")
        self.assertEqual(self.pack["target_exploit"], "http2-crash")
        exploit = self.pack["exploit"]
        self.assertEqual(exploit["method"], "POST")
        self.assertIn("/decode?", exploit["path"])
        self.assertIn("__mockforge_cve={token}", exploit["path"])
        # The reviewed trigger is ~23 MB; the recorder must accept it.
        self.assertGreaterEqual(exploit["max_forwarded_body"], 32 * 1024 * 1024)
        self.assertEqual(self.pack["controls"]["benign_status"], 200)
        self.assertEqual(self.pack["controls"]["ordinary_status"], 200)

    def test_trigger_payload_matches_the_pinned_digest(self):
        import hashlib

        import cve_lab_payloads as payloads

        stream = payloads.trigger_stream()
        digest = hashlib.sha256(stream).hexdigest()
        self.assertEqual(digest, payloads.TRIGGER_SHA256)
        body = stream + payloads.TRAILING_ZEROS + payloads.second_input()
        first, second = payloads.split_input(body)
        self.assertGreater(second, 0)
        self.assertEqual(first + second, len(body))

    def test_probe_template_observes_a_healthy_request(self):
        text = nuclei.template(self.pack, "run1-baseline")
        self.assertIn("id: mockforge-node-111497-observation", text)
        self.assertIn("method: GET", text)
        self.assertIn("/health?__mockforge_cve=run1-baseline", text)

    def test_prepare_refuses_unpinned_images_before_any_engine_call(self):
        import copy

        unpinned = copy.deepcopy(self.pack)
        unpinned["catalog"]["application_image"] = "docker.io/library/node:latest"
        with patch.object(engine, "command") as command:
            with self.assertRaisesRegex(RuntimeError, "not pinned by digest"):
                engine.prepare("docker", unpinned)
        command.assert_not_called()

    def test_gate_accepts_a_healthy_direct_run(self):
        baseline_control = {"status": 200, "root_status": 200}
        baseline = {"status": 200, "marker": True, "crash_confirmed": True}
        evidence.BASELINE_GATES["decoder-crash"](
            self.pack, baseline_control, baseline
        )

    def test_gate_rejects_mismatched_direct_runs(self):
        for baseline_control, baseline in (
            ({"status": 503, "root_status": 200}, {"status": 200, "marker": True}),
            ({"status": 200, "root_status": 404}, {"status": 200, "marker": True}),
            ({"status": 200, "root_status": 200}, {"status": 200, "marker": False}),
        ):
            with self.subTest(baseline_control=baseline_control):
                with self.assertRaisesRegex(RuntimeError, "decoder crash"):
                    evidence.BASELINE_GATES["decoder-crash"](
                        self.pack, baseline_control, baseline
                    )


class Node2ShellVerdictTests(unittest.TestCase):
    """The decoder-crash classifier mirrors the blocking semantics for the pack."""

    def setUp(self):
        self.pack = catalog.PACKS["CVE-2026-111497"]
        self.baseline = {"status": 200, "marker": True, "crash_confirmed": True}
        self.target = {"token": "attack", "status": 403, "marker": False}
        self.before = {
            "token": "before",
            "status": 200,
            "root_token": "before-root",
            "root_status": 200,
        }
        self.after = {
            "token": "after",
            "status": 200,
            "root_token": "after-root",
            "root_status": 200,
        }
        self.records = [
            {"tokens": [control[key]], "status": status}
            for control in (self.before, self.after)
            for key, status in (("token", 200), ("root_token", 200))
        ]

    def outcome(self):
        return evidence.CLASSIFIERS["decoder-crash"](
            self.pack, self.baseline, self.target, self.before, self.after, self.records, 403
        )

    def verdict(self):
        return self.outcome()[0]

    def test_blocked_exploit_with_healthy_controls_passes(self):
        self.assertEqual(self.verdict(), "pass")

    def test_baseline_must_show_the_confirmed_crash(self):
        for baseline in (
            {"status": 200, "marker": True},
            {"status": 200, "marker": True, "crash_confirmed": False},
            {"status": 422, "marker": True, "crash_confirmed": True},
        ):
            with self.subTest(baseline=baseline):
                self.baseline = baseline
                self.assertEqual(self.verdict(), "inconclusive")

    def test_failed_before_control_is_inconclusive(self):
        self.before["status"] = 503
        self.assertEqual(self.verdict(), "inconclusive")

    def test_exploit_reaching_origin_fails_even_when_it_crashed_the_origin(self):
        self.records.append({"tokens": ["attack"], "status": None, "error": "ConnectionReset"})
        self.after = {
            "token": self.after["token"],
            "root_token": self.after["root_token"],
            "error": "connection refused by the origin",
        }
        verdict, reason, checks = self.outcome()
        self.assertEqual(verdict, "fail")
        self.assertIn("reached the origin", reason)
        blocked = [c for c in checks if c["check"] == "exploit request was blocked at the target"]
        self.assertEqual(len(blocked), 1)
        self.assertIn("reached the origin", blocked[0]["outcome"])

    def test_blocked_exploit_with_dead_after_control_is_inconclusive(self):
        self.after = {
            "token": self.after["token"],
            "root_token": self.after["root_token"],
            "error": "connection refused by the origin",
        }
        self.assertEqual(self.verdict(), "inconclusive")

    def test_blanket_block_of_the_ordinary_path_is_inconclusive(self):
        self.before["root_status"] = 403
        self.assertEqual(self.verdict(), "inconclusive")

    def test_unexpected_rejection_is_inconclusive(self):
        self.target["status"] = 502
        self.assertEqual(self.verdict(), "inconclusive")

    def test_uncorrelated_origin_request_cannot_pass(self):
        self.records.append({"tokens": [None, None], "status": 200})
        self.assertEqual(self.verdict(), "inconclusive")


class Node2ShellExploitStepTests(unittest.TestCase):
    """The http2-crash steps turn transport evidence plus container state
    into a confirmed baseline and a target result."""

    def test_baseline_crash_confirms_from_the_container_state(self):
        import cve_lab_exploits as exploits

        pack = catalog.PACKS["CVE-2026-111497"]
        result = {"status": 502, "headers": [], "body": b"", "reset": False,
                  "goaway": False, "error": None}
        with tempfile.TemporaryDirectory() as directory, patch.object(
            exploits.http2, "request", return_value=result
        ) as request, patch.object(
            exploits, "container_running", return_value=False
        ):
            outcome = exploits.http2_baseline_crash(
                "docker", "http://127.0.0.1:1", Path(directory), "run1", pack
            )
            request.assert_called_once()
            sent_path = request.call_args.args[3]
            self.assertIn("__mockforge_cve=run1-baseline-exploit", sent_path)
            self.assertTrue(outcome["crash_confirmed"])
            evidence_file = json.loads(
                (Path(directory) / "baseline-exploit.json").read_text()
            )
            self.assertEqual(evidence_file["status"], 502)

    def test_baseline_without_a_crash_is_refused(self):
        import cve_lab_exploits as exploits

        pack = catalog.PACKS["CVE-2026-111497"]
        result = {"status": 422, "headers": [], "body": b"invalid brotli",
                  "reset": False, "goaway": False, "error": None}
        with tempfile.TemporaryDirectory() as directory, patch.object(
            exploits.http2, "request", return_value=result
        ), patch.object(exploits, "container_running", return_value=True):
            with self.assertRaisesRegex(RuntimeError, "decoder crash"):
                exploits.http2_baseline_crash(
                    "docker", "http://127.0.0.1:1", Path(directory), "run1", pack
                )

    def test_target_crash_reports_transport_evidence(self):
        import cve_lab_exploits as exploits

        pack = catalog.PACKS["CVE-2026-111497"]
        result = {"status": None, "headers": [], "body": b"", "reset": True,
                  "goaway": False, "error": None}
        with tempfile.TemporaryDirectory() as directory, patch.object(
            exploits.http2, "request", return_value=result
        ):
            outcome = exploits.http2_target_crash(
                "docker", "http://127.0.0.1:1", Path(directory), "run1", pack
            )
        self.assertTrue(outcome["reset"])
        self.assertIsNone(outcome["status"])
        self.assertEqual(outcome["token"], "run1-target")


class RecorderProtocolTests(unittest.TestCase):
    """One listening port serves HTTP/1.1 and cleartext h2c (prior knowledge)."""

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
    def recorder(self, **kwargs):
        with socket.socket() as reservation:
            reservation.bind(("127.0.0.1", 0))
            port = reservation.getsockname()[1]
        records = []
        with transport.receiver(
            port, "127.0.0.1", f"http://127.0.0.1:{self.backend_port}", **kwargs
        ) as records:
            yield f"http://127.0.0.1:{port}", records

    def test_h1_requests_still_forward_and_observe(self):
        with self.recorder() as (base, records):
            status, _headers, body = transport.request(
                base, "/login?__mockforge_cve=tok1", {"X-Mockforge-Cve": "tok1"}
            )
        self.assertEqual(status, 201)
        self.assertEqual(body, b"ok")
        self.assertIn("tok1", records[0]["tokens"])

    def test_h2c_requests_forward_and_observe_on_the_same_port(self):
        import cve_lab_http2 as http2

        with self.recorder() as (base, records):
            url = transport.target_url(base)
            result = http2.request(
                url.hostname,
                url.port,
                "POST",
                "/decode?__mockforge_cve=tok2",
                [("content-type", "application/brotli"), ("x-mockforge-cve", "tok2")],
                b"payload-bytes",
                timeout=10,
            )
        self.assertIsNone(result["error"])
        self.assertEqual(result["status"], 201)
        self.assertEqual(self.seen[0][2], b"payload-bytes")
        self.assertIn("tok2", records[0]["tokens"])
        self.assertEqual(records[0]["method"], "POST")

    def test_h2c_oversized_bodies_are_rejected_before_forwarding(self):
        import cve_lab_http2 as http2

        with self.recorder(max_body=8) as (base, records):
            url = transport.target_url(base)
            result = http2.request(
                url.hostname,
                url.port,
                "POST",
                "/decode",
                [],
                b"0123456789abcdef",
                timeout=10,
            )
        self.assertEqual(result["status"], 413)
        self.assertEqual(self.seen, [])
        self.assertEqual(records[0]["status"], 413)

    def test_signature_header_is_recorded_only_when_configured(self):
        import cve_lab_http2 as http2

        with self.recorder(signature_header="x-middleware-subrequest") as (base, records):
            url = transport.target_url(base)
            result = http2.request(
                url.hostname,
                url.port,
                "GET",
                "/?__mockforge_cve=tok3",
                [("x-mockforge-cve", "tok3"), ("x-middleware-subrequest", "middleware:chain")],
                timeout=10,
            )
        self.assertEqual(result["status"], 201)
        self.assertEqual(records[0]["signature"], "middleware:chain")

    def test_control_requests_follow_the_pack_paths(self):
        pack = {
            "controls": {
                "benign_path": "/health?__mockforge_cve={token}",
                "benign_status": 200,
                "ordinary_path": "/?__mockforge_cve={token}",
                "ordinary_status": 200,
            }
        }
        with self.recorder() as (base, records):
            outcome = transport.control(pack, base, "run1", "phase")
        paths = [item[1] for item in self.seen]
        self.assertEqual(len(paths), 2)
        self.assertTrue(paths[0].startswith("/health?__mockforge_cve=run1-phase-"))
        ordinary_token = paths[1].split("__mockforge_cve=")[1]
        self.assertTrue(ordinary_token.startswith("run1-phase-"))
        self.assertTrue(ordinary_token.endswith("-root"))
        self.assertEqual(outcome["root_token"], ordinary_token)
        self.assertEqual(len(records), 2)

    def test_real_curl_h2c_interops_with_the_recorder(self):
        curl = shutil.which("curl")
        if not curl:
            self.skipTest("curl is not on PATH")
        with self.recorder() as (base, records):
            url = transport.target_url(base)
            out = subprocess.run(
                [
                    curl,
                    "-s",
                    "--http2-prior-knowledge",
                    "-X",
                    "POST",
                    "-H",
                    "x-mockforge-cve: tokcurl",
                    "--data-binary",
                    "curl-payload",
                    "-w",
                    "%{http_code}",
                    f"http://127.0.0.1:{url.port}/decode?__mockforge_cve=tokcurl",
                ],
                capture_output=True,
                text=True,
                timeout=20,
            )
        self.assertTrue(out.stdout.endswith("201"), out.stdout + out.stderr)
        self.assertEqual(self.seen[0][2], b"curl-payload")
        self.assertIn("tokcurl", records[0]["tokens"])


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


if __name__ == "__main__":
    unittest.main()
