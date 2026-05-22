# Migrate 7 of 8 core-only `contract_drift` modules to `mockforge-contracts`

- **Status**: Approved
- **Date**: 2026-05-21
- **Issue**: [#604](https://github.com/SaaSy-Solutions/mockforge/issues/604)
- **Depends on**: [#606](https://github.com/SaaSy-Solutions/mockforge/pull/606) (just deleted `contracts/contract_drift/{consumer_mapping,fitness}.rs` — the new modules slot into the now-cleaner `contract_drift` subdir)

## Context

The audit ticket #604 asked: which of the 8 "core-only" `contract_drift` modules (`breaking_change_detector`, `budget_engine`, `field_tracking`, `grpc_contract`, `mqtt_kafka_contracts`, `protocol_contracts`, `types`, `websocket_contract`) should stay in `mockforge-core` permanently, and which could move to `mockforge-contracts`?

The existing NOTE in `crates/mockforge-contracts/src/contract_drift/mod.rs` claims they all stay in core because they "depend on `OpenApiSpec` / `ai_contract_diff::Mismatch` types." The audit found this NOTE is **outdated**:

- **`ai_contract_diff::{Mismatch, ContractDiffResult, MismatchType, MismatchSeverity}` are already in `mockforge-foundation::contract_diff_types`** (Phase 6 / A5 of an earlier migration). `mockforge-intelligence::ai_contract_diff::types` is literally `pub use mockforge_foundation::contract_diff_types::*;`. Anyone (including contracts) can import these types directly from foundation without depending on intelligence.
- **`protocol_abstraction::Protocol` is a re-export of `mockforge_foundation::protocol::Protocol`** — also already foundation-hosted.

After tracing each module's actual dependencies through this lens:

| Module | LOC | Real external deps | Move feasibility |
|---|---:|---|---|
| `field_tracking.rs` | 205 | None (only chrono/serde/std) | **Trivial.** No import changes needed; pure relocation. |
| `types.rs` | 79 | `ContractDiffResult` (foundation) | **Trivial.** One import rewrite. |
| `breaking_change_detector.rs` | 187 | `Mismatch` (foundation) + sibling `types` | **Trivial.** One import rewrite. |
| `protocol_contracts.rs` | 215 | `Mismatch` + `ContractDiffResult` (both foundation) | **Trivial.** One import rewrite. |
| `grpc_contract.rs` | 1022 | Foundation types + `protocol_abstraction::Protocol` (foundation) + `prost_reflect` (external crate) | **Moveable** with a new external dep on contracts. |
| `mqtt_kafka_contracts.rs` | 1268 | Foundation types + Protocol + `jsonschema` (external crate) | **Moveable** with a new external dep on contracts. |
| `websocket_contract.rs` | 1039 | Foundation types + Protocol + `jsonschema` | **Moveable**. |
| `budget_engine.rs` | 518 | `ContractDiffResult` (foundation) + `ConsumerImpactAnalyzer` + `FitnessFunctionRegistry` + `FieldCountTracker` + `OpenApiSpec` (mockforge-openapi) | **Real blocker.** `OpenApiSpec` couples it to `mockforge-openapi`; moving would expand contracts' surface area. |

**7 of 8 modules can move; only `budget_engine` is genuinely core-only.**

This PR moves the 7 movable modules. It corrects the outdated NOTE in `contracts/contract_drift/mod.rs` and reduces the surface area of `mockforge-core::contract_drift` from 11 source files (post-#606) to 2 (`mod.rs` + `budget_engine.rs`).

## Decision

Move all 7 modules to `mockforge-contracts::contract_drift`. `budget_engine.rs` stays in `mockforge-core::contract_drift`.

Contracts gains 2 new external deps (`prost-reflect = "0.16"`, `jsonschema = "0.33.0"`) — both already used by `mockforge-core`, so no new versions enter the workspace.

`mockforge-core` gains a new dep on `mockforge-contracts` so `budget_engine` and `fitness` can import the moved types from their new home.

## Target architecture

```
mockforge-foundation (unchanged)
  ├── contract_diff_types       — Mismatch, ContractDiffResult, MismatchType, MismatchSeverity
  ├── contract_drift_types      — DriftBudget, DriftMetrics, FitnessFunction*, etc.
  └── protocol::Protocol        — the cross-crate Protocol enum

mockforge-contracts (deps: foundation + new: prost-reflect, jsonschema)
  └── src/contract_drift/
      ├── mod.rs                — declares + re-exports 7 new modules (plus existing forecasting/)
      ├── forecasting/          — unchanged (post-#606)
      ├── breaking_change_detector.rs   ← MOVED
      ├── field_tracking.rs             ← MOVED
      ├── grpc_contract.rs              ← MOVED
      ├── mqtt_kafka_contracts.rs       ← MOVED
      ├── protocol_contracts.rs         ← MOVED
      ├── types.rs                      ← MOVED
      └── websocket_contract.rs         ← MOVED

mockforge-core (deps: + mockforge-contracts)
  └── src/contract_drift/
      ├── mod.rs                — drops 7 declarations; updated docstring notes only budget_engine stays
      └── budget_engine.rs      ← STAYS (depends on mockforge-openapi::OpenApiSpec + in-core siblings)
```

**Dep direction**: `core → contracts → foundation`. No cycle.

**LOC migration**: ~4035 LOC of file content moves from `mockforge-core` to `mockforge-contracts`. Net workspace size unchanged.

## Components

### 1. Cargo.toml changes

**`crates/mockforge-contracts/Cargo.toml`** — add two external deps, matching the versions already used in core:

```toml
prost-reflect = "0.16"
jsonschema = "0.33.0"
```

**`crates/mockforge-core/Cargo.toml`** — add the contracts dep:

```toml
mockforge-contracts = { path = "../mockforge-contracts" }
```

(Cycle check: contracts depends only on foundation + the two new external deps + the existing serde/etc. set; no path back to core.)

### 2. Move 7 files via `git mv` (preserves history)

```
crates/mockforge-core/src/contract_drift/breaking_change_detector.rs  → crates/mockforge-contracts/src/contract_drift/breaking_change_detector.rs
crates/mockforge-core/src/contract_drift/field_tracking.rs            → crates/mockforge-contracts/src/contract_drift/field_tracking.rs
crates/mockforge-core/src/contract_drift/grpc_contract.rs             → crates/mockforge-contracts/src/contract_drift/grpc_contract.rs
crates/mockforge-core/src/contract_drift/mqtt_kafka_contracts.rs      → crates/mockforge-contracts/src/contract_drift/mqtt_kafka_contracts.rs
crates/mockforge-core/src/contract_drift/protocol_contracts.rs        → crates/mockforge-contracts/src/contract_drift/protocol_contracts.rs
crates/mockforge-core/src/contract_drift/types.rs                     → crates/mockforge-contracts/src/contract_drift/types.rs
crates/mockforge-core/src/contract_drift/websocket_contract.rs        → crates/mockforge-contracts/src/contract_drift/websocket_contract.rs
```

### 3. Rewrite imports inside the 7 moved files

Each moved file needs its `crate::*` references updated for its new home.

**In all 5 of the files that referenced `ai_contract_diff` types** (`breaking_change_detector`, `grpc_contract`, `mqtt_kafka_contracts`, `protocol_contracts`, `types`, `websocket_contract`):

- Replace `use crate::ai_contract_diff::{Mismatch, ContractDiffResult, MismatchSeverity, MismatchType};` (or whichever subset each file uses) with `use mockforge_foundation::contract_diff_types::{Mismatch, ContractDiffResult, MismatchSeverity, MismatchType};`.

**In the 3 protocol-contract files** (`grpc_contract`, `mqtt_kafka_contracts`, `websocket_contract`):

- Replace `use crate::protocol_abstraction::Protocol;` with `use mockforge_foundation::protocol::Protocol;`.

**Sibling-module references stay as `crate::*`** because the siblings move together:

- `use crate::contract_drift::types::*` continues to resolve (types.rs comes with us).
- `use crate::contract_drift::protocol_contracts::*` continues to resolve (protocol_contracts.rs comes with us).

`field_tracking.rs` has zero `crate::` references to rewrite — only chrono/serde/std imports.

### 4. Update `crates/mockforge-contracts/src/contract_drift/mod.rs`

Add declarations and selective re-exports for the 7 new modules. The exact `pub use` list depends on each module's existing public surface — read each file's existing top-of-file exports and mirror them in mod.rs.

Approximate shape (final list confirmed during execution):

```rust
pub mod breaking_change_detector;
pub mod field_tracking;
pub mod forecasting;          // existing
pub mod grpc_contract;
pub mod mqtt_kafka_contracts;
pub mod protocol_contracts;
pub mod types;
pub mod websocket_contract;

pub use breaking_change_detector::{...};
pub use field_tracking::{...};
pub use forecasting::{...};    // existing
pub use grpc_contract::{...};
pub use mqtt_kafka_contracts::{...};
pub use protocol_contracts::{...};
pub use types::{...};
pub use websocket_contract::{...};
```

Update the docstring at the top of `mod.rs`:

- Drop the old "consumer_mapping + fitness previously lived here" note from #606 (these are still gone — no change needed there).
- Replace the "NOTE: the following remain in core" block with a corrected one that says **only `budget_engine` remains in core** due to its `OpenApiSpec` dep.
- Reference the audit conclusion from #604 and link the relevant commits.

### 5. Update `crates/mockforge-core/src/contract_drift/mod.rs`

- Drop these 7 declarations: `pub mod breaking_change_detector;`, `pub mod field_tracking;`, `pub mod grpc_contract;`, `pub mod mqtt_kafka_contracts;`, `pub mod protocol_contracts;`, `pub mod types;`, `pub mod websocket_contract;`.
- Drop the corresponding `pub use` blocks.
- Keep `pub mod budget_engine;` + its re-exports.
- Update the docstring to document the move and point readers at the new contracts home for everything except `budget_engine`.

### 6. Repoint internal core callers (2 files)

**`crates/mockforge-core/src/contract_drift/budget_engine.rs`** — rewrite imports + inline references:

- `use crate::contract_drift::field_tracking::FieldCountTracker;` → `use mockforge_contracts::contract_drift::field_tracking::FieldCountTracker;`
- `use crate::contract_drift::types::{DriftBudget, DriftBudgetConfig, DriftResult};` → `use mockforge_contracts::contract_drift::types::{DriftBudget, DriftBudgetConfig, DriftResult};`
- Inline path references (~5 occurrences) like `crate::contract_drift::types::DriftMetrics`, `crate::contract_drift::types::drift_result_from_diff(...)` → swap `crate::` prefix for `mockforge_contracts::`.

**`crates/mockforge-core/src/contract_drift/fitness.rs`** — rewrite ~9+ inline references to `crate::contract_drift::protocol_contracts::ProtocolContract` (trait used as method parameters) → `mockforge_contracts::contract_drift::protocol_contracts::ProtocolContract`.

### 7. Repoint external caller (1 file)

**`crates/mockforge-http/src/handlers/protocol_contracts.rs`** — rewrite `mockforge_core::contract_drift::protocol_contracts::*` references to `mockforge_contracts::contract_drift::protocol_contracts::*`. mockforge-http already has a contracts dep (existing forecasting consumers), so no Cargo.toml change.

## Testing strategy

Pure code-motion + import rewrite + Cargo dep additions. No behavior change expected.

**Compile-time gates**
- `cargo check --workspace --all-targets` — clean.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean (excluding pre-existing desktop-app + cli failures unrelated to this PR).
- `cargo fmt --all --check` — clean. Run `cargo fmt --all` proactively after qualifier rewrites.

**Per-crate tests**
- `cargo test -p mockforge-contracts --lib` — exercises the 7 new modules. The tests inside each (e.g. `breaking_change_detector.rs::tests`) come along via `git mv` and should pass identically.
- `cargo test -p mockforge-core --lib` — verifies `budget_engine` + `fitness` still work after import repoints.
- `cargo test -p mockforge-http --lib` — verifies the repointed handler.

**Surface-area greps**
```bash
git grep -nE 'mockforge_core::contract_drift::(breaking_change_detector|field_tracking|grpc_contract|mqtt_kafka_contracts|protocol_contracts|types|websocket_contract)' -- crates/
git grep -nE 'crate::contract_drift::(breaking_change_detector|field_tracking|grpc_contract|mqtt_kafka_contracts|protocol_contracts|types|websocket_contract)' -- crates/mockforge-core/src/
```

Both should return empty.

**Cycle prevention**
```bash
cargo tree -p mockforge-contracts | head -20
```

`mockforge-core` must NOT appear in contracts' dep tree.

**Deliberately not added**
- No new tests for the consolidation. Code-motion only.
- No integration tests.

## Out of scope

- **`budget_engine.rs` migration**: stays in core for this PR. Could move if/when (1) `OpenApiSpec` is promoted to foundation or (2) contracts opts in to a `mockforge-openapi` dep. Neither is done here.
- **The contracts `ai` feature gate**: contracts has an `ai = []` feature in its Cargo.toml (originally for the now-deleted semantic_manager). The moved modules don't need it. Leave the feature in place but flag for future cleanup if it ends up unused — track as a minor follow-up if appropriate.

## Risks

- **Cycle introduction**: low. Contracts only adds two external deps (`prost-reflect`, `jsonschema`) — neither pulls in any in-workspace crate. Core gains a dep on contracts (one-way; contracts has no path back to core).
- **Hidden caller missed by audit**: low. The audit ran `git grep` for both `mockforge_core::contract_drift::*` and internal `crate::contract_drift::*` paths. Only 1 external caller (http) and 2 internal core callers (budget_engine, fitness) were found.
- **`prost-reflect` / `jsonschema` version mismatch with workspace**: low. The same versions are already in mockforge-core's Cargo.toml; contracts will pull from the same source.
- **`mockforge-openapi` indirect pull**: confirm during execution that contracts' dep graph doesn't transitively pull `mockforge-openapi` via the new deps. If it does, we have a subtle dep-tree growth that's worth flagging.

## Approval

Scope: 7 modules moved + budget_engine stays. Single PR. Confirmed via brainstorming session 2026-05-21.
