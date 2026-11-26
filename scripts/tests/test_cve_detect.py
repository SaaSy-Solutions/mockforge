"""Version-compare verdicts for the passive detect packs, with a mocked target.

No network is used: ``cve_lab_transport.request`` is patched to return canned
response headers, so nothing is ever sent anywhere.
"""

import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import cve_lab_catalog as catalog  # noqa: E402
import cve_lab_detect as detect  # noqa: E402
import cve_lab_evidence as evidence  # noqa: E402
import cve_lab_transport as transport  # noqa: E402

PHP = catalog.DETECT_PACKS["CVE-2026-109618"]
NODE = catalog.DETECT_PACKS["CVE-2026-111497"]


def obs(**headers):
    return {"token": "t", "status": 200, "headers": {k.replace("_", "-"): v for k, v in headers.items()}}


def verdict(pack, **headers):
    return evidence.classify_version_fingerprint(pack, obs(**headers))[0]


class CatalogShape(unittest.TestCase):
    def test_no_exploit_fields(self):
        for pack in catalog.DETECT_PACKS.values():
            for field in ("exploit", "baseline_exploit", "target_exploit", "marker"):
                self.assertNotIn(field, pack)
                self.assertNotIn(field, pack["probe"])
            self.assertEqual(pack["probe"]["method"], "GET")
            self.assertIn(pack["classifier"], evidence.CLASSIFIERS)

    def test_existing_packs_untouched(self):
        self.assertIn("CVE-2026-109618", catalog.PACKS)
        self.assertIn("exploit", catalog.PACKS["CVE-2026-109618"])


class PhpVerdicts(unittest.TestCase):
    def test_affected_below_fixed_per_line(self):
        for v in ("8.4.26", "8.3.35", "8.2.34", "8.5.11"):
            self.assertEqual(verdict(PHP, x_powered_by=f"PHP/{v}"), "likely-affected", v)

    def test_not_affected_at_or_above_fixed(self):
        for v in ("8.4.27", "8.3.36", "8.2.35", "8.5.12", "8.5.13", "8.6.0"):
            self.assertEqual(verdict(PHP, x_powered_by=f"PHP/{v}"), "not-affected", v)

    def test_unknown_cases(self):
        self.assertEqual(verdict(PHP, server="Apache/2.4.69"), "unknown")
        self.assertEqual(verdict(PHP, x_powered_by="PHP/8.1.30"), "unknown")
        self.assertEqual(evidence.classify_version_fingerprint(PHP, {"error": "refused"})[0], "unknown")

    def test_version_in_server_header_and_sapi_hint(self):
        v, reason, checks = evidence.classify_version_fingerprint(
            PHP, obs(server="Apache/2.4.69 (Debian) PHP/8.4.26")
        )
        self.assertEqual(v, "likely-affected")
        self.assertIn("backported", reason)
        self.assertTrue(any("mod_php is possible" in c["outcome"] for c in checks))
        _, reason, checks = evidence.classify_version_fingerprint(
            PHP, obs(server="nginx/1.27", x_powered_by="PHP/8.4.26")
        )
        self.assertTrue(any("FPM" in c["outcome"] for c in checks))
        self.assertIn("mod_php", reason)

    def test_second_cve_uses_same_ranges(self):
        pack = catalog.DETECT_PACKS["CVE-2026-109619"]
        self.assertEqual(verdict(pack, x_powered_by="PHP/8.3.35"), "likely-affected")
        self.assertEqual(verdict(pack, x_powered_by="PHP/8.3.36"), "not-affected")


class NodeVerdicts(unittest.TestCase):
    def test_usually_unknown(self):
        v, reason, _ = evidence.classify_version_fingerprint(NODE, obs(server="nginx"))
        self.assertEqual(v, "unknown")
        self.assertIn("rarely announces", reason)

    def test_affected(self):
        for v in ("22.23.3", "24.22.0", "26.12.0", "25.1.0", "23.9.0", "20.18.0", "18.0.0"):
            self.assertEqual(verdict(NODE, x_powered_by=f"Node.js/{v}"), "likely-affected", v)

    def test_not_affected(self):
        for v in ("22.23.4", "24.22.1", "26.12.1", "26.13.0", "27.0.0"):
            self.assertEqual(verdict(NODE, x_node_version=v), "not-affected", v)


class DetectRun(unittest.TestCase):
    def run_detect(self, headers):
        class Args:
            cve = "CVE-2026-109618"
            skip_tls_verify = False
            i_am_authorized = True

        with tempfile.TemporaryDirectory() as tmp:
            args = Args()
            args.target = "http://mock.invalid"
            args.output_dir = str(Path(tmp) / "out")
            calls = []

            def fake(base, path, hdrs=None, **kw):
                calls.append((base, path, kw.get("method", "GET")))
                return 200, list(headers.items()), b"ok"

            with patch.object(transport, "request", fake):
                rc = detect.detect(args)
            report = json.loads((Path(args.output_dir) / "report.json").read_text())
            return rc, report, calls

    def test_end_to_end_mocked(self):
        rc, report, calls = self.run_detect({"X-Powered-By": "PHP/8.4.26", "Server": "Apache/2.4.69"})
        self.assertEqual(rc, 1)
        self.assertEqual(report["verdict"], "likely-affected")
        self.assertEqual(report["mode"], "detect-only")
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0][2], "GET")
        self.assertTrue(calls[0][1].startswith("/?__mockforge_cve="))
        rc, report, _ = self.run_detect({"X-Powered-By": "PHP/8.4.27"})
        self.assertEqual((rc, report["verdict"]), (0, "not-affected"))
        rc, report, _ = self.run_detect({})
        self.assertEqual((rc, report["verdict"]), (0, "unknown"))
        self.assertTrue(report["authorization"]["authorized"])

    def test_refused_without_attestation_sends_nothing(self):
        class Args:
            cve = "CVE-2026-109618"
            skip_tls_verify = False
            i_am_authorized = False
            target = "http://mock.invalid"

        calls = []
        with tempfile.TemporaryDirectory() as tmp:
            args = Args()
            args.output_dir = str(Path(tmp) / "out")
            with patch.dict("os.environ", {}, clear=True), patch(
                "sys.stdin", None
            ), patch.object(transport, "request", lambda *a, **k: calls.append(a)):
                with self.assertRaises(SystemExit) as ctx:
                    detect.detect(args)
            self.assertEqual(ctx.exception.code, 2)
            self.assertFalse(Path(args.output_dir).exists())
        self.assertEqual(calls, [])


if __name__ == "__main__":
    unittest.main()
