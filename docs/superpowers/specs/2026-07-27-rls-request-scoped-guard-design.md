# RLS tenant-isolation: request-scoped org guard (Option 2, #832/#960)

## Problem

The RLS policies (migration `20250101000082`) fail-close every covered-table
query (`projects`, `audit_logs`, `hosted_mocks`, `templates`, `scenarios`) to 0
rows unless the connection has `app.current_org_id` set. Activating the
`NOBYPASSRLS` runtime role therefore requires **every** query on those tables —
in handlers, the store, and the model layer — to run with the GUC bound to the
request's org.

The per-method `with_org_context(pool, org_id, ...)` approach cannot achieve
this: the store (`PgRegistryStore`) owns a shared pool and delegates ~20
covered-table queries to model methods (`HostedMock::find_by_id(&self.pool, id)`
etc.). Many store methods are id-based and carry no `org_id`, so they cannot set
a per-request GUC. A pre-cutover audit confirmed activation as-is would fail
`find_hosted_mock_by_id`, `list_hosted_mocks_by_org`, `list_templates_by_org`,
`get_scenario_reviews`, and more → full outage of those surfaces.

## Approach: bind the org ONCE per request, cover everything uniformly

Instead of threading `org_id` through dozens of method signatures, carry it in a
task-local set by middleware, and have every covered-table query run inside a
transaction that sets the GUC from that task-local.

### 1. Task-local org context

```rust
tokio::task_local! { pub static CURRENT_ORG: uuid::Uuid; }
```

### 2. Request middleware (`rls_org_scope`)

Runs AFTER `auth_middleware` on the authenticated router. Extracts `user_id`
from request extensions, resolves the org (`resolve_org_context`), and scopes
the entire downstream in the task-local:

```rust
match resolve_org(...).await {
    Ok(org_id) => CURRENT_ORG.scope(org_id, next.run(req)).await,
    Err(_)     => next.run(req).await, // no org in scope (e.g. list-my-orgs)
}
```

### 3. `with_current_org` helper (store/org_context.rs)

Reads the task-local; when present runs `f` in a tx with the tx-local GUC set
(reuses existing `with_org_context`); when absent runs `f` on the pool directly.
Tx-local GUC (`set_config(..., true)`) is required by Neon's transaction pooler.

### 4. Executor-generic model methods (the mechanical bulk)

Convert the covered-table model methods in `hosted_mock.rs`, `project.rs`,
`scenario.rs`, `template.rs`, `audit_log.rs` from `pool: &sqlx::PgPool` to
`executor: impl sqlx::PgExecutor<'_>` (or `&mut PgConnection`). Backwards
compatible: `&PgPool` already implements `PgExecutor`, so existing handler
callers are unaffected; the store can now pass `&mut **tx`.

**Caveat:** methods that issue MORE THAN ONE query (e.g. `create` with a
`RETURNING` after an insert, or select-then-update) cannot take a
consumed-once `impl Executor`. Take `&mut PgConnection` and reborrow, or keep an
internal `tx`. Audit each covered-table model method for query count.

### 5. Store wrapping

Each `PgRegistryStore` method that queries a covered table (direct or via a
model) wraps in `with_current_org(&self.pool, |tx| Box::pin(async move { ... }))`,
passing `&mut **tx` to the (now executor-generic) model method. Cross-org store
methods (`get_admin_analytics_snapshot`) stay on `self.owner_pool`.

### 6. The gate: e2e suite as `NOBYPASSRLS`

CI/local: run the registry server with `APP_DATABASE_URL` pointed at a
`NOBYPASSRLS` role (migration applied) and run the existing `*_e2e.rs` suite
against it. Any query that forgets the GUC fail-closes → the test fails. This
makes coverage **proven and self-enforcing**, and is what would have caught the
store→model gap automatically. Build against this failing gate.

## Rollout (unchanged, reversible)

Deploy (migration inert on owner role) → provision `mockforge_app` on Neon prod
→ set `APP_DATABASE_URL` (activate) → smoke test → rollback = unset
`APP_DATABASE_URL` (instant, back to owner/BYPASSRLS). **Only after the e2e gate
passes.**

## Scope / status

