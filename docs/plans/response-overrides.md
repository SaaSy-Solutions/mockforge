# Response overrides plan

Response overrides let a user patch generated mock responses with rules (target an operation, tag, or path; apply JSON-pointer add, replace, or remove; optionally only when a condition holds). The engine exists in `mockforge-core`, but `mockforge serve` never loads it, nothing can change rules at runtime, and hosted mocks have no way to receive them. This program makes overrides a managed feature: live rules in the runtime, per-deployment rules in the registry, and an Overrides page in the admin UI for both self-hosted and cloud users.

The rule it enforces: one ruleset per running mock, replaced whole (PUT is idempotent), validated before it is stored or applied.

PRs in order: A (runtime), B (registry), C (UI), then the rollout.

## How to read this

One box is one unit of work, checked only when its evidence exists. Execution runs inline in this session. The pstack ten-lane live swarm per PR is replaced by the repo's own gates (`/verify` checklist, per-crate clippy and tests) plus a real-binary run of `mockforge serve` for A and a prod check after rollout. That substitution is deliberate. This repo's CI and the self-verification rule are the review bar here.

## Make overrides live in the runtime (PR A)

**Depends on.** None.

**Files.**

- [ ] Edit `crates/mockforge-core/src/overrides/models.rs`, `overrides.rs`, `overrides/loader.rs`, `overrides/matcher.rs`.
- [ ] Edit `crates/mockforge-openapi/src/response_rewriter.rs`, `openapi_routes.rs`.
- [ ] Edit `crates/mockforge-core/src/openapi_rewriter.rs`.
- [ ] Edit `crates/mockforge-http/src/lib.rs`, `crates/mockforge-cli/src/serve.rs`, `crates/mockforge-ui/src/lib.rs`.
- [ ] Delete `crates/mockforge-http/src/overrides.rs` (not compiled, a stale duplicate).

**Build.**

- [ ] `mockforge.yaml` gains a top-level `overrides:` rule list (the book already documented one that nothing read).
- [ ] `OverrideRule` gains `name: Option<String>` and `enabled: bool` (default true). Serde defaults keep existing YAML valid.
- [ ] `Overrides::new(rules)` is the only constructor. It validates targets and pointers and compiles regexes, returning `OverridesError`. The loader goes through it.
- [ ] `SharedOverrides` (an `Arc<RwLock<Overrides>>`) with `rules()`, `replace()`, and the apply path.
- [ ] `mockforge-openapi` gets an `OverrideHook` trait and an `OverrideRequest` view (method, path, template, headers, query, body, operation id, real OpenAPI tags). `RouterContext.overrides` replaces the `overrides_enabled` flag. `ResponseRewriter` keeps only token expansion.
- [ ] `OpenApiRouteRegistry::with_overrides(hook)` carries the hook into every `build_router_*` variant.
- [ ] `serve` builds one `SharedOverrides` from `MOCKFORGE_HTTP_OVERRIDES_GLOB` (files) and `MOCKFORGE_HTTP_OVERRIDES` (inline JSON array) and passes it to the HTTP router and the admin server.
- [ ] Admin server mounts `GET` and `PUT /__mockforge/overrides`. PUT validates and swaps.

**You see.**

- [ ] `mockforge serve --spec examples/openapi-demo.json --admin` then `PUT :9080/__mockforge/overrides` with a rule makes the next `GET :3000/...` response carry the patch, with no restart.

**Verify, unit.**

- [ ] Core tests for validation errors, disabled rules, real tag targeting, and `when` seeing request headers. Run `cargo test -p mockforge-core overrides`.
- [ ] Router test that swapping rules changes the next response. Run `cargo test -p mockforge-http`.

**Verify, live.**

- [ ] Real binary run as in **You see**, including a rejected invalid rule (400) that leaves the old rules in place.

## Store and push per-deployment rules (PR B)

**Depends on.** A.

**Build.**

- [ ] Store the rules under `hosted_mocks.metadata_json.override_rules`, written with `jsonb_set` like `flyio_machine_id` and `custom_domain`. No migration, no SQLite mirror, no `FromRow` change; the typed parse happens in the registry handler.
- [ ] `GET` and `PUT /api/v1/hosted-mocks/{id}/overrides`, guarded by `Permission::HostedMockUpdate` and the org match. PUT validates with `Overrides::new`, stores, then pushes to `http://{app}.internal:9080/__mockforge/overrides` (3s timeout). The response reports `runtime` as `applied`, `unreachable`, or `outdated` (404 from a runtime older than A).
- [ ] Every machine env builder sets `MOCKFORGE_HTTP_OVERRIDES` so rules survive redeploys and restarts.

**Verify.**

- [ ] Registry E2E case for PUT then GET round-trip, validation 400, and cross-org 404.

## Add the Overrides page (PR C)

**Depends on.** B for cloud, A for self-hosted.

**Build.**

- [ ] `OverridesPage` at `/overrides`, in the nav and the cloud allowlist, with search keywords.
- [ ] Cloud mode picks a hosted mock and uses the registry API. Self-hosted mode uses the admin server.
- [ ] Rule list with enable toggles, and an editor for name, targets, mode, condition, post-templating, and patch ops. Save sends the whole list.

**Verify.**

- [ ] Vitest for the editor round-trip and validation display. Browser run against a local `mockforge serve --admin`.

## Roll out

- [ ] Release the crates and the `mockforge` image so hosted mocks can run A.
- [ ] Deploy the registry (B) and the UI (C) to Ashburn per the hosting runbook.

## Appendix B. Alternatives rejected

Mounting the API on the public mock port like route chaos. Rejected because `/__mockforge` bypasses auth there (#1085). A dedicated `override_rules` column. Rejected because `metadata_json` is already the keyed per-deployment bag and a column would need Postgres and SQLite migrations plus a `FromRow` field for no extra guarantee. Per-rule CRUD with ids. Rejected because whole-list PUT is idempotent and the UI edits the list anyway. `arc-swap` for the handle. Rejected to match the repo's `std::sync::RwLock` convention.

## Appendix C. Risks

Hosted mocks only pick up A on redeploy, so B must treat 404 as `outdated`, not an error. The loader expanded `{{…}}` tokens once at load time; A keeps that for file rules so existing YAML behaves the same.
