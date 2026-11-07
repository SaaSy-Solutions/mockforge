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
        return self.outcome()[0]

    def outcome(self):
        return evidence.classify(
            self.baseline, self.target, self.before, self.after, self.records, 403
        )

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
                    "outcome": "not satisfied — reached the origin with status 200",
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
        self.baseline = {"status": 403, "dashboard": True}
        _verdict, _reason, checks = self.outcome()
        self.assertEqual(len(checks), 1)


if __name__ == "__main__":
    unittest.main()
