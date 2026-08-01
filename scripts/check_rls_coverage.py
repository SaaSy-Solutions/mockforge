#!/usr/bin/env python3
"""
RLS coverage audit (#832 / #960).

Goal: keep the Postgres tenant-isolation backstop honest about what it
actually covers.

## Why this exists (the blind spot)

Migration `20250101000082_rls_tenant_isolation` puts `ENABLE`/`FORCE` RLS on
five tables:

    projects, audit_logs, hosted_mocks, templates, scenarios

Those policies only bind on connections from the request-path pool — the
`NOBYPASSRLS` role wired via `APP_DATABASE_URL`. The owner pool
(`state.db.pool()` / `DATABASE_URL`) is the table owner and has `BYPASSRLS`,
so RLS is inert on it.

`scripts/rls-e2e-gate.sh` runs the E2E suite with the request-path pool on the
`NOBYPASSRLS` role. That catches every query that would **fail closed** — the
outage risk. It structurally CANNOT catch the opposite problem: a covered-table
query that runs on the **owner** pool. Such a query neither breaks nor gets
protected; it silently sits outside the backstop, and the gate stays green.

So a green gate proves "nothing fail-closes". It does NOT prove "everything is
covered". This script is the other half: it enumerates covered-table queries
that execute on the owner pool, so that surface is counted, reviewed, and
ratcheted down rather than mistaken for zero.

## What it reports

For every `sqlx::query*` expression and every call to a covered-table model
method in the registry crates, it resolves which pool the statement executes
on and classifies it:

  COVERED    — runs on the request-path pool (`self.pool` inside the store,
               `db.runtime_pool()`, or inside a `with_org_context` /
               `with_current_org` transaction). RLS applies.
  UNCOVERED  — covered-table statement on the owner pool (`db.pool()`).
               RLS does not apply. App-layer `WHERE org_id` is the only
               thing standing between this query and a cross-tenant read.
  ELEVATED   — explicitly allowlisted cross-org query (platform admin
               aggregates). Intentionally on the owner pool.

Exits non-zero when UNCOVERED exceeds the baseline below, so new
owner-pool covered-table queries have to be a deliberate, reviewed choice.

Usage:
    scripts/check_rls_coverage.py            # audit, honor baseline
    scripts/check_rls_coverage.py --list     # print every finding
    scripts/check_rls_coverage.py --update-baseline
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# Tables carrying an `org_isolation` RLS policy in migration 20250101000082.
COVERED_TABLES = ("projects", "audit_logs", "hosted_mocks", "templates", "scenarios")

# Model types whose backing table is covered. A call to one of these executes
# a covered-table query even though no table name appears at the call site.
COVERED_MODELS = ("HostedMock", "Project", "Scenario", "Template", "AuditLog")

CRATE_ROOTS = (
    REPO_ROOT / "crates" / "mockforge-registry-server" / "src",
    REPO_ROOT / "crates" / "mockforge-registry-core" / "src",
)

BASELINE_PATH = REPO_ROOT / "scripts" / "rls-coverage-baseline.json"

# Call sites that are legitimately cross-org and must stay on the owner pool.
# Keyed by "<path suffix>::<fn name>". Every entry needs a reason, and the
# reason has to be about the REQUEST not having an org to bind — not about
# convenience.
ELEVATED_ALLOWLIST = {
    # Platform-admin analytics deliberately counts every tenant; binding a
    # single org would fail-close the aggregate to that org.
    "store/postgres.rs::get_admin_analytics_snapshot": "cross-org platform admin aggregate",
    # Audit WRITES are fire-and-forget and carry an explicit org_id argument;
    # under RLS they get dropped when the request's bound org differs from the
    # row's org (e.g. creating a new org). Reads stay covered. See the comment
    # on PgRegistryStore::record_audit_event.
    "store/postgres.rs::record_audit_event": "append-only audit write, explicit org_id, must not be droppable",
    # Data plane: routes inbound traffic to a deployed mock by org+slug. This
    # is unauthenticated public traffic with no user and no org to bind, so the
    # RLS pool would fail-close every hosted mock.
    "deployment/router.rs::route_request": "public data-plane routing, no auth context to bind an org from",
    # Same, resolving a custom domain to a deployment across all orgs by design.
    "deployment/router.rs::custom_domain_fallback": "cross-org lookup by hostname, no org in scope",
    # Internal service-to-service API authenticated by a shared bearer token
    # (MOCKFORGE_INTERNAL_API_TOKEN). No user, therefore no org context.
    "handlers/internal_test_runs.rs::proxy_chaos_toggle": "internal shared-token API, no user org context",
}

# Expressions that mean "this statement runs on the RLS-enforced request path".
COVERED_EXECUTOR_PATTERNS = (
    r"&mut \*\*tx",
    r"&mut \*tx",
    r"&self\.pool",
    r"\bdb\.runtime_pool\(\)",
    r"state\.db\.runtime_pool\(\)",
    r"\bexecutor\b",
)

# Expressions that mean "this statement runs on the owner (BYPASSRLS) pool".
OWNER_EXECUTOR_PATTERNS = (
    r"state\.db\.pool\(\)",
    r"\bdb\.pool\(\)",
    r"&self\.owner_pool",
    r"self\.owner_pool",
)

TABLE_RE = re.compile(
    r"\b(?:FROM|INTO|UPDATE|JOIN|DELETE\s+FROM)\s+(?:" + "|".join(COVERED_TABLES) + r")\b",
    re.IGNORECASE,
)
MODEL_CALL_RE = re.compile(r"\b(" + "|".join(COVERED_MODELS) + r")::(\w+)\s*\(")
FN_RE = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)")


def enclosing_fn(lines: list[str], idx: int) -> str:
    """Nearest preceding `fn` declaration, for reporting and allowlisting."""
    for i in range(idx, -1, -1):
        m = FN_RE.match(lines[i])
        if m:
            return m.group(1)
    return "<top-level>"


def classify(snippet: str) -> str:
    """Resolve which pool a statement executes on."""
    for pat in OWNER_EXECUTOR_PATTERNS:
        if re.search(pat, snippet):
            return "UNCOVERED"
    for pat in COVERED_EXECUTOR_PATTERNS:
        if re.search(pat, snippet):
            return "COVERED"
    return "UNKNOWN"


def statement_span(lines: list[str], start: int) -> tuple[str, int]:
    """
    Text of the statement beginning at `start`, up to its terminating `;` (or a
    30-line cap). sqlx statements are chained builders, so the executor lands
    several lines below the SQL text.
    """
    buf = []
    for i in range(start, min(start + 30, len(lines))):
        buf.append(lines[i])
        if lines[i].rstrip().endswith(";"):
            break
    return "\n".join(buf), i


def scan_file(path: Path) -> list[dict]:
    text = path.read_text(encoding="utf-8", errors="replace")
    lines = text.split("\n")
    rel = str(path.relative_to(REPO_ROOT))
    findings: list[dict] = []

    consumed_until = -1
    for i, line in enumerate(lines):
        if i <= consumed_until:
            continue

        is_sql = "sqlx::query" in line or "QueryBuilder::new" in line
        model_hit = MODEL_CALL_RE.search(line)
        if not is_sql and not model_hit:
            continue

        snippet, end = statement_span(lines, i)
        consumed_until = end

        # A sqlx statement only matters here if it names a covered table.
        if is_sql and not TABLE_RE.search(snippet):
            # QueryBuilder splits the table name onto a later push(); widen once.
            wider = "\n".join(lines[i : min(i + 30, len(lines))])
            if not TABLE_RE.search(wider):
                continue
            snippet = wider

        kind = classify(snippet)
        # `with_org_or_elevated` names the owner pool, but only as the org-less
        # fallback for the public marketplace surface — when an org IS bound it
        # runs on the RLS pool. Classify as ELEVATED (a reviewed escalation),
        # not UNCOVERED (an unreviewed one).
        if "with_org_or_elevated" in snippet:
            kind = "ELEVATED"
        fn = enclosing_fn(lines, i)
        key = f"{rel}::{fn}"
        for allow_key in ELEVATED_ALLOWLIST:
            path_part, _, fn_part = allow_key.rpartition("::")
            if fn == fn_part and rel.replace("\\", "/").endswith(path_part):
                kind = "ELEVATED"
                break

        findings.append(
            {
                "file": rel,
                "line": i + 1,
                "fn": fn,
                "kind": kind,
                "via": "model" if (model_hit and not is_sql) else "sql",
                "key": key,
            }
        )
    return findings


def collect() -> list[dict]:
    out: list[dict] = []
    for root in CRATE_ROOTS:
        if not root.exists():
            continue
        for path in sorted(root.rglob("*.rs")):
            out.extend(scan_file(path))
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--list", action="store_true", help="print every finding")
    ap.add_argument("--update-baseline", action="store_true", help="rewrite the baseline file")
    args = ap.parse_args()

    findings = collect()
    counts: dict[str, int] = {}
    for f in findings:
        counts[f["kind"]] = counts.get(f["kind"], 0) + 1

    uncovered = [f for f in findings if f["kind"] == "UNCOVERED"]

    print("RLS coverage audit (#832)")
    print(f"  covered tables: {', '.join(COVERED_TABLES)}")
    print()
    for kind in ("COVERED", "UNCOVERED", "ELEVATED", "UNKNOWN"):
        print(f"  {kind:10} {counts.get(kind, 0)}")
    print()

    if args.list:
        for f in sorted(findings, key=lambda x: (x["kind"], x["file"], x["line"])):
            print(f"  {f['kind']:10} {f['file']}:{f['line']}  {f['fn']}()  [{f['via']}]")
        print()

    if args.update_baseline:
        BASELINE_PATH.write_text(
            json.dumps({"uncovered": len(uncovered)}, indent=2) + "\n", encoding="utf-8"
        )
        print(f"baseline written: uncovered={len(uncovered)}")
        return 0

    baseline = 0
    if BASELINE_PATH.exists():
        baseline = json.loads(BASELINE_PATH.read_text(encoding="utf-8")).get("uncovered", 0)

    if len(uncovered) > baseline:
        print(f"FAIL: {len(uncovered)} uncovered covered-table queries, baseline is {baseline}.")
        print("These run on the owner pool, where RLS is inert:")
        for f in uncovered[:40]:
            print(f"  {f['file']}:{f['line']}  {f['fn']}()  [{f['via']}]")
        print()
        print("Route them through the request-path pool (store methods, or")
        print("`with_org_context(state.db.runtime_pool(), org_id, ..)`), or add an")
        print("ELEVATED_ALLOWLIST entry with a reason if the query is genuinely")
        print("cross-org.")
        return 1

    if len(uncovered) < baseline:
        print(f"Uncovered count dropped to {len(uncovered)} (baseline {baseline}).")
        print("Ratchet it down: scripts/check_rls_coverage.py --update-baseline")

    print(f"OK: {len(uncovered)} uncovered (baseline {baseline}).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
