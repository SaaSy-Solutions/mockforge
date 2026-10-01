#!/usr/bin/env python3
"""
RLS coverage audit (#832 / #960 / #1087).

Keeps the Postgres tenant-isolation backstop honest about what it covers.

## The two failure modes this guards

Two roles talk to the database:

  * the OWNER role (`DATABASE_URL`, `state.db.pool()`, `self.owner_pool`). It
    has `BYPASSRLS`, so RLS is inert on it. Migrations, background workers and
    genuinely cross-org paths run here.
  * the RUNTIME role (`APP_DATABASE_URL`, `state.db.runtime_pool()`,
    `self.pool` inside the store). It is `NOBYPASSRLS`, so every forced table
    is filtered by `app.current_org_id`.

A statement on a forced table can go wrong in two opposite ways:

  UNCOVERED  it runs on the owner pool. It never breaks and is never
             protected: app-layer `WHERE org_id` is the only thing between it
             and a cross-tenant read. `scripts/rls-e2e-gate.sh` cannot see it.
  UNBOUND    it runs on the runtime pool OUTSIDE any GUC-binding helper
             (`with_current_org`, `with_optional_org`, `with_org_context`,
             `with_org_or_elevated`). Under `NOBYPASSRLS` it fail-closes to
             zero rows / a WITH CHECK error. That is an outage, and the e2e gate
             only catches it on the paths its tests happen to exercise.

Both are counted and ratcheted against a checked-in baseline. The target for
both is 0.

The other classes:

  COVERED    runs on the runtime pool inside a GUC-binding helper (or inside a
             `&mut **tx` / `executor` handed down from one).
  ELEVATED   deliberately on the owner pool, with a stated reason: either an
             ELEVATED_ALLOWLIST entry or an ELEVATED_PATH_PREFIXES entry.
  UNKNOWN    the executor could not be resolved (e.g. a `pool` parameter).
             Also ratcheted, so the resolver cannot quietly go blind.

## What counts as "a statement on a forced table"

  * any `sqlx::query*` / `QueryBuilder::new` whose SQL names a forced table;
  * any call to a model method whose body names a forced table. The model map
    is built per METHOD (not per type), so `Organization::find_by_id` (the
    `organizations` table, not forced) is not confused with
    `Organization::create` (which also inserts into `org_members`).

The forced-table set is read from the migrations (`FORCE ROW LEVEL SECURITY`),
so adding a policy automatically widens the audit.

## Which GUC a statement needs

Each table's required GUC is read from its policies (`app.current_org_id`,
`app.current_user_id`). A statement inside a helper that binds none of them is
UNBOUND. `user_public_keys` reads both (owner via the user GUC, org-shared keys
via the org GUC); under an org-only helper a statement on it is accepted only
if its SQL filters on `org_id = $n`, otherwise it is a personal-key query that
would see nothing and is reported UNBOUND. The check is lexical: it looks at
the statement text, not at what the bound value is.

## Known blind spots (it is a lexical scanner, not a type checker)

  * Variable SQL: `sqlx::query(&query)` / `query_as(&sql)` where the SQL text
    was built earlier (format!, push_str) is invisible, because no forced table
    name appears in the statement. `QueryBuilder::new(..)` is followed through
    the rest of the function; plain string-built SQL is not.
  * Statements longer than 30 lines (the span cap) can lose their executor
    argument and fall to UNKNOWN or be missed.
  * Executor provenance is resolved within one function: a pool or tx passed
    in as a parameter is classified at the CALLER only if the caller's call is
    itself a mapped model call; otherwise the callee reports UNKNOWN.
  * Macros (`sqlx::query!`) and SQL in non-Rust files are not scanned.

Review new raw SQL with these in mind; the NOBYPASSRLS e2e gate is the
runtime backstop for whatever this misses.

Usage:
    scripts/check_rls_coverage.py            # audit, honor baseline
    scripts/check_rls_coverage.py --list     # print every finding
    scripts/check_rls_coverage.py --list --kind UNCOVERED,UNBOUND
    scripts/check_rls_coverage.py --update-baseline
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

MIGRATIONS_DIR = REPO_ROOT / "crates" / "mockforge-registry-server" / "migrations"
MODELS_DIR = REPO_ROOT / "crates" / "mockforge-registry-core" / "src" / "models"

CRATE_ROOTS = (
    REPO_ROOT / "crates" / "mockforge-registry-server" / "src",
    REPO_ROOT / "crates" / "mockforge-registry-core" / "src",
)

# SQLite has no RLS; the SQLite store is a separate backend and irrelevant here.
# Model files are audited at their CALL sites, not at their SQL.
SKIP_PATH_PARTS = ("/store/sqlite.rs", "/models/")

BASELINE_PATH = REPO_ROOT / "scripts" / "rls-coverage-baseline.json"

KINDS = ("COVERED", "UNCOVERED", "UNBOUND", "ELEVATED", "UNKNOWN")
RATCHETED = ("UNCOVERED", "UNBOUND", "UNKNOWN")

# Whole files/directories that are elevated by construction. Every entry must
# say why the code in it has no request org to bind.
ELEVATED_PATH_PREFIXES = {
    "mockforge-registry-server/src/workers/": (
        "background workers sweep every tenant on a timer; there is no request "
        "and therefore no org to bind"
    ),
    "mockforge-registry-server/src/deployment/": (
        "deployment orchestrator / health / metrics / cleanup / data-plane router "
        "sweep or route across all orgs"
    ),
}

# Individual functions that must stay on the owner pool. Keyed by
# "<path suffix>::<fn name>". The reason has to be about the code path not
# having an org to bind (or legitimately spanning orgs) -- not convenience.
ELEVATED_ALLOWLIST = {
    # ---- cross-org platform admin -------------------------------------------
    "store/postgres.rs::get_admin_analytics_snapshot": "cross-org platform admin aggregate",
    "store/postgres.rs::get_conversion_funnel_snapshot": "cross-org platform admin aggregate",
    # ---- append-only writes with an explicit, already-authorized org_id -----
    # Fire-and-forget. Under RLS they are dropped when the request's bound org
    # differs from the row's org (e.g. creating a new org). The WITH CHECK adds
    # no authorization, only data loss. Reads of the same tables stay covered.
    "store/postgres.rs::record_audit_event": "append-only audit write, explicit org_id, must not be droppable",
    "store/postgres.rs::record_feature_usage": "append-only telemetry write, explicit org_id, must not be droppable",
    "store/postgres.rs::record_suspicious_activity": "append-only security telemetry, often pre-auth with no org",
    "platform_signing.rs::audited_begin_handover": "append-only audit of a platform-operator action",
    "platform_signing.rs::audited_retire_old": "append-only audit of a platform-operator action",
    "platform_signing.rs::audited_emergency_revoke": "append-only audit of a platform-operator action",
    # ---- pre-tenant / user-scoped across orgs -------------------------------
    # These run BEFORE there is an org to bind, or deliberately span every org
    # the user belongs to. The access rule is the user_id filter.
    "store/postgres.rs::create_organization": "creates a new tenant and its owner membership; no org exists yet to bind",
    "store/postgres.rs::get_or_create_personal_org": "signup/login creates the user's personal tenant; no org exists yet",
    "store/postgres.rs::list_organizations_by_user": "lists the caller's orgs across tenants (user-scoped)",
    "store/postgres.rs::list_user_api_tokens": "GDPR export of the user's own tokens across orgs (user-scoped)",
    "store/postgres.rs::delete_user_data_cascade": "GDPR erasure of the user across every org they belong to",
    "store/postgres.rs::list_keys_for_publisher": "publish-time signature check: keys of every org the author belongs to",
    "store/postgres.rs::find_user_public_key_by_id": "learns a key's org tag so the caller's admin role on THAT org can be checked; the org to bind comes from the answer",
    "handlers/users_me.rs::find_default_org_id": "mirrors resolve_org_context's default-org pick across the user's orgs",
    "handlers/public_keys.rs::enforce_publisher_key_quota": "finds the user's owned org across tenants to pick the quota",
    # ---- the lookups that PRODUCE the org to bind ---------------------------
    # RLS on these is circular for the lookup itself: the GUC is derived from
    # the answer. The membership/token management endpoints stay covered.
    "middleware/org_context.rs::resolve_org_context": "org resolution that produces the org binding",
    "middleware/org_context.rs::verify_org_access": "membership check inside org resolution",
    "middleware/api_token_auth.rs::authenticate_api_token": "pre-auth token lookup by hash; the token names the org",
    "store/postgres.rs::verify_api_token": "pre-auth token lookup; the token names the org",
    "store/postgres.rs::find_sso_config_by_email_domain": "pre-auth SSO discovery by email domain, cross-org by definition",
    # ---- genuinely cross-tenant lookups -------------------------------------
    "handlers/tunnels.rs::subdomain_taken_by_any_org": "tunnel subdomains are globally unique across tenants",
    "handlers/scenario_promotions.rs::find_promotable_scenario": "reads another org's PUBLISHED marketplace scenario to promote it; visibility rule (own / public / non-yanked version) is in the SQL",
    # Marketplace rating aggregates: the reviewer is (by construction) usually
    # in a different org than the template/scenario owner; the stats are
    # derived solely from the reviews table.
    "store/postgres.rs::update_template_review_stats": "derived rating stats on another org's published template",
    "store/postgres.rs::update_scenario_review_stats": "derived rating stats on another org's published scenario",
    # ---- data plane / internal service APIs ---------------------------------
    "incident_bus.rs::raise": "constructed only with the owner pool by internal runner callbacks (shared token, no user org)",
    "handlers/internal_test_runs.rs::proxy_chaos_toggle": "internal shared-token API, no user org context",
    "handlers/internal_test_runs.rs::run_started": "internal shared-token runner callback, no user org context",
    "handlers/internal_test_runs.rs::run_finished": "internal shared-token runner callback, no user org context",
    "handlers/internal_test_runs.rs::mirror_kind_status": "internal shared-token runner callback, no user org context",
    "handlers/internal_test_runs.rs::maybe_raise_finding_incident": "internal shared-token runner callback, no user org context",
    "handlers/internal_test_runs.rs::get_capture_exchanges": "internal shared-token replay callback, no user org context; SQL constrains captures to the session's org",
    "handlers/internal_test_runs.rs::get_tunnel_reservation_by_subdomain": "internal shared-token tunnel lookup by subdomain, cross-org",
    "handlers/token_rotation.rs::send_rotation_reminders": "cross-org reminder sweep driven by a worker",
    "handlers/internal_test_runs.rs::ingest_runner_event": "internal shared-token runner callback, no user org context",
    "handlers/internal_test_runs.rs::get_workspace_endpoint_hits": "internal shared-token runner API, no user org context",
    "handlers/internal_test_runs.rs::get_capture_exchanges": "internal shared-token runner API, no user org context",
    "handlers/internal_test_runs.rs::get_fitness_function": "internal shared-token runner API, no user org context",
    "handlers/internal_test_runs.rs::get_deployment_latency_stats": "internal shared-token runner API, no user org context",
    "handlers/internal_test_runs.rs::get_monitored_service_contract_stability": "internal shared-token runner API, no user org context",
    "handlers/internal_contract_diff.rs::fetch_samples": "internal shared-token runner API; the org comes from the runner payload, not a user",
    # Data-plane ingest: authenticated by a per-deployment ingest token, not a
    # user. The token names the deployment; there is no request org to bind.
    "handlers/hosted_mocks.rs::ingest_runtime_logs": "data-plane log shipper, per-deployment ingest token, no user org",
    "handlers/hosted_mocks.rs::ingest_runtime_captures": "data-plane recorder sync, per-deployment ingest token, no user org",
    "handlers/otlp.rs::persist_span_rows": "OTLP span ingest (HTTP + gRPC), per-deployment token, no user org",
    # Stripe webhooks: no user; the org comes from the signed payload.
    "handlers/billing.rs::handle_subscription_event": "Stripe webhook: no user, org resolved from the signed payload",
    "handlers/billing.rs::handle_subscription_deleted": "Stripe webhook: no user, org resolved from the signed payload",
    "handlers/billing.rs::handle_payment_succeeded": "Stripe webhook: no user, org resolved from the signed payload",
    "handlers/billing.rs::handle_payment_failed": "Stripe webhook: no user, org resolved from the signed payload",
}

# Expressions that mean "this statement runs on the request-path (runtime) pool".
RUNTIME_EXECUTOR_PATTERNS = (
    r"&self\.pool\b",
    r"\bself\.pool\b",
    r"\bdb\.runtime_pool\(\)",
)

# Expressions that mean "handed down from a GUC-bound transaction".
BOUND_EXECUTOR_PATTERNS = (
    r"&mut \*\*tx\b",
    r"&mut \*tx\b",
    r"\bexecutor\b",
    r"&mut \*\*conn\b",
    r"&mut \*conn\b",
    # Bare `tx` as an argument (clippy's explicit_auto_deref prefers it over
    # `&mut **tx` when the callee takes `&mut PgConnection`).
    r"[(,]\s*tx\s*[,)]",
    r"(?m)^\s*tx,\s*$",
)

# Expressions that mean "this statement runs on the owner (BYPASSRLS) pool".
OWNER_EXECUTOR_PATTERNS = (
    r"state\.db\.pool\(\)",
    r"\bdb\.pool\(\)",
    r"\bself\.owner_pool\b",
    r"\bowner_pool\b",
    r"\bstore\.pool\(\)",
)

BINDING_HELPERS = (
    "with_current_org",
    "with_optional_org",
    "with_org_context",
    "with_org_or_elevated",
    "with_current_user",
    "with_user_context",
)

# Which GUC each helper binds. A statement inside a helper is only COVERED when
# the helper binds a GUC that one of the table's policies actually reads:
# a `user_public_keys` query inside `with_org_context` sees only org-shared
# keys, and a `flows` query inside `with_current_user` sees nothing.
HELPER_GUCS = {
    "with_current_org": {"org"},
    "with_optional_org": {"org"},
    "with_org_context": {"org"},
    "with_org_or_elevated": {"org"},
    "with_current_user": {"user"},
    "with_user_context": {"user"},
}

INSTANCE_CALL_RE = re.compile(
    r"\.(\w+)\(\s*(?:&self\.pool|self\.pool|&?self\.owner_pool|state\.db\.pool\(\)"
    r"|state\.db\.runtime_pool\(\)|&mut \*\*tx|&mut \*tx|&mut \*conn|executor\b|&?pool\b)"
)
FN_RE = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)")
IMPL_RE = re.compile(r"^impl(?:<[^>]*>)?\s+(?:[\w:]+\s+for\s+)?(\w+)")
USE_ALIAS_RE = re.compile(r"\b(\w+)\s+as\s+(\w+)\b")


# ---------------------------------------------------------------------------
# Forced tables, from the migrations.
# ---------------------------------------------------------------------------
def forced_tables() -> tuple[str, ...]:
    forced: set[str] = set()
    unforced: set[str] = set()
    for mig in sorted(MIGRATIONS_DIR.glob("*.sql")):
        # Strip SQL line comments so the documented rollback block does not count.
        text = "\n".join(l.split("--", 1)[0] for l in mig.read_text(encoding="utf-8").split("\n"))
        for m in re.finditer(r"ALTER\s+TABLE\s+(?:IF\s+EXISTS\s+)?(?:ONLY\s+)?(?:public\.)?(\w+)\s+FORCE\s+ROW\s+LEVEL\s+SECURITY", text, re.I):
            forced.add(m.group(1).lower())
            unforced.discard(m.group(1).lower())
        for m in re.finditer(r"ALTER\s+TABLE\s+(?:IF\s+EXISTS\s+)?(?:ONLY\s+)?(?:public\.)?(\w+)\s+NO\s+FORCE\s+ROW\s+LEVEL\s+SECURITY", text, re.I):
            forced.discard(m.group(1).lower())
    return tuple(sorted(forced))


def table_gucs() -> dict[str, set[str]]:
    """
    {table: {"org", "user"}}: the GUCs the table's policies read, from every
    `CREATE POLICY .. ON <table> ..;` in the migrations. Policies are OR-ed, so
    binding any one of them makes the table reachable.
    """
    gucs: dict[str, set[str]] = {}
    for mig in sorted(MIGRATIONS_DIR.glob("*.sql")):
        text = "\n".join(l.split("--", 1)[0] for l in mig.read_text(encoding="utf-8").split("\n"))
        for m in re.finditer(r"CREATE\s+POLICY\s+\w+\s+ON\s+(?:public\.)?(\w+)(.*?);", text, re.I | re.S):
            found = set(re.findall(r"app\.current_(org|user)_id", m.group(2)))
            gucs.setdefault(m.group(1).lower(), set()).update(found)
    return gucs


TABLE_GUCS: dict[str, set[str]] = {}
TABLE_GUCS_NEEDED: dict[str, set[str]] = {}

# Tables whose org-GUC policy only SHARES rows owned by a user (the owner is
# policed by app.current_user_id). An org-only binding is accepted for them
# only when the statement filters on the sharing column.
ORG_SHARED_ONLY_WHEN_FILTERED = {"user_public_keys": "org_id"}


def table_re(tables: tuple[str, ...]) -> re.Pattern:
    return re.compile(
        r"\b(?:FROM|INTO|UPDATE|JOIN|DELETE\s+FROM)\s+(?:public\.)?(" + "|".join(tables) + r")\b",
        re.IGNORECASE,
    )


# ---------------------------------------------------------------------------
# Model method -> forced tables it touches.
# ---------------------------------------------------------------------------
def model_method_map(table_pattern: re.Pattern) -> tuple[dict[str, set[str]], dict[str, set[str]]]:
    """
    Returns ({"Type::method": {tables}}, {"free_fn": {tables}}) for every model
    method / free function whose body names a forced table.
    """
    methods: dict[str, set[str]] = {}
    free: dict[str, set[str]] = {}
    calls: dict[str, set[str]] = {}
    for path in sorted(MODELS_DIR.rglob("*.rs")):
        # Blank `#[cfg(test)] mod .. { }` blocks by brace matching and resume
        # after them. A test module is often in the MIDDLE of a model file
        # (models/hosted_mock.rs), so "skip everything after the first
        # #[cfg(test)]" hid every impl below it.
        lines = strip_tests(path.read_text(encoding="utf-8", errors="replace").split("\n"))
        cur_impl: str | None = None
        impl_depth = None
        depth = 0
        cur_fn: str | None = None
        cur_key_free = False
        for line in lines:
            m_impl = IMPL_RE.match(line)
            if m_impl and depth == 0:
                cur_impl = m_impl.group(1)
                impl_depth = 0
            m_fn = FN_RE.match(line)
            if m_fn:
                cur_fn = m_fn.group(1)
                cur_key_free = depth == 0
            depth += line.count("{") - line.count("}")
            if depth <= 0:
                depth = 0
                if cur_impl is not None and impl_depth is not None and "}" in line:
                    cur_impl = None
            if cur_fn:
                key = cur_fn if cur_key_free else (f"{cur_impl}::{cur_fn}" if cur_impl else None)
                if key is None:
                    continue
                for t in table_pattern.findall(line):
                    (free if cur_key_free else methods).setdefault(key, set()).add(t.lower())
                # Record model-to-model calls so a method that only reaches a
                # forced table through another method (e.g. `ApiToken::rotate`
                # -> `Self::create`) is still mapped.
                for cm in re.finditer(r"\b(Self|[A-Z]\w+)::(\w+)\s*\(", line):
                    callee_t = cur_impl if cm.group(1) == "Self" else cm.group(1)
                    if callee_t:
                        calls.setdefault(key, set()).add(f"{callee_t}::{cm.group(2)}")
    # Transitive closure over model-to-model calls.
    changed = True
    while changed:
        changed = False
        for caller, callees in calls.items():
            reach: set[str] = set()
            for callee in callees:
                reach |= methods.get(callee, set())
            target = free if caller in free or "::" not in caller else methods
            have = target.get(caller, set())
            if not reach <= have:
                target[caller] = have | reach
                changed = True
    return methods, free


# ---------------------------------------------------------------------------
# Scanning.
# ---------------------------------------------------------------------------
def enclosing_fn(lines: list[str], idx: int) -> tuple[str, int]:
    for i in range(idx, -1, -1):
        m = FN_RE.match(lines[i])
        if m:
            return m.group(1), i
    return "<top-level>", 0


def fn_body(lines: list[str], start: int) -> str:
    """Text of the function starting at `start`, by brace matching."""
    depth = 0
    seen = False
    out = []
    for i in range(start, len(lines)):
        out.append(lines[i])
        depth += lines[i].count("{") - lines[i].count("}")
        if "{" in lines[i]:
            seen = True
        if seen and depth <= 0:
            break
    return "\n".join(out)


def statement_span(lines: list[str], start: int) -> tuple[str, int]:
    buf = []
    i = start
    for i in range(start, min(start + 30, len(lines))):
        if i > start and FN_RE.match(lines[i]):
            i -= 1
            break
        buf.append(lines[i])
        stripped = lines[i].strip()
        # `;` ends a statement; a bare `}` / `})` ends an expression-bodied fn
        # or closure, so the NEXT method's call is not swallowed into this one.
        if stripped.endswith(";") or (i > start and stripped.startswith("}")):
            break
    return "\n".join(buf), i


def pool_aliases(body: str) -> tuple[set[str], set[str]]:
    """`let pool = state.db.pool();` style aliases within one function."""
    owner, runtime = set(), set()
    for m in re.finditer(r"let\s+(\w+)\s*(?::\s*&?\s*[\w:]+)?\s*=\s*&?\s*([^;]+);", body):
        name, rhs = m.group(1), m.group(2)
        if any(re.search(p, rhs) for p in OWNER_EXECUTOR_PATTERNS):
            owner.add(name)
        elif any(re.search(p, rhs) for p in RUNTIME_EXECUTOR_PATTERNS):
            runtime.add(name)
    return owner, runtime


OWNER_BEGIN_RE = re.compile(r"(?:state\.db\.pool\(\)|\bdb\.pool\(\)|\bowner_pool)\s*\.begin\(\)")
RUNTIME_BEGIN_RE = re.compile(r"(?:\bself\.pool|runtime_pool\(\))\s*\.begin\(\)")
TX_PARAM_RE = re.compile(r"\b(?:tx|conn|executor)\s*:\s*(?:&|impl\b)")


def enclosing_helper(body: str, offset: int) -> tuple[str, str] | None:
    """
    Name of the GUC-binding helper whose argument list (paren-matched) contains
    character `offset` of `body`, or None.

    Provenance matters more than presence: a function can call
    `with_current_org(..)` for one statement and run another on a bare
    `self.pool.begin()` tx. Only statements lexically inside the helper call
    receive the helper's bound `tx`.
    """
    best: tuple[int, str, str] | None = None
    for m in re.finditer(r"\b(" + "|".join(BINDING_HELPERS) + r")\s*\(", body):
        open_at = m.end() - 1
        depth = 0
        close_at = len(body)
        for k in range(open_at, len(body)):
            c = body[k]
            if c == "(":
                depth += 1
            elif c == ")":
                depth -= 1
                if depth == 0:
                    close_at = k
                    break
        if open_at < offset < close_at and (best is None or open_at > best[0]):
            best = (open_at, m.group(1), body[open_at:close_at])
    return (best[1], best[2]) if best else None


def classify(
    snippet: str,
    body: str,
    owner_alias: set[str],
    runtime_alias: set[str],
    helper_span: tuple[str, str] | None,
) -> str:
    """
    `helper` is the binding helper the statement is lexically inside (see
    `enclosing_helper`), or None.
    """
    # A transaction variable (`&mut *tx`, `&mut **otx`, ..) is classified by
    # where THAT variable came from, not by what else the function does.
    helper = helper_span[0] if helper_span else None
    tx_var = re.search(r"&mut \*{1,2}(\w+)\b", snippet) or re.search(r"[(,]\s*(tx)\s*[,)]", snippet)
    if tx_var and helper_span and re.search(rf"\|\s*{re.escape(tx_var.group(1))}\s*\|", helper_span[1]):
        # The helper's own closure argument: bound by construction.
        return "ELEVATED" if helper == "with_org_or_elevated" else "COVERED"
    if tx_var:
        name = tx_var.group(1)
        origin = re.search(rf"let\s+(?:mut\s+)?{re.escape(name)}\s*=\s*([^;]*?)\.begin\(\)", body)
        if origin:
            src = origin.group(1)
            if any(re.search(p, src) for p in OWNER_EXECUTOR_PATTERNS) or src.strip("&* ") in owner_alias:
                return "UNCOVERED"
            if any(re.search(p, src) for p in RUNTIME_EXECUTOR_PATTERNS) or src.strip("&* ") in runtime_alias:
                return "UNBOUND"
            return "UNKNOWN"
    for pat in BOUND_EXECUTOR_PATTERNS:
        if re.search(pat, snippet):
            # Only the helper's closure argument carries the GUC, or a
            # connection handed in by a caller (which is then responsible).
            if helper == "with_org_or_elevated":
                return "ELEVATED"
            if helper:
                return "COVERED"
            signature = body.split("{", 1)[0]
            if TX_PARAM_RE.search(signature):
                return "COVERED"
            return "UNKNOWN"
    for pat in OWNER_EXECUTOR_PATTERNS:
        if re.search(pat, snippet):
            return "UNCOVERED"
    for a in owner_alias:
        if re.search(rf"[(,]\s*&?\*?{re.escape(a)}\b|\.(?:fetch\w*|execute)\(\s*&?\*?{re.escape(a)}\b", snippet):
            return "UNCOVERED"
    # The runtime pool used directly is unbound even inside a helper closure:
    # only the closure's `tx` carries the GUC.
    for pat in RUNTIME_EXECUTOR_PATTERNS:
        if re.search(pat, snippet):
            return "UNBOUND"
    for a in runtime_alias:
        if re.search(rf"\b{re.escape(a)}\b", snippet):
            return "UNBOUND"
    return "UNKNOWN"


def strip_tests(lines: list[str]) -> list[str]:
    """Blank out `#[cfg(test)] mod ... { }` blocks, keeping line numbers."""
    out = list(lines)
    i = 0
    while i < len(out):
        if out[i].strip().startswith("#[cfg(test)]"):
            j = i + 1
            while j < len(out) and not out[j].strip():
                j += 1
            if j < len(out) and re.match(r"\s*(?:pub\s+)?mod\s+\w+\s*\{", out[j]):
                depth = 0
                k = j
                while k < len(out):
                    depth += out[k].count("{") - out[k].count("}")
                    out[k] = ""
                    if depth <= 0 and k > j:
                        break
                    k += 1
                out[i] = ""
                i = k
        i += 1
    return out


def global_aliases() -> dict[str, str]:
    """`pub use cloud_workspace::Workspace as CloudWorkspace;` anywhere in the
    crates. Re-exports make an alias visible in files that never spell it."""
    aliases: dict[str, str] = {}
    for root in CRATE_ROOTS:
        for path in root.rglob("*.rs"):
            text = path.read_text(encoding="utf-8", errors="replace")
            for m in re.finditer(r"^\s*(?:pub(?:\([^)]*\))?\s+)?use\s+[^;]*;", text, re.M | re.S):
                for a in USE_ALIAS_RE.finditer(m.group(0)):
                    aliases.setdefault(a.group(2), a.group(1))
    return aliases


GLOBAL_ALIASES: dict[str, str] = {}


def scan_file(path: Path, table_pattern: re.Pattern, methods: dict, free: dict) -> list[dict]:
    rel = str(path.relative_to(REPO_ROOT)).replace("\\", "/")
    raw = path.read_text(encoding="utf-8", errors="replace").split("\n")
    lines = strip_tests(raw)
    text = "\n".join(lines)

    # `use crate::models::cloud_workspace::Workspace as CloudWorkspace;`
    aliases: dict[str, str] = dict(GLOBAL_ALIASES)
    for m in re.finditer(r"^\s*(?:pub\s+)?use\s+[^;]*;", text, re.M | re.S):
        for a in USE_ALIAS_RE.finditer(m.group(0)):
            aliases[a.group(2)] = a.group(1)

    type_names = {k.split("::")[0] for k in methods}
    callable_types = type_names | {a for a, orig in aliases.items() if orig in type_names}
    model_call_re = (
        re.compile(r"\b(" + "|".join(sorted(callable_types)) + r")::(\w+)\s*\(") if callable_types else None
    )
    free_call_re = (
        re.compile(r"(?<![\w:.])(?:[\w:]+::)?(" + "|".join(sorted(free)) + r")\s*\(") if free else None
    )

    instance_methods: dict[str, set[str]] = {}
    for k, v in methods.items():
        instance_methods.setdefault(k.split("::", 1)[1], set()).update(v)

    findings: list[dict] = []
    consumed_until = -1
    for i, line in enumerate(lines):
        if i <= consumed_until:
            continue
        if FN_RE.match(line):
            continue

        tables: set[str] = set()
        via = None
        is_sql = "sqlx::query" in line or "QueryBuilder::new" in line
        snippet, end = statement_span(lines, i)
        if is_sql:
            tables = {t.lower() for t in table_pattern.findall(snippet)}
            if "QueryBuilder::new" in line:
                # QueryBuilder pushes the table / runs the executor on later
                # statements: use the rest of the enclosing function.
                _, fs = enclosing_fn(lines, i)
                body_lines = fn_body(lines, fs).split("\n")
                rest = "\n".join(body_lines[i - fs :])
                tables = tables or {t.lower() for t in table_pattern.findall(rest)}
                snippet = rest
            via = "sql"
        if not tables and model_call_re:
            for mm in model_call_re.finditer(line):
                typ = aliases.get(mm.group(1), mm.group(1))
                key = f"{typ}::{mm.group(2)}"
                if key in methods:
                    tables |= methods[key]
                    via = f"model {mm.group(1)}::{mm.group(2)}"
        if not tables:
            # Instance-method form: `template.update(&self.pool, ..)`. Only
            # counted when the first argument is a pool / executor expression,
            # so unrelated `.delete(..)` calls do not match.
            for im in INSTANCE_CALL_RE.finditer(line):
                hits = instance_methods.get(im.group(1))
                if hits:
                    tables |= hits
                    via = f"instance .{im.group(1)}"
        if not tables and free_call_re and "/models/" not in rel:
            for fm in free_call_re.finditer(line):
                if fm.group(1) in free:
                    tables |= free[fm.group(1)]
                    via = f"fn {fm.group(1)}"
        if not tables:
            continue
        consumed_until = end

        fn, fn_start = enclosing_fn(lines, i)
        body = fn_body(lines, fn_start)
        owner_alias, runtime_alias = pool_aliases(body)
        # Offset of this statement inside `body` (body starts at fn_start).
        stmt_offset = sum(len(l) + 1 for l in lines[fn_start:i]) + len(line) // 2
        helper_span = enclosing_helper(body, stmt_offset)
        kind = classify(snippet, body, owner_alias, runtime_alias, helper_span)
        reason = None
        if kind == "COVERED" and helper_span:
            bound = HELPER_GUCS.get(helper_span[0], set())
            wrong = sorted(t for t in tables if TABLE_GUCS.get(t) and not (TABLE_GUCS[t] & bound))
            # A table whose policies read BOTH GUCs (user_public_keys: owner OR
            # org-shared) is satisfied by either binding on paper, but under an
            # org-only binding it shows just the org-shared rows. A personal-key
            # statement there silently reads/writes nothing. Require the SQL to
            # filter on the sharing column to accept an org-only binding.
            if bound == {"org"}:
                for t, col in ORG_SHARED_ONLY_WHEN_FILTERED.items():
                    if t in tables and t not in wrong and not re.search(
                        rf"\b(?:\w+\.)?{col}\s*=\s*\$\d", snippet
                    ):
                        wrong.append(t)
                        TABLE_GUCS_NEEDED[t] = {"user"}
            if wrong:
                kind = "UNBOUND"
                need = set().union(*(TABLE_GUCS_NEEDED.get(t, TABLE_GUCS[t]) for t in wrong))
                reason = f"{helper_span[0]} binds {sorted(bound)}, but {','.join(wrong)} need {sorted(need)}"
        if kind in ("UNCOVERED", "UNKNOWN"):
            for prefix, why in ELEVATED_PATH_PREFIXES.items():
                if prefix in rel:
                    kind, reason = "ELEVATED", why
                    break
        if kind in ("UNCOVERED", "UNKNOWN", "UNBOUND"):
            for allow_key, why in ELEVATED_ALLOWLIST.items():
                path_part, _, fn_part = allow_key.rpartition("::")
                if fn == fn_part and rel.endswith(path_part):
                    # An allowlisted fn may only escape to the OWNER pool. An
                    # unbound runtime-pool query there is still an outage.
                    if kind != "UNBOUND":
                        kind, reason = "ELEVATED", why
                    break

        findings.append(
            {
                "file": rel,
                "line": i + 1,
                "fn": fn,
                "kind": kind,
                "via": via,
                "tables": sorted(tables),
                "reason": reason,
            }
        )
    return findings


def collect(table_pattern, methods, free) -> list[dict]:
    out: list[dict] = []
    for root in CRATE_ROOTS:
        if not root.exists():
            continue
        for path in sorted(root.rglob("*.rs")):
            p = str(path).replace("\\", "/")
            if any(s in p for s in SKIP_PATH_PARTS):
                continue
            out.extend(scan_file(path, table_pattern, methods, free))
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--list", action="store_true", help="print findings")
    ap.add_argument("--kind", default="", help="comma-separated kinds to print with --list")
    ap.add_argument("--update-baseline", action="store_true", help="rewrite the baseline file")
    ap.add_argument(
        "--print-forced-tables",
        action="store_true",
        help="print the FORCE-RLS tables, one per line (used by scripts/rls-e2e-gate.sh)",
    )
    args = ap.parse_args()

    if args.print_forced_tables:
        print("\n".join(forced_tables()))
        return 0

    tables = forced_tables()
    TABLE_GUCS.update(table_gucs())
    missing_policy = sorted(t for t in tables if not TABLE_GUCS.get(t))
    if missing_policy:
        print(f"error: FORCE RLS without a policy reading a GUC: {', '.join(missing_policy)}", file=sys.stderr)
        return 2
    if not tables:
        print("error: no FORCE ROW LEVEL SECURITY tables found in migrations", file=sys.stderr)
        return 2
    table_pattern = table_re(tables)
    methods, free = model_method_map(table_pattern)
    GLOBAL_ALIASES.update(global_aliases())
    findings = collect(table_pattern, methods, free)

    counts = {k: 0 for k in KINDS}
    for f in findings:
        counts[f["kind"]] += 1

    print("RLS coverage audit (#832 / #1087)")
    print(f"  forced tables ({len(tables)}): {', '.join(tables)}")
    print()
    for kind in KINDS:
        marker = "  (ratcheted)" if kind in RATCHETED else ""
        print(f"  {kind:10} {counts[kind]}{marker}")
    print()

    if args.list:
        want = {k.strip().upper() for k in args.kind.split(",") if k.strip()} or set(KINDS)
        for f in sorted(findings, key=lambda x: (x["kind"], x["file"], x["line"])):
            if f["kind"] in want:
                short = f["file"].split("/src/", 1)[-1]
                why = f"  ({f['reason']})" if f["kind"] == "UNBOUND" and f["reason"] else ""
                print(f"  {f['kind']:10} {short}:{f['line']}  {f['fn']}()  [{f['via']}]  {','.join(f['tables'])}{why}")
        print()

    # Stale allowlist entries would silently cover whatever lands in that fn
    # next, so every entry must still match at least one finding.
    used = set()
    for f in findings:
        for allow_key in ELEVATED_ALLOWLIST:
            path_part, _, fn_part = allow_key.rpartition("::")
            if f["fn"] == fn_part and f["file"].endswith(path_part):
                used.add(allow_key)
    stale = sorted(set(ELEVATED_ALLOWLIST) - used)

    current = {k.lower(): counts[k] for k in RATCHETED}
    if args.update_baseline:
        BASELINE_PATH.write_text(json.dumps(current, indent=2) + "\n", encoding="utf-8")
        print(f"baseline written: {current}")
        return 0

    baseline = {k.lower(): 0 for k in RATCHETED}
    if BASELINE_PATH.exists():
        baseline.update(json.loads(BASELINE_PATH.read_text(encoding="utf-8")))

    failed = False
    if stale:
        failed = True
        print("FAIL: ELEVATED_ALLOWLIST entries that no longer match any statement (remove them):")
        for k in stale:
            print(f"  {k}")
        print()
    for k in RATCHETED:
        lk = k.lower()
        if current[lk] > baseline.get(lk, 0):
            failed = True
            print(f"FAIL: {current[lk]} {k} findings, baseline is {baseline.get(lk, 0)}:")
            for f in [f for f in findings if f["kind"] == k][:40]:
                short = f["file"].split("/src/", 1)[-1]
                print(f"  {short}:{f['line']}  {f['fn']}()  [{f['via']}]  {','.join(f['tables'])}")
            print()
        elif current[lk] < baseline.get(lk, 0):
            print(f"{k} dropped to {current[lk]} (baseline {baseline.get(lk, 0)}). Ratchet it: --update-baseline")

    if failed:
        print("UNCOVERED: route through `with_org_context(state.db.runtime_pool(), org_id, ..)`")
        print("           or a store method that uses `with_current_org`, or add an")
        print("           ELEVATED_ALLOWLIST entry with a reason if it is genuinely cross-org.")
        print("UNBOUND:   wrap the runtime-pool statement in a GUC-binding helper; as written it")
        print("           fail-closes under the NOBYPASSRLS role.")
        return 1

    print(f"OK: {current} (baseline {baseline}).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
