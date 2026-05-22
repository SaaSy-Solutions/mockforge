# Consolidate remaining `contract_drift` duplications between mockforge-core and mockforge-contracts

- **Status**: Approved
- **Date**: 2026-05-21
- **Issue**: [#602](https://github.com/SaaSy-Solutions/mockforge/issues/602)
- **Depends on**: [#603](https://github.com/SaaSy-Solutions/mockforge/pull/603) (deletes the only contracts-internal caller of the modules this PR removes; wait for it to merge before basing this work)
- **Follow-up issues filed**: [#604](https://github.com/SaaSy-Solutions/mockforge/issues/604) (audit core-only `contract_drift` modules), [#605](https://github.com/SaaSy-Solutions/mockforge/issues/605) (consider promoting ConsumerMappingRegistry/Analyzer to foundation)

## Context

The WS1 architectural overhaul (commit `cc4d1eba`) extracted `mockforge-contracts` from `mockforge-core`. The intent was for `mockforge-contracts` to host the contract-drift modules that could be cleanly separated from core's heavier dependencies (`OpenApiSpec`, `ai_contract_diff`, LLM). Two modules made it across in that overhaul but were never fully decoupled from their core originals:

### `consumer_mapping.rs` — near-100% duplicate

| File | LOC | What's there |
|---|---:|---|
| `mockforge-core/src/contract_drift/consumer_mapping.rs` | 471 | Type re-exports from `mockforge-foundation::contract_drift_types` + `ConsumerMappingRegistry` + `ConsumerImpactAnalyzer` (full logic) |
| `mockforge-contracts/src/contract_drift/consumer_mapping.rs` | 470 | Same type re-exports + same logic, plus 2 tiny diffs (docstring wording + a one-line stylistic refactor) |

Both files re-export the shared types (`AppType`, `ConsumerImpact`, `ConsumerMapping`, `ConsumingApp`, `SDKMethod`) from `mockforge-foundation::contract_drift_types`. Both implement `ConsumerMappingRegistry` and `ConsumerImpactAnalyzer` independently. The diff between the two is:
- One docstring paragraph (cosmetic).
- One stylistic refactor in the `update_mapping` method (`let app_id = app.app_id.clone(); ... existing_app_ids.insert(app_id);` in contracts vs. `existing.consuming_apps.push(app); existing_app_ids.insert(existing.consuming_apps.last().unwrap().app_id.clone());` in core). Behaviorally equivalent; contracts' shape is cleaner.

### `fitness.rs` — type-definition duplication

| File | LOC | What's there |
|---|---:|---|
| `mockforge-core/src/contract_drift/fitness.rs` | 1493 | `FitnessFunction` + `FitnessScope` + `FitnessFunctionType` (locally defined as Rust structs/enums), plus `FitnessTestResult` re-export from foundation, plus `FitnessFunctionRegistry` evaluator logic (depends on `OpenApiSpec` and `ai_contract_diff::Mismatch`) |
| `mockforge-contracts/src/contract_drift/fitness.rs` | 93 | The same 3 type definitions (locally defined again — **not** shared), plus the `FitnessTestResult` re-export |

The 3 type definitions (`FitnessFunction`, `FitnessScope`, `FitnessFunctionType`) are pure POD with serde derives; they have no dependency reason to be defined twice. Only `FitnessTestResult` is correctly hosted in foundation. The evaluator (`FitnessFunctionRegistry`) is core-only because of its `OpenApiSpec` + `ai_contract_diff` deps.

### Callers (verified pre-spec)

- **External callers** of `mockforge_core::contract_drift::{consumer_mapping, fitness}`: zero.
- **External callers** of `mockforge_contracts::contract_drift::{consumer_mapping, fitness}`: zero.
- **Internal core callers**: `mockforge-core::contract_drift::budget_engine` (uses `ConsumerImpactAnalyzer` and `FitnessFunctionRegistry`, both core-only logic types).
- **Internal contracts callers**: `mockforge-contracts::incidents::manager.rs` (uses `consumer_mapping::ConsumerImpact` and `fitness::FitnessTestResult` types). **This file is deleted by #603.** After #603 lands, the contracts copies have zero callers anywhere.

## Decision

After #603 lands, **delete the two contracts files entirely** as the dead duplicates they are. Take the opportunity to **promote the three `FitnessFunction*` type definitions to `mockforge-foundation::contract_drift_types`** so they sit alongside the other cross-crate-shared contract-drift types (`FitnessTestResult`, `AppType`, etc.) and follow the established re-export-from-foundation pattern.

The logic types (`ConsumerMappingRegistry`, `ConsumerImpactAnalyzer`, `FitnessFunctionRegistry`) stay in `mockforge-core`. They have no cross-crate consumers today, and moving them adds foundation surface area for no caller benefit. If that changes, that's [#605](https://github.com/SaaSy-Solutions/mockforge/issues/605)'s job.

The 8 core-only `contract_drift` modules (`breaking_change_detector`, `budget_engine`, `field_tracking`, `grpc_contract`, `mqtt_kafka_contracts`, `protocol_contracts`, `types`, `websocket_contract`) stay in core. The architectural rationale ("depends on `OpenApiSpec` / `ai_contract_diff::Mismatch`") is already documented in `mockforge-contracts/src/contract_drift/mod.rs`'s NOTE block; this PR preserves that NOTE. The future re-audit of whether any of them should move once foundation grows is [#604](https://github.com/SaaSy-Solutions/mockforge/issues/604)'s job.

## Target architecture

```
mockforge-foundation/src/
  └── contract_drift_types.rs
      ├── EXISTING: AppType, ConsumerImpact, ConsumerMapping, ConsumingApp,
      │   SDKMethod, FitnessTestResult, SemanticDriftResult, SemanticChangeType
      └── NEW: FitnessFunction, FitnessScope, FitnessFunctionType   ← promoted from core

mockforge-core/src/contract_drift/
  ├── consumer_mapping.rs       (unchanged — full Registry + Analyzer)
  ├── fitness.rs                (drops the 3 type defs; adds a `pub use` from
  │                              foundation; FitnessFunctionRegistry stays)
  └── (8 other modules unchanged)

mockforge-contracts/src/contract_drift/
  ├── consumer_mapping.rs       ← DELETED
  ├── fitness.rs                ← DELETED
  ├── forecasting/              (unchanged)
  └── mod.rs                    (drop `pub mod consumer_mapping;`, `pub mod fitness;`
                                 + corresponding `pub use` blocks + docstring bullets;
                                 keep the "NOTE: …remain in mockforge-core::contract_drift…"
                                 block intact)
```

**Dep direction**: unchanged. Nobody gains a new crate dependency.

**Net code change**: ~566 LOC removed (two contracts files). ~95 LOC moved (foundation gains type defs, core loses them). Core's `fitness.rs` gains a 1-line `pub use` block.

## Components

### 1. Promote 3 type definitions to `mockforge-foundation::contract_drift_types`

In `crates/mockforge-foundation/src/contract_drift_types.rs`, paste the existing definitions of `FitnessFunction`, `FitnessScope`, `FitnessFunctionType` from `crates/mockforge-core/src/contract_drift/fitness.rs:13-93`. Match the existing foundation file's derive style:

- `#[derive(Debug, Clone, Serialize, Deserialize)]` (plus `PartialEq` / `Eq` where the existing definition has them — `FitnessScope` and `FitnessFunctionType` use `PartialEq`/`Eq`)
- `#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]` if the existing foundation types in the same file have it (audit during execution; if foundation does not currently gate on `schema`, add the gate consistently with other types in the file)

If the existing core definition has additional attributes (`#[serde(tag = "...")]`, `#[serde(rename_all = "snake_case")]`), carry them verbatim.

### 2. Rewrite `crates/mockforge-core/src/contract_drift/fitness.rs`

- Delete lines 13–93 (the three type definitions). Pin the exact line range during execution by reading the file first.
- Add at the top of the file, alongside the existing `use serde::{...}` and `use std::collections::HashMap`:

  ```rust
  pub use mockforge_foundation::contract_drift_types::{
      FitnessFunction, FitnessFunctionType, FitnessScope,
  };
  ```

- Leave `FitnessFunctionRegistry` and the evaluator logic untouched. They reference `FitnessFunction`, `FitnessScope`, `FitnessFunctionType` by short name, which resolve to the foundation-sourced re-exports.
- The existing `pub use mockforge_foundation::contract_drift_types::FitnessTestResult;` (currently at the bottom) stays.

### 3. Delete `crates/mockforge-contracts/src/contract_drift/consumer_mapping.rs`

Pure dead code after #603. `git rm`.

### 4. Delete `crates/mockforge-contracts/src/contract_drift/fitness.rs`

Same reasoning. `git rm`.

### 5. Update `crates/mockforge-contracts/src/contract_drift/mod.rs`

Drop the lines that reference the deleted modules:

- The `pub mod consumer_mapping;` and `pub mod fitness;` declarations.
- The `pub use consumer_mapping::{...}` re-export block.
- The `pub use fitness::{...}` re-export block.
- The docstring bullets that list `consumer_mapping` and `fitness` under the "independently extractable" section.

**Keep:**
- The `pub mod forecasting;` declaration and its `pub use forecasting::{...}` block.
- The "NOTE: The following remain in `mockforge-core::contract_drift` due to dependencies…" block. That architectural decision is unchanged; [#604](https://github.com/SaaSy-Solutions/mockforge/issues/604) tracks a future re-audit.

### 6. Verify

No other files need touching. The audit during brainstorming confirmed zero callers (external or internal-to-contracts after #603) of the deleted modules.

## Testing strategy

Code-motion + type-promotion. No new tests.

**Compile-time gates**
- `cargo check --workspace --all-targets` — clean.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo fmt --all --check` — clean. After deleting the contracts files and updating mod.rs, the remaining `pub use forecasting::{...}` block may be shaped enough that rustfmt rewrites it; run `cargo fmt --all` proactively.

**Existing per-crate tests**
- `cargo test -p mockforge-foundation --lib` — exercises the new 3 types' serde derives (foundation's existing serde-derive tests cover the pattern).
- `cargo test -p mockforge-core --lib` — verifies that the evaluator logic in `fitness.rs` still works with `FitnessFunction` / `FitnessScope` / `FitnessFunctionType` sourced from foundation. The existing `#[cfg(test) mod tests` block inside `fitness.rs` is the safety net.
- `cargo test -p mockforge-contracts --lib` — verifies the post-deletion contracts crate still compiles and tests pass.
- `cargo test -p mockforge-http --lib` — defensive (http consumes contracts; should be unaffected).

**Manual surface checks**
- `git grep -nE 'pub struct FitnessFunction|pub enum FitnessScope|pub enum FitnessFunctionType' -- crates/` — should match exactly one location each, all in `crates/mockforge-foundation/src/contract_drift_types.rs`.
- `git grep -nE 'mockforge_contracts::contract_drift::(consumer_mapping|fitness)' -- crates/` — empty.
- `git grep -nE 'crate::contract_drift::(consumer_mapping|fitness)' -- crates/mockforge-contracts/` — empty.

**Deliberately not added**
- No new tests for the consolidation.
- No integration tests.

## Out of scope (with follow-up tickets)

- **Promoting `ConsumerMappingRegistry` / `ConsumerImpactAnalyzer` to foundation.** Tracked in [#605](https://github.com/SaaSy-Solutions/mockforge/issues/605). Deferred because there's no cross-crate consumer asking for it today; promoting now would grow foundation's surface area ~470 LOC for no caller benefit.

- **Re-evaluating the 8 core-only `contract_drift` modules** (`breaking_change_detector`, `budget_engine`, `field_tracking`, `grpc_contract`, `mqtt_kafka_contracts`, `protocol_contracts`, `types`, `websocket_contract`). Tracked in [#604](https://github.com/SaaSy-Solutions/mockforge/issues/604). They stay in core for this PR; the architectural rationale (deps on `OpenApiSpec` / `ai_contract_diff::Mismatch`) is recorded in `mockforge-contracts/src/contract_drift/mod.rs`'s NOTE block, which this PR preserves.

## Risks

- **Foundation derive consistency**: when promoting the 3 types, the `#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]` gating must match foundation's existing convention. If foundation doesn't currently use the `schema` feature for `contract_drift_types`, add it consistently (the existing types in that file are the pattern to follow). Audit during execution; if anything is unclear, pause and surface rather than guess.
- **Cycle introduction**: none expected. Foundation has no crate deps to invert. The promoted types are pure POD.
- **CI surprises**: `cargo doc --workspace` (if it runs in CI) may emit warnings for broken intra-doc links if the contracts file's docstrings linked to the deleted modules. Spot-check during execution.

## Approval

Scope: delete duplicates + promote fitness types to foundation. Base: wait for #603 to merge. Confirmed via brainstorming session 2026-05-21.
