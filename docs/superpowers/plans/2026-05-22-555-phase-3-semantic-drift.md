# #555 Phase 3: semantic_drift handler extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move `crates/mockforge-http/src/handlers/semantic_drift.rs` (428 LOC) to `crates/mockforge-intelligence/src/handlers/semantic_drift.rs`, mirroring #610's pattern for `pr_generation`.

**Architecture:** Two commits. (1) Move the file, rewrite its imports (intelligence-internal + foundation-direct), declare in intelligence's handlers/mod.rs, drop from http's handlers/mod.rs, repoint http's route caller, audit the database feature-gate propagation. (2) Workspace verification + push + PR.

**Tech Stack:** Rust 2021 workspace, cargo, `git mv` for history preservation, Edit tool.

**Spec:** `docs/superpowers/specs/2026-05-22-555-phase-3-semantic-drift-design.md`
**Issue:** [#555](https://github.com/SaaSy-Solutions/mockforge/issues/555), phase 3.
**Branch:** `refactor/555-phase-3-semantic-drift` (already created off post-#611 main in worktree `/mnt/projects/mockforge-worktrees/brainstorm-555-p3`; spec committed there).
**Worktree:** `/mnt/projects/mockforge-worktrees/brainstorm-555-p3`. Run every command from this directory.

**Pre-flight facts (verified during planning):**
- `mockforge-intelligence/src/handlers/mod.rs` exists (from #610) and contains `pub mod pr_generation;`.
- `mockforge-intelligence/src/lib.rs` already has `pub mod handlers;` (from #610).
- `mockforge-intelligence/src/ai_contract_diff/mod.rs:88` has `pub struct ContractDiffAnalyzer` (since #562 phase 4).
- `mockforge-intelligence/src/incidents/semantic_manager.rs:150` has `pub struct SemanticIncidentManager` (since #601).
- `mockforge-intelligence/src/database.rs` exists (since #611).
- `mockforge-foundation/src/incidents_types.rs:15` has `pub enum IncidentStatus`, line 39 has `pub enum IncidentSeverity`.
- The only external caller of `handlers::semantic_drift` is `crates/mockforge-http/src/lib.rs` (verified via `git grep -lE 'handlers::semantic_drift'`).
- `semantic_drift.rs` has 7 `#[cfg(feature = "database")]` blocks.
- `mockforge-intelligence/Cargo.toml` already has a `database` feature (gating sqlx) since #611.

---

## File Structure

**Files moved:**
- `crates/mockforge-http/src/handlers/semantic_drift.rs` → `crates/mockforge-intelligence/src/handlers/semantic_drift.rs`

**Files modified:**
- `crates/mockforge-intelligence/src/handlers/semantic_drift.rs` (after move): rewrite 2 top-of-file `mockforge_core::*` imports to `crate::*`; rewrite 1 function-local `mockforge_core::incidents::types::*` import + 4 inline path references to `mockforge_foundation::incidents_types::*`.
- `crates/mockforge-intelligence/src/handlers/mod.rs`: add `pub mod semantic_drift;`.
- `crates/mockforge-http/src/handlers/mod.rs`: drop `pub mod semantic_drift;` (and any related `pub use`).
- `crates/mockforge-http/src/lib.rs`: change route wiring from `crate::handlers::semantic_drift::*` to `mockforge_intelligence::handlers::semantic_drift::*`.
- Possibly `crates/mockforge-http/Cargo.toml`: add `database = ["mockforge-intelligence/database"]` to the `database` feature definition if not already propagating.

---

## Task 1: Move + rewrite + caller update

**Why one task:** All these changes must land together for the workspace to compile. Splitting would leave intermediate broken states.

**Files:** Listed in File Structure section above.

- [ ] **Step 1: Move the file with `git mv`**

```bash
cd /mnt/projects/mockforge-worktrees/brainstorm-555-p3
git mv crates/mockforge-http/src/handlers/semantic_drift.rs \
       crates/mockforge-intelligence/src/handlers/semantic_drift.rs
```

Expected: 1 file staged as a rename.

- [ ] **Step 2: Read the moved file's top-of-file imports**

```bash
sed -n '1,35p' crates/mockforge-intelligence/src/handlers/semantic_drift.rs
```

Expected (lines 13–14):
```rust
use mockforge_core::ai_contract_diff::{ContractDiffAnalyzer, ContractDiffConfig};
use mockforge_core::incidents::semantic_manager::{SemanticIncident, SemanticIncidentManager};
```

And line 24:
```rust
use crate::database::Database;
```

If anything differs, adjust the Edit calls below.

- [ ] **Step 3: Rewrite the two top-of-file `mockforge_core::*` imports**

Use the Edit tool on `crates/mockforge-intelligence/src/handlers/semantic_drift.rs`:

```
old_string: use mockforge_core::ai_contract_diff::{ContractDiffAnalyzer, ContractDiffConfig};
use mockforge_core::incidents::semantic_manager::{SemanticIncident, SemanticIncidentManager};
new_string: use crate::ai_contract_diff::{ContractDiffAnalyzer, ContractDiffConfig};
use crate::incidents::semantic_manager::{SemanticIncident, SemanticIncidentManager};
```

`crate::database::Database` (line 24) stays as-is — `database` is a sibling module in intelligence.

- [ ] **Step 4: Find the function-local `IncidentStatus` import + verify line numbers**

```bash
grep -nE 'mockforge_core::incidents::types::' crates/mockforge-intelligence/src/handlers/semantic_drift.rs
```

Expected output:
- 1 line at ~31 — `use mockforge_core::incidents::types::{IncidentSeverity, IncidentStatus};`
- 4 lines at ~220–223 — inline `mockforge_core::incidents::types::IncidentStatus::*` references

If line numbers differ slightly, that's OK — the Edit tool uses string matching.

- [ ] **Step 5: Rewrite the function-local `use` (around line 31)**

```
old_string: use mockforge_core::incidents::types::{IncidentSeverity, IncidentStatus};
new_string: use mockforge_foundation::incidents_types::{IncidentSeverity, IncidentStatus};
```

- [ ] **Step 6: Rewrite the 4 inline path references (lines ~220–223)**

```
old_string:         "open" => Some(mockforge_core::incidents::types::IncidentStatus::Open),
        "acknowledged" => Some(mockforge_core::incidents::types::IncidentStatus::Acknowledged),
        "resolved" => Some(mockforge_core::incidents::types::IncidentStatus::Resolved),
        "closed" => Some(mockforge_core::incidents::types::IncidentStatus::Closed),
new_string:         "open" => Some(mockforge_foundation::incidents_types::IncidentStatus::Open),
        "acknowledged" => Some(mockforge_foundation::incidents_types::IncidentStatus::Acknowledged),
        "resolved" => Some(mockforge_foundation::incidents_types::IncidentStatus::Resolved),
        "closed" => Some(mockforge_foundation::incidents_types::IncidentStatus::Closed),
```

(Indentation may differ — read lines 218–225 first to confirm the exact whitespace. Adjust the pre-image to match.)

- [ ] **Step 7: Defensive grep — confirm no `mockforge_core::*` left in the moved file**

```bash
grep -nE 'mockforge_core::' crates/mockforge-intelligence/src/handlers/semantic_drift.rs
```

Expected: empty. If anything matches, find and rewrite it.

- [ ] **Step 8: Add `pub mod semantic_drift;` to intelligence's handlers mod.rs**

Read the current state:

```bash
cat crates/mockforge-intelligence/src/handlers/mod.rs
```

Expected to see (from #610):
```rust
//! HTTP handlers for AI-powered intelligence features.
//! ...
pub mod pr_generation;
```

Use Edit:

```
old_string: pub mod pr_generation;
new_string: pub mod pr_generation;
pub mod semantic_drift;
```

If the file's structure differs (e.g. there are more handler declarations already, or the format isn't quite this shape), adapt — the goal is alphabetical-ish placement of `pub mod semantic_drift;` among the existing `pub mod` declarations.

- [ ] **Step 9: Drop `pub mod semantic_drift;` from http's handlers mod.rs**

Read:

```bash
grep -nE 'semantic_drift' crates/mockforge-http/src/handlers/mod.rs
```

Expected: at least one line declaring `pub mod semantic_drift;`. There may also be a `pub use semantic_drift::*;` re-export.

Use Edit to remove these declarations. Read the surrounding context first to construct precise `old_string` values.

- [ ] **Step 10: Repoint the route caller in http's lib.rs**

Find the current usage:

```bash
grep -nE 'handlers::semantic_drift|semantic_drift::' crates/mockforge-http/src/lib.rs
```

Expected: a small handful of references (route registrations, possibly an import block). The mockforge-http crate already depends on `mockforge-intelligence`, so the rewrite is just a path swap.

Use the Edit tool. The most common patterns:

**Pattern A — module-level import block**:
```
old_string: use crate::handlers::semantic_drift::*;
new_string: use mockforge_intelligence::handlers::semantic_drift::*;
```

**Pattern B — qualified call sites in route registration**:
```
old_string: crate::handlers::semantic_drift::route_handler
new_string: mockforge_intelligence::handlers::semantic_drift::route_handler
```

Apply whichever shape matches. There may be 1–5 occurrences depending on how `lib.rs` wires the routes. Use `perl -i -pe` if there are many identical small substitutions:

```bash
perl -i -pe 's{\bcrate::handlers::semantic_drift\b}{mockforge_intelligence::handlers::semantic_drift}g' crates/mockforge-http/src/lib.rs
```

Then verify:

```bash
grep -nE 'crate::handlers::semantic_drift|mockforge_intelligence::handlers::semantic_drift' crates/mockforge-http/src/lib.rs
```

Expected: only `mockforge_intelligence::handlers::semantic_drift` references remain. NO `crate::handlers::semantic_drift`.

- [ ] **Step 11: Audit `mockforge-http`'s `database` feature propagation**

The moved file has 7 `#[cfg(feature = "database")]` blocks. For them to compile when http enables the feature, `mockforge-intelligence/database` must also be enabled.

Read the current http feature config:

```bash
grep -A2 'database' crates/mockforge-http/Cargo.toml | head -20
grep -A2 'mockforge-intelligence' crates/mockforge-http/Cargo.toml | head -10
```

Look for the `[features]` block's `database = [...]` definition. If it does NOT already include `"mockforge-intelligence/database"`, add it. The typical syntax:

```toml
database = [
    "sqlx",
    "mockforge-intelligence/database",
]
```

Use the Edit tool with whatever the actual pre-image is. If the propagation is already in place (which may have been done in #611), no edit needed.

- [ ] **Step 12: Verify compile (default features)**

```bash
cargo check -p mockforge-intelligence -p mockforge-http --all-targets 2>&1 | tail -5
```

Expected: clean exit 0.

If `mockforge-intelligence::handlers::semantic_drift` fails to compile because of a missing module (e.g. `crate::incidents::semantic_manager` doesn't exist), surface as BLOCKED — the fact must have changed since planning.

- [ ] **Step 13: Verify compile WITH database feature**

```bash
cargo check -p mockforge-intelligence -p mockforge-http --all-targets --features mockforge-http/database 2>&1 | tail -10
```

Expected: clean exit 0. This exercises the 7 `#[cfg(feature = "database")]` blocks.

If it fails with "feature `database` does not exist for crate `mockforge-intelligence`" or similar, return to Step 11 and add the feature propagation.

- [ ] **Step 14: Run tests**

```bash
cargo test -p mockforge-intelligence -p mockforge-http --lib 2>&1 | tail -10
```

Expected: all existing tests pass.

- [ ] **Step 15: cargo clippy + fmt**

```bash
cargo clippy -p mockforge-intelligence -p mockforge-http --all-targets -- -D warnings 2>&1 | tail -10
cargo fmt --all --check
```

Expected: clippy clean for our crates (pre-existing desktop-app + cli failures unrelated). fmt clean.

If clippy fires `unused import` warnings on the moved file (e.g. an import that was needed in http but isn't needed in intelligence's compile environment), tidy them up — that's part of the move.

- [ ] **Step 16: Commit**

```bash
cd /mnt/projects/mockforge-worktrees/brainstorm-555-p3
git add -A
git commit -m "$(cat <<'EOF'
refactor(intelligence,http): move semantic_drift handler to mockforge-intelligence (#555 phase 3)

Mirrors #610's pattern for pr_generation. ADR 0001 marked semantic_drift
as INTELLIGENCE-bucket (428 LOC); all its `mockforge_core::*` imports
resolved to forwarding re-exports of types that actually live in
intelligence (since #562 phase 4 + #601) or foundation (since A6), so
the move just repoints those imports to their canonical homes.

## What changed

- `git mv crates/mockforge-http/src/handlers/semantic_drift.rs →
  crates/mockforge-intelligence/src/handlers/semantic_drift.rs`.
- Rewrote 2 top-of-file imports to intelligence-siblings:
  `mockforge_core::ai_contract_diff::*` → `crate::ai_contract_diff::*`
  `mockforge_core::incidents::semantic_manager::*` → `crate::incidents::semantic_manager::*`
- Rewrote 1 function-local import + 4 inline path references to source
  `IncidentStatus`/`IncidentSeverity` directly from foundation:
  `mockforge_core::incidents::types::*` → `mockforge_foundation::incidents_types::*`
- `crate::database::Database` (1 import) stays unchanged — `database` is
  a sibling module in intelligence post-#611.
- `mockforge-intelligence/src/handlers/mod.rs` gains `pub mod semantic_drift;`.
- `mockforge-http/src/handlers/mod.rs` drops the same declaration.
- `mockforge-http/src/lib.rs` route wiring repointed from
  `crate::handlers::semantic_drift::*` to
  `mockforge_intelligence::handlers::semantic_drift::*`.
- Possibly `mockforge-http/Cargo.toml`'s `database` feature gains
  `mockforge-intelligence/database` propagation if not already present.

## Why this handler

The original ADR pick (`behavioral_cloning`) is blocked by a
`ScenarioDefinition` cycle that intelligence cannot resolve without a
prior foundation-promotion. semantic_drift is the genuinely-unblocked
next intelligence-bucket move.

Cycle check: `mockforge-intelligence` still does not depend on
`mockforge-core` (per #562 phase 1). No new crate deps introduced.
EOF
)"
```

---

## Task 2: Workspace verify + push + PR

**Why now:** Belt-and-suspenders before push.

- [ ] **Step 1: Workspace clippy (warnings as errors)**

```bash
cd /mnt/projects/mockforge-worktrees/brainstorm-555-p3
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -30
```

Expected: clean for the crates this PR touches (`mockforge-intelligence`, `mockforge-http`). Pre-existing failures in `desktop-app` (deprecated Tauri APIs, dead_code, unused-variable) and `mockforge-cli` (`CARGO_BIN_EXE_mockforge` test-harness) are unrelated to this PR.

- [ ] **Step 2: Workspace tests**

```bash
cargo test --workspace --lib --bins 2>&1 | tail -20
```

Same caveat. Tests in `mockforge-intelligence` and `mockforge-http` should all pass.

- [ ] **Step 3: Final surface checks**

```bash
echo "=== semantic_drift now lives in intelligence ==="
ls crates/mockforge-intelligence/src/handlers/ 2>&1 | head

echo "=== semantic_drift gone from http ==="
ls crates/mockforge-http/src/handlers/semantic_drift.rs 2>&1 | head

echo "=== no remaining mockforge_core::* in the moved file ==="
grep -nE 'mockforge_core::' crates/mockforge-intelligence/src/handlers/semantic_drift.rs | head

echo "=== no remaining crate::handlers::semantic_drift in http ==="
git grep -nE 'crate::handlers::semantic_drift' -- crates/mockforge-http/ | head

echo "=== route caller uses intelligence path ==="
grep -nE 'mockforge_intelligence::handlers::semantic_drift' crates/mockforge-http/src/lib.rs | head

echo "=== handlers/mod.rs declarations updated ==="
grep -nE 'semantic_drift' crates/mockforge-intelligence/src/handlers/mod.rs crates/mockforge-http/src/handlers/mod.rs
```

Each expected:
- First listing: includes `semantic_drift.rs` alongside `pr_generation.rs` + `mod.rs`.
- Second listing: "No such file or directory" or similar.
- Third grep: empty.
- Fourth grep: empty.
- Fifth grep: at least one match (the repointed route caller).
- Sixth grep: 1 line in intelligence's mod.rs (`pub mod semantic_drift;`), 0 lines in http's.

If anything mismatches, surface as a finding.

- [ ] **Step 4: cargo fmt + final cargo check workspace**

```bash
cargo fmt --all --check
cargo check --workspace --all-targets 2>&1 | tail -5
```

Expected: both clean.

- [ ] **Step 5: Push the branch**

```bash
git push -u origin refactor/555-phase-3-semantic-drift
```

- [ ] **Step 6: Open the PR**

```bash
gh pr create --title "refactor(intelligence,http): move semantic_drift handler to mockforge-intelligence (#555 phase 3)" --body "$(cat <<'EOF'
## Summary

#555 phase 3. Mirrors the pattern from #610 (pr_generation) and #611 (database wrapper prereq). Moves `semantic_drift.rs` (428 LOC, INTELLIGENCE-bucket per ADR 0001) from `mockforge-http/src/handlers/` to `mockforge-intelligence/src/handlers/`.

## Why semantic_drift, not behavioral_cloning

ADR 0001 originally marked `behavioral_cloning.rs` as the next-after-`pr_generation` move. Exploration found behavioral_cloning's own source (lines 591–597) documents that it bridges `mockforge_intelligence::BehavioralSequence` ↔ `mockforge_core::scenarios::ScenarioDefinition`, and intelligence cannot depend on core (would re-create the cycle #562 phase 1 broke). The handler is blocked until `ScenarioDefinition` moves to foundation in a separate prereq.

`semantic_drift.rs` is genuinely unblocked: all its `mockforge_core::*` imports are forwarding re-exports of types that actually live in intelligence (since #562 phase 4 + #601) or foundation (since A6).

## What changed

- `git mv crates/mockforge-http/src/handlers/semantic_drift.rs → crates/mockforge-intelligence/src/handlers/semantic_drift.rs`. History preserved.
- 2 top-of-file imports rewritten to intelligence-siblings:
  - `mockforge_core::ai_contract_diff::*` → `crate::ai_contract_diff::*` (sibling in intelligence since #562 phase 4)
  - `mockforge_core::incidents::semantic_manager::*` → `crate::incidents::semantic_manager::*` (sibling since #601)
- 1 function-local import + 4 inline path references rewritten to source from foundation:
  - `mockforge_core::incidents::types::*` → `mockforge_foundation::incidents_types::*`
- `use crate::database::Database;` (line 24) unchanged — `database` is a sibling module in intelligence post-#611.
- `mockforge-intelligence/src/handlers/mod.rs` gains `pub mod semantic_drift;`.
- `mockforge-http/src/handlers/mod.rs` drops the same declaration.
- `mockforge-http/src/lib.rs` route wiring repointed to `mockforge_intelligence::handlers::semantic_drift::*`.
- If applicable: `mockforge-http/Cargo.toml`'s `database` feature gains `mockforge-intelligence/database` propagation (needed so the 7 `#[cfg(feature = "database")]` blocks in the moved file compile under http's `database` feature).

## Why this is safe

- **No new crate deps**: intelligence already had axum + sqlx + the destination modules.
- **No cycle**: `mockforge-intelligence` still does not depend on `mockforge-core` (per #562 phase 1).
- **No behavior change**: pure code motion + import rewrites pointing at the canonical homes of the same types.
- **Test plan exercises the database feature gate**: cargo check + tests with `--features mockforge-http/database` confirm the 7 feature-gated blocks compile.

## Out of scope (deferred to later phases)

The blocker map for the other intelligence-bucket handlers (compiled during this PR's planning):

| Handler | Blocker |
|---|---|
| `behavioral_cloning` (678 LOC) | `mockforge_core::scenarios::ScenarioDefinition` — needs foundation promotion |
| `consistency` (780 LOC) | `mockforge_core::consistency::ConsistencyEngine` |
| `contract_health` (373 LOC) | `mockforge_core::incidents::IncidentManager` (structural, stayed in core per #601) |
| `drift_budget` (784 LOC) | `mockforge_core::contract_drift::budget_engine::DriftBudgetEngine` |
| `failure_designer` (174 LOC) | `mockforge_chaos::*` (chaos → core dep direction) |
| `fidelity` (171 LOC) | `mockforge_core::fidelity::FidelityCalculator` |
| `incident_replay` (156 LOC) | `mockforge_chaos::*` |
| `risk_simulation` (168 LOC) | `crate::auth::risk_engine::RiskEngine` (http-internal, ADR splits to mockforge-auth) |
| `snapshot_diff` (490 LOC) | `crate::management::ManagementState` (http-internal) |
| `xray` (281 LOC) | `mockforge_core::consistency::ConsistencyEngine` |

Each will need its own targeted prereq similar to how #611 unblocked this PR.

## Test plan
- [x] `cargo check --workspace --all-targets`
- [x] `cargo check -p mockforge-intelligence -p mockforge-http --all-targets --features mockforge-http/database`
- [x] `cargo clippy --workspace --all-targets -- -D warnings` (modulo pre-existing desktop-app + cli failures)
- [x] `cargo test --workspace --lib --bins`
- [x] `cargo fmt --all --check`
- [x] No `mockforge_core::*` references in the moved file
- [x] No `crate::handlers::semantic_drift` references anywhere in http
EOF
)"
```

- [ ] **Step 7: Enable auto-merge**

```bash
PR_NUMBER=$(gh pr view --json number -q '.number')
gh pr merge "$PR_NUMBER" --auto --squash
gh pr view "$PR_NUMBER" --json autoMergeRequest,mergeStateStatus,state -q '{auto: .autoMergeRequest.mergeMethod, mergeState: .mergeStateStatus, state: .state}'
```

Expected: `{"auto":"SQUASH","mergeState":"BLOCKED","state":"OPEN"}` (or `"DIRTY"` briefly before CI re-evaluates).

- [ ] **Step 8: Final cleanup (after merge)**

Once the PR merges:

```bash
# Don't run until merge confirmed
git -C /mnt/projects/mockforge worktree remove /mnt/projects/mockforge-worktrees/brainstorm-555-p3
git -C /mnt/projects/mockforge branch -D refactor/555-phase-3-semantic-drift
```

---

## Notes for the executing agent

- **Pattern alignment**: This PR mirrors #610 (pr_generation extraction) and uses #611's database wrapper. If a step's exact pre-image doesn't match (because the file has shifted slightly since planning), the surrounding logic and the target final state are what matters. Use grep + sed/perl to find the actual line layout if the Edit pre-images need adjustment.
- **Database feature propagation (Step 11)**: this is the most likely point of friction. If `cargo check --features mockforge-http/database` fails in Step 13, the fix is one line in http's Cargo.toml's `[features]` block. Don't bail out before applying that fix.
- **Memory check: rustfmt qualifier collapse**. Run `cargo fmt --all` after qualifier changes — repo has been bitten before.
- **Memory check: pre-existing CI failures**. `desktop-app` clippy + `mockforge-cli` `CARGO_BIN_EXE_mockforge` test-harness — neither introduced by this PR, both auto-merge can land through.
- **No new tests**. Code motion only.
- **Audit, don't expand**: if any handler in the "Out of scope" map looks ALSO unblocked during execution (e.g. a forwarding-only `mockforge_core::*` chain like semantic_drift turned out to be), do NOT extract it in this PR. File a follow-up or surface as a finding. Each phase is one handler.
