# core-only `contract_drift` audit migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate 7 of 8 "core-only" `contract_drift` modules from `mockforge-core` to `mockforge-contracts`, leaving only `budget_engine.rs` in core. Correct the outdated NOTE in `contracts/contract_drift/mod.rs`.

**Architecture:** Four commits. (1) Add the two external Cargo deps to contracts + the `mockforge-contracts` dep to core. (2) Move 3 files with mutual `types ↔ breaking_change_detector` dependency, plus the standalone `field_tracking.rs`; repoint `budget_engine` (the one in-core caller). (3) Move 4 files in the `protocol_contracts` cluster (the trait + its 3 implementations: grpc / mqtt_kafka / websocket); repoint `fitness` and the one http handler; update the contracts mod.rs docstring NOTE. (4) Workspace verification + push + PR.

**Tech Stack:** Rust 2021 workspace, cargo, `git mv` for history preservation, Edit tool for in-file rewrites.

**Spec:** `docs/superpowers/specs/2026-05-21-contract-drift-core-only-audit-design.md`
**Issue:** [#604](https://github.com/SaaSy-Solutions/mockforge/issues/604).
**Depends on:** [#606](https://github.com/SaaSy-Solutions/mockforge/pull/606) — but #606 is auto-merging on CI; this branch is already rebased onto post-#603 main, and #606 only touches `consumer_mapping.rs`/`fitness.rs` deletions in contracts + the `FitnessFunction*` promotion to foundation. None of those files conflict with what this PR touches.
**Branch:** `refactor/contract-drift-core-only-audit` (already created in worktree `/mnt/projects/mockforge-worktrees/brainstorm-604`; spec already committed there).
**Worktree:** `/mnt/projects/mockforge-worktrees/brainstorm-604`. Run every command from this directory.

**Pre-flight facts (verified during planning):**
- `mockforge-foundation::contract_diff_types` has `Mismatch`, `ContractDiffResult`, `MismatchType`, `MismatchSeverity` (at lines 12, 34, 68, 117 of `contract_diff_types.rs`).
- `mockforge-foundation::protocol::Protocol` is the canonical home of the Protocol enum (`protocol.rs:13`); `mockforge-core::protocol_abstraction::Protocol` is a re-export at line 32 of `protocol_abstraction/mod.rs`.
- `mockforge-contracts/Cargo.toml` currently depends on: foundation, serde, serde_json, chrono, uuid, tracing, thiserror, tokio, async-trait, reqwest, regex, hmac, sha2, hex, openapiv3. No `prost-reflect` or `jsonschema` yet.
- `mockforge-core/Cargo.toml` has `prost-reflect = "0.16"` (line 72) and `jsonschema = "0.33.0"` (line 58) — same versions for contracts.
- `mockforge-core/Cargo.toml` does NOT yet have `mockforge-contracts` as a dep. (mockforge-http already does.)
- Inter-module deps among the 7 movable files:
  - `types.rs` ↔ `breaking_change_detector.rs` (mutual via `crate::contract_drift::BreakingChangeDetector` symbol)
  - `breaking_change_detector.rs` → `types`
  - `grpc_contract.rs` → `protocol_contracts`
  - `mqtt_kafka_contracts.rs` → `protocol_contracts`
  - `websocket_contract.rs` → `protocol_contracts`
  - `field_tracking.rs`, `protocol_contracts.rs` — standalone (no inter-module deps)
- External callers (verified empty for 6 of 7 movable modules; the only external caller is `mockforge-http/src/handlers/protocol_contracts.rs`).
- Internal core callers (after the moves): `budget_engine.rs` references `field_tracking` + `types`; `fitness.rs` references `protocol_contracts`.

---

## File Structure

**Files modified:**
- `crates/mockforge-contracts/Cargo.toml` — add 2 deps
- `crates/mockforge-core/Cargo.toml` — add 1 dep
- `crates/mockforge-contracts/src/contract_drift/mod.rs` — declarations + re-exports for 7 new modules + docstring NOTE update
- `crates/mockforge-core/src/contract_drift/mod.rs` — drop 7 declarations + corresponding re-exports + docstring update
- `crates/mockforge-core/src/contract_drift/budget_engine.rs` — repoint imports for moved siblings
- `crates/mockforge-core/src/contract_drift/fitness.rs` — repoint `ProtocolContract` trait references
- `crates/mockforge-http/src/handlers/protocol_contracts.rs` — repoint `mockforge_core::contract_drift::protocol_contracts::*` → `mockforge_contracts::contract_drift::protocol_contracts::*`

**Files moved** (via `git mv`):
- `crates/mockforge-core/src/contract_drift/breaking_change_detector.rs` → `crates/mockforge-contracts/src/contract_drift/breaking_change_detector.rs`
- `crates/mockforge-core/src/contract_drift/field_tracking.rs` → `crates/mockforge-contracts/src/contract_drift/field_tracking.rs`
- `crates/mockforge-core/src/contract_drift/grpc_contract.rs` → `crates/mockforge-contracts/src/contract_drift/grpc_contract.rs`
- `crates/mockforge-core/src/contract_drift/mqtt_kafka_contracts.rs` → `crates/mockforge-contracts/src/contract_drift/mqtt_kafka_contracts.rs`
- `crates/mockforge-core/src/contract_drift/protocol_contracts.rs` → `crates/mockforge-contracts/src/contract_drift/protocol_contracts.rs`
- `crates/mockforge-core/src/contract_drift/types.rs` → `crates/mockforge-contracts/src/contract_drift/types.rs`
- `crates/mockforge-core/src/contract_drift/websocket_contract.rs` → `crates/mockforge-contracts/src/contract_drift/websocket_contract.rs`

**Each moved file also gets in-place edits** to rewrite `use crate::ai_contract_diff::*` and `use crate::protocol_abstraction::*` imports to source from foundation directly (so the files compile in their new home).

---

## Task 1: Add Cargo deps

**Why first:** Subsequent tasks move files that use `prost-reflect` and `jsonschema`. Adding those deps to contracts up front means the file moves can compile immediately. Also adds `mockforge-contracts` to core's Cargo.toml so `budget_engine` + `fitness` can import the moved types in Task 2/3.

**Files:**
- Modify: `crates/mockforge-contracts/Cargo.toml`
- Modify: `crates/mockforge-core/Cargo.toml`

- [ ] **Step 1: Inspect current `[dependencies]` blocks**

```bash
cd /mnt/projects/mockforge-worktrees/brainstorm-604
grep -nE 'prost-reflect|jsonschema|mockforge-contracts' crates/mockforge-contracts/Cargo.toml crates/mockforge-core/Cargo.toml
```

Expected:
- contracts: no `prost-reflect`, no `jsonschema`
- core: has `prost-reflect = "0.16"` (line 72), `jsonschema = "0.33.0"` (line 58), no `mockforge-contracts`

If anything differs (e.g. contracts already has one of those deps after a parallel PR), adapt the next steps accordingly and pause if it's unclear.

- [ ] **Step 2: Add `prost-reflect` + `jsonschema` to `mockforge-contracts/Cargo.toml`**

Use the Edit tool. Locate the existing `openapiv3 = { workspace = true }` line near the end of the `[dependencies]` block. Transform:

```
old_string: openapiv3 = { workspace = true }
new_string: openapiv3 = { workspace = true }
prost-reflect = "0.16"
jsonschema = "0.33.0"
```

(Inserts the two new deps directly after `openapiv3`. If the file's ordering differs, pick an equivalent anchor that uniquely identifies the end of the `[dependencies]` block.)

- [ ] **Step 3: Add `mockforge-contracts` to `mockforge-core/Cargo.toml`**

Read the file first to find an appropriate insertion point. The convention is to group path-deps together. Find the existing `mockforge-foundation = ...` line in core's Cargo.toml and add `mockforge-contracts = { path = "../mockforge-contracts" }` directly after it.

Use Edit tool. Concrete shape (verify the exact pre-image during execution):

```
old_string: mockforge-foundation = { path = "../mockforge-foundation" }
new_string: mockforge-foundation = { path = "../mockforge-foundation" }
mockforge-contracts = { path = "../mockforge-contracts" }
```

If the foundation line has a different style (e.g. `mockforge-foundation = { workspace = true }`), match it. The dep must be a path dep.

- [ ] **Step 4: Verify compile**

```bash
cargo check -p mockforge-contracts -p mockforge-core --lib
```

Expected: clean exit 0. Unused-dep warnings on the new deps are OK at this stage — they get used in Tasks 2–3.

- [ ] **Step 5: Cycle check**

```bash
cargo tree -p mockforge-contracts --depth 1 2>&1 | head -15
```

Expected: contracts now lists `prost-reflect` and `jsonschema` plus its existing deps. `mockforge-core` must NOT appear in contracts' dep tree. If it does, **stop** as BLOCKED — there's a cycle we didn't anticipate.

- [ ] **Step 6: Commit**

```bash
cd /mnt/projects/mockforge-worktrees/brainstorm-604
git add crates/mockforge-contracts/Cargo.toml crates/mockforge-core/Cargo.toml
git commit -m "$(cat <<'EOF'
chore(deps): add prost-reflect + jsonschema to contracts; contracts dep to core (#604 prep)

Subsequent tasks (#604) move 7 contract_drift modules from
mockforge-core to mockforge-contracts. Two of those modules
(grpc_contract, mqtt_kafka_contracts, websocket_contract) use
prost-reflect and jsonschema respectively — contracts needs them
as direct deps so the file moves compile in their new home. Same
versions as already used by mockforge-core (prost-reflect = "0.16",
jsonschema = "0.33.0"), so no new versions enter the workspace.

mockforge-core gains a mockforge-contracts path dep so budget_engine
(stays in core) and fitness (stays in core) can import the moved
types from their new home.

Cycle check: mockforge-contracts depends only on mockforge-foundation
(+ leaf external crates); no path back to core.
EOF
)"
```

---

## Task 2: Move `types`, `breaking_change_detector`, `field_tracking` + repoint `budget_engine`

**Why second:** These three files have no dependency on the `protocol_contracts` cluster, so they move independently. Within them, `types` and `breaking_change_detector` have a mutual dependency (via the `crate::contract_drift::BreakingChangeDetector` symbol) — moving them together is necessary. `field_tracking` is standalone. After this task, `budget_engine` (still in core) imports the moved types from contracts.

**Files:**
- Move: `crates/mockforge-core/src/contract_drift/types.rs` → `crates/mockforge-contracts/src/contract_drift/types.rs`
- Move: `crates/mockforge-core/src/contract_drift/breaking_change_detector.rs` → `crates/mockforge-contracts/src/contract_drift/breaking_change_detector.rs`
- Move: `crates/mockforge-core/src/contract_drift/field_tracking.rs` → `crates/mockforge-contracts/src/contract_drift/field_tracking.rs`
- Modify (after move): `crates/mockforge-contracts/src/contract_drift/types.rs` — rewrite `use crate::ai_contract_diff::ContractDiffResult;` → `use mockforge_foundation::contract_diff_types::ContractDiffResult;`
- Modify (after move): `crates/mockforge-contracts/src/contract_drift/breaking_change_detector.rs` — rewrite `use crate::ai_contract_diff::Mismatch;` → `use mockforge_foundation::contract_diff_types::Mismatch;`
- Modify: `crates/mockforge-contracts/src/contract_drift/mod.rs` — add `pub mod` + `pub use` lines for the 3 new modules
- Modify: `crates/mockforge-core/src/contract_drift/mod.rs` — drop `pub mod` + `pub use` lines for the 3 moved modules
- Modify: `crates/mockforge-core/src/contract_drift/budget_engine.rs` — repoint imports + inline path references

- [ ] **Step 1: Move the 3 files with `git mv`**

```bash
cd /mnt/projects/mockforge-worktrees/brainstorm-604
git mv crates/mockforge-core/src/contract_drift/types.rs crates/mockforge-contracts/src/contract_drift/types.rs
git mv crates/mockforge-core/src/contract_drift/breaking_change_detector.rs crates/mockforge-contracts/src/contract_drift/breaking_change_detector.rs
git mv crates/mockforge-core/src/contract_drift/field_tracking.rs crates/mockforge-contracts/src/contract_drift/field_tracking.rs
```

Expected: 3 files staged as renames (cargo won't compile yet — mod declarations need updating, imports need rewriting).

- [ ] **Step 2: Rewrite imports in the moved `types.rs`**

The moved file currently has `use crate::ai_contract_diff::ContractDiffResult;` (line 11). It needs to source `ContractDiffResult` from foundation directly. Use Edit tool on `crates/mockforge-contracts/src/contract_drift/types.rs`:

```
old_string: use crate::ai_contract_diff::ContractDiffResult;
new_string: use mockforge_foundation::contract_diff_types::ContractDiffResult;
```

The other import in `types.rs` (`pub use mockforge_foundation::contract_drift_types::{...}`) stays — it already sources from foundation.

The inline reference at line ~33 (`use crate::contract_drift::BreakingChangeDetector;` inside the `drift_result_from_diff` function) stays as-is — it resolves to `crate::contract_drift::BreakingChangeDetector` which now means `mockforge_contracts::contract_drift::BreakingChangeDetector`. Since we're also moving `breaking_change_detector` to contracts and adding the re-export in mod.rs (Step 4 below), this resolves correctly.

- [ ] **Step 3: Rewrite imports in the moved `breaking_change_detector.rs`**

The file uses `use crate::ai_contract_diff::Mismatch;` and `use crate::contract_drift::types::{...}`. Use Edit tool on `crates/mockforge-contracts/src/contract_drift/breaking_change_detector.rs`:

```
old_string: use crate::ai_contract_diff::Mismatch;
new_string: use mockforge_foundation::contract_diff_types::Mismatch;
```

The `use crate::contract_drift::types::{...}` line stays as-is — types.rs is also in contracts now, same `crate::` path.

- [ ] **Step 4: No imports to rewrite in `field_tracking.rs`**

`field_tracking.rs` has only chrono/serde/std imports. Verify:

```bash
grep -nE '^use ' crates/mockforge-contracts/src/contract_drift/field_tracking.rs
```

Expected: 3 lines (chrono, serde, std::collections::HashMap). No `crate::` references. No rewrite needed.

- [ ] **Step 5: Read current `crates/mockforge-contracts/src/contract_drift/mod.rs`**

```bash
cat crates/mockforge-contracts/src/contract_drift/mod.rs
```

Post-#606, this file's content is:

```rust
//! Pillars: [Contracts]
//!
//! Contract drift detection — independent subsystems
//!
//! This module contains the independently extractable parts of contract drift:
//! - forecasting: API change forecasting based on historical drift patterns
//!
//! NOTE: The following remain in `mockforge-core::contract_drift` due to dependencies:
//! - budget_engine (depends on `OpenApiSpec`)
//! - breaking_change_detector (depends on `ai_contract_diff::Mismatch`)
//! - field_tracking, types (depend on `ai_contract_diff` types)
//! - threat_modeling (depends on `OpenApiSpec` and LLM)
//! - protocol contracts (gRPC, WebSocket, MQTT/Kafka — depend on `ai_contract_diff` types)
//!
//! `consumer_mapping` and `fitness` previously lived here as duplicates of the
//! core copies; they were deleted in #602 since the contracts copies had no
//! callers. The 3 shared `FitnessFunction*` types live in
//! `mockforge_foundation::contract_drift_types`.

pub mod forecasting;

pub use forecasting::{
    ChangeForecast, ForecastAggregationLevel, ForecastPattern, ForecastStatistics, Forecaster,
    ForecastingConfig, PatternAnalysis, PatternAnalyzer, PatternSignature, PatternType,
    SeasonalPattern, StatisticalModel,
};
```

If the actual contents differ significantly, capture them precisely before Step 6.

- [ ] **Step 6: Add the 3 new modules to `crates/mockforge-contracts/src/contract_drift/mod.rs`**

Use the Edit tool. Two edits:

(a) Add new module declarations + re-exports (these go between `pub mod forecasting;` and the `pub use forecasting::{...}` block, alphabetically):

```
old_string: pub mod forecasting;

pub use forecasting::{
new_string: pub mod breaking_change_detector;
pub mod field_tracking;
pub mod forecasting;
pub mod types;

pub use breaking_change_detector::BreakingChangeDetector;
pub use field_tracking::{FieldCountTracker, FieldHistory};
pub use forecasting::{
```

The exact `pub use` items must match each module's existing public surface — verify against the moved files before applying. The list above is approximate; treat the precise type-name list as something to read from each file's existing top-level exports.

Read each moved file's existing `pub struct`/`pub enum`/`pub fn` declarations (or any existing `pub use` block at the top) and mirror them. If a file has lots of public items, conservatively re-export the small set that other crates / `budget_engine` / `fitness` actually use. (Run `git grep -nE 'mockforge_contracts::contract_drift::(types|breaking_change_detector|field_tracking)::' -- crates/` after Step 9 to verify consumers find what they need.)

(b) Update the docstring (just the "independently extractable" bullet list at the top). Replace:

```
old_string: //! This module contains the independently extractable parts of contract drift:
//! - forecasting: API change forecasting based on historical drift patterns
new_string: //! This module contains the independently extractable parts of contract drift:
//! - breaking_change_detector: Three-way classification of contract diffs (breaking / potentially-breaking / non-breaking)
//! - field_tracking: Field-count tracking + history for drift-budget calculations
//! - forecasting: API change forecasting based on historical drift patterns
//! - types: Shared drift-related types (DriftBudget, DriftMetrics, etc.) + drift_result_from_diff helper
```

Leave the "NOTE: The following remain" block in place for now; Task 3 updates it after the protocol-contract cluster moves.

- [ ] **Step 7: Drop the 3 declarations from `crates/mockforge-core/src/contract_drift/mod.rs`**

Read the file first:

```bash
cat crates/mockforge-core/src/contract_drift/mod.rs
```

Use Edit tool to remove the three `pub mod breaking_change_detector;`, `pub mod field_tracking;`, `pub mod types;` lines, plus any corresponding `pub use breaking_change_detector::...`, `pub use field_tracking::...`, `pub use types::...` re-export blocks.

The exact text depends on the file's current layout. Read first, identify the lines/blocks, then craft precise Edit calls.

- [ ] **Step 8: Repoint `budget_engine.rs` imports**

`crates/mockforge-core/src/contract_drift/budget_engine.rs` has these references that need rewriting:
- Line 7: `use crate::contract_drift::field_tracking::FieldCountTracker;` → `use mockforge_contracts::contract_drift::field_tracking::FieldCountTracker;`
- Line 9: `use crate::contract_drift::types::{DriftBudget, DriftBudgetConfig, DriftResult};` → `use mockforge_contracts::contract_drift::types::{DriftBudget, DriftBudgetConfig, DriftResult};`
- Inline references at lines 109, 138, 185, 240, 269, 316 — rewrite `crate::contract_drift::types::` → `mockforge_contracts::contract_drift::types::` (the inline path uses include `DriftMetrics` and the function call `drift_result_from_diff`).

Use Edit tool. For the imports, two precise edits. For the inline references, prefer `perl -i -pe` with a word-boundary anchor:

```bash
cd /mnt/projects/mockforge-worktrees/brainstorm-604
perl -i -pe 's{\bcrate::contract_drift::types::}{mockforge_contracts::contract_drift::types::}g; s{\bcrate::contract_drift::field_tracking::}{mockforge_contracts::contract_drift::field_tracking::}g; s{\bcrate::contract_drift::BreakingChangeDetector\b}{mockforge_contracts::contract_drift::BreakingChangeDetector}g' crates/mockforge-core/src/contract_drift/budget_engine.rs
```

(The third pattern catches any `crate::contract_drift::BreakingChangeDetector` references — verify with a grep below.)

- [ ] **Step 9: Defensive caller grep**

```bash
cd /mnt/projects/mockforge-worktrees/brainstorm-604
git grep -nE 'crate::contract_drift::(types|breaking_change_detector|field_tracking|BreakingChangeDetector)' -- crates/mockforge-core/src/
```

Expected: empty. Anything matching means a reference wasn't repointed.

Also check external callers (should be empty since we verified during planning):

```bash
git grep -nE 'mockforge_core::contract_drift::(types|breaking_change_detector|field_tracking)' -- crates/
```

Expected: empty.

- [ ] **Step 10: Verify compile**

```bash
cargo check -p mockforge-contracts -p mockforge-core --lib
```

Expected: clean exit 0. Workspace is in a buildable state after this task.

- [ ] **Step 11: Run tests**

```bash
cargo test -p mockforge-contracts -p mockforge-core --lib
```

Expected: all existing tests pass. The tests inside `breaking_change_detector.rs`, `field_tracking.rs`, `types.rs` come along via `git mv` and run from their new home in contracts.

- [ ] **Step 12: cargo fmt**

```bash
cargo fmt --all
cargo fmt --all --check
```

Expected: `--check` clean. Rebrand/qualifier shortenings may give rustfmt a chance to reshape; apply if needed.

- [ ] **Step 13: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
refactor(contracts): move types + breaking_change_detector + field_tracking from core (#604)

Three of the seven moveable contract_drift modules migrate to
mockforge-contracts. Their inter-dependency forces the group:
- types.rs depends on BreakingChangeDetector
- breaking_change_detector.rs depends on types::DriftBudget etc.
- field_tracking.rs is standalone

Imports rewritten in the moved files to source `Mismatch` /
`ContractDiffResult` from `mockforge_foundation::contract_diff_types`
directly (the existing `crate::ai_contract_diff::*` path resolved
through a re-export chain into foundation anyway).

Internal core caller `budget_engine.rs` repointed to source the
moved types from `mockforge_contracts::contract_drift::*`.

No external callers needed repointing (verified).

The contracts mod.rs gets the new declarations + re-exports + an
updated docstring bullet list. The remaining "NOTE: The following
remain in core" block is preserved; Task 3 updates it after the
protocol-contract cluster also moves.
EOF
)"
```

---

## Task 3: Move `protocol_contracts` cluster (4 files) + repoint `fitness` + repoint http handler + finalize docstring

**Why now:** The 4 files (`protocol_contracts` + `grpc_contract` + `mqtt_kafka_contracts` + `websocket_contract`) all reference each other and must move together. After this task, every moveable module is in contracts; only `budget_engine` remains in core.

**Files:**
- Move: `crates/mockforge-core/src/contract_drift/protocol_contracts.rs` → `crates/mockforge-contracts/src/contract_drift/protocol_contracts.rs`
- Move: `crates/mockforge-core/src/contract_drift/grpc_contract.rs` → `crates/mockforge-contracts/src/contract_drift/grpc_contract.rs`
- Move: `crates/mockforge-core/src/contract_drift/mqtt_kafka_contracts.rs` → `crates/mockforge-contracts/src/contract_drift/mqtt_kafka_contracts.rs`
- Move: `crates/mockforge-core/src/contract_drift/websocket_contract.rs` → `crates/mockforge-contracts/src/contract_drift/websocket_contract.rs`
- Modify (after move): each moved file's imports
- Modify: `crates/mockforge-contracts/src/contract_drift/mod.rs` — add 4 declarations + re-exports + finalize docstring
- Modify: `crates/mockforge-core/src/contract_drift/mod.rs` — drop 4 declarations + corresponding re-exports + docstring update
- Modify: `crates/mockforge-core/src/contract_drift/fitness.rs` — repoint `ProtocolContract` trait references
- Modify: `crates/mockforge-http/src/handlers/protocol_contracts.rs` — repoint module path

- [ ] **Step 1: Move the 4 files with `git mv`**

```bash
cd /mnt/projects/mockforge-worktrees/brainstorm-604
git mv crates/mockforge-core/src/contract_drift/protocol_contracts.rs crates/mockforge-contracts/src/contract_drift/protocol_contracts.rs
git mv crates/mockforge-core/src/contract_drift/grpc_contract.rs crates/mockforge-contracts/src/contract_drift/grpc_contract.rs
git mv crates/mockforge-core/src/contract_drift/mqtt_kafka_contracts.rs crates/mockforge-contracts/src/contract_drift/mqtt_kafka_contracts.rs
git mv crates/mockforge-core/src/contract_drift/websocket_contract.rs crates/mockforge-contracts/src/contract_drift/websocket_contract.rs
```

Expected: 4 files staged as renames.

- [ ] **Step 2: Rewrite imports in the moved `protocol_contracts.rs`**

The file uses `use crate::ai_contract_diff::{ContractDiffResult, Mismatch};`. Replace:

```
old_string: use crate::ai_contract_diff::{ContractDiffResult, Mismatch};
new_string: use mockforge_foundation::contract_diff_types::{ContractDiffResult, Mismatch};
```

- [ ] **Step 3: Rewrite imports in the moved `grpc_contract.rs`**

Two rewrites. Read the file first to confirm the exact pre-images:

```
old_string: use crate::ai_contract_diff::{ContractDiffResult, Mismatch, MismatchSeverity, MismatchType};
new_string: use mockforge_foundation::contract_diff_types::{ContractDiffResult, Mismatch, MismatchSeverity, MismatchType};
```

```
old_string: use crate::protocol_abstraction::Protocol;
new_string: use mockforge_foundation::protocol::Protocol;
```

The `use crate::contract_drift::protocol_contracts::{...};` line stays — that path resolves in the new home because `protocol_contracts.rs` is also in contracts now (`crate::` is `mockforge_contracts`).

- [ ] **Step 4: Rewrite imports in the moved `mqtt_kafka_contracts.rs`**

Same shape as Step 3. Two rewrites:

```
old_string: use crate::ai_contract_diff::{ContractDiffResult, Mismatch, MismatchSeverity, MismatchType};
new_string: use mockforge_foundation::contract_diff_types::{ContractDiffResult, Mismatch, MismatchSeverity, MismatchType};
```

```
old_string: use crate::protocol_abstraction::Protocol;
new_string: use mockforge_foundation::protocol::Protocol;
```

The `use jsonschema::*;` line stays — contracts now has the dep (Task 1).

- [ ] **Step 5: Rewrite imports in the moved `websocket_contract.rs`**

Same shape. Two rewrites:

```
old_string: use crate::ai_contract_diff::{ContractDiffResult, Mismatch, MismatchSeverity, MismatchType};
new_string: use mockforge_foundation::contract_diff_types::{ContractDiffResult, Mismatch, MismatchSeverity, MismatchType};
```

```
old_string: use crate::protocol_abstraction::Protocol;
new_string: use mockforge_foundation::protocol::Protocol;
```

- [ ] **Step 6: Update `crates/mockforge-contracts/src/contract_drift/mod.rs`**

Add the 4 new declarations + re-exports + finalize the docstring. Read the file first (it was already partially updated in Task 2).

Two edits expected:

(a) Add the new modules to the `pub mod` block (alphabetically, between existing entries):

```
old_string: pub mod breaking_change_detector;
pub mod field_tracking;
pub mod forecasting;
pub mod types;
new_string: pub mod breaking_change_detector;
pub mod field_tracking;
pub mod forecasting;
pub mod grpc_contract;
pub mod mqtt_kafka_contracts;
pub mod protocol_contracts;
pub mod types;
pub mod websocket_contract;
```

(b) Add corresponding re-exports — read each moved file's existing top-of-file `pub` items and mirror them. As a starting list (adjust to match actual public surface):

```rust
pub use grpc_contract::{GrpcContract, GrpcContractRegistry};
pub use mqtt_kafka_contracts::{KafkaContract, MqttContract};
pub use protocol_contracts::{ProtocolContract, ProtocolContractRegistry};
pub use websocket_contract::{WebSocketContract, WebSocketContractRegistry};
```

(Verify the exact type names against each moved file's existing `pub struct`/`pub trait` declarations before applying.)

(c) Finalize the docstring NOTE block. Current content (post-Task 2) still has the stale "NOTE: The following remain in mockforge-core::contract_drift due to dependencies" block listing 5 things. Replace it with a corrected version:

```
old_string: //! NOTE: The following remain in `mockforge-core::contract_drift` due to dependencies:
//! - budget_engine (depends on `OpenApiSpec`)
//! - breaking_change_detector (depends on `ai_contract_diff::Mismatch`)
//! - field_tracking, types (depend on `ai_contract_diff` types)
//! - threat_modeling (depends on `OpenApiSpec` and LLM)
//! - protocol contracts (gRPC, WebSocket, MQTT/Kafka — depend on `ai_contract_diff` types)
//!
//! `consumer_mapping` and `fitness` previously lived here as duplicates of the
//! core copies; they were deleted in #602 since the contracts copies had no
//! callers. The 3 shared `FitnessFunction*` types live in
//! `mockforge_foundation::contract_drift_types`.
new_string: //! NOTE: Only `budget_engine` remains in `mockforge-core::contract_drift`,
//! because it depends on `mockforge-openapi::OpenApiSpec` and on
//! in-core sibling types (`consumer_mapping::ConsumerImpactAnalyzer`,
//! `fitness::FitnessFunctionRegistry`). Moving it would require contracts
//! to depend on `mockforge-openapi`, expanding the contracts surface area.
//! Tracked for future re-audit at issue #604's closing comment.
//!
//! Update from earlier NOTE: the previously-listed blockers
//! (`ai_contract_diff::Mismatch`, `ContractDiffResult`, etc.) are
//! actually in `mockforge-foundation::contract_diff_types` already —
//! they were promoted in an earlier migration (Phase 6 / A5). Modules
//! that used those types as their only "core-only" dep moved to
//! contracts in #604 (this PR).
//!
//! `consumer_mapping` and `fitness` previously lived here as duplicates of the
//! core copies; they were deleted in #602 since the contracts copies had no
//! callers. The 3 shared `FitnessFunction*` types live in
//! `mockforge_foundation::contract_drift_types`.
```

- [ ] **Step 7: Drop the 4 declarations from `crates/mockforge-core/src/contract_drift/mod.rs`**

Use Edit tool to remove these lines from core's mod.rs (read it first to confirm exact text):
- `pub mod protocol_contracts;`
- `pub mod grpc_contract;`
- `pub mod mqtt_kafka_contracts;`
- `pub mod websocket_contract;`
- All corresponding `pub use` re-export blocks

After this, core's contract_drift mod.rs declares only `pub mod budget_engine;` (the one module left in core).

Also update the docstring to document the new reality:

```
new_string (replaces docstring header):
//! Pillars: [Contracts]
//!
//! Contract drift detection — core-only pieces
//!
//! As of #604, most of contract_drift moved to `mockforge_contracts::contract_drift`.
//! Only `budget_engine` remains here, because it depends on
//! `mockforge_openapi::OpenApiSpec` and on `crate::ai_contract_diff::ContractDiffResult`
//! (sourceable from `mockforge_foundation::contract_diff_types` if budget_engine
//! eventually wants to move too).
```

Read the existing docstring before crafting the Edit `old_string`.

- [ ] **Step 8: Repoint `crates/mockforge-core/src/contract_drift/fitness.rs`**

The file has ~9+ inline references to `crate::contract_drift::protocol_contracts::ProtocolContract` (used as trait-object method parameters: `&dyn crate::contract_drift::protocol_contracts::ProtocolContract`).

Use perl to rewrite all of them:

```bash
cd /mnt/projects/mockforge-worktrees/brainstorm-604
perl -i -pe 's{\bcrate::contract_drift::protocol_contracts::}{mockforge_contracts::contract_drift::protocol_contracts::}g' crates/mockforge-core/src/contract_drift/fitness.rs
```

Verify the rewrite:

```bash
grep -nE 'protocol_contracts::' crates/mockforge-core/src/contract_drift/fitness.rs | head -5
```

Expected: all matches now use `mockforge_contracts::contract_drift::protocol_contracts::`.

- [ ] **Step 9: Repoint `crates/mockforge-http/src/handlers/protocol_contracts.rs`**

```bash
perl -i -pe 's{\bmockforge_core::contract_drift::protocol_contracts}{mockforge_contracts::contract_drift::protocol_contracts}g; s{\bmockforge_core::contract_drift::grpc_contract}{mockforge_contracts::contract_drift::grpc_contract}g; s{\bmockforge_core::contract_drift::mqtt_kafka_contracts}{mockforge_contracts::contract_drift::mqtt_kafka_contracts}g; s{\bmockforge_core::contract_drift::websocket_contract}{mockforge_contracts::contract_drift::websocket_contract}g' crates/mockforge-http/src/handlers/protocol_contracts.rs
```

Verify:

```bash
grep -nE 'mockforge_core::contract_drift::(protocol_contracts|grpc_contract|mqtt_kafka_contracts|websocket_contract)' crates/mockforge-http/src/handlers/protocol_contracts.rs
```

Expected: empty.

- [ ] **Step 10: Defensive workspace-wide caller grep**

```bash
git grep -nE 'mockforge_core::contract_drift::(protocol_contracts|grpc_contract|mqtt_kafka_contracts|websocket_contract|types|breaking_change_detector|field_tracking)' -- crates/
git grep -nE 'crate::contract_drift::(protocol_contracts|grpc_contract|mqtt_kafka_contracts|websocket_contract|types|breaking_change_detector|field_tracking|BreakingChangeDetector)' -- crates/mockforge-core/src/
```

Both should be empty. Anything matching means a caller was missed.

- [ ] **Step 11: Verify compile**

```bash
cargo check -p mockforge-contracts -p mockforge-core -p mockforge-http --lib
```

Expected: clean exit 0.

- [ ] **Step 12: Run tests**

```bash
cargo test -p mockforge-contracts -p mockforge-core -p mockforge-http --lib
```

Expected: all existing tests pass.

- [ ] **Step 13: cargo fmt**

```bash
cargo fmt --all
cargo fmt --all --check
```

Expected: `--check` clean.

- [ ] **Step 14: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
refactor(contracts): move protocol_contracts cluster + finalize NOTE (#604)

Migrates the four-file `protocol_contracts` cluster (the trait + its
3 implementations) from mockforge-core to mockforge-contracts:
- protocol_contracts.rs (the trait)
- grpc_contract.rs       (depends on prost_reflect)
- mqtt_kafka_contracts.rs (depends on jsonschema)
- websocket_contract.rs   (depends on jsonschema)

The 3 implementations all reference `crate::contract_drift::protocol_contracts::*`,
so they must move together with the trait. Both external crates
(`prost_reflect`, `jsonschema`) were added to contracts' Cargo.toml
in the prep commit.

Import rewrites in the 4 moved files:
- `use crate::ai_contract_diff::{ContractDiffResult, Mismatch, ...}`
  → `use mockforge_foundation::contract_diff_types::{...}` (3 files)
- `use crate::protocol_abstraction::Protocol`
  → `use mockforge_foundation::protocol::Protocol` (3 files)

Repointed callers:
- `mockforge-core::contract_drift::fitness` — ~9 `ProtocolContract`
  trait-object references (used as method parameters)
- `mockforge-http::handlers::protocol_contracts` — module-path imports

mod.rs updates:
- contracts/contract_drift/mod.rs: declarations + re-exports for the
  4 new modules; docstring NOTE rewritten to correctly say only
  `budget_engine` remains in core (the previous list was outdated —
  the ai_contract_diff types had already been foundation-promoted).
- core/contract_drift/mod.rs: drops the 4 declarations + re-exports;
  docstring rewritten to document budget_engine as the lone remainder.

Closes #604.
EOF
)"
```

---

## Task 4: Workspace verification + push + PR

**Why now:** Belt-and-suspenders. Per-task verification already ran cargo check + tests for affected crates; a workspace-wide pass catches anything that slipped between crates.

- [ ] **Step 1: Run workspace clippy with warnings-as-errors**

```bash
cd /mnt/projects/mockforge-worktrees/brainstorm-604
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -50
```

Expected: clean exit 0 for the crates this PR touches (mockforge-foundation, mockforge-contracts, mockforge-core, mockforge-http). Pre-existing warnings in `desktop-app` (deprecated Tauri APIs, dead-code, unused-variables) and `mockforge-cli` (`CARGO_BIN_EXE_mockforge` test-harness) are unrelated to this PR — flag them in the PR description as DONE_WITH_CONCERNS observations but do not try to fix them.

- [ ] **Step 2: Run workspace tests (lib + bins)**

```bash
cargo test --workspace --lib --bins 2>&1 | tail -30
```

Expected: all tests pass for affected crates. Same caveat for pre-existing failures.

- [ ] **Step 3: Final surface-area greps**

```bash
echo "=== homes of the moved types ==="
git grep -nE 'pub (struct|enum|trait) (BreakingChangeDetector|FieldCountTracker|ProtocolContract|GrpcContract|KafkaContract|MqttContract|WebSocketContract)\b' -- crates/

echo "=== no remaining core dead-paths ==="
git grep -nE 'mockforge_core::contract_drift::(protocol_contracts|grpc_contract|mqtt_kafka_contracts|websocket_contract|types|breaking_change_detector|field_tracking)' -- crates/

echo "=== no remaining core-internal dead-paths ==="
git grep -nE 'crate::contract_drift::(protocol_contracts|grpc_contract|mqtt_kafka_contracts|websocket_contract|types|breaking_change_detector|field_tracking|BreakingChangeDetector)' -- crates/mockforge-core/src/

echo "=== core/contract_drift/ contents (should be 2 files: mod.rs + budget_engine.rs) ==="
ls crates/mockforge-core/src/contract_drift/

echo "=== contracts/contract_drift/ contents (should be 9 entries: 7 moved + forecasting/ + mod.rs) ==="
ls crates/mockforge-contracts/src/contract_drift/
```

Each expected output:
- First grep: matches in `crates/mockforge-contracts/src/contract_drift/` only (no other location).
- Second + third greps: empty.
- core listing: `budget_engine.rs`, `mod.rs`.
- contracts listing: `breaking_change_detector.rs`, `field_tracking.rs`, `forecasting`, `grpc_contract.rs`, `mod.rs`, `mqtt_kafka_contracts.rs`, `protocol_contracts.rs`, `types.rs`, `websocket_contract.rs`.

If anything mismatches, **stop** and surface.

- [ ] **Step 4: Final cycle check**

```bash
cargo tree -p mockforge-contracts --depth 1 2>&1 | head -20
```

Expected: contracts depends on foundation + (new) prost-reflect + (new) jsonschema + the existing serde/tokio/etc. `mockforge-core` must NOT appear.

- [ ] **Step 5: Push the branch**

```bash
git push -u origin refactor/contract-drift-core-only-audit
```

- [ ] **Step 6: Open the PR**

```bash
gh pr create --title "refactor: migrate 7 contract_drift modules from core to contracts (#604)" --body "$(cat <<'EOF'
## Summary

Closes #604. Migrates 7 of 8 "core-only" `contract_drift` modules from `mockforge-core` to `mockforge-contracts`. Only `budget_engine.rs` stays in core (it depends on `mockforge-openapi::OpenApiSpec` + in-core siblings).

The audit found the previous "NOTE: stay in core due to ai_contract_diff deps" block in `mockforge-contracts::contract_drift::mod.rs` was outdated — `Mismatch`, `ContractDiffResult`, `MismatchType`, `MismatchSeverity`, and `Protocol` are already in `mockforge-foundation`, so any caller (including contracts) can source them directly without depending on `mockforge-intelligence` or `mockforge-core`.

## What changed

### Commit 1 — Cargo deps
- `mockforge-contracts/Cargo.toml`: add `prost-reflect = "0.16"` + `jsonschema = "0.33.0"` (same versions as already used by core).
- `mockforge-core/Cargo.toml`: add `mockforge-contracts = { path = "../mockforge-contracts" }` so `budget_engine` and `fitness` can import the moved types.

### Commit 2 — Move types + breaking_change_detector + field_tracking
Moved via `git mv` (history preserved). Imports rewritten to source `Mismatch`/`ContractDiffResult` from `mockforge_foundation::contract_diff_types` directly. `budget_engine.rs` repointed to import the moved types from `mockforge_contracts::contract_drift::*`.

### Commit 3 — Move protocol_contracts cluster (4 files) + finalize NOTE
Moved `protocol_contracts.rs` + `grpc_contract.rs` + `mqtt_kafka_contracts.rs` + `websocket_contract.rs` together (the 3 implementations depend on `protocol_contracts`). Imports rewritten. `mockforge-core::contract_drift::fitness` and `mockforge-http::handlers::protocol_contracts` repointed.

The outdated docstring NOTE in `mockforge-contracts::contract_drift::mod.rs` rewritten to correctly identify `budget_engine` as the lone core-only module remaining.

## Why this is safe

- **No cycle**: `mockforge-contracts → mockforge-foundation` only. `mockforge-core → mockforge-contracts` is one-way (added in commit 1; no path back).
- **No new external versions**: `prost-reflect` and `jsonschema` are already in `mockforge-core/Cargo.toml` at the same versions.
- **No public-API regression for in-workspace consumers**: the only external caller (`mockforge-http/src/handlers/protocol_contracts.rs`) was repointed in commit 3. Tests pass for `mockforge-foundation`, `mockforge-contracts`, `mockforge-core`, `mockforge-http`.
- **No behavior change**: pure code motion + import rewrites. The types are byte-identical (same foundation source).

## Out of scope (tracked separately)

- **`budget_engine.rs` migration** — would require contracts to depend on `mockforge-openapi`, which significantly expands the contracts surface area. Defer until a future PR if/when there's a concrete consumer of `budget_engine` outside core.
- **#605** — Promote `ConsumerMappingRegistry`/`ConsumerImpactAnalyzer` to foundation. YAGNI deferral; trigger condition is "a new cross-crate consumer needs them."

## Test plan
- [x] `cargo check --workspace --all-targets`
- [x] `cargo clippy --workspace --all-targets -- -D warnings` (excluding pre-existing desktop-app + cli failures unrelated to this PR)
- [x] `cargo test --workspace --lib --bins`
- [x] `cargo fmt --all --check`
- [x] No `mockforge_core::contract_drift::{...moved modules}` references anywhere in `crates/`
- [x] No `crate::contract_drift::{...moved modules}` references inside `mockforge-core`
- [x] `cargo tree -p mockforge-contracts` shows no `mockforge-core` (cycle check)
- [x] `mockforge-core::contract_drift::` contains only `mod.rs` + `budget_engine.rs`
EOF
)"
```

- [ ] **Step 7: Enable auto-merge**

```bash
PR_NUMBER=$(gh pr view --json number -q '.number')
gh pr merge "$PR_NUMBER" --auto --squash
gh pr view "$PR_NUMBER" --json autoMergeRequest,mergeStateStatus,state -q '{auto: .autoMergeRequest.mergeMethod, mergeState: .mergeStateStatus, state: .state}'
```

Expected: `{"auto":"SQUASH","mergeState":"BLOCKED","state":"OPEN"}`. CI will clear (modulo pre-existing breakage) and the PR will auto-merge.

- [ ] **Step 8: Final cleanup (after merge)**

Once the PR merges (notification will arrive):

```bash
# Don't run until merge confirmed
git -C /mnt/projects/mockforge worktree remove /mnt/projects/mockforge-worktrees/brainstorm-604
git -C /mnt/projects/mockforge branch -D refactor/contract-drift-core-only-audit
```

---

## Notes for the executing agent

- **The corrected dep picture is the load-bearing insight.** Many parts of this plan assume `Mismatch`/`ContractDiffResult`/`MismatchType`/`MismatchSeverity` live in `mockforge-foundation::contract_diff_types` (lines 12, 34, 68, 117 of `contract_diff_types.rs`). If at any point a rewrite fails because foundation doesn't have one of these types, **STOP** and surface — something has changed since the plan was written.
- **`Protocol` is in `mockforge-foundation::protocol`** (`protocol.rs:13`). The path `crate::protocol_abstraction::Protocol` in core is a re-export of this same type at `core/src/protocol_abstraction/mod.rs:32`. The 3 protocol-contract files can switch directly to `mockforge_foundation::protocol::Protocol`.
- **`git mv`, not `cp + rm`.** History preservation matters for `git log -- <file>` and `git blame` post-merge.
- **Memory check: rustfmt qualifier collapse.** After any qualifier change, run `cargo fmt --all` proactively — repo has been bitten twice when shortened qualifiers let rustfmt collapse multi-line calls.
- **Memory check: pnpm-only repo.** No UI changes here, but if any task seems to need UI work, that's a scope-creep signal — surface it.
- **No new tests.** Code-motion only. The existing tests on both sides are the safety net.
- **Pre-existing CI failures.** `desktop-app/src/{updater,main,notifications,server,shortcuts}.rs` have pre-existing clippy warnings. `mockforge-cli` has pre-existing `CARGO_BIN_EXE_mockforge` test-harness failures. Both are demonstrably untouched by this PR and were already breaking CI on `origin/main`. Auto-merge will land the PR when the required CI checks pass; the pre-existing failures don't gate the merge.
