"""Monitor public vulnerability feeds and emit candidate CVE rule packs.

Primary trigger is CISA KEV (actively exploited). Each candidate is enriched
with tips from NVD, OSV and EPSS (deduped), triaged to web-/WAF-relevant CWEs,
and — when a drafter command is configured — given a schema-validated signature
draft. Without a drafter it emits an enriched stub to fill. CVEs already covered
by a profile are skipped, and every CVE is keyed once across all sources.

The monitor only reads feeds and writes candidate files; it sends no traffic and
opens no pull request. Review a candidate, then run it with cve_pack.py.
"""

import argparse
import datetime
import json
from pathlib import Path

from cve_pack_draft import CommandDrafter, draft_profile
from cve_pack_enrich import kev_index, tips
from cve_pack_feeds import Fetcher
from cve_pack_profile import SCHEMA_VERSION
from cve_pack_triage import rank, web_relevant

SKELETON_SIGNATURE = {
    "method": "GET",
    "query": {},
    "headers": {"x-REPLACE-with-the-rule-signature-header": "marker"},
}


def existing_cves(dirs):
    """CVE ids already covered by a profile.json under any of `dirs`."""
    covered = set()
    for d in dirs:
        for path in Path(d).rglob("profile.json"):
            try:
                covered.add(json.loads(path.read_text())["cve"])
            except (OSError, ValueError, KeyError):
                continue
    return covered


def _advisory_text(kev_entry):
    parts = [
        kev_entry.get("vulnerabilityName", ""),
        kev_entry.get("shortDescription", ""),
        kev_entry.get("requiredAction", ""),
    ]
    return "\n".join(p for p in parts if p)


def _stub(cve, title, references):
    return {
        "schema_version": SCHEMA_VERSION,
        "cve": cve,
        "title": title[:200] or cve,
        "references": references[:10] or ["https://nvd.nist.gov/vuln/detail/" + cve],
        "route": "/REPLACE/with/the/guarded/route",
        "signature": dict(SKELETON_SIGNATURE),
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


def candidates(fetcher, *, since=None, limit=50, drafter=None, known=()):
    """Enriched candidate dicts for KEV CVEs, most-urgent first.

    Triage and ordering use the cheap per-entry KEV fields (its own `cwes`, the
    ransomware flag, dateAdded), so only the top `limit` web-relevant CVEs are
    enriched with per-CVE NVD/OSV/EPSS calls. Entries without KEV CWEs are not
    triaged in and would need a hand-authored profile."""
    index, meta = kev_index(fetcher)
    covered = set(known)
    pool = []
    for cve, entry in index.items():
        if cve in covered:
            continue
        if since and (entry.get("dateAdded") or "") < since:
            continue
        if not web_relevant(entry.get("cwes")):
            continue
        pool.append((cve, entry))
    # Cheap ordering: ransomware-linked first, then most recently added.
    pool.sort(
        key=lambda r: (
            r[1].get("knownRansomwareCampaignUse") == "Known",
            r[1].get("dateAdded", ""),
        ),
        reverse=True,
    )

    out = []
    for cve, entry in pool[:limit]:
        t = tips(cve, fetcher=fetcher, kev=(index, meta))
        title = entry.get("vulnerabilityName") or entry.get("shortDescription") or cve
        references = t["references"]
        candidate = {
            "cve": cve,
            "title": title[:200],
            "rank": round(rank(t), 3),
            "tips": t,
            "status": "stub",
            "reason": None,
            "profile": _stub(cve, title, references),
        }
        if drafter is not None:
            profile, reason = draft_profile(
                cve,
                title=title[:200],
                references=references[:10]
                or ["https://nvd.nist.gov/vuln/detail/" + cve],
                cwes=t["cwes"],
                advisory_text=_advisory_text(entry),
                drafter=drafter,
            )
            if profile is not None:
                candidate.update(status="drafted", profile=profile)
            else:
                candidate.update(status="needs-review", reason=reason)
        out.append(candidate)
        if len(out) >= limit:
            break
    return {"kev_version": meta["version"], "generated": _now(), "candidates": out}


def _now():
    return datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def write_candidates(result, out_dir):
    """Write each candidate's profile and tips under out_dir/<cve>/."""
    base = Path(out_dir)
    for c in result["candidates"]:
        d = base / c["cve"]
        d.mkdir(parents=True, exist_ok=True)
        name = (
            "profile.json"
            if c["status"] in ("drafted", "stub")
            else "profile.needs-review.json"
        )
        (d / name).write_text(json.dumps(c["profile"], indent=2) + "\n")
        (d / "tips.json").write_text(json.dumps(c["tips"], indent=2) + "\n")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--out-dir", required=True, help="where candidate packs are written"
    )
    parser.add_argument("--report", help="write the run report JSON here")
    parser.add_argument("--since", help="only KEV entries dateAdded >= YYYY-MM-DD")
    parser.add_argument("--limit", type=int, default=50)
    parser.add_argument(
        "--known-dir",
        action="append",
        default=[],
        help="profiles already covered (repeatable); these CVEs are skipped",
    )
    parser.add_argument(
        "--drafter-cmd",
        help="command that reads a JSON context on stdin and prints a JSON "
        "signature proposal; its output is schema-validated before use",
    )
    parser.add_argument("--timeout", type=float, default=20)
    args = parser.parse_args(argv)
    try:
        if not 0 < args.timeout <= 60 or not 1 <= args.limit <= 500:
            raise ValueError("timeout must be 0..60s; limit must be 1..500")
        fetcher = Fetcher(timeout=args.timeout)
        drafter = CommandDrafter(args.drafter_cmd) if args.drafter_cmd else None
        known = existing_cves(args.known_dir) if args.known_dir else set()
        result = candidates(
            fetcher, since=args.since, limit=args.limit, drafter=drafter, known=known
        )
        write_candidates(result, args.out_dir)
        if args.report:
            summary = {
                "kev_version": result["kev_version"],
                "generated": result["generated"],
                "candidates": [
                    {k: c[k] for k in ("cve", "title", "rank", "status", "reason")}
                    | {"advice": c["tips"]["advice"]}
                    for c in result["candidates"]
                ],
            }
            Path(args.report).write_text(json.dumps(summary, indent=2) + "\n")
    except (ValueError, OSError) as exc:
        parser.error(str(exc))
    for c in result["candidates"]:
        print(f"{c['cve']} [{c['status']}] rank={c['rank']}: {c['tips']['advice']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
