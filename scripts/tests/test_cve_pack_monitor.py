"""Feeds, enrichment, triage, drafter and monitor — all offline.

The network is stubbed; no feed is contacted. The drafter is a stub, and the
central safety property is asserted directly: a drafter that proposes a payload
(as a prompt-injected advisory might cause) is rejected by the schema, never
written as a runnable profile.
"""

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))


def _load(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / f"scripts/{name}.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


feeds = _load("cve_pack_feeds")
enrich = _load("cve_pack_enrich")
triage = _load("cve_pack_triage")
draft = _load("cve_pack_draft")
monitor = _load("cve_pack_monitor")


class FakeFetcher:
    """Dispatches by URL to canned feed data for two CVEs."""

    NVD = {
        "CVE-2025-29927": {
            "vulnerabilities": [
                {
                    "cve": {
                        "id": "CVE-2025-29927",
                        "weaknesses": [
                            {
                                "description": [
                                    {"value": "CWE-285"},
                                    {"value": "CWE-863"},
                                ]
                            }
                        ],
                        "configurations": [
                            {
                                "nodes": [
                                    {
                                        "cpeMatch": [
                                            {
                                                "criteria": "cpe:2.3:a:vercel:next.js:*:*",
                                                "versionEndExcluding": "15.2.3",
                                            },
                                            {
                                                "criteria": "cpe:2.3:a:x:firmware:*",
                                                "versionEndExcluding": "2.7.0",
                                            },
                                        ]
                                    }
                                ]
                            }
                        ],
                        "references": [{"url": "https://nvd.example/29927"}],
                    }
                }
            ]
        },
        "CVE-2024-00000": {
            "vulnerabilities": [{"cve": {"id": "CVE-2024-00000", "weaknesses": []}}]
        },
    }
    OSV = {
        "CVE-2025-29927": {
            "id": "CVE-2025-29927",
            "aliases": ["GHSA-f82v-jwr5-mffw"],
            "affected": [],
            "references": [{"url": "https://osv.example/cve"}],
        },
        "GHSA-f82v-jwr5-mffw": {
            "id": "GHSA-f82v-jwr5-mffw",
            "affected": [
                {
                    "package": {"ecosystem": "npm", "name": "next"},
                    "ranges": [
                        {"type": "ECOSYSTEM", "events": [{"fixed": "15.2.3"}]},
                        {"type": "GIT", "events": [{"fixed": "deadbeef" * 5}]},
                    ],
                }
            ],
            "references": [{"url": "https://osv.example/ghsa"}],
        },
    }
    EPSS = {"CVE-2025-29927": "0.992", "CVE-2024-00000": None}

    def __init__(self):
        self.kev = {
            "catalogVersion": "2026.10.26",
            "dateReleased": "2026-10-26T00:00:00Z",
            "vulnerabilities": [
                {
                    "cveID": "CVE-2025-29927",
                    "vendorProject": "Vercel",
                    "product": "Next.js",
                    "vulnerabilityName": "Next.js middleware bypass",
                    "shortDescription": "Authorization bypass via a crafted header.",
                    "requiredAction": "Apply updates.",
                    "dueDate": "2025-04-17",
                    "dateAdded": "2025-03-25",
                    "knownRansomwareCampaignUse": "Known",
                    "cwes": ["CWE-863"],
                    "notes": "https://nextjs.example/advisory",
                },
                {
                    "cveID": "CVE-2024-00000",
                    "vulnerabilityName": "A memory bug",
                    "shortDescription": "Local overflow.",
                    "dateAdded": "2024-01-01",
                    "knownRansomwareCampaignUse": "Unknown",
                    "cwes": ["CWE-119"],
                    "notes": "",
                },
            ],
        }

    def get_json(self, url):
        if url.startswith(feeds.KEV_URL):
            return self.kev
        if url.startswith(feeds.NVD_URL):
            cve = url.split("cveId=")[1]
            return self.NVD.get(cve, {"vulnerabilities": []})
        if url.startswith(feeds.OSV_VULN_URL):
            key = url.rsplit("/", 1)[1]
            if key not in self.OSV:
                raise OSError("not found")
            return self.OSV[key]
        if url.startswith(feeds.EPSS_URL):
            cve = url.split("cve=")[1]
            score = self.EPSS.get(cve)
            return {"data": [{"epss": score, "percentile": "0.99"}] if score else []}
        raise KeyError(url)


class FeedTests(unittest.TestCase):
    def test_osv_follows_alias_and_skips_git(self):
        entries, refs = feeds.osv_fixed_versions(FakeFetcher(), "CVE-2025-29927")
        self.assertIn(
            {"ecosystem": "npm", "package": "next", "fixed": "15.2.3"}, entries
        )
        self.assertFalse(
            any(len(e["fixed"]) == 40 for e in entries)
        )  # no commit hashes
        self.assertIn("https://osv.example/ghsa", refs)

    def test_nvd_cwes_and_fixed(self):
        obj = FakeFetcher().NVD["CVE-2025-29927"]["vulnerabilities"][0]["cve"]
        self.assertEqual(feeds.nvd_cwes(obj), ["CWE-285", "CWE-863"])
        fixed = feeds.nvd_fixed_versions(obj)
        self.assertTrue(any(e["fixed"] == "15.2.3" for e in fixed))


class EnrichTests(unittest.TestCase):
    def test_tips_aggregate_dedupe_and_prefer_osv(self):
        f = FakeFetcher()
        t = enrich.tips("CVE-2025-29927", fetcher=f, kev=enrich.kev_index(f))
        self.assertTrue(t["actively_exploited"])
        self.assertTrue(t["known_ransomware"])
        self.assertAlmostEqual(t["epss"]["score"], 0.992)
        self.assertIn("next 15.2.3", t["advice"])
        self.assertNotIn("firmware", t["advice"])  # CPE noise not in the one-liner
        # references deduped across OSV + NVD + KEV
        self.assertEqual(len(t["references"]), len(set(t["references"])))
        self.assertIn("https://nextjs.example/advisory", t["references"])


class TriageTests(unittest.TestCase):
    def test_web_relevance(self):
        self.assertTrue(triage.web_relevant(["CWE-863"]))
        self.assertTrue(triage.web_relevant(["CWE-89"]))
        self.assertFalse(triage.web_relevant(["CWE-119"]))
        self.assertFalse(triage.web_relevant([]))

    def test_rank_prioritizes_exploited(self):
        hot = {
            "actively_exploited": True,
            "known_ransomware": True,
            "epss": {"score": 0.9},
        }
        cold = {"actively_exploited": False, "known_ransomware": False, "epss": None}
        self.assertGreater(triage.rank(hot), triage.rank(cold))


class DraftTests(unittest.TestCase):
    GOOD = {
        "route": "/dashboard",
        "signature": {
            "method": "GET",
            "query": {},
            "headers": {"x-middleware-subrequest": "marker"},
        },
        "expect": {"blocked_status": 403, "origin": "absent"},
        "controls": [
            {
                "id": "benign",
                "query": {},
                "headers": {},
                "expect": {"status": 200, "origin": "seen"},
            }
        ],
    }

    def _draft(self, proposal):
        return draft.draft_profile(
            "CVE-2025-29927",
            title="Next.js bypass",
            references=["https://nextjs.example/advisory"],
            cwes=["CWE-863"],
            advisory_text="crafted x-middleware-subrequest header",
            drafter=draft.StubDrafter(proposal),
        )

    def test_valid_draft_becomes_a_profile(self):
        profile, reason = self._draft(self.GOOD)
        self.assertIsNone(reason)
        self.assertEqual(profile["cve"], "CVE-2025-29927")
        self.assertEqual(profile["expect"], {"status": 403, "origin": "absent"})

    def test_payload_draft_is_rejected_not_written(self):
        # A prompt-injected advisory makes the drafter emit a real payload.
        for bad in [
            "${jndi:ldap://x/y}",
            "a; rm -rf /",
            "<script>alert(1)</script>",
            "../../etc/passwd",
        ]:
            proposal = json.loads(json.dumps(self.GOOD))
            proposal["signature"]["headers"]["x-middleware-subrequest"] = bad
            with self.subTest(bad=bad):
                profile, reason = self._draft(proposal)
                self.assertIsNone(profile)
                self.assertIn("schema rejected", reason)

    def test_drafter_cannot_add_extra_keys_or_fabricate_identity(self):
        proposal = dict(self.GOOD, cve="CVE-9999-0001", extra="x")
        profile, reason = self._draft(proposal)
        self.assertIsNone(profile)
        self.assertIn("propose only", reason)
        # identity always comes from the feeds, never the drafter
        profile, _ = self._draft(self.GOOD)
        self.assertEqual(profile["cve"], "CVE-2025-29927")
        self.assertEqual(profile["references"], ["https://nextjs.example/advisory"])


class MonitorTests(unittest.TestCase):
    def test_triage_filters_and_enriches_only_relevant(self):
        result = monitor.candidates(FakeFetcher(), limit=10)
        ids = [c["cve"] for c in result["candidates"]]
        self.assertEqual(
            ids, ["CVE-2025-29927"]
        )  # the CWE-119 memory bug is filtered out
        c = result["candidates"][0]
        self.assertEqual(c["status"], "stub")
        self.assertTrue(c["tips"]["actively_exploited"])

    def test_known_cves_are_deduped(self):
        result = monitor.candidates(FakeFetcher(), limit=10, known={"CVE-2025-29927"})
        self.assertEqual(result["candidates"], [])

    def test_drafter_fills_signature_and_writes_files(self):
        result = monitor.candidates(
            FakeFetcher(), limit=10, drafter=draft.StubDrafter(DraftTests.GOOD)
        )
        c = result["candidates"][0]
        self.assertEqual(c["status"], "drafted")
        self.assertEqual(c["profile"]["route"], "/dashboard")
        with tempfile.TemporaryDirectory() as d:
            monitor.write_candidates(result, d)
            self.assertTrue((Path(d) / "CVE-2025-29927" / "profile.json").exists())
            self.assertTrue((Path(d) / "CVE-2025-29927" / "tips.json").exists())

    def test_bad_draft_is_written_as_needs_review(self):
        bad = json.loads(json.dumps(DraftTests.GOOD))
        bad["signature"]["headers"]["x-middleware-subrequest"] = "a; rm -rf /"
        result = monitor.candidates(
            FakeFetcher(), limit=10, drafter=draft.StubDrafter(bad)
        )
        c = result["candidates"][0]
        self.assertEqual(c["status"], "needs-review")
        with tempfile.TemporaryDirectory() as d:
            monitor.write_candidates(result, d)
            self.assertTrue(
                (Path(d) / "CVE-2025-29927" / "profile.needs-review.json").exists()
            )


if __name__ == "__main__":
    unittest.main(verbosity=1)
