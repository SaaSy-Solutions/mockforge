"""Managed lab verdict and lifecycle contracts; no container engine required."""

import contextlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
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
        with patch.object(
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
        with patch.object(
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
                    with engine.lab("docker", "our-run", Path(directory)):
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
        self.baseline = {"status": 200, "dashboard": True}
        self.target = {"token": "attack", "status": 403, "dashboard": False}
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
            {"status": 403, "dashboard": True},
            {"status": 200, "dashboard": False},
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
                nuclei.probe("docker", "http://127.0.0.1:1", output, "baseline", "run1")
                args = command.call_args.args[0]
                self.assertNotIn("-insecure", args)

                command.reset_mock()
                nuclei.probe(
                    "docker", "http://127.0.0.1:1", output, "baseline", "run1", insecure=True
                )
                self.assertIn("-insecure", command.call_args.args[0])


if __name__ == "__main__":
    unittest.main()
