# #555 Phase 3: Move `semantic_drift.rs` handler from mockforge-http to mockforge-intelligence

- **Status**: Approved
- **Date**: 2026-05-22
- **Issue**: [#555](https://github.com/SaaSy-Solutions/mockforge/issues/555)
- **Phase 1**: [#607](https://github.com/SaaSy-Solutions/mockforge/pull/607) — moved `proxy_server.rs` to `mockforge-proxy`.
- **Phase 2**: [#610](https://github.com/SaaSy-Solutions/mockforge/pull/610) — moved `pr_generation.rs` handler to `mockforge-intelligence`.
- **Prereq**: [#611](https://github.com/SaaSy-Solutions/mockforge/pull/611) — moved the database wrapper to `mockforge-intelligence`, which this PR depends on.

## Context

ADR 0001 (`docs/adr/0001-mockforge-http-extraction.md`) classifies 17 handlers in `mockforge-http/src/handlers/` as **INTELLIGENCE-bucket** — meant to move to `mockforge-intelligence::handlers`. #610 moved the first one (`pr_generation`, 146 LOC) as the "smallest cleanest first axum-into-intelligence move." This PR moves the next one.

### Why semantic_drift over behavioral_cloning

The brainstorming session started by targeting `behavioral_cloning.rs` (ADR called it "Unblocked — can move now"). Exploration found that ADR's claim is no longer accurate: `behavioral_cloning.rs` lines 591–597 explicitly document that the handler bridges `mockforge_intelligence::behavioral_cloning::BehavioralSequence` and `mockforge_core::scenarios::ScenarioDefinition`, and that intelligence cannot depend on core (would break #562 phase 1's cycle break). Same pattern blocks several other intelligence-bucket handlers:

| Handler | LOC | Blocker |
|---|---:|---|
| `behavioral_cloning.rs` | 678 | `mockforge_core::scenarios::ScenarioDefinition` |
| `consistency.rs` | 780 | `mockforge_core::consistency::ConsistencyEngine` |
| `contract_health.rs` | 373 | `mockforge_core::incidents::IncidentManager` (structural pieces stayed in core per #601) |
| `drift_budget.rs` | 784 | `mockforge_core::contract_drift::budget_engine::DriftBudgetEngine` |
| `failure_designer.rs` | 174 | `mockforge_chaos::*` (chaos → core dep direction) |
| `fidelity.rs` | 171 | `mockforge_core::fidelity::FidelityCalculator` |
| `incident_replay.rs` | 156 | `mockforge_chaos::*` |
| `risk_simulation.rs` | 168 | `crate::auth::risk_engine::RiskEngine` (http-internal, ADR splits to mockforge-auth) |
| `snapshot_diff.rs` | 490 | `crate::management::ManagementState` (http-internal) |
| `xray.rs` | 281 | `mockforge_core::consistency::ConsistencyEngine` |

**`semantic_drift.rs` (428 LOC) is genuinely unblocked**. All its "mockforge_core::*" imports resolve to forwarding re-exports of types that actually live in intelligence (since #562 phase 4 + #601) or foundation (since A6):

- `mockforge_core::ai_contract_diff::{ContractDiffAnalyzer, ContractDiffConfig}` → actual home: `mockforge-intelligence::ai_contract_diff` (#562 phase 4).
- `mockforge_core::incidents::semantic_manager::{SemanticIncident, SemanticIncidentManager}` → actual home: `mockforge-intelligence::incidents::semantic_manager` (#601).
- `mockforge_core::incidents::types::{IncidentSeverity, IncidentStatus}` → actual home: `mockforge_foundation::incidents_types` (A6).
- `crate::database::Database` → actual home: `mockforge-intelligence::database` (#611).
- `mockforge_openapi::OpenApiSpec` → intelligence already depends on `mockforge-openapi`.

After the move, every import resolves either to a sibling module in intelligence (`crate::ai_contract_diff`, `crate::incidents::semantic_manager`, `crate::database`) or to a foundation dep that intelligence already has.

## Decision

Mirror #610's pattern exactly. Single PR, single commit (plus the spec + plan docs commits).

## Target architecture

```
mockforge-intelligence
  └── src/
      ├── ai_contract_diff/                (existing, since #562 phase 4)
      ├── database.rs                      (existing, since #611)
      ├── incidents/
      │   └── semantic_manager.rs          (existing, since #601)
      └── handlers/
          ├── mod.rs                       (existing, since #610)
          ├── pr_generation.rs             (existing, since #610)
          └── semantic_drift.rs            ← MOVED HERE

mockforge-http
  └── src/
      ├── handlers/
      │   └── (semantic_drift.rs deleted)
      ├── lib.rs                           (route wiring updated to import from intelligence)
      └── handlers/mod.rs                  (pub mod semantic_drift declaration removed)
```

**Dep direction**: unchanged. `mockforge-http → mockforge-intelligence → mockforge-foundation`. No new crate deps.

## Components

### 1. Move the file

```bash
git mv crates/mockforge-http/src/handlers/semantic_drift.rs \
       crates/mockforge-intelligence/src/handlers/semantic_drift.rs
```

History preserved via `git mv`.

### 2. Rewrite imports in the moved file

Use the Edit tool with these substitutions. Read the file first to confirm exact line positions before editing.

**a) Top-of-file imports (lines ~13–14)**

```
old_string: use mockforge_core::ai_contract_diff::{ContractDiffAnalyzer, ContractDiffConfig};
use mockforge_core::incidents::semantic_manager::{SemanticIncident, SemanticIncidentManager};
new_string: use crate::ai_contract_diff::{ContractDiffAnalyzer, ContractDiffConfig};
use crate::incidents::semantic_manager::{SemanticIncident, SemanticIncidentManager};
```

`crate::database::Database` (line 24) stays as-is — `database` is a sibling module in intelligence post-#611.

**b) Function-local `use` (around line 31)**

```
old_string: use mockforge_core::incidents::types::{IncidentSeverity, IncidentStatus};
new_string: use mockforge_foundation::incidents_types::{IncidentSeverity, IncidentStatus};
```

**c) Inline path references (lines 220–223)**

```
old_string: "open" => Some(mockforge_core::incidents::types::IncidentStatus::Open),
"acknowledged" => Some(mockforge_core::incidents::types::IncidentStatus::Acknowledged),
"resolved" => Some(mockforge_core::incidents::types::IncidentStatus::Resolved),
"closed" => Some(mockforge_core::incidents::types::IncidentStatus::Closed),
new_string: "open" => Some(mockforge_foundation::incidents_types::IncidentStatus::Open),
"acknowledged" => Some(mockforge_foundation::incidents_types::IncidentStatus::Acknowledged),
"resolved" => Some(mockforge_foundation::incidents_types::IncidentStatus::Resolved),
"closed" => Some(mockforge_foundation::incidents_types::IncidentStatus::Closed),
```

Alternatively, use the function-local `use` from (b) and write bare `IncidentStatus::Open` etc. Either works; pick whichever matches the surrounding style.

### 3. Declare the new module in intelligence

`crates/mockforge-intelligence/src/handlers/mod.rs` — add a `pub mod semantic_drift;` line. Preserve alphabetical ordering with `pr_generation`.

```
old_string: pub mod pr_generation;
new_string: pub mod pr_generation;
pub mod semantic_drift;
```

### 4. Drop the module declaration in http

`crates/mockforge-http/src/handlers/mod.rs` — drop the `pub mod semantic_drift;` line and any corresponding `pub use semantic_drift::*` re-export if present.

### 5. Update the route caller in http

`crates/mockforge-http/src/lib.rs` (1 external caller — verified via `git grep -lE 'handlers::semantic_drift' -- crates/` returns only the http lib.rs).

Replace any local-handler references (`crate::handlers::semantic_drift::*` or `handlers::semantic_drift::*`) with `mockforge_intelligence::handlers::semantic_drift::*`. Read the file's existing usage pattern before editing — likely a small import block plus a `Router::new().route(...)` chain.

### 6. Feature-gate continuity

The moved file has 7 `#[cfg(feature = "database")]` blocks. Both `mockforge-intelligence` and `mockforge-http` already have a `database` feature (from #611). The handler's compile-time behavior must be preserved:

- When `mockforge-http::Cargo.toml`'s `database` feature is enabled, `mockforge-intelligence`'s `database` feature must also be enabled.
- Verify by inspecting whether `mockforge-http::Cargo.toml`'s `database = [...]` feature already enables `mockforge-intelligence/database` (likely needs adding).

Audit during execution; the #610/#611 commits established a pattern.

## Testing strategy

Code motion + import rewrite. No new tests; existing tests are the safety net.

**Compile-time gates**
- `cargo check -p mockforge-intelligence -p mockforge-http --all-targets` — clean.
- `cargo check -p mockforge-intelligence -p mockforge-http --all-targets --features mockforge-http/database` — clean (exercises the 7 feature-gated blocks).
- `cargo clippy --workspace --all-targets -- -D warnings` — clean (modulo pre-existing desktop-app + cli failures unrelated to this PR).
- `cargo fmt --all --check` — clean.

**Per-crate tests**
- `cargo test -p mockforge-intelligence --lib` — verify the moved handler still compiles under intelligence.
- `cargo test -p mockforge-http --lib` — verify the route wiring still works after the caller update.

**Surface checks**
- `git grep -nE 'handlers::semantic_drift' -- crates/mockforge-http/` should return references via `mockforge_intelligence::handlers::semantic_drift::*` only, not `crate::handlers::semantic_drift::*`.
- `git grep -nE 'mockforge_core::ai_contract_diff|mockforge_core::incidents::semantic_manager' -- crates/mockforge-intelligence/src/handlers/semantic_drift.rs` — empty (we rewrote those).

## Risks

- **Feature-gate propagation**: if `mockforge-http::Cargo.toml`'s `database` feature doesn't already enable `mockforge-intelligence/database`, the `cargo check --features mockforge-http/database` step will fail with a compile error. The fix is one-line in `mockforge-http/Cargo.toml`'s `[features]` table:

  ```toml
  database = ["mockforge-intelligence/database"]
  ```

  This is the same propagation pattern #611 established when it moved the database wrapper.

- **Hidden caller**: if `git grep -lE 'handlers::semantic_drift' -- crates/` returns matches outside `mockforge-http/src/lib.rs` (e.g. a test file, another crate), repoint those too. Verified during planning that only `lib.rs` is the caller, but defensive verification at execution time is cheap.

- **`mockforge_core::*` shim paths still resolve post-move?**: in `mockforge-http` itself, callers can still write `mockforge_core::ai_contract_diff::*` because core continues to forward-re-export. This PR doesn't touch those shims. The change is scoped to the semantic_drift file + its mod-decl + route caller.

## Out of scope

- Promoting `ScenarioDefinition` to foundation (would unblock `behavioral_cloning` for a future phase) — separate concern.
- Splitting the http-internal `crate::auth::*` and `crate::management::*` modules into `mockforge-auth` + their own crates (would unblock `risk_simulation` + `snapshot_diff`) — ADR Phase E.
- Other INTELLIGENCE-bucket handler moves — each will need its own targeted prereq similar to how #611 unblocked this one. The graph of remaining handlers' blockers is now well-mapped in this spec for future use.

## Approval

Pivot from `behavioral_cloning` to `semantic_drift` (genuine unblock status). Confirmed via brainstorming session 2026-05-22.