This is a multi-file, auth-critical refactor:
- ~30-40 model methods → executor-generic (watch multi-query methods).
- ~20 store methods → wrap in `with_current_org`.
- 1 middleware + task-local + helper + router wiring.
- e2e-as-NOBYPASSRLS harness.

Order: (a) task-local + helper + middleware [core, bounded]; (b) e2e gate
scaffolding [failing]; (c) models executor-generic; (d) store wrapping until the
gate passes; (e) rollout. Steps (c)/(d) are the bulk and must be driven by (b).

---

# Findings from building it (2026-07-30)

Three things the design above got wrong or did not anticipate. All were found by
actually running the gate (`scripts/rls-e2e-gate.sh`), which is the argument for
building the gate first.

## 1. The reverted GUC is `''`, not unset — activation would have been an outage

The migration header claimed "Unset GUC → NULL → 0 rows → fail closed". That is
only true on a connection that has *never* bound the GUC.
`set_config(..., is_local => true)` reverts at COMMIT to the **empty string**,
not to unset:

```
fresh conn:      current_setting('app.current_org_id', true)  -> NULL
after a bound tx: current_setting('app.current_org_id', true)  -> ''
                  ''::uuid                                     -> ERROR 22P02
```

So with the original policies, the first org-scoped request permanently poisons
that physical connection: every later covered-table query on it that runs
without an org bound raises `invalid input syntax for type uuid: ""` → HTTP 500,
instead of returning zero rows. Under a pooler this spreads across the pool
within seconds of activation. It reproduced as marketplace search 500s in the
gate.

Fix: every policy now uses `nullif(current_setting('app.current_org_id', true), '')::uuid`,
which collapses both states to NULL and restores the intended fail-closed
behavior. Migration `20250101000082` is unreleased (PR #960 is a draft), so it
was corrected in place.

**This is the single most important reason not to have activated `APP_DATABASE_URL`
on the strength of a code review.**

## 2. Audit writes must stay on the owner pool

`PgRegistryStore::record_audit_event` wrote through the request-path pool.
`record_audit_event` is fire-and-forget (it only WARNs), so under RLS any event
whose org differs from the request's bound org is **silently dropped** — e.g.
`POST /api/v1/organizations`, where the GUC is still bound to the actor's
existing org while the audit row belongs to the org being created. The gate
surfaced this as `new row violates row-level security policy for table "audit_logs"`.

The row's `org_id` is an explicit argument from an already-authorized handler,
so the `WITH CHECK` adds no authorization — only data loss. Writes moved to the
owner pool; reads (`list_audit_logs`, `count_audit_logs`) stay RLS-enforced,
which is where cross-tenant exposure actually matters.

## 3. The gate has a blind spot: owner-pool queries

The design claims the gate makes coverage "proven and self-enforcing". It makes
**fail-closure** self-enforcing. It cannot see a covered-table query that runs
on the **owner** pool, because such a query neither breaks nor is protected —
the gate stays green while the query sits outside the backstop entirely.

That matters here because most handler code reaches for `state.db.pool()` (the
owner pool) rather than the store: at the time of writing, ~330 handler call
sites use `state.db.pool()` versus ~200 that go through `state.store`. The
backstop only covers the store path plus the handful of handlers explicitly
routed through `with_org_context(state.db.runtime_pool(), ..)`.

So a green gate means "nothing fail-closes", NOT "everything is covered".

`scripts/check_rls_coverage.py` is the other half: it classifies every
covered-table statement as COVERED / UNCOVERED / ELEVATED and fails when the
UNCOVERED count rises above a checked-in baseline, with an allowlist that
requires a stated reason for each genuinely cross-org site (public data-plane
routing, internal shared-token APIs, platform-admin aggregates).

## What is still NOT done

- The `templates` / `scenarios` policies allow `org_id IS NULL` public rows, so
  public marketplace reads work unbound. Verify this holds for every public
  browse path before activation.
- `workspaces` and `runtime_captures` are still uncovered by any policy (noted
  in the migration header as follow-ups); they need join-based policies.
- Prod rollout steps 3-5 (provision `mockforge_app` on Neon, set
  `APP_DATABASE_URL`, smoke) remain untouched. Do not activate until the gate is
  green in CI over several runs and the coverage baseline is 0.
