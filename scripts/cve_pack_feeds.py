"""Read-only clients for the public vulnerability feeds the monitor uses.

All network access goes through a Fetcher, so tests inject a fixed mapping and
never touch the network. Live use is GET-only, bounded, with a short timeout.

Feeds (deduped by the enrich layer):
- CISA KEV      the actively-exploited catalog (primary trigger)
- NVD API 2.0   CWE weaknesses and affected/fixed version ranges (CPE)
- OSV           per-package 'fixed in' version, following GHSA aliases
- EPSS (FIRST)  probability a CVE will be exploited (prioritization)
"""

import json
import urllib.request

KEV_URL = "https://www.cisa.gov/sites/default/files/feeds/known_exploited_vulnerabilities.json"
NVD_URL = "https://services.nvd.nist.gov/rest/json/cves/2.0"
OSV_VULN_URL = "https://api.osv.dev/v1/vulns"
EPSS_URL = "https://api.first.org/data/v1/epss"
USER_AGENT = "mockforge-cve-pack-monitor (+https://mockforge.dev)"
MAX_BYTES = 8 * 1024 * 1024


class Fetcher:
    """Bounded GET -> parsed JSON. Pass `responses` (a {url_prefix: object}
    mapping, values may be Exceptions) to stub the network in tests."""

    def __init__(self, timeout=15, responses=None):
        self.timeout = timeout
        self.responses = responses

    def get_json(self, url):
        if self.responses is not None:
            for prefix, value in self.responses.items():
                if url.startswith(prefix):
                    if isinstance(value, Exception):
                        raise value
                    return value
            raise KeyError(f"no stubbed response for {url}")
        request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        with urllib.request.urlopen(request, timeout=self.timeout) as response:
            return json.loads(response.read(MAX_BYTES))


def kev_catalog(fetcher):
    """The KEV catalog as {version, released, vulnerabilities: [...]}."""
    doc = fetcher.get_json(KEV_URL)
    return {
        "version": doc.get("catalogVersion"),
        "released": doc.get("dateReleased"),
        "vulnerabilities": doc.get("vulnerabilities", []),
    }


def nvd_cve(fetcher, cve):
    """The NVD `cve` object for one id, or None."""
    doc = fetcher.get_json(f"{NVD_URL}?cveId={cve}")
    items = doc.get("vulnerabilities") or []
    return items[0]["cve"] if items else None


def nvd_cwes(cve_obj):
    out = []
    for weakness in (cve_obj or {}).get("weaknesses", []):
        for desc in weakness.get("description", []):
            value = desc.get("value", "")
            if value.startswith("CWE-") and value not in out:
                out.append(value)
    return out


def nvd_fixed_versions(cve_obj):
    """Fixed versions implied by CPE `versionEndExcluding` across configs."""
    out = []
    for config in (cve_obj or {}).get("configurations", []):
        for node in config.get("nodes", []):
            for match in node.get("cpeMatch", []):
                end = match.get("versionEndExcluding")
                if not end:
                    continue
                parts = match.get("criteria", "").split(":")
                product = parts[4] if len(parts) > 4 else "?"
                out.append({"ecosystem": "cpe", "package": product, "fixed": end})
    return out


def _affected_fixed(vuln):
    out = []
    for affected in (vuln or {}).get("affected", []):
        pkg = affected.get("package", {})
        for rng in affected.get("ranges", []):
            # Skip GIT ranges: their 'fixed' is a commit hash, not a version.
            if rng.get("type") == "GIT":
                continue
            for event in rng.get("events", []):
                if "fixed" in event and pkg.get("name"):
                    out.append(
                        {
                            "ecosystem": pkg.get("ecosystem", "?"),
                            "package": pkg["name"],
                            "fixed": event["fixed"],
                        }
                    )
    return out


def osv_fixed_versions(fetcher, cve):
    """Per-package 'fixed in' versions from OSV, following GHSA aliases.

    Returns (entries, references). The CVE record and each aliased GHSA record
    are merged; the enrich layer dedupes against the other feeds."""
    try:
        root = fetcher.get_json(f"{OSV_VULN_URL}/{cve}")
    except (OSError, KeyError, ValueError):
        return [], []
    entries = _affected_fixed(root)
    references = [r.get("url") for r in root.get("references", []) if r.get("url")]
    for alias in root.get("aliases", []):
        if not alias.startswith("GHSA-"):
            continue
        try:
            record = fetcher.get_json(f"{OSV_VULN_URL}/{alias}")
        except (OSError, KeyError, ValueError):
            continue
        entries.extend(_affected_fixed(record))
        references.extend(
            r.get("url") for r in record.get("references", []) if r.get("url")
        )
    return entries, references


def epss(fetcher, cve):
    """(score, percentile) floats for a CVE, or (None, None)."""
    try:
        doc = fetcher.get_json(f"{EPSS_URL}?cve={cve}")
    except (OSError, KeyError, ValueError):
        return None, None
    data = doc.get("data") or []
    if not data:
        return None, None
    try:
        return float(data[0]["epss"]), float(data[0]["percentile"])
    except (KeyError, ValueError, TypeError):
        return None, None
