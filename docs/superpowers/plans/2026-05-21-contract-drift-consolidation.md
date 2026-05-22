# contract_drift duplicate consolidation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate the dead-duplicate `consumer_mapping` and `fitness` modules in `mockforge-contracts`, and promote the 3 fitness type definitions (`FitnessFunction`, `FitnessScope`, `FitnessFunctionType`) to `mockforge-foundation::contract_drift_types` so the shared-types pattern matches the rest of the foundation file.

**Architecture:** Three commits, each buildable + tested before the next. (1) Move the 3 type defs into foundation; rewrite core's `fitness.rs` to `pub use` them from foundation. (2) Delete contracts' two duplicate files and clean up `contracts/contract_drift/mod.rs`. (3) Workspace verification + push + PR.

**Tech Stack:** Rust 2021 workspace. No new dependencies. `git mv` not used (no file moves; only deletions + in-place edits + line additions in foundation). `perl -i -pe` not needed.

**Spec:** `docs/superpowers/specs/2026-05-21-contract-drift-consolidation-design.md`
**Issue:** [#602](https://github.com/SaaSy-Solutions/mockforge/issues/602). Out-of-scope follow-ups: [#604](https://github.com/SaaSy-Solutions/mockforge/issues/604), [#605](https://github.com/SaaSy-Solutions/mockforge/issues/605).
**Branch:** `refactor/contract-drift-consolidation` (already created in worktree `/mnt/projects/mockforge-worktrees/brainstorm-602`; spec already committed there; rebased onto post-#603 main).
**Worktree:** `/mnt/projects/mockforge-worktrees/brainstorm-602`. Run every command from this directory.

**Pre-flight checks performed during planning:**
- `mockforge-contracts/src/incidents/` confirmed deleted on current `origin/main` (was the only contracts-internal caller of the modules we're removing).
- Foundation's existing `contract_drift_types.rs` types from `FitnessTestResult` onward (line 266+) do NOT use `#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]`. Core's existing `FitnessFunction*` types also don't. So the promotion is a pure copy without adding schemars gating.
- Core's `fitness.rs` has the 3 type definitions at lines 13–92 (doc comment for `FitnessFunction` at 13; closing brace of `FitnessFunctionType` at 92). Line 98 is the existing `pub use mockforge_foundation::contract_drift_types::FitnessTestResult;`.
- Foundation's `FitnessTestResult` is at line 266 — good adjacent placement for the new types.

---

## File Structure

This is a small refactor. No new files; one foundation file gets ~95 new lines, one core file shrinks by ~80 lines and gains a 3-line `pub use` block, two contracts files are deleted, one contracts mod.rs is trimmed.

**Files modified:**
- `crates/mockforge-foundation/src/contract_drift_types.rs` — add 3 type defs (+~95 LOC)
- `crates/mockforge-core/src/contract_drift/fitness.rs` — delete lines 13–92, add `pub use` block (~-80 LOC net)
- `crates/mockforge-contracts/src/contract_drift/mod.rs` — drop 2 `pub mod` + 2 `pub use` blocks + 2 docstring bullets

**Files deleted:**
- `crates/mockforge-contracts/src/contract_drift/consumer_mapping.rs` (470 LOC)
- `crates/mockforge-contracts/src/contract_drift/fitness.rs` (93 LOC)

---

## Task 1: Promote 3 fitness types to foundation; rewrite core's `fitness.rs` to source them

**Why first:** The contracts deletion in Task 2 doesn't need the foundation promotion to work — but the foundation promotion + core repoint must land together (otherwise core's `fitness.rs` either has duplicate type definitions or references nonexistent types). One atomic commit avoids that broken intermediate state.

**Files:**
- Modify: `crates/mockforge-foundation/src/contract_drift_types.rs`
- Modify: `crates/mockforge-core/src/contract_drift/fitness.rs`

- [ ] **Step 1: Read the exact source text to migrate**

```bash
cd /mnt/projects/mockforge-worktrees/brainstorm-602
sed -n '13,92p' crates/mockforge-core/src/contract_drift/fitness.rs > /tmp/fitness-types-to-promote.txt
wc -l /tmp/fitness-types-to-promote.txt
cat /tmp/fitness-types-to-promote.txt
```

Expected: 80 lines, beginning with the `/// A fitness function...` doc comment and ending with the closing `}` of `FitnessFunctionType`.

- [ ] **Step 2: Find the insertion point in foundation**

```bash
grep -n 'pub struct FitnessTestResult' crates/mockforge-foundation/src/contract_drift_types.rs
```

Expected: line 266 (or close). The new types go directly above `FitnessTestResult` — they're its inputs (a `FitnessTestResult` is the result of evaluating one of these `FitnessFunction`s), so adjacency is logical and the file remains readable.

- [ ] **Step 3: Read the exact context around line 266 in foundation**

```bash
sed -n '258,270p' crates/mockforge-foundation/src/contract_drift_types.rs
```

This shows the doc comment + derive block immediately before `FitnessTestResult`. Note the exact blank-line spacing so the new additions match.

- [ ] **Step 4: Insert the 3 type definitions into foundation**

Use the Edit tool on `crates/mockforge-foundation/src/contract_drift_types.rs`. The `old_string` is the line immediately above `FitnessTestResult`'s doc comment (e.g. the `// ============================================================================` separator or the blank line above the doc comment — read step 3 output to find it precisely). The `new_string` is that same line plus the 3 type definitions plus a blank line, so `FitnessTestResult`'s definition stays at the same position relative to the rest of the file.

The 3 types to paste (exactly from `/tmp/fitness-types-to-promote.txt`):

```rust
/// A fitness function that evaluates contract changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessFunction {
    /// Unique identifier for this fitness function
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what this fitness function checks
    pub description: String,
    /// Type of fitness function
    pub function_type: FitnessFunctionType,
    /// Additional configuration (JSON)
    pub config: serde_json::Value,
    /// Scope where this function applies
    pub scope: FitnessScope,
    /// Whether this function is enabled
    pub enabled: bool,
    /// Timestamp when this function was created
    #[serde(default)]
    pub created_at: i64,
    /// Timestamp when this function was last updated
    #[serde(default)]
    pub updated_at: i64,
}

/// Scope where a fitness function applies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FitnessScope {
    /// Applies globally to all endpoints
    Global,
    /// Applies to a specific workspace
    Workspace {
        /// The workspace ID
        workspace_id: String,
    },
    /// Applies to a specific service (by OpenAPI tag or service name)
    Service {
        /// The service name or OpenAPI tag
        service_name: String,
    },
    /// Applies to a specific endpoint pattern (e.g., "/v1/mobile/*")
    Endpoint {
        /// The endpoint pattern (supports * wildcard)
        pattern: String,
    },
}

/// Type of fitness function
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FitnessFunctionType {
    /// Response size must not increase by more than a percentage
    ResponseSize {
        /// Maximum allowed increase percentage (e.g., 25.0 for 25%)
        max_increase_percent: f64,
    },
    /// No new required fields under a path pattern
    RequiredField {
        /// Path pattern to check (e.g., "/v1/mobile/*")
        path_pattern: String,
        /// Whether new required fields are allowed
        allow_new_required: bool,
    },
    /// Field count must not exceed a threshold
    FieldCount {
        /// Maximum number of fields allowed
        max_fields: u32,
    },
    /// Schema complexity (depth) must not exceed a threshold
    SchemaComplexity {
        /// Maximum schema depth allowed
        max_depth: u32,
    },
    /// Custom fitness function (for future plugin support)
    Custom {
        /// Identifier for the custom evaluator
        evaluator: String,
    },
}

```

Verify against `/tmp/fitness-types-to-promote.txt` that you pasted the same text.

- [ ] **Step 5: Verify foundation compiles standalone**

```bash
cargo check -p mockforge-foundation --lib
```

Expected: clean exit 0. If serde derives fail (e.g. for `serde_json::Value` field) — verify that `serde_json` is in foundation's deps:

```bash
grep -E '^serde_json' crates/mockforge-foundation/Cargo.toml
```

It should be. If not — **stop** and surface as BLOCKED; the spec assumed it was.

- [ ] **Step 6: Replace core's fitness.rs type defs with a foundation re-export**

In `crates/mockforge-core/src/contract_drift/fitness.rs`, the lines 13–92 (the 3 type definitions) need to go. Use the Edit tool with the existing text as the `old_string`. Read the file first to ensure the exact match.

The simplest `new_string` is a single blank line — the re-export will be added at a known position higher up. Specifically:

(a) **First edit** — delete the type defs. Match the exact text of the three definitions (the same 80 lines from `/tmp/fitness-types-to-promote.txt`) and replace with `""` (empty). After this edit, lines 13–92 are gone.

(b) **Second edit** — add the foundation re-export to the use-block at the top of the file. Locate the existing `use std::sync::Arc;` line (it should be just above where the deleted types used to start). Use Edit to transform:

```
old_string:
use crate::ai_contract_diff::{ContractDiffResult, MismatchType};
use crate::openapi::OpenApiSpec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
```

```
new_string:
use crate::ai_contract_diff::{ContractDiffResult, MismatchType};
use crate::openapi::OpenApiSpec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

pub use mockforge_foundation::contract_drift_types::{
    FitnessFunction, FitnessFunctionType, FitnessScope,
};
```

(Adds two blank lines + a 3-line `pub use` block after the existing imports. This preserves the line of `pub use mockforge_foundation::contract_drift_types::FitnessTestResult;` that already exists lower in the file.)

- [ ] **Step 7: Verify compile**

```bash
cargo check -p mockforge-foundation -p mockforge-core --lib
```

Expected: clean exit 0.

If a downstream caller fails because `FitnessFunction` is now ambiguous (foundation type + local type — shouldn't happen after Step 6a, but defensive check): re-grep `pub struct FitnessFunction`/`pub enum FitnessScope`/`pub enum FitnessFunctionType` to confirm each appears in exactly one location.

```bash
git grep -nE '^pub (struct|enum) (FitnessFunction|FitnessScope|FitnessFunctionType)' -- crates/
```

Expected: exactly 3 matches, all in `crates/mockforge-foundation/src/contract_drift_types.rs`.

- [ ] **Step 8: Run affected tests**

```bash
cargo test -p mockforge-foundation -p mockforge-core --lib
```

Expected: all pre-existing tests pass.

- [ ] **Step 9: cargo fmt — qualifier changes can give rustfmt a chance to collapse**

```bash
cargo fmt --all
cargo fmt --all --check
```

Expected: `--check` clean.

- [ ] **Step 10: Commit**

```bash
cd /mnt/projects/mockforge-worktrees/brainstorm-602
git add -A
git commit -m "$(cat <<'EOF'
refactor(foundation): promote FitnessFunction/Scope/FunctionType from core (#602)

The three POD type definitions (`FitnessFunction`, `FitnessScope`,
`FitnessFunctionType`) moved from `mockforge-core::contract_drift::fitness`
into `mockforge-foundation::contract_drift_types` alongside the existing
`FitnessTestResult` (which was already there).

Why: the same three types were duplicated in
`mockforge-contracts::contract_drift::fitness` — defined as separate
Rust structs in two places. Promoting them to foundation breaks the
duplication and matches the pattern used for the other shared
contract-drift types (`AppType`, `ConsumerImpact`, `ConsumerMapping`,
`FitnessTestResult`, `SemanticDriftResult`, etc.).

Core's `fitness.rs` keeps its `FitnessFunctionRegistry` evaluator logic;
it now sources the three types from foundation via `pub use`. The
contracts duplicate of `fitness.rs` is deleted in the next commit.

No new derives added. Foundation's existing types from `FitnessTestResult`
onward do not gate on the `schema` feature, and core's existing
`FitnessFunction*` types didn't either; the promoted definitions preserve
that convention.
EOF
)"
```

---

## Task 2: Delete contracts' duplicate `consumer_mapping.rs` + `fitness.rs`; clean up mod.rs

**Why second:** The foundation promotion in Task 1 is independent of this — but doing them in separate commits keeps each diff small and reviewable. After Task 1, `mockforge_core::contract_drift::fitness` exposes the foundation-hosted types via `pub use`. After Task 2, the contracts duplicates are gone.

**Files:**
- Delete: `crates/mockforge-contracts/src/contract_drift/consumer_mapping.rs`
- Delete: `crates/mockforge-contracts/src/contract_drift/fitness.rs`
- Modify: `crates/mockforge-contracts/src/contract_drift/mod.rs`

- [ ] **Step 1: Defensive caller check**

```bash
cd /mnt/projects/mockforge-worktrees/brainstorm-602
git grep -nE 'mockforge_contracts::contract_drift::(consumer_mapping|fitness)' -- crates/ 2>/dev/null
git grep -nE 'crate::contract_drift::(consumer_mapping|fitness)' -- crates/mockforge-contracts/ 2>/dev/null
```

Expected: both grep commands return empty. If either returns matches, **stop** and surface as BLOCKED — the spec's caller audit was wrong.

- [ ] **Step 2: Read current contracts/contract_drift/mod.rs**

```bash
cat crates/mockforge-contracts/src/contract_drift/mod.rs
```

Note the exact text of the lines to be removed. The file should contain (verbatim, from current `origin/main`):

```rust
//! Pillars: [Contracts]
//!
//! Contract drift detection — independent subsystems
//!
//! This module contains the independently extractable parts of contract drift:
//! - consumer_mapping: Endpoint to SDK method to consuming app relationships
//! - fitness: Fitness function types for validating contract changes
//! - forecasting: API change forecasting based on historical drift patterns
//!
//! NOTE: The following remain in `mockforge-core::contract_drift` due to dependencies:
//! - budget_engine (depends on `OpenApiSpec`)
//! - breaking_change_detector (depends on `ai_contract_diff::Mismatch`)
//! - field_tracking, types (depend on `ai_contract_diff` types)
//! - threat_modeling (depends on `OpenApiSpec` and LLM)
//! - protocol contracts (gRPC, WebSocket, MQTT/Kafka — depend on `ai_contract_diff` types)

pub mod consumer_mapping;
pub mod fitness;
pub mod forecasting;

pub use consumer_mapping::{
    AppType, ConsumerImpact, ConsumerImpactAnalyzer, ConsumerMapping, ConsumerMappingRegistry,
    ConsumingApp, SDKMethod,
};
pub use fitness::{FitnessFunction, FitnessFunctionType, FitnessScope, FitnessTestResult};
pub use forecasting::{
    ChangeForecast, ForecastAggregationLevel, ForecastPattern, ForecastStatistics, Forecaster,
    ForecastingConfig, PatternAnalysis, PatternAnalyzer, PatternSignature, PatternType,
    SeasonalPattern, StatisticalModel,
};
```

If the file differs significantly from this, re-read it precisely before editing.

- [ ] **Step 3: Update mod.rs with one Edit**

Use Edit tool on `crates/mockforge-contracts/src/contract_drift/mod.rs`:

```
old_string:
//! This module contains the independently extractable parts of contract drift:
//! - consumer_mapping: Endpoint to SDK method to consuming app relationships
//! - fitness: Fitness function types for validating contract changes
//! - forecasting: API change forecasting based on historical drift patterns
//!
//! NOTE: The following remain in `mockforge-core::contract_drift` due to dependencies:
//! - budget_engine (depends on `OpenApiSpec`)
//! - breaking_change_detector (depends on `ai_contract_diff::Mismatch`)
//! - field_tracking, types (depend on `ai_contract_diff` types)
//! - threat_modeling (depends on `OpenApiSpec` and LLM)
//! - protocol contracts (gRPC, WebSocket, MQTT/Kafka — depend on `ai_contract_diff` types)

pub mod consumer_mapping;
pub mod fitness;
pub mod forecasting;

pub use consumer_mapping::{
    AppType, ConsumerImpact, ConsumerImpactAnalyzer, ConsumerMapping, ConsumerMappingRegistry,
    ConsumingApp, SDKMethod,
};
pub use fitness::{FitnessFunction, FitnessFunctionType, FitnessScope, FitnessTestResult};
pub use forecasting::{
```

```
new_string:
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
```

This removes the two docstring bullets, the two `pub mod` declarations, and the two `pub use` blocks, while preserving the "NOTE: …" block (architectural decision unchanged) and the `forecasting` declarations. It also adds a brief explanatory paragraph in the docstring pointing future readers at the new homes.

- [ ] **Step 4: Delete the two files**

```bash
cd /mnt/projects/mockforge-worktrees/brainstorm-602
git rm crates/mockforge-contracts/src/contract_drift/consumer_mapping.rs
git rm crates/mockforge-contracts/src/contract_drift/fitness.rs
```

Expected: 2 files staged for deletion.

- [ ] **Step 5: Verify compile**

```bash
cargo check -p mockforge-contracts -p mockforge-core -p mockforge-http -p mockforge-cli --lib
```

Expected: clean exit 0. If anything fails, a caller existed somewhere the audit missed — **stop** and surface.

- [ ] **Step 6: Run tests**

```bash
cargo test -p mockforge-contracts -p mockforge-core --lib
```

Expected: all existing tests pass.

- [ ] **Step 7: cargo fmt**

```bash
cargo fmt --all
cargo fmt --all --check
```

Expected: `--check` clean. (The remaining `pub use forecasting::{...}` block in mod.rs may have shifted; rustfmt may reshape it. Apply if it does.)

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
refactor(contracts): delete dead duplicate consumer_mapping + fitness (#602)

`mockforge-contracts::contract_drift::{consumer_mapping, fitness}` were
near-100% duplicates of the core copies. After #603 (which deleted
`mockforge-contracts::incidents::manager`, the only contracts-internal
caller of these modules), they have zero callers anywhere — external
or internal.

Removed:
- `consumer_mapping.rs` (470 LOC; full duplicate of core's Registry +
  Analyzer + foundation-type re-exports)
- `fitness.rs` (93 LOC; type-def duplicate — the FitnessFunction/Scope/
  FunctionType definitions now live in `mockforge_foundation::contract_drift_types`
  per the previous commit)

`contract_drift/mod.rs` updated: drop the two `pub mod` + `pub use`
blocks + docstring bullets. Preserve the "NOTE: The following remain
in mockforge-core::contract_drift due to dependencies…" block — that
architectural decision is unchanged (tracked for re-audit in #604).

The contracts crate's `contract_drift` module now contains only
`forecasting/`, which has its own external callers (mockforge-http
forecasting handlers + mockforge-cli governance commands).
EOF
)"
```

---

## Task 3: Workspace verification + push + PR

**Why now:** Sanity check that nothing slipped between crates, then ship.

- [ ] **Step 1: Run workspace clippy with warnings-as-errors**

```bash
cd /mnt/projects/mockforge-worktrees/brainstorm-602
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: clean exit 0. If unused-import warnings fire in `core/fitness.rs`, that means a now-orphaned `use crate::...` line survived the type deletion — fix and re-run.

- [ ] **Step 2: Run workspace tests (lib + bins)**

```bash
cargo test --workspace --lib --bins
```

Expected: all tests pass. The full workspace test is slower than per-crate, but worth running once before push.

- [ ] **Step 3: Final surface-area greps**

```bash
echo "=== unique homes of the 3 promoted types ==="
git grep -nE '^pub (struct|enum) (FitnessFunction|FitnessScope|FitnessFunctionType)' -- crates/
echo "=== contracts dead-paths gone ==="
git grep -nE 'mockforge_contracts::contract_drift::(consumer_mapping|fitness)' -- crates/
echo "=== contracts-internal paths gone ==="
git grep -nE 'crate::contract_drift::(consumer_mapping|fitness)' -- crates/mockforge-contracts/
```

Expected output for each:
- First grep: exactly 3 matches, all in `crates/mockforge-foundation/src/contract_drift_types.rs`.
- Second + third greps: empty.

If anything is off, **stop** and surface.

- [ ] **Step 4: Push the branch**

```bash
git push -u origin refactor/contract-drift-consolidation
```

- [ ] **Step 5: Open the PR**

```bash
gh pr create --title "refactor: consolidate duplicate contract_drift modules (#602)" --body "$(cat <<'EOF'
## Summary

Closes #602. Eliminates the dead-duplicate `consumer_mapping` and `fitness` modules in `mockforge-contracts::contract_drift` (no callers anywhere after #603), and promotes the 3 fitness type definitions (`FitnessFunction`, `FitnessScope`, `FitnessFunctionType`) to `mockforge-foundation::contract_drift_types` alongside the other shared contract-drift types.

## What changed

**Commit 1** — promote fitness types to foundation:
- `mockforge-foundation::contract_drift_types`: `FitnessFunction`, `FitnessScope`, `FitnessFunctionType` added next to the existing `FitnessTestResult`. Same derive style as the surrounding types (no `schema` feature gating; serde derives + struct/enum-specific attributes preserved verbatim).
- `mockforge-core::contract_drift::fitness`: the 3 local type definitions deleted (lines 13–92); replaced with a `pub use mockforge_foundation::contract_drift_types::{FitnessFunction, FitnessFunctionType, FitnessScope};` block alongside the existing `use` imports. `FitnessFunctionRegistry` and the evaluator logic are unchanged.

**Commit 2** — delete dead contracts duplicates:
- `mockforge-contracts::contract_drift::consumer_mapping` deleted (470 LOC; full duplicate of core's Registry + Analyzer).
- `mockforge-contracts::contract_drift::fitness` deleted (93 LOC; type-defs duplicate).
- `mockforge-contracts::contract_drift::mod.rs` updated: drop the two `pub mod` and `pub use` blocks plus their docstring bullets. The "NOTE: The following remain in `mockforge-core::contract_drift` due to dependencies…" block is preserved.

## Why this is safe

Verified during planning that neither contracts module had any callers — external or internal:

```
git grep -nE 'mockforge_contracts::contract_drift::(consumer_mapping|fitness)' -- crates/
git grep -nE 'crate::contract_drift::(consumer_mapping|fitness)' -- crates/mockforge-contracts/
```

returns empty. The only contracts-internal caller (`mockforge-contracts::incidents::manager.rs`) was deleted by #603.

The 3 promoted types are pure POD with serde derives. Foundation already hosts the related `FitnessTestResult`, `ConsumerImpact`, etc. with the same pattern.

## Out of scope (tracked separately)

- **#604** — Audit the 8 core-only `contract_drift` modules (`breaking_change_detector`, `budget_engine`, `field_tracking`, `grpc/mqtt_kafka/protocol/websocket_contract`, `types`). They depend on `OpenApiSpec` / `ai_contract_diff::Mismatch` and currently can't move to contracts without further foundation promotions.
- **#605** — Consider promoting `ConsumerMappingRegistry` / `ConsumerImpactAnalyzer` (the logic types in core) to foundation. Deferred as YAGNI — no cross-crate consumer asking for it today.

## Test plan
- [x] `cargo check --workspace --all-targets`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `cargo test --workspace --lib --bins`
- [x] `cargo fmt --all --check`
- [x] No `mockforge_contracts::contract_drift::{consumer_mapping, fitness}` references anywhere in `crates/`
- [x] `FitnessFunction` / `FitnessScope` / `FitnessFunctionType` defined in exactly one place (`mockforge-foundation::contract_drift_types`)
EOF
)"
```

- [ ] **Step 6: Enable auto-merge**

```bash
PR_NUMBER=$(gh pr view --json number -q '.number')
gh pr merge "$PR_NUMBER" --auto --squash
gh pr view "$PR_NUMBER" --json autoMergeRequest,mergeStateStatus,state -q '{auto: .autoMergeRequest.mergeMethod, mergeState: .mergeStateStatus, state: .state}'
```

Expected: `{"auto":"SQUASH","mergeState":"BLOCKED","state":"OPEN"}`.

- [ ] **Step 7: Final cleanup (after merge)**

Once the PR merges (notification will arrive):

```bash
# Don't run until merge confirmed
git -C /mnt/projects/mockforge worktree remove /mnt/projects/mockforge-worktrees/brainstorm-602
git -C /mnt/projects/mockforge branch -D refactor/contract-drift-consolidation
```

---

## Notes for the executing agent

- **The audit during planning checked for foundation feature gating consistency.** Foundation's types from `FitnessTestResult` onward (line 266+ in `contract_drift_types.rs`) do NOT use `#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]`. The promoted types should match that convention — no schemars gating. Same as core's existing definitions.
- **Memory check: rustfmt qualifier collapse.** After any qualifier change, run `cargo fmt --all` proactively — the repo has been bitten twice when shortened qualifiers let rustfmt collapse multi-line calls.
- **Trust but verify subagent reports.** Spec compliance reviewer will independently re-read every code change. Don't claim DONE if you skipped a verification step.
- **No new tests.** Code-motion + type-promotion. The existing tests on both sides are the safety net.
- **If the foundation insertion point in Task 1 Step 4 looks weird** (e.g. an unexpected `// ===========` separator above `FitnessTestResult`), surface and ask. Don't guess on file layout decisions that affect readability.
