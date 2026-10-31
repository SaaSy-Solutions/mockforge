"""Aggregate actionable 'tips' for a CVE across every feed, deduped.

When the monitor flags a CVE, or `cve_pack run` finds a rule gap, these tips
tell the user what to do: the version to upgrade to, how urgent it is (actively
exploited, exploitation probability), and the authoritative remediation action.
Nothing here sends traffic; it only reads public feeds.
"""

import http.client
import urllib.error

from cve_pack_feeds import (
    epss,
    kev_catalog,
    nvd_cve,
    nvd_cwes,
    nvd_fixed_versions,
    osv_fixed_versions,
)


def kev_index(fetcher):
    """{cveID: entry} for fast membership, plus the catalog version/date."""
    catalog = kev_catalog(fetcher)
    index = {v["cveID"]: v for v in catalog["vulnerabilities"] if v.get("cveID")}
    return index, {"version": catalog["version"], "released": catalog["released"]}


def _dedupe(items, key):
    seen, out = set(), []
    for item in items:
        k = key(item)
        if k not in seen:
            seen.add(k)
            out.append(item)
    return out


def _kev_tip(entry):
    if not entry:
        return None
    notes = entry.get("notes", "") or ""
    refs = [
        u.strip()
        for u in notes.replace(",", ";").split(";")
        if u.strip().startswith("http")
    ]
    return {
        "listed": True,
        "date_added": entry.get("dateAdded"),
        "required_action": entry.get("requiredAction"),
        "due_date": entry.get("dueDate"),
        "known_ransomware": entry.get("knownRansomwareCampaignUse") == "Known",
        "references": refs,
    }


class FeedResults:
    """Record failed reads, including optional OSV aliases, without losing other feeds."""

    def __init__(self, fetcher):
        self.fetcher = fetcher
        self.errors = []

    def get_json(self, url):
        try:
            return self.fetcher.get_json(url)
        except (OSError, ValueError, http.client.HTTPException) as error:
            # OSV uses 404 for an identifier absent from its catalog.
            if (
                isinstance(error, urllib.error.HTTPError)
                and error.code == 404
                and url.startswith("https://api.osv.dev/v1/vulns/")
            ):
                return {}
            self.errors.append(
                {
                    "url": url,
                    "error": type(error).__name__,
                    "http_status": getattr(error, "code", None),
                }
            )
            raise OSError(f"Feed unavailable: {url}") from error


def tips(cve, *, fetcher, kev=None):
    """Aggregate tips for one CVE. `kev` is an optional (index, meta) from
    kev_index(); pass it when enriching many CVEs to avoid refetching KEV."""
    fetcher = FeedResults(fetcher)
    if kev is None:
        try:
            kev = kev_index(fetcher)
        except (OSError, ValueError):
            kev = None
    kev_entry = (kev[0] if kev else {}).get(cve)
    kev_tip = _kev_tip(kev_entry)

    try:
        cve_obj = nvd_cve(fetcher, cve)
    except (OSError, ValueError):
        cve_obj = None
    cwes = nvd_cwes(cve_obj)
    osv_entries, osv_refs = osv_fixed_versions(fetcher, cve)
    osv_entries = _dedupe(
        osv_entries, key=lambda e: (e["ecosystem"], e["package"], e["fixed"])
    )
    # OSV ecosystem fixes name the actual package; NVD CPE adds bundled-product
    # noise (firmware that ships the library), kept only as a capped fallback.
    cpe_entries = _dedupe(
        nvd_fixed_versions(cve_obj), key=lambda e: (e["package"], e["fixed"])
    )
    fixed = osv_entries + cpe_entries[:20]
    score, percentile = epss(fetcher, cve)

    references = _dedupe(
        [{"url": u} for u in osv_refs]
        + [
            {"url": r.get("url")}
            for r in (cve_obj or {}).get("references", [])
            if r.get("url")
        ]
        + [{"url": u} for u in (kev_tip["references"] if kev_tip else [])],
        key=lambda r: r["url"],
    )

    return {
        "cve": cve,
        "complete": not fetcher.errors,
        "feed_errors": fetcher.errors,
        "actively_exploited": bool(kev_tip) if kev is not None else None,
        "known_ransomware": bool(kev_tip and kev_tip["known_ransomware"])
        if kev is not None
        else None,
        "epss": None if score is None else {"score": score, "percentile": percentile},
        "cwes": cwes,
        "fixed_versions": fixed,
        "kev": kev_tip,
        "references": [r["url"] for r in references][:15],
        # Prefer the named-package (OSV) fixes for the one-line advice; CPE
        # firmware fallback is kept in fixed_versions but not dumped here.
        "advice": _advice(kev_tip, osv_entries or cpe_entries, score),
    }


def _advice(kev_tip, fixed, score):
    bits = []
    if kev_tip:
        bits.append("actively exploited (CISA KEV)")
        if kev_tip["known_ransomware"]:
            bits.append("used in ransomware")
        if kev_tip.get("due_date"):
            bits.append(f"CISA remediation due {kev_tip['due_date']}")
    if score is not None:
        bits.append(f"EPSS {score:.0%}")
    if fixed:
        where = ", ".join(sorted({f"{e['package']} {e['fixed']}" for e in fixed})[:4])
        bits.append(f"upgrade to {where}")
    else:
        bits.append(
            "no fixed version found in available feed data; consult the vendor advisory"
        )
    return "; ".join(bits)
