# ADR 0001: Draining the `mockforge-http` umbrella crate

- **Status**: Proposed
- **Date**: 2026-05-18
- **Issue**: [#555](https://github.com/SaaSy-Solutions/mockforge/issues/555)
- **TODO Source**: `crates/mockforge-http/src/lib.rs:1`

## Context

`mockforge-http` is the largest crate in the workspace at roughly **35,000 lines** spread across 74 source files. The crate's name implies a focused HTTP serving library, but in practice it carries four loosely coupled responsibilities:

1. **HTTP serving** – router construction, middleware, OpenAPI integration, request/response handling, TLS, SSE, health probes.
2. **Reality/proxy switching** – `reality_proxy.rs`, `proxy_server.rs`, and `management/proxy.rs` wire upstream forwarding and the reality-continuum slider.
3. **Intelligence handlers** – ~16 handlers under `handlers/` that drive AI-powered features: semantic drift, threat modeling, AI Studio, forecasting, behavioral cloning, contract health, fidelity, etc.
4. **Domain-specific admin APIs** – consumer contracts, OAuth2 server, snapshot diff, scenario studio, token lifecycle, world state, x-ray, etc.

The TODO at `lib.rs:1` referenced an earlier intent to spin out `mockforge-intelligence` and `mockforge-proxy` as separate crates. **Both crates already exist** and `mockforge-http` already consumes them, but only a small fraction of the AI-related code has actually moved. The intelligence crate's own docstring documents the blocker:

> Full migration of `intelligent_behavior`, `ai_contract_diff`, `ai_studio`, and `behavioral_economics` is blocked by circular dependencies with non-deprecated core code (openapi, reality, priority_handler, etc.) that require a foundation-types crate to untangle.

This ADR audits every module in `mockforge-http/src/` and assigns it to one of three buckets so the next refactor session can execute mechanically.

## Decision

Each module gets a target: **STAY** (core HTTP serving), **INTELLIGENCE** (move to `mockforge-intelligence` once core deps untangle), or **PROXY** (move to `mockforge-proxy`). A fourth bucket — **SPLIT** — covers crates that should be re-homed elsewhere entirely (e.g. `mockforge-auth`, `mockforge-admin-api`) and are out of scope for the issue but flagged here.

### Top-level modules (`crates/mockforge-http/src/*.rs`)

| Module | LOC | Bucket | Notes |
|---|---:|---|---|
| `lib.rs` | 3490 | STAY | Router orchestrator. Trim once handlers leave. |
| `builder.rs` | 344 | STAY | Router builder primitives. |
| `chain_handlers.rs` | 372 | STAY | Multi-step request chains. |
| `counting_listener.rs` | 347 | STAY | TCP-accept counter. |
| `coverage.rs` | 730 | STAY | Route coverage report. |
| `database.rs` | 197 | STAY | sqlx pool shared by handlers. Re-export once handlers move. |
| `file_generator.rs` | 348 | STAY | PDF/CSV/JSON mock file generation. |
| `file_server.rs` | 299 | STAY | Static-file serving. |
| `fixtures_api.rs` | 428 | STAY | Hosted-mock fixtures REST API. |
| `health.rs` | 763 | STAY | K8s liveness/readiness probes. |
| `http_tracing_middleware.rs` | 170 | STAY | OTEL HTTP spans. |
| `latency_profiles.rs` | 243 | STAY | Latency profile config. |
| `management_ws.rs` | 382 | STAY | WS management API. |
| `metrics_middleware.rs` | 503 | STAY | Prometheus middleware. |
| `network_profile_runtime.rs` | 199 | STAY | Network profile runtime switching. |
| `op_middleware.rs` | 463 | STAY | OpenAPI operation metadata. |
| `overrides.rs` | 646 | STAY | Per-route response overrides. |
| `protocol_server.rs` | 88 | STAY | Protocol-server lifecycle trait. |
| `quick_mock.rs` | 789 | STAY | Quick-mock generation. |
| `replay_listing.rs` | 231 | STAY | Fixture replay listing. |
| `request_logging.rs` | 291 | STAY | Request logger. |
| `route_chaos_runtime.rs` | 327 | STAY | Per-route chaos runtime. |
| `scenarios_runtime.rs` | 206 | STAY | Scenario activation runtime. |
| `schema_diff.rs` | 690 | STAY | JSON Schema diffing helper. |
| `spec_import.rs` | 551 | STAY | OpenAPI/AsyncAPI spec import. |
| `sse.rs` | 651 | STAY | Server-Sent Events stream. |
| `state_machine_api.rs` | 985 | STAY | Scenario state-machine API. |
| `time_travel_api.rs` | 287 | STAY | Time-travel runtime mirror. |
| `tls.rs` | 508 | STAY | TLS configuration. |
| `token_response.rs` | 400 | STAY | Token-response utilities. |
| `ui_builder.rs` | 2247 | STAY | Low-code UI builder API. Candidate for own crate (out of scope). |
| `auth.rs` | 437 | SPLIT (`mockforge-auth`) | Currently re-exports `auth/` subtree. Out of scope. |
| `verification.rs` | 544 | STAY | Request-verification API. |
| `mockai_api.rs` | 217 | INTELLIGENCE | Standalone MockAI HTTP API. Only touches `mockforge-data::ai`. |
| `ai_handler.rs` | 274 | INTELLIGENCE | `process_response_with_ai` glue. Depends on `mockforge_core::{ai_response, openapi::response::AiGenerator}` and `mockforge_data::rag`. **Blocked** until those move. |
| `rag_ai_generator.rs` | 565 | INTELLIGENCE | RAG-backed AI generator. Same `mockforge-data::rag` dep. |
| `reality_proxy.rs` | 282 | PROXY | Reality-continuum middleware (#222). Depends on `mockforge_core::consistency::UnifiedState`. Move with consistency. |
| `proxy_server.rs` | 416 | PROXY | Browser/mobile proxy server. Already uses `mockforge_proxy::body_transform`. **Clean move.** |
| `contract_diff_api.rs` | 299 | INTELLIGENCE | Contract-diff API surface. |
| `contract_diff_middleware.rs` | 160 | INTELLIGENCE | Contract-diff capture middleware. |

### Subdirectories

| Subdir | Bucket | Notes |
|---|---|---|
| `src/auth/` (11 files) | SPLIT (`mockforge-auth`) | OAuth2/OIDC/JWKS/risk-engine. Distinct concern; deserves its own crate but **out of scope** for issue #555. |
| `src/management/` (8 files) | STAY (mostly) | `migration.rs` and `proxy.rs` already re-export from `mockforge-proxy` and can move alongside `reality_proxy.rs`. `ai_gen.rs` → INTELLIGENCE. The rest is core admin surface. |
| `src/middleware/` (12 files) | MIXED | `behavioral_cloning.rs` → INTELLIGENCE (already imports `mockforge_intelligence`). `deceptive_canary.rs`, `ab_testing.rs`, `drift_tracking.rs` → INTELLIGENCE (drift/canary/AB are behavioral features). Others STAY (rate_limit, security, keepalive, response_buffer, production_headers, conn_diagnostics). |
| `src/services/` (1 module) | INTELLIGENCE | `forecasting_service.rs` is an intelligence helper. |
| `src/consistency/` (4 files) | PROXY | Cross-protocol consistency adapter. Tightly bound to reality-proxy. |
| `src/handlers/` (37 files) | See below | The bulk of the work. |

### `src/handlers/` cleavage plane

Of the 37 handler files, **17 are intelligence-bound** and **20 stay** with the core HTTP surface (admin/contract APIs that the UI needs but aren't AI-driven).

| Handler | LOC | Bucket | Core dep that blocks the move |
|---|---:|---|---|
| `ai_studio.rs` | 586 | INTELLIGENCE | `mockforge_core::ai_studio::{api_critique, system_generator}`, `mockforge_core::intelligent_behavior::types` |
| `semantic_drift.rs` | 428 | INTELLIGENCE | `mockforge_core::ai_contract_diff::*`, `mockforge_core::incidents::semantic_manager` |
| `threat_modeling.rs` | 648 | INTELLIGENCE | `mockforge_core::contract_drift::threat_modeling`, `mockforge_core::incidents::integrations` |
| `forecasting.rs` | 608 | INTELLIGENCE | `mockforge_contracts::contract_drift::forecasting`, `mockforge_core::incidents::types` |
| `behavioral_cloning.rs` | 640 | INTELLIGENCE | Already on `mockforge_intelligence::behavioral_cloning`. **Unblocked** — can move now. |
| `consistency.rs` | 780 | INTELLIGENCE | `mockforge_core::consistency::*` |
| `drift_budget.rs` | 784 | INTELLIGENCE | Drift budget engine (likely core). |
| `fidelity.rs` | 171 | INTELLIGENCE | Fidelity score (AI metric). |
| `contract_health.rs` | 373 | INTELLIGENCE | Contract health (drift-related). |
| `risk_assessment.rs` | 527 | INTELLIGENCE | `mockforge_core::security::risk` (overlaps with auth — verify before moving). |
| `risk_simulation.rs` | 168 | INTELLIGENCE | Pairs with `risk_assessment`. |
| `pr_generation.rs` | 146 | INTELLIGENCE | `mockforge_core::pr_generation` |
| `scenario_studio.rs` | 306 | INTELLIGENCE | LLM-backed scenario synthesis. |
| `incident_replay.rs` | 156 | INTELLIGENCE | Pairs with `semantic_drift`. |
| `xray.rs` | 281 | INTELLIGENCE | AI explainability surface. |
| `snapshot_diff.rs` | 490 | INTELLIGENCE | Snapshot-diff analyzer (semantic). |
| `compliance_dashboard.rs` | 419 | INTELLIGENCE | Compliance scoring (AI metric). |
| `failure_designer.rs` | 174 | INTELLIGENCE | Failure rule generator. |
| `access_review.rs` | 469 | STAY | Vanilla access-review CRUD. |
| `change_management.rs` | 512 | STAY | Vanilla change-management CRUD. |
| `consent.rs` | 474 | STAY | Consent ledger. |
| `consumer_contracts.rs` | 311 | STAY | Pact-style consumer contracts. |
| `conformance.rs` | 332 | STAY | k6/bench conformance API. Already gated by `conformance` feature. |
| `oauth2_server.rs` | 418 | SPLIT (`mockforge-auth`) | Belongs with `src/auth/`. |
| `performance.rs` | 285 | STAY | Performance test results. |
| `pipelines.rs` | 368 | STAY | Pipeline activation. |
| `privileged_access.rs` | 298 | STAY | Privileged-access ledger. |
| `protocol_contracts.rs` | 894 | STAY | Cross-protocol contract validation. |
| `snapshots.rs` | 296 | STAY | Snapshot CRUD (no AI). |
| `token_lifecycle.rs` | 295 | STAY | Token lifecycle CRUD. |
| `webhook_test.rs` | 188 | STAY | Webhook test runner. |
| `world_state.rs` | 343 | STAY | World-state graph API. |
| `ab_testing.rs` | 293 | STAY | A/B routing config (not the AI behavior cloning kind). |
| `auth_helpers.rs` | 45 | STAY | Auth claim extractor. |
| `deceptive_canary.rs` | 69 | INTELLIGENCE | Tiny — pairs with `middleware/deceptive_canary.rs`. |
| `mod.rs` | 86 | STAY | Module manifest. |

### Summary of bucket sizes (LOC)

- **STAY**: ~20,500 LOC across 50 files (core HTTP serving + admin/contract APIs)
- **INTELLIGENCE**: ~8,400 LOC across 17 handlers + 5 top-level modules + 3 middleware = ~25 files
- **PROXY**: ~1,500 LOC (`reality_proxy.rs` + `proxy_server.rs` + `consistency/` + `management/{proxy,migration}.rs`)
- **SPLIT (out of scope)**: ~1,400 LOC (`src/auth/` + `oauth2_server.rs`) → future `mockforge-auth` crate

Post-extraction `mockforge-http` would shrink from ~35K to ~20.5K LOC, hitting the issue's "shrink ~50%" acceptance criterion when the auth split is also done in a follow-up.

## The blocker: `mockforge-core` is the real umbrella

The reason Phase 2 cannot be executed mechanically today is that **the AI handlers depend on AI logic that still lives in `mockforge-core`**:

- `mockforge_core::ai_contract_diff::{ContractDiffAnalyzer, ConfidenceScorer, CorrectionProposer, SemanticAnalyzer, RecommendationEngine}` (7 files)
- `mockforge_core::ai_studio::{api_critique, system_generator}`
- `mockforge_core::contract_drift::threat_modeling::ThreatAnalyzer`
- `mockforge_core::intelligent_behavior::*` (full subtree)
- `mockforge_core::incidents::{semantic_manager, integrations, types}`
- `mockforge_core::behavioral_economics::*`
- `mockforge_core::pr_generation::*`
- `mockforge_core::reality_continuum::*`
- `mockforge_core::consistency::*`

Moving the *handlers* without moving these *implementations* would just turn the handler crate into a wrapper that still pulls all the AI weight via `mockforge-core`. The existing `mockforge-intelligence/src/lib.rs` comment confirms the previous extraction attempt hit exactly this wall.

## Recommended sequencing (revised from issue #555)

The original issue proposed handler-first extraction. Audit findings reverse the order:

1. **Phase A (prerequisite, separate issue)**: Move `mockforge_core::{ai_contract_diff, ai_studio, intelligent_behavior, behavioral_economics, contract_drift::threat_modeling, pr_generation, reality_continuum, consistency}` into `mockforge-intelligence`. This requires a `mockforge-foundation-types` crate to break the circular deps (already noted in `mockforge-intelligence/src/lib.rs`). **This is the actual hard problem.**
2. **Phase B (mechanical, this issue)**: Move the 17 intelligence handlers out of `mockforge-http/src/handlers/` into `mockforge-intelligence::handlers` once their core-side deps are available. Re-export from `mockforge-http` for one minor version. `behavioral_cloning.rs` can move today as a proof-of-concept (its core dep is already in `mockforge-intelligence`).
3. **Phase C (mechanical)**: Move `reality_proxy.rs`, `proxy_server.rs`, `consistency/`, and `management/{proxy,migration}.rs` into `mockforge-proxy`. Smaller blast radius and the proxy crate is already structurally ready.
4. **Phase D (cleanup)**: Drop re-exports, fix CLI/SDK/UI/test callers, delete the TODO at `lib.rs:1`. Only one downstream call site (`mockforge-ui/src/routes.rs`) references a handler submodule directly (`mockforge_http::handlers::conformance`); the others all consume top-level re-exports, so churn is small.
5. **Phase E (follow-up issue)**: Split `src/auth/` + `oauth2_server.rs` into `mockforge-auth`. Tracked separately because it's not what #555 asked for.

## Status of this issue

This ADR is **Phase 1**. The implementer judged Phase 2 too risky to bundle: the AI handlers cannot be moved cleanly without first shifting their core-side implementations, which is a multi-day refactor that needs its own issue, plan, and review. A follow-up issue will track Phase A (the core-side untangle). `behavioral_cloning.rs` is the only handler whose dependencies already live in `mockforge-intelligence`; it is a candidate for a small standalone "proof of concept" PR.

## Consequences

**Positive:**
- Clear inventory: every module has a target and a justification.
- The real blocker (`mockforge-core` AI submodules) is named explicitly so the next attempt doesn't repeat the previous one's discovery.
- Follow-up phases are sized so each can ship in a single PR.

**Negative:**
- The TODO at `crates/mockforge-http/src/lib.rs:1` survives this PR. It will be removed in Phase D.
- `mockforge-http` remains the largest crate in the workspace for now.

**Neutral:**
- The existence of `mockforge-intelligence` and `mockforge-proxy` as partially-populated crates is intentional and called out in the intelligence crate's own docstring; this ADR formalises that they are mid-migration, not abandoned.
