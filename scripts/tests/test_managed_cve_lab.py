"""Managed lab verdict and lifecycle contracts; no container engine required."""

import argparse
import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import secrets
import shutil
import socket
import shutil
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
import cve_lab_exploits as exploits
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


def completed_output(stdout="", stderr="", returncode=0):
    return subprocess.CompletedProcess([], returncode, stdout, stderr)


class LifecycleTests(unittest.TestCase):
    def test_pack_capabilities_are_narrow_and_ordered_after_cap_drop(self):
        for cve in ("CVE-2026-109618", "CVE-2026-109619"):
            with self.subTest(cve=cve), tempfile.TemporaryDirectory() as directory:
                with patch.object(
                    engine,
                    "command",
                    side_effect=[
                        completed(),
                        completed("127.0.0.1:12345\n"),
                        completed("application log"),
                    ],
                ) as command, patch.object(engine, "request", return_value=(200, [], b"ok")), patch.object(
                    engine, "owned_remove"
                ):
                    with engine.lab(
                        "docker", "our-run", Path(directory), catalog.PACKS[cve]
                    ):
                        pass
                run_args = command.call_args_list[0].args[0]
                drop = run_args.index("--cap-drop=ALL")
                add_uid = run_args.index("--cap-add=SETUID")
                add_gid = run_args.index("--cap-add=SETGID")
                self.assertLess(drop, add_uid)
                self.assertLess(drop, add_gid)
                self.assertLess(add_uid, add_gid)
                self.assertIn("--security-opt=no-new-privileges", run_args)

        with tempfile.TemporaryDirectory() as directory:
            with patch.object(
                engine, "command", side_effect=[completed(), completed("127.0.0.1:12345\n"), completed("log")]
            ) as command, patch.object(engine, "request", return_value=(200, [], b"ok")), patch.object(
                engine, "owned_remove"
            ):
                with engine.lab(
                    "docker", "our-run", Path(directory), catalog.PACKS["CVE-2025-29927"]
                ):
                    pass
            run_args = command.call_args_list[0].args[0]
            self.assertIn("--cap-drop=ALL", run_args)
            self.assertFalse(any(arg.startswith("--cap-add=") for arg in run_args))

    def test_lab_rejects_unapproved_app_capability(self):
        pack = catalog.PACKS["CVE-2026-109618"]
        pack = {**pack, "app": {**pack["app"], "cap_add": ["SYS_ADMIN"]}}
        with tempfile.TemporaryDirectory() as directory:
            with patch.object(engine, "command") as command, patch.object(
                engine, "owned_remove"
            ):
                with self.assertRaisesRegex(RuntimeError, "allowlist: SETUID, SETGID, CHOWN"):
                    with engine.lab("docker", "our-run", Path(directory), pack):
                        pass
            command.assert_not_called()

    def test_missing_port_mapping_raises_clean_diagnostic(self):
        # When the container never binds app["port"] (it exited or failed to
        # start), `docker port` returns no mapping. The engine must raise the
        # readiness RuntimeError that run() catches, not let an IndexError
        # escape as a raw traceback. General robustness fix for every pack.
        with tempfile.TemporaryDirectory() as directory:
            with patch.object(
                engine,
                "command",
                side_effect=[
                    completed(),  # run -d
                    completed(""),  # port -> no mapping
                    completed("application log"),  # logs
                ],
            ), patch.object(engine, "owned_remove"):
                with self.assertRaisesRegex(RuntimeError, "failed to start"):
                    with engine.lab(
                        "docker", "our-run", Path(directory), catalog.PACKS[catalog.CVE]
                    ):
                        pass

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
    def test_unverified_packs_are_hidden_and_refused_unless_opted_in(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            self.assertEqual(lab.main(["list"]), 0)
        packs = json.loads(output.getvalue())
        self.assertIn("CVE-2025-29927", packs)
        for cve in ("CVE-2026-109618", "CVE-2026-109619", "CVE-2026-111497"):
            self.assertNotIn(cve, packs)

        for action, hidden in (
            (action, hidden)
            for action in ("run", "prepare")
            for hidden in ("CVE-2026-109618", "CVE-2026-111497")
        ):
            err = io.StringIO()
            with self.subTest(action=action, cve=hidden), patch.object(lab, "prepare") as prepare, patch.object(lab, "run") as run, contextlib.redirect_stderr(err):
                args = [action, hidden]
                if action == "run":
                    args += ["--output-dir", "unused-output"]
                self.assertEqual(lab.main(args), 1)
                self.assertIn(
                    f"{hidden} is an unverified lab pack (its trigger has not reproduced on a lab host); pass --include-unverified to run it for development",
                    err.getvalue(),
                )
                prepare.assert_not_called()
                run.assert_not_called()

    def test_include_unverified_lists_and_runs_development_pack(self):
        for flag_order in (
            ["--include-unverified", "list"],
        ):
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                self.assertEqual(lab.main(flag_order), 0)
            packs = json.loads(output.getvalue())
            self.assertIn("CVE-2026-109618", packs)
            self.assertIn("CVE-2026-109619", packs)

        with patch.object(lab, "prepare") as prepare:
            self.assertEqual(
                lab.main(["--include-unverified", "prepare", "CVE-2026-109618"]),
                0,
            )
        prepare.assert_called_once()

        with patch.object(lab, "run", return_value=0) as run:
            self.assertEqual(
                lab.main([
                    "--include-unverified", "run", "CVE-2026-109618",
                    "--output-dir", "unused-output",
                ]),
                0,
            )
        run.assert_called_once()

    def test_include_unverified_is_helped_and_global_only(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output), self.assertRaises(SystemExit) as error:
            lab.main(["--help"])
        self.assertEqual(error.exception.code, 0)
        self.assertIn("--include-unverified", output.getvalue())

        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as error:
            lab.main(["down", "--include-unverified", "--output-dir", "unused-output"])
        self.assertEqual(error.exception.code, 2)

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

    def setUp(self):
        import tempfile

        self._temp = tempfile.TemporaryDirectory()
        self.directory1 = self._temp.name

    def tearDown(self):
        self._temp.cleanup()

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
        text = nuclei.template(catalog.PACKS["CVE-2025-29927"], "run1-baseline")
        self.assertIn("id: mockforge-nextjs-29927-observation", text)
        self.assertIn("method: GET", text)
        self.assertIn('X-Mockforge-Cve: "run1-baseline"', text)
        self.assertIn("x-middleware-subrequest: \"middleware:", text)
        self.assertIn("__mockforge_cve=run1-baseline", text)

    def test_template_renders_post_bodies_for_body_packs(self):
        import copy

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

    def test_prepare_touches_no_registry_when_images_are_present(self):
        """Offline pin (a): with both pinned images already local, prepare
        only inspects by digest - it never pulls, so later runs need no
        registry connection."""
        pack = catalog.PACKS["CVE-2025-29927"]
        # prepare's loop inspects each image twice when it is already
        # present: the pull-if-missing check and the by-digest assertion.
        with patch.object(engine, "doctor"), patch.object(
            engine,
            "command",
            return_value=completed(),
        ) as command:
            engine.prepare("docker", pack)
        self.assertEqual(len(command.call_args_list), 4)
        for call in command.call_args_list:
            self.assertEqual(call.args[0][1], "image")
            self.assertEqual(call.args[0][2], "inspect")
        pulls = [call for call in command.call_args_list if call.args[0][1] == "pull"]
        self.assertEqual(pulls, [])

    def test_lab_uses_only_the_pinned_application_image_on_loopback(self):
        """Offline pin (a): the application container runs from the pack's
        pinned image and publishes on loopback only; nothing pulls."""
        pack = catalog.PACKS["CVE-2025-29927"]
        with tempfile.TemporaryDirectory() as directory, patch.object(
            engine,
            "command",
            side_effect=[
                completed(),
                completed("127.0.0.1:12345\n"),
                completed("application log"),
            ],
        ) as command, patch.object(
            engine, "request", return_value=(200, [], b"login")
        ), patch.object(
            engine, "owned_remove"
        ):
            with engine.lab("docker", "our-run", Path(directory), pack):
                pass
            run_args = command.call_args_list[0].args[0]
            pulls = [call.args[0] for call in command.call_args_list if call.args[0][1] == "pull"]
        self.assertIn(pack["catalog"]["application_image"], run_args)
        self.assertIn("-p", run_args)
        self.assertIn("127.0.0.1::3000/tcp", run_args)
        self.assertEqual(pulls, [])

    def test_probe_uses_the_pinned_runner_image_with_update_checks_disabled(self):
        """Offline pin (a): the Nuclei client runs from the pack's pinned
        image with its update check disabled, so no registry or vendor
        connection happens during a run."""
        output = Path(self.directory1)
        pack = catalog.PACKS["CVE-2025-29927"]
        findings = json.dumps(
            {
                "template-id": "mockforge-nextjs-29927-observation",
                "response": "HTTP/1.1 200 OK\r\n",
            }
        )
        with patch.object(nuclei, "owned_remove"), patch.object(
            nuclei, "command", return_value=completed(stdout=f"{findings}\n")
        ) as command:
            nuclei.probe(
                "docker", "http://127.0.0.1:1", output, "baseline", "run1", pack
            )
        args = command.call_args.args[0]
        self.assertIn(pack["catalog"]["runner_image"], args)
        self.assertIn("-duc", args)
        self.assertNotIn("pull", args)


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
            self.assertEqual(lab.main(["--include-unverified", "list"]), 0)
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



class Node2ShellTlsVerifyTests(unittest.TestCase):
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

    def test_probe_never_passes_an_undefined_tls_flag_when_opted_in(self):
        # Pinned Nuclei 3.11.1 defines no "-insecure" flag (#1601), so opting in
        # to --skip-tls-verify adds no runner flag.
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
                self.assertNotIn("-insecure", command.call_args.args[0])

    def test_probe_failure_writes_outputs_and_reports_stdout(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            pack = catalog.PACKS[catalog.CVE]
            with patch.object(nuclei, "owned_remove"), patch.object(
                nuclei,
                "command",
                return_value=completed_output(stdout="container failed", returncode=2),
            ) as command:
                with self.assertRaises(RuntimeError) as raised:
                    nuclei.probe(
                        "docker", "http://127.0.0.1:1", output, "baseline", "run1", pack
                    )

            message = str(raised.exception)
            self.assertIn("baseline", message)
            self.assertIn("code 2", message)
            self.assertIn("container failed", message)
            self.assertIn(str(output / "baseline.log"), message)
            self.assertIn(str(output / "baseline.jsonl"), message)
            self.assertTrue((output / "baseline.log").exists())
            self.assertEqual((output / "baseline.log").read_text(), "")
            self.assertEqual((output / "baseline.jsonl").read_text(), "container failed")
            self.assertEqual(command.call_count, 3)
            self.assertFalse(command.call_args_list[-1].kwargs.get("check", True))

    def test_probe_success_still_writes_findings_and_returns_observation(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            pack = catalog.PACKS[catalog.CVE]
            finding = json.dumps(
                {
                    "template-id": "mockforge-nextjs-29927-observation",
                    "response": "HTTP/1.1 200 OK\r\n",
                }
            ) + "\n"
            with patch.object(nuclei, "owned_remove"), patch.object(
                nuclei,
                "command",
                return_value=completed_output(stdout=finding, stderr="debug output"),
            ):
                result = nuclei.probe(
                    "docker", "http://127.0.0.1:1", output, "target", "run1", pack
                )

            self.assertEqual(result["status"], 200)
            self.assertTrue((output / "target.jsonl").exists())
            self.assertEqual((output / "target.jsonl").read_text(), finding)
            self.assertEqual((output / "target.log").read_text(), "debug output")


if __name__ == "__main__":
    unittest.main()

class Php2Shell2026PackTests(unittest.TestCase):
    """The PHP2Shell 2026 packs (CVE-2026-109618/109619) are distinct from
    the CVE-2024-4577 pack and carry advisory-derived, digest-pinned input."""

    def setUp(self):
        self.packs = (
            catalog.PACKS["CVE-2026-109618"],
            catalog.PACKS["CVE-2026-109619"],
        )

    def test_packs_are_distinct_from_the_2024_windows_cgi_pack(self):
        legacy = catalog.PACKS["CVE-2024-4577"]
        for pack in self.packs:
            self.assertNotEqual(pack["catalog"]["cve"], legacy["catalog"]["cve"])
            self.assertNotEqual(pack["catalog"]["title"], legacy["catalog"]["title"])
            self.assertNotEqual(
                pack["catalog"]["application_image"],
                legacy["catalog"]["application_image"],
            )
            self.assertIn("PHP2Shell 2026", pack["catalog"]["title"])
            self.assertNotIn("Wine", pack["catalog"]["application_version"])

    def test_packs_carry_runner_config(self):
        for pack in self.packs:
            for key in ("catalog", "probe", "app", "controls", "exploit", "application_files"):
                self.assertIn(key, pack)
            # Unprivileged port: the harness runs --cap-drop=ALL, so Apache
            # cannot bind :80 (no CAP_NET_BIND_SERVICE).
            self.assertEqual(pack["app"]["port"], "8080/tcp")
            self.assertEqual(
                pack["app"]["readiness"], {"path": "/health.php", "status": 200}
            )
            self.assertTrue(pack["app"]["mounts"])
            mount_targets = {m["container"] for m in pack["app"]["mounts"]}
            self.assertIn("/etc/apache2/ports.conf", mount_targets)
            self.assertIn(
                "/etc/apache2/sites-enabled/000-default.conf", mount_targets
            )

    def test_images_are_digest_pinned_and_verification_is_declared(self):
        for pack in self.packs:
            self.assertIn("@sha256:", pack["catalog"]["application_image"])
            self.assertIn("@sha256:", pack["catalog"]["runner_image"])
            self.assertEqual(
                pack["catalog"]["verification_status"],
                "pending-lab-host-verification",
            )
        self.assertEqual(
            self.packs[0]["catalog"]["application_image"],
            self.packs[1]["catalog"]["application_image"],
        )

    def test_application_files_review_the_trigger_configuration(self):
        files = self.packs[0]["application_files"]
        self.assertEqual(files, self.packs[1]["application_files"])
        self.assertIn("memory_limit=16M", files["lab.ini"])
        self.assertIn("post_max_size=48M", files["lab.ini"])
        self.assertIn("mockforge-php2shell-2026", files["health.php"])
        self.assertIn("X-Worker-Pid", files["index.php"])
        self.assertIn("php_value memory_limit", files[".htaccess"])
        self.assertIn("Listen 8080", files["ports.conf"])
        self.assertIn("*:8080", files["000-default.conf"])

    def test_exploit_steps_declare_the_advisory_choreography(self):
        self.assertEqual(self.packs[0]["baseline_exploit"], "modphp-oom")
        self.assertEqual(self.packs[0]["target_exploit"], "modphp-oom")
        self.assertEqual(self.packs[1]["baseline_exploit"], "modphp-worker-sequence")
        self.assertEqual(self.packs[1]["target_exploit"], "modphp-worker-sequence")
        for pack in self.packs:
            self.assertGreaterEqual(
                pack["exploit"]["max_forwarded_body"], 32 * 1024 * 1024
            )
            self.assertIn("__mockforge_cve={token}", pack["exploit"]["path"])

    def test_trigger_bodies_match_the_pinned_digests(self):
        import hashlib

        import cve_lab_payloads as payloads

        self.assertEqual(
            hashlib.sha256(payloads.oom_body()).hexdigest(), payloads.OOM_BODY_SHA256
        )
        self.assertEqual(
            hashlib.sha256(payloads.oom_body_with_progress()).hexdigest(),
            payloads.OOM_BODY_WITH_PROGRESS_SHA256,
        )
        self.assertEqual(
            hashlib.sha256(payloads.consumer_body()).hexdigest(),
            payloads.CONSUMER_BODY_SHA256,
        )

    def test_producer_body_exceeds_the_reviewed_memory_limit(self):
        import cve_lab_payloads as payloads

        # memory_limit=16M in the reviewed lab configuration; the parsed
        # part headers must exceed it to reach the OOM bail-out.
        self.assertGreater(len(payloads.oom_body()), 16 * 1024 * 1024)
        self.assertGreater(len(payloads.oom_body_with_progress()), 16 * 1024 * 1024)
        self.assertLess(len(payloads.oom_body()), 48 * 1024 * 1024)
        self.assertIn(b"PHP_SESSION_UPLOAD_PROGRESS", payloads.oom_body_with_progress())
        self.assertIn(b"PHP_SESSION_UPLOAD_PROGRESS", payloads.consumer_body())
        self.assertNotIn(b"PHP_SESSION_UPLOAD_PROGRESS", payloads.oom_body())

    def test_probe_template_observes_a_healthy_request(self):
        for pack in self.packs:
            text = nuclei.template(pack, "run1-baseline")
            self.assertIn("/health.php?__mockforge_cve=run1-baseline", text)
            self.assertIn("method: GET", text)

    def test_prepare_refuses_unpinned_images_before_any_engine_call(self):
        import copy

        for pack in self.packs:
            unpinned = copy.deepcopy(pack)
            unpinned["catalog"]["application_image"] = "docker.io/library/php:8.4-apache"
            with patch.object(engine, "command") as command:
                with self.assertRaisesRegex(RuntimeError, "not pinned by digest"):
                    engine.prepare("docker", unpinned)
            command.assert_not_called()

    def test_run_of_the_packs_stops_at_the_platform_gate_honestly(self):
        # Simulate the platform gate directly so this remains engine-free on
        # hosts where Docker is available.
        stdout = io.StringIO()
        with tempfile.TemporaryDirectory() as directory, contextlib.redirect_stdout(
            stdout
        ), patch.object(engine, "doctor", side_effect=RuntimeError("unsupported platform")), patch.object(engine, "command") as command:
            output = Path(directory) / "out"
            self.assertEqual(
                lab.main([
                    "--include-unverified", "run", "CVE-2026-109618",
                    "--output-dir", str(output),
                ]), 1
            )
            report = json.loads((output / "report.json").read_text())
        command.assert_not_called()
        self.assertEqual(report["verdict"], "inconclusive")


class Php2Shell2026GateTests(unittest.TestCase):
    """The mod_php gate demands healthy controls and the observed marker."""

    def setUp(self):
        self.pack = catalog.PACKS["CVE-2026-109618"]
        self.control = {"status": 200, "root_status": 200}
        self.baseline = {"status": 200, "marker": True, "crash_confirmed": True}

    def test_gate_accepts_a_reproduced_direct_run(self):
        evidence.BASELINE_GATES["modphp-segfault"](
            self.pack, self.control, self.baseline
        )

    def test_gate_rejects_mismatched_direct_runs(self):
        for control, baseline in (
            ({"status": 500, "root_status": 200}, self.baseline),
            ({"status": 200, "root_status": 404}, self.baseline),
            ({"status": 200, "root_status": 200}, {"status": 200, "marker": False}),
        ):
            with self.subTest(control=control):
                with self.assertRaisesRegex(
                    RuntimeError, "request-lifecycle corruption"
                ):
                    evidence.BASELINE_GATES["modphp-segfault"](
                        self.pack, control, baseline
                    )


class Php2Shell2026VerdictTests(unittest.TestCase):
    """The mod_php classifier mirrors the blocking semantics for both packs."""

    def setUp(self):
        self.pack = catalog.PACKS["CVE-2026-109618"]
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
        return evidence.CLASSIFIERS["modphp-segfault"](
            self.pack, self.baseline, self.target, self.before, self.after, self.records, 403
        )

    def verdict(self):
        return self.outcome()[0]

    def test_blocked_exploit_with_healthy_controls_passes(self):
        self.assertEqual(self.verdict(), "pass")

    def test_baseline_must_show_the_confirmed_corruption(self):
        for baseline in (
            {"status": 200, "marker": True},
            {"status": 200, "marker": True, "crash_confirmed": False},
            {"status": 200, "marker": False, "crash_confirmed": True},
        ):
            with self.subTest(baseline=baseline):
                self.baseline = baseline
                self.assertEqual(self.verdict(), "inconclusive")

    def test_failed_before_control_is_inconclusive(self):
        self.before["status"] = 503
        self.assertEqual(self.verdict(), "inconclusive")

    def test_exploit_reaching_origin_fails_even_when_it_crashed_the_worker(self):
        self.records.append(
            {"tokens": ["attack"], "status": None, "error": "ConnectionReset"}
        )
        self.after = {
            "token": self.after["token"],
            "root_token": self.after["root_token"],
            "error": "connection refused",
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
            "error": "connection refused",
        }
        self.assertEqual(self.verdict(), "inconclusive")

    def test_blanket_block_of_the_ordinary_path_is_inconclusive(self):
        self.before["root_status"] = 403
        self.assertEqual(self.verdict(), "inconclusive")

    def test_uncorrelated_origin_request_cannot_pass(self):
        self.records.append({"tokens": [None, None], "status": 200})
        self.assertEqual(self.verdict(), "inconclusive")


class Php2Shell2026ExploitStepTests(unittest.TestCase):
    """The mod_php steps confirm baselines from the segfault log marker."""

    def setUp(self):
        import cve_lab_exploits as exploits

        self.exploits = exploits
        self.pack = catalog.PACKS["CVE-2026-109618"]

    def test_baseline_confirms_from_the_log_marker_delta(self):
        result = {"status": 200, "error": None, "worker_pid": "42", "body_preview": "upload processed"}
        with tempfile.TemporaryDirectory() as directory, patch.object(
            self.exploits, "_post_tolerant", return_value=result
        ), patch.object(
            self.exploits,
            "container_log",
            side_effect=["log", "log crash exit signal Segmentation fault (11)"],
        ):
            outcome = self.exploits.modphp_oom_baseline(
                "docker", "http://127.0.0.1:1", Path(directory), "run1", self.pack
            )
            self.assertTrue(outcome["crash_confirmed"])
            evidence_file = json.loads(
                (Path(directory) / "baseline-exploit.json").read_text()
            )
        self.assertEqual(evidence_file["segfault_markers_before"], 0)
        self.assertEqual(evidence_file["segfault_markers_after"], 1)
        self.assertTrue(evidence_file["segfault_confirmed"])

    def test_baseline_without_a_marker_delta_is_refused(self):
        result = {"status": 200, "error": None, "worker_pid": "42", "body_preview": "upload processed"}
        with tempfile.TemporaryDirectory() as directory, patch.object(
            self.exploits, "_post_tolerant", return_value=result
        ), patch.object(
            self.exploits, "container_log", side_effect=["log", "log"]
        ):
            with self.assertRaisesRegex(RuntimeError, "request-lifecycle corruption"):
                self.exploits.modphp_oom_baseline(
                    "docker", "http://127.0.0.1:1", Path(directory), "run1", self.pack
                )

    def test_sequence_baseline_confirms_and_records_worker_identity(self):
        pack = catalog.PACKS["CVE-2026-109619"]
        phases = [
            {"path": "/upload.php", "status": None, "error": "ConnectionReset", "worker_pid": None},
            {"path": "/upload.php", "status": 200, "error": None, "worker_pid": "42"},
        ]
        with tempfile.TemporaryDirectory() as directory, patch.object(
            self.exploits, "_post_sequence", return_value=phases
        ), patch.object(
            self.exploits,
            "container_log",
            side_effect=["log", "log crash exit signal Segmentation fault (11)"],
        ):
            outcome = self.exploits.modphp_sequence_baseline(
                "docker", "http://127.0.0.1:1", Path(directory), "run1", pack
            )
            self.assertTrue(outcome["crash_confirmed"])
            evidence_file = json.loads(
                (Path(directory) / "baseline-exploit.json").read_text()
            )
        self.assertEqual(evidence_file["worker_pids_seen"], ["42"])
        self.assertEqual(evidence_file["same_worker_observed"], True)
        self.assertTrue(evidence_file["segfault_confirmed"])

    def test_target_steps_report_transport_evidence(self):
        result = {"status": 403, "error": None, "worker_pid": "7", "body_preview": ""}
        with tempfile.TemporaryDirectory() as directory, patch.object(
            self.exploits, "_post_tolerant", return_value=result
        ):
            outcome = self.exploits.modphp_oom_target(
                "docker", "http://127.0.0.1:1", Path(directory), "run1", self.pack
            )
        self.assertEqual(outcome["status"], 403)
        self.assertEqual(outcome["token"], "run1-target")

    def test_sequence_steps_send_producer_then_consumer_on_one_connection(self):
        import cve_lab_payloads as payloads

        pack = catalog.PACKS["CVE-2026-109619"]
        seen_phases = []

        def record_phases(host, port, phases, timeout):
            seen_phases.extend(phases)
            return [
                {"path": phases[0][0], "status": 200, "error": None, "worker_pid": "7"},
                {"path": phases[1][0], "status": 403, "error": None, "worker_pid": "7"},
            ]

        with tempfile.TemporaryDirectory() as directory, patch.object(
            self.exploits, "_post_sequence", side_effect=record_phases
        ):
            self.exploits.modphp_sequence_target(
                "docker", "http://127.0.0.1:1", Path(directory), "run1", pack
            )
        self.assertEqual(len(seen_phases), 2)
        first_path, _headers, first_body = seen_phases[0]
        second_path, _headers, second_body = seen_phases[1]
        self.assertEqual(first_path, second_path)
        self.assertEqual(first_body, payloads.oom_body_with_progress())
        self.assertEqual(second_body, payloads.consumer_body())


def pinned_runner_engine():
    """A local engine that already holds the pack's pinned runner image, or None.

    Decides with the same by-digest ``image inspect`` ``prepare`` performs and
    NEVER pulls: a machine without the image skips instead of downloading.
    Runs each inspect with the shared temp dir as cwd because rootless engines
    cannot start from a cwd they cannot see (e.g. this host's /mnt/build).
    """
    image = catalog.PACKS[catalog.CVE]["catalog"]["runner_image"]
    for candidate in ("docker", "podman"):
        if shutil.which(candidate) is None:
            continue
        result = subprocess.run(
            [candidate, "image", "inspect", image],
            capture_output=True,
            text=True,
            check=False,
            cwd=tempfile.gettempdir(),
        )
        if result.returncode == 0:
            return candidate
    return None


RUNNER_ENGINE = pinned_runner_engine()
NO_RUNNER_IMAGE_SKIP = (
    "no local container engine (docker/podman) already holds the pinned Nuclei "
    "runner image; the test never pulls"
)
NO_CERT_TOOL_SKIP = (
    "openssl is not on PATH, so no throwaway self-signed certificate can be minted"
)



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

    def test_probe_never_passes_an_undefined_tls_flag_when_opted_in(self):
        """The pinned Nuclei 3.11.1 defines no "-insecure" flag and does not
        verify TLS certificates at all, so --skip-tls-verify must add no
        runner flag: baseline and target probes keep the same valid flag set
        with and without the opt-in."""
        import cve_lab_exploits as exploits

        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            pack = catalog.PACKS[catalog.CVE]
            findings = json.dumps(
                {
                    "template-id": "mockforge-nextjs-29927-observation",
                    "response": "HTTP/1.1 200 OK\r\n",
                }
            )
            commands = {}
            with patch.object(nuclei, "owned_remove"), patch.object(
                nuclei,
                "command",
                return_value=completed(stdout=f"{findings}\n"),
            ) as command:
                for label, call in (
                    (
                        "baseline-default",
                        lambda: nuclei.probe(
                            "docker", "http://127.0.0.1:1", output, "baseline", "run1", pack
                        ),
                    ),
                    (
                        "baseline-insecure",
                        lambda: nuclei.probe(
                            "docker",
                            "http://127.0.0.1:1",
                            output,
                            "baseline",
                            "run1",
                            pack,
                            insecure=True,
                        ),
                    ),
                    (
                        "target-default",
                        lambda: exploits.nuclei_target(
                            "docker", "http://127.0.0.1:1", output, "run1", pack
                        ),
                    ),
                    (
                        "target-insecure",
                        lambda: exploits.nuclei_target(
                            "docker", "http://127.0.0.1:1", output, "run1", pack, insecure=True
                        ),
                    ),
                ):
                    command.reset_mock()
                    call()
                    commands[label] = list(command.call_args.args[0])

            for label, args in commands.items():
                self.assertNotIn("-insecure", args, label)
                self.assertNotIn("-insecure", " ".join(args), label)
            self.assertEqual(commands["baseline-default"], commands["baseline-insecure"])
            self.assertEqual(commands["target-default"], commands["target-insecure"])

    def test_probe_failure_writes_outputs_and_reports_stdout(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            pack = catalog.PACKS[catalog.CVE]
            with patch.object(nuclei, "owned_remove"), patch.object(
                nuclei,
                "command",
                return_value=completed_output(stdout="container failed", returncode=2),
            ) as command:
                with self.assertRaises(RuntimeError) as raised:
                    nuclei.probe(
                        "docker", "http://127.0.0.1:1", output, "baseline", "run1", pack
                    )

            message = str(raised.exception)
            self.assertIn("baseline", message)
            self.assertIn("code 2", message)
            self.assertIn("container failed", message)
            self.assertIn(str(output / "baseline.log"), message)
            self.assertIn(str(output / "baseline.jsonl"), message)
            self.assertTrue((output / "baseline.log").exists())
            self.assertEqual((output / "baseline.log").read_text(), "")
            self.assertEqual((output / "baseline.jsonl").read_text(), "container failed")
            self.assertEqual(command.call_count, 3)
            self.assertFalse(command.call_args_list[-1].kwargs.get("check", True))

    def test_probe_creates_copies_starts_and_cleans_up(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            pack = catalog.PACKS[catalog.CVE]
            finding = json.dumps(
                {
                    "template-id": pack["probe"]["template_id"],
                    "response": "HTTP/1.1 200 OK\r\n",
                }
            ) + "\n"
            with patch.object(nuclei, "owned_remove") as remove, patch.object(
                nuclei,
                "command",
                side_effect=[completed(), completed(), completed_output(stdout=finding)],
            ) as command:
                nuclei.probe("docker", "http://127.0.0.1:1", output, "baseline", "run1", pack)

            calls = [call.args[0] for call in command.call_args_list]
            create, copy, start = calls
            self.assertEqual(create[1], "create")
            self.assertIn("--label", create)
            self.assertIn(f"{nuclei.LABEL}=run1", create)
            for flag in (
                "--network=host",
                "--cap-drop=ALL",
                "--security-opt=no-new-privileges",
                "--memory=512m",
                "--pids-limit=128",
            ):
                self.assertIn(flag, create)
            self.assertNotIn("-v", create)
            self.assertEqual(create[create.index("-t") + 1], "/tmp/baseline.yaml")
            self.assertEqual(
                copy,
                [
                    "docker",
                    "cp",
                    str(output / "baseline.yaml"),
                    "mockforge-cve-client-run1-baseline:/tmp/baseline.yaml",
                ],
            )
            self.assertEqual(
                start,
                ["docker", "start", "-a", "mockforge-cve-client-run1-baseline"],
            )
            self.assertEqual(command.call_args_list[0].kwargs["timeout"], 90)
            self.assertFalse(command.call_args_list[-1].kwargs["check"])
            remove.assert_called_once_with("docker", "mockforge-cve-client-run1-baseline", "run1")

    def test_probe_copy_failure_cleans_up_and_raises_clear_error(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            pack = catalog.PACKS[catalog.CVE]
            with patch.object(nuclei, "owned_remove") as remove, patch.object(
                nuclei,
                "command",
                side_effect=[completed(), RuntimeError("docker cp failed: no such file")],
            ) as command:
                with self.assertRaisesRegex(RuntimeError, "failed to copy Nuclei template"):
                    nuclei.probe(
                        "docker", "http://127.0.0.1:1", output, "baseline", "run1", pack
                    )
            self.assertEqual(
                [call.args[0][1] for call in command.call_args_list], ["create", "cp"]
            )
            remove.assert_called_once_with("docker", "mockforge-cve-client-run1-baseline", "run1")

    def test_probe_start_failure_cleans_up(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            pack = catalog.PACKS[catalog.CVE]
            with patch.object(nuclei, "owned_remove") as remove, patch.object(
                nuclei,
                "command",
                side_effect=[completed(), completed(), completed_output(stderr="failed", returncode=3)],
            ) as command:
                with self.assertRaisesRegex(RuntimeError, "Nuclei exited with code 3"):
                    nuclei.probe(
                        "docker", "http://127.0.0.1:1", output, "baseline", "run1", pack
                    )
            self.assertEqual(
                [call.args[0][1] for call in command.call_args_list],
                ["create", "cp", "start"],
            )
            remove.assert_called_once_with("docker", "mockforge-cve-client-run1-baseline", "run1")

    def test_probe_success_still_writes_findings_and_returns_observation(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            pack = catalog.PACKS[catalog.CVE]
            finding = json.dumps(
                {
                    "template-id": "mockforge-nextjs-29927-observation",
                    "response": "HTTP/1.1 200 OK\r\n",
                }
            ) + "\n"
            with patch.object(nuclei, "owned_remove"), patch.object(
                nuclei,
                "command",
                return_value=completed_output(stdout=finding, stderr="debug output"),
            ):
                result = nuclei.probe(
                    "docker", "http://127.0.0.1:1", output, "target", "run1", pack
                )

            self.assertEqual(result["status"], 200)
            self.assertTrue((output / "target.jsonl").exists())
            self.assertEqual((output / "target.jsonl").read_text(), finding)
            self.assertEqual((output / "target.log").read_text(), "debug output")

    @unittest.skipUnless(RUNNER_ENGINE, NO_RUNNER_IMAGE_SKIP)
    @unittest.skipUnless(shutil.which("openssl"), NO_CERT_TOOL_SKIP)
    def test_real_pinned_runner_observes_self_signed_https_when_opted_in(self):
        """Regression against the real pinned runner: ``probe()`` used to append
        "-insecure" on --skip-tls-verify, a flag Nuclei 3.11.1 does not define,
        so every opted-in probe died at flag parsing ("flag provided but not
        defined: -insecure") before touching the network. The pinned image
        never verifies certificates, so with the fix it must complete one
        observation against a self-signed HTTPS target."""
        from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
        import ssl

        pack = catalog.PACKS[catalog.CVE]
        marker = pack["probe"]["marker"]
        seen = []

        class Handler(BaseHTTPRequestHandler):
            protocol_version = "HTTP/1.1"

            def do_GET(self):
                seen.append((self.command, self.path, dict(self.headers)))
                body = f"{marker}\n".encode()
                self.send_response(200)
                self.send_header("Content-Type", "text/plain; charset=utf-8")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

            def log_message(self, *_args):
                pass

        with (
            tempfile.TemporaryDirectory() as certs,
            tempfile.TemporaryDirectory() as directory,
        ):
            certificate = Path(certs) / "cert.pem"
            private_key = Path(certs) / "key.pem"
            subprocess.run(
                [
                    "openssl",
                    "req",
                    "-x509",
                    "-newkey",
                    "rsa:2048",
                    "-sha256",
                    "-days",
                    "2",
                    "-nodes",
                    "-subj",
                    "/CN=127.0.0.1",
                    "-addext",
                    "subjectAltName=IP:127.0.0.1",
                    "-keyout",
                    str(private_key),
                    "-out",
                    str(certificate),
                ],
                capture_output=True,
                text=True,
                check=True,
            )
            tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
            tls.load_cert_chain(str(certificate), str(private_key))
            server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
            server.socket = tls.wrap_socket(server.socket, server_side=True)
            threading.Thread(target=server.serve_forever, daemon=True).start()
            self.addCleanup(server.shutdown)
            self.addCleanup(server.server_close)

            output = Path(directory)
            run_id = "tls" + secrets.token_hex(4)
            token = f"{run_id}-baseline"
            # The engine subprocess inherits this cwd, and rootless engines
            # cannot start from a cwd they cannot see (this host's /mnt/build),
            # so serve them the temp dir.
            previous_cwd = os.getcwd()
            os.chdir(directory)
            try:
                observation = nuclei.probe(
                    RUNNER_ENGINE,
                    f"https://127.0.0.1:{server.server_address[1]}",
                    output,
                    "baseline",
                    run_id,
                    pack,
                    insecure=True,
                )
            finally:
                os.chdir(previous_cwd)

            self.assertEqual(observation["token"], token)
            self.assertEqual(observation["status"], 200)
            self.assertTrue(observation["marker"])
            self.assertRegex(observation["response_sha256"], r"^[0-9a-f]{64}$")
            self.assertEqual(
                [entry[0] for entry in seen],
                ["GET"],
                "the runner must make exactly one probe request over TLS",
            )
            request_path, request_headers = seen[0][1], seen[0][2]
            self.assertEqual(request_path, f"/?__mockforge_cve={token}")
            self.assertEqual(request_headers.get("X-Mockforge-Cve"), token)
            self.assertEqual(
                request_headers.get("x-middleware-subrequest"),
                pack["probe"]["headers"]["x-middleware-subrequest"],
            )


class DataDrivenCaseTests(unittest.TestCase):
    """The data-driven case mechanism: extra cases are pack data, checked
    generically, with no per-case code."""

    def synthetic_pack(self):
        return {
            "catalog": {
                "schema_version": 1,
                "pack_revision": 1,
                "cve": "CVE-0000-0001",
                "title": "Synthetic data-driven pack",
                "platform": "test",
                "application_version": "synthetic application 1.0",
                "application_image": "docker.io/example/app@sha256:" + "0" * 64,
                "runner_version": "Nuclei 3.11.1",
                "runner_image": "docker.io/projectdiscovery/nuclei@sha256:" + "5" * 64,
                "references": [],
                "coverage": "synthetic",
                "origin_port": 18082,
                "listen_address_guidance": "synthetic",
            },
            "probe": {
                "template_id": "mockforge-synthetic-observation",
                "template_name": "MockForge synthetic observation",
                "method": "GET",
                "path": "/?__mockforge_cve={token}",
                "headers": {"X-Mockforge-Cve": "{token}"},
                "body": None,
                "signature_header": None,
                "marker": "synthetic-marker",
            },
            "app": {
                "port": "3000/tcp",
                "env": {},
                "readiness": {"path": "/", "status": 200},
            },
            "controls": {
                "benign_path": "/benign?__mockforge_cve={token}",
                "benign_status": 200,
                "ordinary_path": "/?__mockforge_cve={token}",
                "ordinary_status": 200,
            },
            "extra_cases": [
                {
                    "id": "robots",
                    "path": "/robots.txt?__mockforge_cve={token}",
                    "expect_status": 200,
                },
                {
                    "id": "apiversion",
                    "method": "GET",
                    "path": "/api/version?__mockforge_cve={token}",
                    "headers": {"Accept": "application/json"},
                    "expect_status": 404,
                },
            ],
            "baseline_gate": "data-driven",
            "classifier": "data-driven",
        }

    def test_control_sends_extra_cases_and_returns_their_outcomes(self):
        from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

        seen = []

        class Backend(BaseHTTPRequestHandler):
            def _reply(self):
                seen.append((self.command, self.path, dict(self.headers)))
                self.send_response(404 if "api/version" in self.path else 200)
                self.send_header("Content-Length", "2")
                self.end_headers()
                self.wfile.write(b"ok")

            do_GET = _reply

            def log_message(self, *_args):
                pass

        backend = ThreadingHTTPServer(("127.0.0.1", 0), Backend)
        threading.Thread(target=backend.serve_forever, daemon=True).start()
        self.addCleanup(backend.shutdown)
        self.addCleanup(backend.server_close)
        with socket.socket() as reservation:
            reservation.bind(("127.0.0.1", 0))
            recorder_port = reservation.getsockname()[1]
        records = []
        with transport.receiver(
            recorder_port,
            "127.0.0.1",
            f"http://127.0.0.1:{backend.server_address[1]}",
        ) as records:
            outcome = transport.control(
                self.synthetic_pack(), f"http://127.0.0.1:{recorder_port}", "run1", "phase"
            )
        self.assertEqual(outcome["status"], 200)
        self.assertEqual(outcome["root_status"], 200)
        self.assertEqual(set(outcome["extras"]), {"robots", "apiversion"})
        self.assertEqual(outcome["extras"]["robots"]["status"], 200)
        self.assertEqual(outcome["extras"]["apiversion"]["status"], 404)
        self.assertTrue(
            outcome["extras"]["robots"]["token"].startswith("run1-phase-")
        )
        self.assertIn("-robots", outcome["extras"]["robots"]["token"])
        # every request reached the backend with its own correlation token
        self.assertEqual(len(seen), 4)
        api = [item for item in seen if "api/version" in item[1]][0]
        self.assertEqual(
            api[2].get("X-Mockforge-Cve"), outcome["extras"]["apiversion"]["token"]
        )
        self.assertEqual(api[2].get("Accept"), "application/json")
        # the recorder observed all four with tokens
        self.assertEqual(len(records), 4)
        robots_records = [
            record
            for record in records
            if outcome["extras"]["robots"]["token"] in record["tokens"]
        ]
        self.assertEqual(len(robots_records), 1)
        self.assertEqual(robots_records[0]["status"], 200)

    def test_extra_case_evidence_verifies_status_and_origin_observation(self):
        pack = self.synthetic_pack()
        outcome = {
            "extras": {
                "robots": {"token": "tok-robots", "status": 200},
                "apiversion": {"token": "tok-api", "status": 404},
            }
        }
        records = [
            {"tokens": ["tok-robots"], "status": 200},
            {"tokens": ["tok-api"], "status": 404},
        ]
        entries, ok, tokens = evidence.extra_case_evidence(pack, outcome, records)
        self.assertTrue(ok)
        self.assertEqual(len(entries), 2)
        self.assertTrue(all(entry["outcome"] == "ok" for entry in entries))
        self.assertEqual(tokens, {"tok-robots", "tok-api"})

    def test_extra_case_evidence_fails_on_wrong_status(self):
        pack = self.synthetic_pack()
        outcome = {
            "extras": {
                "robots": {"token": "tok-robots", "status": 403},
                "apiversion": {"token": "tok-api", "status": 404},
            }
        }
        records = [
            {"tokens": ["tok-robots"], "status": 403},
            {"tokens": ["tok-api"], "status": 404},
        ]
        entries, ok, _tokens = evidence.extra_case_evidence(pack, outcome, records)
        self.assertFalse(ok)
        robots = [entry for entry in entries if "robots" in entry["check"]][0]
        self.assertIn("origin status 403", robots["outcome"])

    def test_extra_case_evidence_fails_when_the_origin_never_saw_it(self):
        pack = self.synthetic_pack()
        outcome = {
            "extras": {
                "robots": {"token": "tok-robots", "status": 200},
                "apiversion": {"token": "tok-api", "status": 404},
            }
        }
        entries, ok, _tokens = evidence.extra_case_evidence(pack, outcome, [])
        self.assertFalse(ok)
        robots = [entry for entry in entries if "robots" in entry["check"]][0]
        self.assertIn("no origin observation", robots["outcome"])

    def test_extra_case_evidence_fails_on_transport_error(self):
        pack = self.synthetic_pack()
        outcome = {
            "extras": {
                "robots": {"token": "tok-robots", "error": "connection refused"},
                "apiversion": {"token": "tok-api", "status": 404},
            }
        }
        records = [{"tokens": ["tok-api"], "status": 404}]
        entries, ok, _tokens = evidence.extra_case_evidence(pack, outcome, records)
        self.assertFalse(ok)
        robots = [entry for entry in entries if "robots" in entry["check"]][0]
        self.assertIn("connection refused", robots["outcome"])

    def test_extra_case_evidence_is_vacuous_without_declared_cases(self):
        import copy

        pack = self.synthetic_pack()
        del pack["extra_cases"]
        entries, ok, tokens = evidence.extra_case_evidence(pack, {}, [])
        self.assertEqual(entries, [])
        self.assertTrue(ok)
        self.assertEqual(tokens, set())
        # legacy entry point (pack=None) stays vacuous too
        entries, ok, tokens = evidence.extra_case_evidence(None, {}, [])
        self.assertEqual((entries, ok, tokens), ([], True, set()))

    def outcome_for(self, pack, **overrides):
        baseline = overrides.get(
            "baseline", {"status": 200, "marker": True}
        )
        target = overrides.get("target", {"token": "attack", "status": 403})
        before = overrides.get(
            "before",
            {
                "token": "before",
                "status": 200,
                "root_token": "before-root",
                "root_status": 200,
                "extras": {
                    "robots": {"token": "before-robots", "status": 200},
                    "apiversion": {"token": "before-api", "status": 404},
                },
            },
        )
        after = overrides.get(
            "after",
            {
                "token": "after",
                "status": 200,
                "root_token": "after-root",
                "root_status": 200,
                "extras": {
                    "robots": {"token": "after-robots", "status": 200},
                    "apiversion": {"token": "after-api", "status": 404},
                },
            },
        )
        records = overrides.get(
            "records",
            [
                {"tokens": ["before"], "status": 200},
                {"tokens": ["before-root"], "status": 200},
                {"tokens": ["before-robots"], "status": 200},
                {"tokens": ["before-api"], "status": 404},
                {"tokens": ["after"], "status": 200},
                {"tokens": ["after-root"], "status": 200},
                {"tokens": ["after-robots"], "status": 200},
                {"tokens": ["after-api"], "status": 404},
            ],
        )
        return evidence.CLASSIFIERS["data-driven"](
            pack, baseline, target, before, after, records, 403
        )

    def test_data_driven_classifier_passes_a_healthy_blocked_run(self):
        pack = self.synthetic_pack()
        verdict, reason, checks = self.outcome_for(pack)
        self.assertEqual(verdict, "pass")
        names = [check["check"] for check in checks]
        self.assertIn("extra case robots answered 200", names)
        self.assertIn("extra case apiversion answered 404", names)
        self.assertTrue(all(check["outcome"] == "ok" for check in checks))

    def test_data_driven_classifier_fails_when_the_signature_reaches_the_origin(self):
        pack = self.synthetic_pack()
        records = [
            {"tokens": ["before"], "status": 200},
            {"tokens": ["before-root"], "status": 200},
            {"tokens": ["before-robots"], "status": 200},
            {"tokens": ["before-api"], "status": 404},
            {"tokens": ["attack"], "status": 200},
        ]
        verdict, reason, checks = self.outcome_for(pack, records=records)
        self.assertEqual(verdict, "fail")
        self.assertIn("reached the origin", reason)

    def test_data_driven_classifier_is_inconclusive_on_a_wrong_extra_status(self):
        pack = self.synthetic_pack()
        before = {
            "token": "before",
            "status": 200,
            "root_token": "before-root",
            "root_status": 200,
            "extras": {
                "robots": {"token": "before-robots", "status": 500},
                "apiversion": {"token": "before-api", "status": 404},
            },
        }
        records = [
            {"tokens": ["before"], "status": 200},
            {"tokens": ["before-root"], "status": 200},
            {"tokens": ["before-robots"], "status": 500},
            {"tokens": ["before-api"], "status": 404},
            {"tokens": ["after"], "status": 200},
            {"tokens": ["after-root"], "status": 200},
            {"tokens": ["after-robots"], "status": 200},
            {"tokens": ["after-api"], "status": 404},
        ]
        verdict, reason, checks = self.outcome_for(pack, before=before, records=records)
        self.assertEqual(verdict, "inconclusive")
        self.assertIn("extra case", reason)

    def test_data_driven_classifier_is_inconclusive_on_a_wrong_block_status(self):
        pack = self.synthetic_pack()
        target = {"token": "attack", "status": 502}
        verdict, _reason, _checks = self.outcome_for(pack, target=target)
        self.assertEqual(verdict, "inconclusive")

    def test_data_driven_classifier_is_inconclusive_when_the_baseline_marker_is_absent(
        self,
    ):
        pack = self.synthetic_pack()
        verdict, _reason, _checks = self.outcome_for(
            pack, baseline={"status": 200, "marker": False}
        )
        self.assertEqual(verdict, "inconclusive")

    def test_data_driven_classifier_correlates_extra_tokens(self):
        # an uncorrelated record (no expected token) blocks a pass
        pack = self.synthetic_pack()
        records = [
            {"tokens": ["before"], "status": 200},
            {"tokens": ["before-root"], "status": 200},
            {"tokens": ["before-robots"], "status": 200},
            {"tokens": ["before-api"], "status": 404},
            {"tokens": ["after"], "status": 200},
            {"tokens": ["after-root"], "status": 200},
            {"tokens": ["after-robots"], "status": 200},
            {"tokens": ["after-api"], "status": 404},
            {"tokens": [None, None], "status": 200},
        ]
        verdict, _reason, _checks = self.outcome_for(pack, records=records)
        self.assertEqual(verdict, "inconclusive")

    def test_existing_classifiers_add_no_checks_without_declared_cases(self):
        # the shipped packs must be untouched by the mechanism
        next_pack = catalog.PACKS["CVE-2025-29927"]
        self.assertNotIn("extra_cases", next_pack)
        baseline = {"status": 200, "marker": True}
        target = {"token": "attack", "status": 403}
        before = {
            "token": "before",
            "status": 200,
            "root_token": "before-root",
            "root_status": 307,
        }
        after = {
            "token": "after",
            "status": 200,
            "root_token": "after-root",
            "root_status": 307,
        }
        records = [
            {"tokens": ["before"], "status": 200},
            {"tokens": ["before-root"], "status": 307},
            {"tokens": ["after"], "status": 200},
            {"tokens": ["after-root"], "status": 307},
        ]
        _verdict, _reason, checks = evidence.CLASSIFIERS["middleware-bypass"](
            next_pack, baseline, target, before, after, records, 403
        )
        self.assertFalse(
            [check for check in checks if check["check"].startswith("extra case")]
        )

    def test_validator_accepts_the_synthetic_pack_and_rejects_broken_ones(self):
        from cve_lab_catalog import validate_data_driven_pack

        pack = self.synthetic_pack()
        validate_data_driven_pack(pack)

        broken = self.synthetic_pack()
        broken["baseline_gate"] = "middleware-bypass"
        with self.assertRaisesRegex(ValueError, "data-driven gate and classifier"):
            validate_data_driven_pack(broken)

        broken = self.synthetic_pack()
        broken["catalog"]["application_image"] = "docker.io/example/app:latest"
        with self.assertRaisesRegex(ValueError, "not pinned by digest"):
            validate_data_driven_pack(broken)

        broken = self.synthetic_pack()
        del broken["controls"]["ordinary_status"]
        with self.assertRaisesRegex(ValueError, "controls are missing"):
            validate_data_driven_pack(broken)

        broken = self.synthetic_pack()
        broken["extra_cases"][1]["id"] = "robots"
        with self.assertRaisesRegex(ValueError, "duplicate extra case id"):
            validate_data_driven_pack(broken)

        broken = self.synthetic_pack()
        del broken["extra_cases"][0]["expect_status"]
        with self.assertRaisesRegex(ValueError, "missing expect_status"):
            validate_data_driven_pack(broken)

    def test_a_new_pack_runs_end_to_end_as_pure_data(self):
        # inject the synthetic pack, run the CLI; the pack passes validation
        # and the run plan, and stops only at the (absent) container engine -
        # no code change was involved anywhere.
        pack = self.synthetic_pack()
        catalog.PACKS[pack["catalog"]["cve"]] = pack
        try:
            stdout = io.StringIO()
            with tempfile.TemporaryDirectory() as directory, patch.object(
                lab, "prepare", side_effect=RuntimeError("no engine in unit test")
            ), contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(
                io.StringIO()
            ):
                output = Path(directory) / "out"
                self.assertEqual(
                    lab.main(["run", pack["catalog"]["cve"], "--output-dir", str(output)]),
                    1,
                )
                report = json.loads((output / "report.json").read_text())
        finally:
            catalog.PACKS.pop(pack["catalog"]["cve"], None)
        plan = stdout.getvalue()
        self.assertIn("synthetic application 1.0", plan)
        self.assertEqual(report["verdict"], "inconclusive")
        self.assertEqual(report["pack"]["cve"], "CVE-0000-0001")


def base_synthetic_pack():
    """The DataDrivenCaseTests synthetic pack without needing an instance."""
    return DataDrivenCaseTests("synthetic_pack").synthetic_pack()


_MARKER_TEMPLATE = "progress=PHP_SESSION_UPLOAD_PROGRESS"
_MARKER_TEMPLATE_SHA256 = hashlib.sha256(_MARKER_TEMPLATE.encode()).hexdigest()


def synthetic_variants():
    """Three data-only signature variants: two pinned via body_template and
    one bodyless GET probe (the field cases' plain-GET shape)."""
    return [
        {
            "id": "marker-in-query",
            "method": "POST",
            "path": "/attack?marker=PHP_SESSION_UPLOAD_PROGRESS&__mockforge_cve={token}",
            "headers": {"X-Mockforge-Cve": "{token}"},
            "body_template": _MARKER_TEMPLATE,
            "body_sha256": _MARKER_TEMPLATE_SHA256,
            "expect": "blocked",
        },
        {
            "id": "marker-in-header",
            "method": "POST",
            "path": "/attack?__mockforge_cve={token}",
            "headers": {
                "X-Mockforge-Cve": "{token}",
                "X-Progress": "PHP_SESSION_UPLOAD_PROGRESS",
            },
            "body_template": _MARKER_TEMPLATE,
            "body_sha256": _MARKER_TEMPLATE_SHA256,
            "expect": "blocked",
        },
        {
            "id": "marker-in-query-get",
            "method": "GET",
            "path": "/attack?marker=PHP_SESSION_UPLOAD_PROGRESS&__mockforge_cve={token}",
            "headers": {"X-Mockforge-Cve": "{token}"},
            "expect": "blocked",
        },
    ]


class SignatureVariantValidationTests(unittest.TestCase):
    """Malformed signature variants are refused up front, before any
    download; well-formed variants validate as pure pack data."""

    def pack_with(self, variants):
        pack = base_synthetic_pack()
        pack["signature_variants"] = variants
        return pack

    def test_validator_accepts_declared_variants(self):
        from cve_lab_catalog import validate_data_driven_pack

        validate_data_driven_pack(self.pack_with(synthetic_variants()))

    def broken_variant(self, **edits):
        variant = {
            "id": "broken",
            "method": "POST",
            "path": "/attack?__mockforge_cve={token}",
            "headers": {"X-Mockforge-Cve": "{token}"},
            "body_template": _MARKER_TEMPLATE,
            "body_sha256": _MARKER_TEMPLATE_SHA256,
            "expect": "blocked",
        }
        removals = edits.pop("remove", ())
        variant.update(edits)
        for key in removals:
            variant.pop(key, None)
        return variant

    def test_validator_refuses_malformed_variants_up_front(self):
        from cve_lab_catalog import validate_data_driven_pack

        cases = {
            "missing id": {"remove": ("id",)},
            "missing path": {"remove": ("path",)},
            "missing expect": {"remove": ("expect",)},
            "expect not blocked": {"expect": "forwarded"},
            "empty id": {"id": ""},
            "path not an origin path": {"path": "http://example.test/attack"},
            "method not a token": {"method": "post;/drop"},
            "headers not a mapping": {"headers": ["not", "a", "mapping"]},
            "header value not a string": {"headers": {"X-A": 7}},
            "both body sources": {
                "body_ref": "oom_body",
                "body_template": _MARKER_TEMPLATE,
            },
            "template without a pin": {"remove": ("body_sha256",)},
            "pin mismatch": {"body_sha256": "0" * 64},
            "token inside the template": {
                "body_template": "progress={token}",
                "body_sha256": hashlib.sha256(b"progress={token}").hexdigest(),
            },
            "unknown body_ref": {
                "body_ref": "not_a_builder",
                "remove": ("body_template", "body_sha256"),
            },
            "reproduces without crash evidence": {"reproduces_directly": True},
        }
        for description, edits in cases.items():
            with self.subTest(description=description):
                pack = self.pack_with([self.broken_variant(**edits)])
                with self.assertRaisesRegex(ValueError, "signature variant"):
                    validate_data_driven_pack(pack)

    def test_validator_refuses_duplicate_variant_ids(self):
        from cve_lab_catalog import validate_data_driven_pack

        variants = synthetic_variants()
        variants[1]["id"] = variants[0]["id"]
        with self.assertRaisesRegex(ValueError, "duplicate signature variant id"):
            validate_data_driven_pack(self.pack_with(variants))

    def test_run_refuses_malformed_variants_before_any_download(self):
        pack = self.pack_with([self.broken_variant(expect="forwarded")])
        pack["catalog"]["cve"] = "CVE-0000-0002"
        catalog.PACKS[pack["catalog"]["cve"]] = pack
        errors = io.StringIO()
        try:
            with tempfile.TemporaryDirectory() as directory, patch.object(
                lab, "prepare"
            ) as prepare, contextlib.redirect_stderr(errors):
                output = Path(directory) / "out"
                self.assertEqual(
                    lab.main(
                        [
                            "run",
                            pack["catalog"]["cve"],
                            "--output-dir",
                            str(output),
                        ]
                    ),
                    1,
                )
        finally:
            catalog.PACKS.pop(pack["catalog"]["cve"], None)
        # refused up front: validation fails before any download is attempted
        prepare.assert_not_called()
        self.assertFalse((output / "report.json").exists())
        self.assertIn('must expect "blocked"', errors.getvalue())

    def test_a_new_pack_with_variants_runs_as_pure_data(self):
        pack = self.pack_with(synthetic_variants())
        pack["catalog"]["cve"] = "CVE-0000-0003"
        catalog.PACKS[pack["catalog"]["cve"]] = pack
        try:
            with tempfile.TemporaryDirectory() as directory, patch.object(
                lab, "prepare", side_effect=RuntimeError("no engine in unit test")
            ), contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(
                io.StringIO()
            ):
                output = Path(directory) / "out"
                self.assertEqual(
                    lab.main(
                        [
                            "run",
                            pack["catalog"]["cve"],
                            "--output-dir",
                            str(output),
                        ]
                    ),
                    1,
                )
                report = json.loads((output / "report.json").read_text())
        finally:
            catalog.PACKS.pop(pack["catalog"]["cve"], None)
        # Validation passed and the run reached prepare (which failed only
        # on the absent engine): the pack ran as data with no code change.
        self.assertEqual(report["verdict"], "inconclusive")


class SignatureVariantRunnerTests(unittest.TestCase):
    """The variant runner sends exactly the reviewed request, with the
    correlation token and the digest-pinned body, and never aborts on a
    transport outcome that is itself evidence."""

    def pack_with(self, variants, **pack_edits):
        pack = base_synthetic_pack()
        pack["signature_variants"] = variants
        pack.update(pack_edits)
        return pack

    def test_run_keeps_main_signature_failure_with_inconclusive_variant(self):
        pack = self.pack_with(synthetic_variants())
        pack.pop("extra_cases")
        cve = pack["catalog"]["cve"]
        catalog.PACKS[cve] = pack
        lab.PACKS[cve] = pack

        class FakeLab:
            def __enter__(self):
                return "http://127.0.0.1:18082"

            def __exit__(self, *_args):
                return False

        def fake_receiver(*_args, **_kwargs):
            @contextlib.contextmanager
            def records_context():
                records = [
                    {"tokens": ["attack"], "status": 200},
                    *[
                        {"tokens": [f"bv-{variant['id']}"], "status": 200}
                        for variant in pack["signature_variants"]
                    ],
                ]
                yield records

            return records_context()

        def fake_controls(_pack, _target, _run_id, phase, **_kwargs):
            return {
                "token": phase,
                "status": 200,
                "root_token": f"{phase}-root",
                "root_status": 200,
                "body_sha256": "same-control-body",
            }

        try:
            with tempfile.TemporaryDirectory() as directory, patch.object(
                lab, "prepare"
            ), patch.object(lab, "verify_images", return_value=[]), patch.object(
                lab, "lab", return_value=FakeLab()
            ), patch.object(lab, "receiver", side_effect=fake_receiver), patch.object(
                lab, "control", side_effect=fake_controls
            ), patch.object(
                lab, "cleanup"
            ), patch.object(
                lab, "probe", return_value={"status": 200, "marker": True}
            ), patch.object(
                nuclei, "owned_remove"
            ), patch.dict(
                lab.BASELINE_GATES, {"data-driven": lambda *_args: None}
            ), patch.dict(
                lab.CLASSIFIERS,
                {"data-driven": lambda *_args, **_kwargs: (
                    "fail", "main signature reached the origin", []
                )},
            ), patch.dict(
                lab.TARGET_EXPLOITS, {"nuclei": lambda *_args, **_kwargs: {
                    "token": "attack", "status": 403
                }}
            ), patch.object(
                lab, "variant_baseline_steps", return_value=(
                    {
                        variant["id"]: {
                            "token": f"bv-{variant['id']}", "status": 200
                        }
                        for variant in pack["signature_variants"]
                    },
                    [],
                )
            ), patch.object(
                lab, "variant_target_steps", return_value={
                    "marker-in-query": {
                        "token": "tv-marker-in-query", "status": 200
                    },
                    "marker-in-header": {
                        "token": "tv-marker-in-header", "status": 403
                    },
                    "marker-in-query-get": {
                        "token": "tv-marker-in-query-get", "status": 403
                    },
                }
            ), contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(
                io.StringIO()
            ):
                output = Path(directory) / "out"
                exit_code = lab.main(
                    [
                        "--include-unverified",
                        "run",
                        cve,
                        "--target",
                        "http://127.0.0.1:8080",
                        "--output-dir",
                        str(output),
                    ]
                )
                report = json.loads((output / "report.json").read_text())
        finally:
            catalog.PACKS.pop(cve, None)
            lab.PACKS.pop(cve, None)

        self.assertEqual(exit_code, 1)
        self.assertEqual(report["verdict"], "fail", report)
        self.assertEqual(report["reason"], "main signature reached the origin")
        self.assertEqual(
            report["signature_variants"]["marker-in-query"]["verdict"],
            "inconclusive",
        )

    def captured_backend(self, status=200):
        from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

        seen = []

        class Backend(BaseHTTPRequestHandler):
            def _reply(self):
                length = int(self.headers.get("Content-Length") or 0)
                seen.append(
                    (
                        self.command,
                        self.path,
                        dict(self.headers),
                        self.rfile.read(length),
                    )
                )
                self.send_response(status)
                self.send_header("Content-Length", "2")
                self.end_headers()
                self.wfile.write(b"ok")

            do_GET = _reply
            do_POST = _reply

            def log_message(self, *_args):
                pass

        backend = ThreadingHTTPServer(("127.0.0.1", 0), Backend)
        threading.Thread(target=backend.serve_forever, daemon=True).start()
        self.addCleanup(backend.shutdown)
        self.addCleanup(backend.server_close)
        return backend, seen

    def test_variant_baseline_steps_send_each_reviewed_request(self):
        with tempfile.TemporaryDirectory() as directory:
            backend, seen = self.captured_backend()
            pack = self.pack_with(synthetic_variants())
            outcomes, not_reproducing = exploits.variant_baseline_steps(
                None,
                f"http://127.0.0.1:{backend.server_address[1]}",
                Path(directory),
                "run1",
                pack,
            )
            self.assertEqual(not_reproducing, [])
            self.assertEqual(
                set(outcomes),
                {"marker-in-query", "marker-in-header", "marker-in-query-get"},
            )
            self.assertEqual(len(seen), 3)
            for variant_id, outcome in outcomes.items():
                matching = [
                    (method, path, headers, body)
                    for method, path, headers, body in seen
                    if headers.get("X-Mockforge-Cve") == outcome["token"]
                ]
                self.assertEqual(len(matching), 1)
                method, path, headers, body = matching[0]
                self.assertIn(f"__mockforge_cve={outcome['token']}", path)
                self.assertIn(f"-baseline-variant-{variant_id}", outcome["token"])
                self.assertEqual(outcome["status"], 200)
                if variant_id == "marker-in-query-get":
                    # the field cases' plain-GET shape: GET, no body
                    self.assertEqual(method, "GET")
                    self.assertEqual(body, b"")
                else:
                    self.assertEqual(method, "POST")
                    self.assertEqual(body, _MARKER_TEMPLATE.encode())
            self.assertTrue((Path(directory) / "baseline-variants.json").exists())

    def test_variant_baseline_steps_demand_the_reviewed_crash_when_marked(self):
        variant = {
            "id": "marked",
            "method": "POST",
            "path": "/attack?__mockforge_cve={token}",
            "headers": {"X-Mockforge-Cve": "{token}"},
            "body_template": _MARKER_TEMPLATE,
            "body_sha256": _MARKER_TEMPLATE_SHA256,
            "expect": "blocked",
            "reproduces_directly": True,
        }
        pack = self.pack_with([variant], baseline_exploit="modphp-oom")
        with tempfile.TemporaryDirectory() as directory:
            with patch.object(
                exploits, "_marker_count", side_effect=[0, 0]
            ):
                outcomes, not_reproducing = exploits.variant_baseline_steps(
                    None, "http://127.0.0.1:1", Path(directory), "run1", pack
                )
            self.assertEqual(not_reproducing, ["marked"])
            self.assertFalse(outcomes["marked"]["segfault_confirmed"])
            with patch.object(
                exploits, "_marker_count", side_effect=[3, 4]
            ):
                outcomes, not_reproducing = exploits.variant_baseline_steps(
                    None, "http://127.0.0.1:1", Path(directory), "run1", pack
                )
            self.assertEqual(not_reproducing, [])
            self.assertTrue(outcomes["marked"]["segfault_confirmed"])

    def test_variant_target_steps_send_through_the_target(self):
        with tempfile.TemporaryDirectory() as directory:
            backend, seen = self.captured_backend(status=403)
            pack = self.pack_with(synthetic_variants())
            outcomes = exploits.variant_target_steps(
                f"http://127.0.0.1:{backend.server_address[1]}",
                Path(directory),
                "run1",
                pack,
            )
            self.assertEqual(
                set(outcomes),
                {"marker-in-query", "marker-in-header", "marker-in-query-get"},
            )
            for outcome in outcomes.values():
                self.assertIn("-target-variant-", outcome["token"])
                self.assertEqual(outcome["status"], 403)
            self.assertEqual(len(seen), 3)
            self.assertTrue((Path(directory) / "target-variants.json").exists())

    def test_variant_target_steps_use_https_for_https_targets(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(
            exploits, "_request_tolerant", return_value={}
        ) as tolerant:
            exploits.variant_target_steps(
                "https://127.0.0.1:8443",
                Path(directory),
                "run1",
                self.pack_with(synthetic_variants()),
            )
        self.assertEqual(tolerant.call_args.kwargs.get("scheme"), "https")

    def test_tampered_bodies_break_the_digest_check(self):
        import cve_lab_payloads as payloads

        variant = dict(synthetic_variants()[0])
        variant["body_template"] = "tampered body"
        with self.assertRaisesRegex(RuntimeError, "pinned review digest"):
            exploits._variant_body(variant)

        tampered_registry = {
            "oom_body": (
                lambda: b"tampered bytes",
                payloads.OOM_BODY_SHA256,
                "multipart/form-data; boundary=x",
            )
        }
        with patch.dict(payloads.PINNED_BODIES, tampered_registry):
            with self.assertRaisesRegex(RuntimeError, "pinned review digest"):
                exploits._variant_body({"id": "x", "body_ref": "oom_body"})


class SignatureVariantEvidenceTests(unittest.TestCase):
    """Per-variant verdicts: blocked variants pass, a variant that reached
    the origin fails the run and is named, and variant evidence can only
    downgrade the main verdict."""

    def pack(self):
        pack = base_synthetic_pack()
        pack["signature_variants"] = synthetic_variants()
        return pack

    def healthy_evidence(self, pack, target_records=()):
        variant_ids = tuple(variant["id"] for variant in pack["signature_variants"])
        baseline_variants = {
            variant_id: {"token": f"bv-{variant_id}", "status": 200}
            for variant_id in variant_ids
        }
        target_variants = {
            variant_id: {"token": f"tv-{variant_id}", "status": 403}
            for variant_id in variant_ids
        }
        baseline_records = [
            {"tokens": [outcome["token"]], "status": 200}
            for outcome in baseline_variants.values()
        ]
        return evidence.signature_variant_evidence(
            pack,
            baseline_variants,
            target_variants,
            baseline_records,
            list(target_records),
            403,
        )

    def test_blocked_variants_pass_and_are_reported_per_variant(self):
        entries, all_ok, got_through, verdicts = self.healthy_evidence(self.pack())
        self.assertTrue(all_ok)
        self.assertEqual(got_through, [])
        self.assertEqual(verdicts, {
            "marker-in-query": "pass",
            "marker-in-header": "pass",
            "marker-in-query-get": "pass",
        })
        names = [entry["check"] for entry in entries]
        for variant_id in verdicts:
            self.assertIn(
                f"variant {variant_id} was blocked at the target", names
            )
            self.assertIn(
                f"variant {variant_id} reached the origin in the direct baseline",
                names,
            )
        self.assertTrue(all(entry["outcome"] == "ok" for entry in entries))

    def test_a_variant_forwarded_to_the_origin_fails_and_is_named(self):
        records = [{"tokens": ["tv-marker-in-query"], "status": 200}]
        entries, all_ok, got_through, verdicts = self.healthy_evidence(
            self.pack(), target_records=records
        )
        self.assertFalse(all_ok)
        self.assertEqual(got_through, ["marker-in-query"])
        self.assertEqual(verdicts["marker-in-query"], "fail")
        self.assertEqual(verdicts["marker-in-header"], "pass")
        blocked = [
            entry
            for entry in entries
            if entry["check"] == "variant marker-in-query was blocked at the target"
        ][0]
        self.assertIn("reached the origin with status 200", blocked["outcome"])
        override, reason = evidence.variant_verdict(all_ok, got_through)
        self.assertEqual(override, "fail")
        self.assertIn("marker-in-query", reason)

    def test_variant_evidence_is_inconclusive_on_transport_or_status_drift(self):
        # a blocked variant answered with the wrong status: not a failure,
        # but never a pass
        pack = self.pack()

        def evidence_with(target_variants, target_records):
            baseline_variants = {
                variant_id: {"token": f"bv-{variant_id}", "status": 200}
                for variant_id in ("marker-in-query", "marker-in-header")
            }
            baseline_records = [
                {"tokens": [outcome["token"]], "status": 200}
                for outcome in baseline_variants.values()
            ]
            return evidence.signature_variant_evidence(
                pack,
                baseline_variants,
                target_variants,
                baseline_records,
                target_records,
                403,
            )


        wrong_status = {
            "marker-in-query": {"token": "tv-marker-in-query", "status": 200},
            "marker-in-header": {"token": "tv-marker-in-header", "status": 403},
        }
        _entries, all_ok, got_through, verdicts = evidence_with(wrong_status, [])
        self.assertEqual(got_through, [])
        self.assertEqual(verdicts["marker-in-query"], "inconclusive")
        self.assertEqual(verdicts["marker-in-header"], "pass")
        override, _reason = evidence.variant_verdict(all_ok, got_through)
        self.assertEqual(override, "inconclusive")

        transport_error = {
            "marker-in-query": {
                "token": "tv-marker-in-query",
                "status": None,
                "error": "connection refused",
            },
            "marker-in-header": {"token": "tv-marker-in-header", "status": 403},
        }
        entries, all_ok, got_through, verdicts = evidence_with(
            transport_error, []
        )
        self.assertEqual(got_through, [])
        self.assertEqual(verdicts["marker-in-query"], "inconclusive")
        blocked = [
            entry
            for entry in entries
            if entry["check"] == "variant marker-in-query was blocked at the target"
        ][0]
        self.assertIn("connection refused", blocked["outcome"])

    def test_variant_evidence_marks_a_missing_direct_receipt(self):
        pack = self.pack()
        entries, all_ok, got_through, verdicts = evidence.signature_variant_evidence(
            pack,
            {"marker-in-query": {"token": "bv-marker-in-query", "error": "no route"}},
            {"marker-in-query": {"token": "tv-marker-in-query", "status": 403}},
            [],
            [],
            403,
        )
        self.assertFalse(all_ok)
        self.assertEqual(got_through, [])
        self.assertEqual(verdicts["marker-in-query"], "inconclusive")
        direct = [
            entry
            for entry in entries
            if "reached the origin in the direct baseline" in entry["check"]
        ][0]
        self.assertIn("no route", direct["outcome"])

    def test_variant_verdict_never_upgrades_a_failed_run(self):
        self.assertEqual(evidence.variant_verdict(True, []), (None, None))
        override, _reason = evidence.variant_verdict(False, [])
        self.assertEqual(override, "inconclusive")
        override, reason = evidence.variant_verdict(False, ["zeta", "alpha"])
        self.assertEqual(override, "fail")
        self.assertEqual(reason, (
            "signature variant(s) alpha, zeta reached the origin; "
            "blocking assertion failed"
        ))

    def test_variant_evidence_never_softens_a_failed_main_signature(self):
        # A forwarded main signature plus one wrong-status variant stays fail
        # with the classifier's reason; it must not become inconclusive.
        override, override_reason = evidence.variant_verdict(False, [])
        self.assertEqual(
            evidence.merge_variant_verdict(
                "fail", "main signature reached the origin", override, override_reason
            ),
            ("fail", "main signature reached the origin"),
        )
        self.assertEqual(
            evidence.merge_variant_verdict("pass", "blocked", override, override_reason),
            ("inconclusive", override_reason),
        )
        failed, failed_reason = evidence.variant_verdict(False, ["alpha"])
        self.assertEqual(
            evidence.merge_variant_verdict("inconclusive", "x", failed, failed_reason),
            ("fail", failed_reason),
        )
        self.assertEqual(
            evidence.merge_variant_verdict("pass", "blocked", None, None),
            ("pass", "blocked"),
        )

    def test_forwarded_variant_traffic_stays_correlated_for_the_classifier(self):
        # The classifier must not mistake a forwarded variant for uncorrelated
        # traffic: its token is expected, and the per-variant evidence (not
        # the correlation check) turns the run into a named failure.
        variant_token = "run1-target-variant-marker-in-query"
        records = [
            {"tokens": ["before"], "status": 200},
            {"tokens": ["before-root"], "status": 200},
            {"tokens": ["after"], "status": 200},
            {"tokens": ["after-root"], "status": 200},
            {"tokens": [variant_token], "status": 200},
        ]
        verdict, reason, _checks = evidence.CLASSIFIERS["modphp-segfault"](
            catalog.PACKS["CVE-2026-109618"],
            {"status": 200, "marker": True, "crash_confirmed": True},
            {"token": "attack", "status": 403},
            {"token": "before", "status": 200, "root_token": "before-root", "root_status": 200},
            {"token": "after", "status": 200, "root_token": "after-root", "root_status": 200},
            records,
            403,
            variant_tokens={variant_token},
        )
        self.assertEqual(verdict, "pass")
        self.assertNotIn("uncorrelated", reason)

        verdict, reason, _checks = evidence.CLASSIFIERS["modphp-segfault"](
            catalog.PACKS["CVE-2026-109618"],
            {"status": 200, "marker": True, "crash_confirmed": True},
            {"token": "attack", "status": 403},
            {"token": "before", "status": 200, "root_token": "before-root", "root_status": 200},
            {"token": "after", "status": 200, "root_token": "after-root", "root_status": 200},
            records,
            403,
        )
        self.assertEqual(verdict, "inconclusive")
        self.assertIn("uncorrelated", reason)


class Php2Shell2026SignatureVariantTests(unittest.TestCase):
    """The shipped PHP2Shell pack (CVE-2026-109618) carries Srikanth's field
    cases from issue #79 as reviewed marker placements, with pinned bodies."""

    def setUp(self):
        self.pack = catalog.PACKS["CVE-2026-109618"]

    def test_variants_validate_and_expect_blocking(self):
        from cve_lab_catalog import validate_data_driven_pack

        validate_data_driven_pack(self.pack)
        variants = self.pack["signature_variants"]
        self.assertEqual(len(variants), 5)
        for variant in variants:
            self.assertEqual(variant["expect"], "blocked")
            self.assertIn("reproduces_directly", variant)
        self.assertEqual(self.pack["catalog"]["pack_revision"], 2)

    def test_variants_cover_uri_query_header_and_body_positions(self):
        from urllib.parse import parse_qs, urlsplit

        by_id = {
            variant["id"]: variant for variant in self.pack["signature_variants"]
        }
        self.assertEqual(
            set(by_id),
            {
                "marker-in-uri",
                "marker-in-query",
                "marker-in-query-get",
                "marker-in-urlencoded-field",
                "marker-in-header",
            },
        )
        marker = "PHP_SESSION_UPLOAD_PROGRESS"
        # (a) URI/path: the marker is the first path segment (the exact
        # source case POST /PHP_SESSION_UPLOAD_PROGRESS/upload.php)
        uri = by_id["marker-in-uri"]["path"]
        self.assertEqual(uri.split("?")[0].split("/")[1], marker)
        self.assertTrue(uri.split("?")[0].endswith("/upload.php"))
        # (b) query parameter on a POST (the exact source case
        # POST /upload.php?PHP_SESSION_UPLOAD_PROGRESS=123)
        self.assertEqual(by_id["marker-in-query"]["method"], "POST")
        query = parse_qs(urlsplit(by_id["marker-in-query"]["path"]).query)
        self.assertIn(marker, query)
        # (c) query parameter on a plain GET with no body (the exact source
        # case GET /upload.php?PHP_SESSION_UPLOAD_PROGRESS=1)
        get_variant = by_id["marker-in-query-get"]
        self.assertEqual(get_variant["method"], "GET")
        self.assertIn(marker, parse_qs(urlsplit(get_variant["path"]).query))
        self.assertNotIn("body_ref", get_variant)
        self.assertNotIn("body_template", get_variant)
        self.assertIsNone(exploits._variant_body(get_variant))
        # (d) urlencoded body FIELD NAME - a body position other than the
        # multipart field name (the exact source case body
        # PHP_SESSION_UPLOAD_PROGRESS=123)
        body_variant = by_id["marker-in-urlencoded-field"]
        body = exploits._variant_body(body_variant)
        self.assertEqual(body, b"PHP_SESSION_UPLOAD_PROGRESS=123")
        self.assertNotIn(b'name="PHP_SESSION_UPLOAD_PROGRESS"', body)
        self.assertEqual(
            body_variant["body_ref"], "progress_urlencoded_field_body"
        )
        # (e) request header: the marker names and values a header
        header_variant = by_id["marker-in-header"]
        self.assertIn(
            marker, [value for value in header_variant["headers"].values()]
        )
        self.assertIn(
            "php-session-upload-progress",
            " ".join(name.lower() for name in header_variant["headers"]),
        )

    def test_trigger_position_variants_isolate_the_marker_to_one_position(self):
        import cve_lab_payloads as payloads

        by_id = {
            variant["id"]: variant for variant in self.pack["signature_variants"]
        }
        # the URI/query/header variants all reuse the benign single-part
        # upload body, which carries no marker, so a block must come from
        # the new position; the GET variant carries no body at all
        for variant_id in (
            "marker-in-uri",
            "marker-in-query",
            "marker-in-header",
        ):
            variant = by_id[variant_id]
            self.assertEqual(variant["body_ref"], "file_upload_body")
            self.assertNotIn(
                b"PHP_SESSION_UPLOAD_PROGRESS", payloads.file_upload_body()
            )
        self.assertNotIn(
            b"PHP_SESSION_UPLOAD_PROGRESS", payloads.file_upload_body()
        )

    def test_variant_bodies_are_registry_pinned(self):
        import cve_lab_payloads as payloads

        for variant in self.pack["signature_variants"]:
            if "body_ref" not in variant:
                continue
            builder, expected_digest, content_type = payloads.PINNED_BODIES[
                variant["body_ref"]
            ]
            body = exploits._variant_body(variant)
            self.assertEqual(
                hashlib.sha256(body).hexdigest(), expected_digest
            )
            self.assertTrue(content_type)
        # the URI-position variant reuses the field cases' benign upload
        # body verbatim
        uri_variant = [
            variant
            for variant in self.pack["signature_variants"]
            if variant["id"] == "marker-in-uri"
        ][0]
        self.assertEqual(
            exploits._variant_body(uri_variant), payloads.file_upload_body()
        )

    def test_field_case_variants_are_not_marked_as_reproducing(self):
        # none of the source cases carries the oversized reviewed trigger
        # body, so none is marked as reproducing directly: they are
        # must-be-blocked signature probes
        for variant in self.pack["signature_variants"]:
            self.assertFalse(variant["reproduces_directly"], variant["id"])

    def test_urlencoded_field_body_is_pinned_and_field_named(self):
        import cve_lab_payloads as payloads

        body = payloads.progress_urlencoded_field_body()
        self.assertEqual(
            hashlib.sha256(body).hexdigest(),
            payloads.PROGRESS_URLENCODED_FIELD_BODY_SHA256,
        )
        self.assertEqual(body, b"PHP_SESSION_UPLOAD_PROGRESS=123")
        self.assertNotIn(b'name="PHP_SESSION_UPLOAD_PROGRESS"', body)
        self.assertLess(len(body), 64 * 1024)


class Php2Shell109619SignatureVariantTests(unittest.TestCase):
    """The same-worker PHP2Shell pack (CVE-2026-109619) carries Srikanth's
    field cases from issue #79 as reviewed ini_set/php_value/php_admin
    marker placements, with pinned bodies."""

    def setUp(self):
        self.pack = catalog.PACKS["CVE-2026-109619"]

    def test_variants_validate_and_expect_blocking(self):
        from cve_lab_catalog import validate_data_driven_pack

        validate_data_driven_pack(self.pack)
        variants = self.pack["signature_variants"]
        self.assertEqual(len(variants), 10)
        for variant in variants:
            self.assertEqual(variant["expect"], "blocked")
            self.assertIn("reproduces_directly", variant)
        self.assertEqual(self.pack["catalog"]["pack_revision"], 2)

    def test_variants_cover_body_query_and_path_positions(self):
        from urllib.parse import parse_qs, urlsplit

        by_id = {
            variant["id"]: variant for variant in self.pack["signature_variants"]
        }
        self.assertEqual(
            set(by_id),
            {
                "ini-set-in-body",
                "php-value-in-body",
                "php-admin-in-body",
                "ini-set-in-query",
                "php-value-in-query",
                "php-admin-in-query",
                "ini-set-in-path",
                "php-value-in-path",
                "php-admin-in-path",
                "ini-set-in-header",
            },
        )
        # (a) body position: the marker IS the raw text/plain body (the
        # exact source cases POST /test with the x-php: test header)
        for marker, variant_id in (
            (b"ini_set", "ini-set-in-body"),
            (b"php_value", "php-value-in-body"),
            (b"php_admin", "php-admin-in-body"),
        ):
            variant = by_id[variant_id]
            self.assertEqual(variant["method"], "POST")
            self.assertEqual(variant["path"].split("?")[0], "/test")
            self.assertEqual(variant["headers"].get("X-PHP"), "test")
            self.assertEqual(exploits._variant_body(variant), marker)
            method, path, headers, body, _timeout = (
                exploits._variant_request_spec(self.pack, variant, "tok")
            )
            self.assertEqual(method, "POST")
            self.assertEqual(headers.get("Content-Type"), "text/plain")
            self.assertEqual(headers.get("X-Mockforge-Cve"), "tok")
        # (b) query position with the source cases' literal values and the
        # swapped test: x-php header; bodyless, so a block can only come
        # from the query
        for marker, value, variant_id in (
            ("ini_set", "1", "ini-set-in-query"),
            ("php_value", "1", "php-value-in-query"),
            ("php_admin", "true", "php-admin-in-query"),
        ):
            variant = by_id[variant_id]
            self.assertEqual(variant["method"], "POST")
            self.assertEqual(variant["path"].split("?")[0], "/testing")
            query = parse_qs(urlsplit(variant["path"]).query)
            self.assertEqual(query.get(marker), [value])
            self.assertEqual(variant["headers"].get("Test"), "x-php")
            self.assertEqual(variant["headers"].get("Content-Type"), "text/plain")
            self.assertIsNone(exploits._variant_body(variant))
        # (c) path position: /testing/<marker>/<tail>; bodyless, so a block
        # can only come from the path
        for marker, tail, variant_id in (
            ("ini_set", "one", "ini-set-in-path"),
            ("php_value", "two", "php-value-in-path"),
            ("php_admin", "three", "php-admin-in-path"),
        ):
            variant = by_id[variant_id]
            segments = variant["path"].split("?")[0].strip("/").split("/")
            self.assertEqual(segments, ["testing", marker, tail])
            self.assertEqual(variant["headers"].get("Test"), "x-php")
            self.assertIsNone(exploits._variant_body(variant))
        # (d) header position: the marker names and values a header, with
        # no body, so a block can only come from the header
        header_variant = by_id["ini-set-in-header"]
        self.assertIn(
            "ini_set", [value for value in header_variant["headers"].values()]
        )
        self.assertIn(
            "ini-set",
            " ".join(name.lower() for name in header_variant["headers"]),
        )
        self.assertIsNone(exploits._variant_body(header_variant))

    def test_variant_bodies_are_registry_pinned(self):
        import cve_lab_payloads as payloads

        for variant in self.pack["signature_variants"]:
            if "body_ref" not in variant:
                continue
            builder, expected_digest, content_type = payloads.PINNED_BODIES[
                variant["body_ref"]
            ]
            body = exploits._variant_body(variant)
            self.assertEqual(
                hashlib.sha256(body).hexdigest(), expected_digest
            )
            self.assertEqual(content_type, "text/plain")
        # the body-position variant carries its marker verbatim
        body_variant = [
            variant
            for variant in self.pack["signature_variants"]
            if variant["id"] == "ini-set-in-body"
        ][0]
        self.assertEqual(exploits._variant_body(body_variant), b"ini_set")

    def test_marker_text_bodies_are_pinned(self):
        import cve_lab_payloads as payloads

        for builder, digest, expected in (
            (payloads.ini_set_body, payloads.INI_SET_BODY_SHA256, b"ini_set"),
            (payloads.php_value_body, payloads.PHP_VALUE_BODY_SHA256, b"php_value"),
            (payloads.php_admin_body, payloads.PHP_ADMIN_BODY_SHA256, b"php_admin"),
        ):
            body = builder()
            self.assertEqual(body, expected)
            self.assertEqual(hashlib.sha256(body).hexdigest(), digest)
            self.assertLess(len(body), 64 * 1024)

    def test_field_case_variants_are_not_marked_as_reproducing(self):
        # none of the source cases carries the oversized reviewed trigger
        # body, so none is marked as reproducing directly: they are
        # must-be-blocked signature probes
        for variant in self.pack["signature_variants"]:
            self.assertFalse(variant["reproduces_directly"], variant["id"])

    def test_blocked_variants_pass_and_a_forwarded_variant_fails_named(self):
        variant_ids = tuple(
            variant["id"] for variant in self.pack["signature_variants"]
        )
        baseline_variants = {
            variant_id: {"token": f"bv-{variant_id}", "status": 200}
            for variant_id in variant_ids
        }
        target_variants = {
            variant_id: {"token": f"tv-{variant_id}", "status": 403}
            for variant_id in variant_ids
        }
        baseline_records = [
            {"tokens": [outcome["token"]], "status": 200}
            for outcome in baseline_variants.values()
        ]
        entries, all_ok, got_through, verdicts = evidence.signature_variant_evidence(
            self.pack,
            baseline_variants,
            target_variants,
            baseline_records,
            [],
            403,
        )
        # every variant blocked by the LB passes, and is reported per variant
        self.assertTrue(all_ok)
        self.assertEqual(got_through, [])
        self.assertEqual(set(verdicts.values()), {"pass"})
        names = [entry["check"] for entry in entries]
        for variant_id in verdicts:
            self.assertIn(
                f"variant {variant_id} was blocked at the target", names
            )
        # a variant forwarded to the origin fails and is named
        forwarded = "ini-set-in-body"
        records = [{"tokens": [f"tv-{forwarded}"], "status": 200}]
        entries, all_ok, got_through, verdicts = evidence.signature_variant_evidence(
            self.pack,
            baseline_variants,
            target_variants,
            baseline_records,
            records,
            403,
        )
        self.assertFalse(all_ok)
        self.assertEqual(got_through, [forwarded])
        self.assertEqual(verdicts[forwarded], "fail")
        self.assertEqual(verdicts["php-value-in-query"], "pass")
        blocked = [
            entry
            for entry in entries
            if entry["check"] == f"variant {forwarded} was blocked at the target"
        ][0]
        self.assertIn("reached the origin with status 200", blocked["outcome"])
        override, reason = evidence.variant_verdict(all_ok, got_through)
        self.assertEqual(override, "fail")
        self.assertIn(forwarded, reason)

    def test_a_malformed_variant_is_refused_by_validation(self):
        import copy

        from cve_lab_catalog import validate_data_driven_pack

        pack = copy.deepcopy(self.pack)
        pack["signature_variants"][0]["expect"] = "forwarded"
        with self.assertRaisesRegex(ValueError, 'must expect "blocked"'):
            validate_data_driven_pack(pack)
        pack["signature_variants"][0]["expect"] = "blocked"
        pack["signature_variants"][1]["body_ref"] = "not_a_builder"
        with self.assertRaisesRegex(ValueError, "reviewed, digest-pinned"):
            validate_data_driven_pack(pack)
        pack["signature_variants"][1]["body_ref"] = (
            pack["signature_variants"][0]["body_ref"]
        )
        # a duplicated id is refused (the pack's own crash-evidence baseline
        # exploit means reproduces_directly marks are not refused here)
        pack["signature_variants"][1]["id"] = pack["signature_variants"][0]["id"]
        with self.assertRaisesRegex(ValueError, "duplicate signature variant id"):
            validate_data_driven_pack(pack)


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
            self.assertEqual(lab.main(["--include-unverified", "images"]), 0)

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

    def test_images_command_lists_verified_packs_and_exits_clean_when_present(self):
        stdout = io.StringIO()
        with patch.object(lab, "doctor"), patch.object(
            lab, "verify_images", return_value=[{"role": "application", "image": "app@sha256:a", "status": "present"}, {"role": "runner", "image": "runner@sha256:b", "status": "present"}]
        ), contextlib.redirect_stdout(stdout):
            self.assertEqual(lab.main(["images"]), 0)
        listing = json.loads(stdout.getvalue())
        self.assertEqual(set(listing), set(lab.verified_cves()))
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

    def test_images_command_requires_flag_for_unverified_pack(self):
        stderr = io.StringIO()
        with patch.object(lab, "doctor") as doctor, patch.object(
            lab, "verify_images"
        ) as verify, contextlib.redirect_stderr(stderr):
            self.assertEqual(lab.main(["images", "CVE-2026-109618"]), 1)
        self.assertIn(lab.UNVERIFIED_MESSAGE, stderr.getvalue())
        doctor.assert_not_called()
        verify.assert_not_called()

    def test_images_command_includes_unverified_packs_only_when_requested(self):
        stdout = io.StringIO()
        entries = [{"role": "application", "image": "app:pending", "status": "unpinned"}]
        with patch.object(lab, "doctor"), patch.object(
            lab, "verify_images", return_value=entries
        ), contextlib.redirect_stdout(stdout):
            self.assertEqual(
                lab.main(["--include-unverified", "images", "CVE-2026-109618"]), 0
            )
        self.assertEqual(set(json.loads(stdout.getvalue())), {"CVE-2026-109618"})

    def test_images_command_rejects_unknown_cve_without_engine_access(self):
        stderr = io.StringIO()
        with patch.object(lab, "doctor") as doctor, contextlib.redirect_stderr(stderr), self.assertRaises(SystemExit) as error:
            lab.main(["images", "CVE-2099-99999"])
        self.assertEqual(error.exception.code, 2)
        self.assertIn("invalid choice", stderr.getvalue())
        doctor.assert_not_called()

    def test_images_command_exits_nonzero_when_an_image_is_missing(self):
        stdout = io.StringIO()
        stderr = io.StringIO()

        def verify(engine_arg, pack):
            if pack["catalog"]["cve"] == "CVE-2025-29927":
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
            self.assertEqual(lab.main(["--include-unverified", "images"]), 1)
        listing = json.loads(stdout.getvalue())
        statuses = [
            (cve, entry["role"], entry["status"])
            for cve, entries in listing.items()
            for entry in entries
        ]
        self.assertIn("CVE-2024-4577", listing)
        self.assertIn(("CVE-2025-29927", "runner", "missing"), statuses)
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
        ) as verify, patch.object(lab, "lab", side_effect=RuntimeError("mocked engine failure")), contextlib.redirect_stdout(stdout):
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
        ), patch.object(lab, "lab", side_effect=RuntimeError("mocked engine failure")), contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(
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
