"""Managed lab verdict and lifecycle contracts; no container engine required."""

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
        return evidence.classify(
            self.baseline, self.target, self.before, self.after, self.records, 403
        )[0]

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


if __name__ == "__main__":
    unittest.main()
