# post-#604 cleanups Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the stale comment + mixed imports in `mockforge-http/src/handlers/protocol_contracts.rs` and promote `tempfile`, `bytes` to workspace deps + switch existing literal `jsonschema`/`tempfile`/`bytes` pins to `{ workspace = true }` where versions already match.

**Architecture:** Three commits. (1) HTTP handler cleanup (one file). (2) Workspace dep promotion: add `tempfile = "3.27"` and `bytes = "1.5"` to root `Cargo.toml`'s `[workspace.dependencies]`, then switch per-crate pins to `{ workspace = true }` across jsonschema (3 crates), tempfile (4–5 crates), bytes (3 crates). (3) Workspace verification + push + PR.

**Tech Stack:** Rust 2021 workspace, cargo, Edit tool. No new code; only Cargo.toml + one .rs file's header.

**Spec:** `docs/superpowers/specs/2026-05-22-post-604-cleanups-design.md`
**Issue:** [#609](https://github.com/SaaSy-Solutions/mockforge/issues/609)
**Branch:** `refactor/post-604-cleanups` (already created in worktree `/mnt/projects/mockforge-worktrees/brainstorm-609`; spec committed there).
**Worktree:** `/mnt/projects/mockforge-worktrees/brainstorm-609`. Run every command from this directory.

**Pre-flight facts (verified during planning):**
- Root `Cargo.toml` line 218 already has `jsonschema = "0.33"` in `[workspace.dependencies]`.
- `mockforge-bench/Cargo.toml` and `mockforge-openapi/Cargo.toml` already use `jsonschema = { workspace = true }`. `mockforge-contracts`, `mockforge-data`, `mockforge-core` still pin literal versions.
- `tempfile` is in 5 crates total: `mockforge-amqp`, `mockforge-contracts`, `mockforge-plugin-egress`, `mockforge-plugin-host` (2 occurrences). Versions: 4 at `"3"`, contracts at `"3.27"`.
- `bytes` is in 5 crates: `mockforge-tunnel`, `mockforge-contracts`, `mockforge-tcp`, `mockforge-core`, `mockforge-collab`. Versions: 3 at `"1.5"`, tcp at `"1.0"`, collab at `{ version = "1", optional = true }`.
- `prost-reflect` is in 3 crates: `mockforge-grpc` at `"0.14"`, `mockforge-core` + `mockforge-contracts` at `"0.16"`. **Skip** — out of scope.
- HTTP handler current state at `crates/mockforge-http/src/handlers/protocol_contracts.rs` lines 5–9:
  ```
  // Per-protocol contract impls (GrpcContract, KafkaContract, MqttContract,
  // WebSocketContract, ProtocolContractRegistry) stay in mockforge-core because
  // they hold compiled jsonschema validators. Allow here until a future phase
  // extracts them to mockforge-contracts.
  #![allow(deprecated)]
  ```
- HTTP handler imports (lines ~17–22): mixed — `compare_contracts` + `ProtocolContractRegistry` come from `mockforge_contracts::contract_drift::protocol_contracts::*`; the 4 contract impls come from `mockforge_core::contract_drift::*` (through forwarding re-exports added in #608).

---

## File Structure

**Files modified:**
- `crates/mockforge-http/src/handlers/protocol_contracts.rs` — delete stale comment block; maybe drop `#![allow(deprecated)]`; collapse mixed imports into a single `mockforge_contracts::contract_drift::*` block
- `Cargo.toml` (root) — add 2 lines to `[workspace.dependencies]`
- `crates/mockforge-contracts/Cargo.toml` — 3 deps switch to `{ workspace = true }` (jsonschema, tempfile, bytes)
- `crates/mockforge-data/Cargo.toml` — 1 dep switch (jsonschema)
- `crates/mockforge-core/Cargo.toml` — 3 deps switch (jsonschema, bytes; tempfile not declared there)
- `crates/mockforge-amqp/Cargo.toml` — 1 dep switch (tempfile)
- `crates/mockforge-plugin-egress/Cargo.toml` — 1 dep switch (tempfile)
- `crates/mockforge-plugin-host/Cargo.toml` — 2 dep switches (tempfile twice)
- `crates/mockforge-tunnel/Cargo.toml` — 1 dep switch (bytes)

**Files NOT touched** (out of scope, documented in spec):
- `crates/mockforge-tcp/Cargo.toml` (bytes = "1.0" — different version)
- `crates/mockforge-collab/Cargo.toml` (bytes optional shape)
- `crates/mockforge-grpc/Cargo.toml` (prost-reflect = "0.14")

---

## Task 1: HTTP handler cleanup

**Why first:** Small, isolated, single-file change. Safe baseline for the larger Cargo.toml changes in Task 2.

**Files:**
- Modify: `crates/mockforge-http/src/handlers/protocol_contracts.rs`

- [ ] **Step 1: Read the current file header**

```bash
cd /mnt/projects/mockforge-worktrees/brainstorm-609
sed -n '1,30p' crates/mockforge-http/src/handlers/protocol_contracts.rs
```

Expected (lines 1–30 should match what the spec recorded; verify the line numbers before editing):
- Lines 1–3: `//!` docstring.
- Lines 5–8: 4-line stale comment block about "stay in mockforge-core".
- Line 9: `#![allow(deprecated)]`.
- Lines ~11–14: `use axum::*` block.
- Lines ~16–18: `use mockforge_contracts::contract_drift::protocol_contracts::{compare_contracts, ProtocolContractRegistry};`.
- Lines ~19–21: `use mockforge_core::contract_drift::{GrpcContract, KafkaContract, MqttContract, WebSocketContract};`.

- [ ] **Step 2: Identify whether `#![allow(deprecated)]` is still needed**

Temporarily remove the attribute and run clippy to see if anything fires. Use Edit tool:

```
old_string: // Per-protocol contract impls (GrpcContract, KafkaContract, MqttContract,
// WebSocketContract, ProtocolContractRegistry) stay in mockforge-core because
// they hold compiled jsonschema validators. Allow here until a future phase
// extracts them to mockforge-contracts.
#![allow(deprecated)]

use axum::{
new_string: use axum::{
```

(Removes both the stale comment AND the `#![allow(deprecated)]` attribute. Restored conditionally in Step 4 if clippy fires.)

- [ ] **Step 3: Run clippy on mockforge-http to see if any deprecation warnings surface**

```bash
cargo clippy -p mockforge-http --lib -- -D warnings 2>&1 | tail -20
```

- If exit 0 with no deprecation warnings: continue to Step 4 (the attribute was orphaned; the deletion was correct).
- If clippy fires on a deprecated symbol: note the specific symbol, restore `#![allow(deprecated)]` with a one-line comment naming the symbol (Step 4 has the conditional path).

- [ ] **Step 4 (conditional): Restore `#![allow(deprecated)]` only if Step 3 surfaced a warning**

If Step 3 surfaced something like `warning: use of deprecated function 'foo::bar'`, restore the attribute with a precise comment. Use Edit tool:

```
old_string: use axum::{
new_string: // Required: <foo::bar> is currently deprecated but still used by the handler.
//            Track for replacement when its non-deprecated successor lands.
#![allow(deprecated)]

use axum::{
```

Replace `<foo::bar>` with the actual deprecated symbol from clippy's output. If multiple symbols, list them.

If Step 3 did NOT surface a warning, skip this step entirely.

- [ ] **Step 5: Repoint the mixed imports for consistency**

Find the current two-block import shape:

```bash
sed -n '15,25p' crates/mockforge-http/src/handlers/protocol_contracts.rs
```

The current state (after Step 2) should now show the imports starting around line 11–14 since we removed lines 5–9. Adjust line numbers.

Use Edit tool to collapse the two `mockforge_*::contract_drift::*` blocks into one:

```
old_string: use mockforge_contracts::contract_drift::protocol_contracts::{
    compare_contracts, ProtocolContractRegistry,
};
use mockforge_core::contract_drift::{
    GrpcContract, KafkaContract, MqttContract, WebSocketContract,
};
new_string: use mockforge_contracts::contract_drift::{
    compare_contracts, GrpcContract, KafkaContract, MqttContract,
    ProtocolContractRegistry, WebSocketContract,
};
```

(`mockforge_contracts::contract_drift::mod.rs`'s `pub use` block should re-export all 6 items at the top level, so the collapsed form works. If clippy/cargo errors with "not found", check the actual re-export shape in mod.rs and adjust the paths — e.g., maybe `compare_contracts` needs `mockforge_contracts::contract_drift::protocol_contracts::compare_contracts` explicitly.)

- [ ] **Step 6: Verify compile + clippy + format**

```bash
cargo check -p mockforge-http --lib
cargo clippy -p mockforge-http --all-targets -- -D warnings
cargo fmt --all --check
```

Expected: all three exit 0.

- [ ] **Step 7: Commit**

```bash
cd /mnt/projects/mockforge-worktrees/brainstorm-609
git add crates/mockforge-http/src/handlers/protocol_contracts.rs
git commit -m "$(cat <<'EOF'
refactor(http): post-#604 handler cleanup — drop stale comment, unify imports (#609)

The file-level comment block at lines 5–9 was stale post-#608 (it
claimed `GrpcContract`/`KafkaContract`/`MqttContract`/`WebSocketContract`
"stay in mockforge-core", but #608 moved them to mockforge-contracts).
Also collapsed the mixed two-block import (some symbols pulled from
mockforge-contracts directly, others from mockforge-core's forwarding
re-exports) into a single mockforge-contracts import for consistency.

#![allow(deprecated)] was [retained because <symbol> is still used / removed because no deprecation warnings surfaced] — pick during execution.
EOF
)"
```

Adjust the commit message's bracketed text based on Step 3's outcome.

---

## Task 2: Workspace dep promotion

**Why second:** Cargo.toml changes are mechanical but touch many files. Doing them in one task keeps the diff coherent and easier to review.

**Files:**
- Modify: `Cargo.toml` (root) — add 2 deps to `[workspace.dependencies]`
- Modify: 7 per-crate `Cargo.toml` files — switch literal pins to `{ workspace = true }`

- [ ] **Step 1: Add `tempfile` + `bytes` to root `[workspace.dependencies]`**

Read the current state:

```bash
cd /mnt/projects/mockforge-worktrees/brainstorm-609
grep -nE '^(tempfile|bytes|jsonschema|hex|itertools)' Cargo.toml | head
```

Confirm: `jsonschema = "0.33"` is at line 218. `tempfile` and `bytes` are NOT in workspace.deps.

Add `tempfile = "3.27"` and `bytes = "1.5"` to the `[workspace.dependencies]` block. Place them near other utility deps (e.g., grouped with `hex`, `itertools`, etc.). Use Edit tool — find an appropriate anchor line and insert after it.

For example, if the existing block has:

```
hex = "0.4"
itertools = "0.14"
```

Transform to:

```
hex = "0.4"
itertools = "0.14"
tempfile = "3.27"
bytes = "1.5"
```

Read the surrounding block before crafting the Edit pre-image — the exact lines to anchor on depend on the file's current layout.

- [ ] **Step 2: Verify workspace deps added correctly**

```bash
grep -nE '^(tempfile|bytes|jsonschema|hex|itertools)' Cargo.toml | head
```

Expected: all 5 deps appear in the same `[workspace.dependencies]` block.

- [ ] **Step 3: Switch `jsonschema` to `{ workspace = true }` in 3 crates**

For each of these 3 files, use Edit tool to replace the literal pin with the workspace ref:

**`crates/mockforge-contracts/Cargo.toml`:**
```
old_string: jsonschema = "0.33.0"
new_string: jsonschema = { workspace = true }
```

**`crates/mockforge-data/Cargo.toml`:**
```
old_string: jsonschema = "0.33"
new_string: jsonschema = { workspace = true }
```

**`crates/mockforge-core/Cargo.toml`:**
```
old_string: jsonschema = "0.33.0"
new_string: jsonschema = { workspace = true }
```

- [ ] **Step 4: Switch `tempfile` to `{ workspace = true }` in 4 crates**

**`crates/mockforge-amqp/Cargo.toml`:**
```
old_string: tempfile = "3"
new_string: tempfile = { workspace = true }
```

**`crates/mockforge-contracts/Cargo.toml`:**
```
old_string: tempfile = "3.27"
new_string: tempfile = { workspace = true }
```

**`crates/mockforge-plugin-egress/Cargo.toml`:**
```
old_string: tempfile = "3"
new_string: tempfile = { workspace = true }
```

**`crates/mockforge-plugin-host/Cargo.toml`:** has 2 occurrences of `tempfile = "3"`. Use Edit with `replace_all=true`:
```
old_string: tempfile = "3"
new_string: tempfile = { workspace = true }
replace_all: true
```

- [ ] **Step 5: Switch `bytes` to `{ workspace = true }` in 3 crates**

**`crates/mockforge-tunnel/Cargo.toml`:**
```
old_string: bytes = "1.5"
new_string: bytes = { workspace = true }
```

**`crates/mockforge-contracts/Cargo.toml`:**
```
old_string: bytes = "1.5"
new_string: bytes = { workspace = true }
```

**`crates/mockforge-core/Cargo.toml`:**
```
old_string: bytes = "1.5"
new_string: bytes = { workspace = true }
```

**Do NOT touch** `crates/mockforge-tcp/Cargo.toml` (`bytes = "1.0"`) or `crates/mockforge-collab/Cargo.toml` (optional shape).

- [ ] **Step 6: Audit for any tempfile/bytes occurrences I missed**

```bash
cd /mnt/projects/mockforge-worktrees/brainstorm-609
grep -rnE '^(tempfile|bytes) = "' crates/*/Cargo.toml
```

Expected output (after Step 4 + Step 5):
- `crates/mockforge-tcp/Cargo.toml:NN:bytes = "1.0"` (intentionally not switched)

If anything ELSE matches (e.g., a `tempfile = "3"` not switched), apply the same `{ workspace = true }` switch to it. If a NEW literal pin appears in some other crate's Cargo.toml that wasn't audited during planning, flag it but apply the switch — same-version literal-pin promotions are safe.

- [ ] **Step 7: Verify compile + workspace clippy**

```bash
cargo check --workspace --all-targets 2>&1 | tail -10
```

Expected: clean exit 0.

```bash
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -20
```

Expected: clean for the crates this PR touches (workspace + all crates that switched). Pre-existing desktop-app + cli failures unrelated.

- [ ] **Step 8: Cargo.lock spot-check**

```bash
git diff Cargo.lock 2>&1 | head -30
```

Most expected: Cargo.lock unchanged (versions resolve to same Concrete values). If it DOES change, inspect to confirm the changes are sane (e.g., a `tempfile 3.27.0` → still `tempfile 3.27.0`, just sourced via workspace ref). If a major-version bump appears that shouldn't, surface as a finding.

- [ ] **Step 9: Verify dep-tree dedup**

```bash
cargo tree -d 2>&1 | grep -E '^(tempfile|bytes|jsonschema)\b' | head -10
```

Expected: NO matches (workspace promotion deduplicates). If something matches, the version mismatch is real (e.g., bytes still showing 1.0 from tcp) — that's expected since we deliberately skipped tcp.

- [ ] **Step 10: cargo fmt**

```bash
cargo fmt --all --check 2>&1 | tail -3
```

Expected: clean.

- [ ] **Step 11: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
chore(deps): promote tempfile + bytes to workspace deps; unify jsonschema (#609)

Promotes three already-duplicated deps to the workspace
`[workspace.dependencies]` table where versions already matched
across crates:

- `tempfile = "3.27"` — switched 5 occurrences across mockforge-amqp,
  mockforge-contracts, mockforge-plugin-egress, mockforge-plugin-host (2x)
- `bytes = "1.5"` — switched 3 occurrences across mockforge-tunnel,
  mockforge-contracts, mockforge-core
- `jsonschema` — already in workspace deps at "0.33"; switched 3
  literal-pin holdouts (mockforge-contracts, mockforge-data,
  mockforge-core) to `{ workspace = true }`

Skipped (separate version-mismatch concerns; tracked as #609 out-of-scope):
- `mockforge-tcp::bytes` at "1.0" (vs 1.5 elsewhere)
- `mockforge-collab::bytes` at `{ version = "1", optional = true }` (shape mismatch)
- `mockforge-grpc::prost-reflect` at "0.14" (vs 0.16 in core/contracts)

Cargo.lock should be unchanged or only superficially affected; verified
`cargo tree -d` no longer shows duplicate copies of the promoted deps.
EOF
)"
```

---

## Task 3: Workspace verify + push + PR

**Why now:** Belt-and-suspenders. Per-task verification ran already; a workspace-wide final pass catches anything that slipped between crates.

- [ ] **Step 1: Final workspace clippy**

```bash
cd /mnt/projects/mockforge-worktrees/brainstorm-609
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -20
```

Expected: clean for affected crates. Pre-existing desktop-app + cli failures unrelated.

- [ ] **Step 2: Workspace tests**

```bash
cargo test --workspace --lib --bins 2>&1 | tail -10
```

Expected: all tests pass for crates touched. Pre-existing cli `CARGO_BIN_EXE_mockforge` test-harness failures acceptable.

- [ ] **Step 3: Final greps**

```bash
echo "=== no leftover literal jsonschema/tempfile/bytes pins where workspace would be cleaner ==="
grep -rnE '^(jsonschema|tempfile|bytes) = "' crates/*/Cargo.toml

echo "=== HTTP handler no longer has stale comment ==="
grep -nE 'stay in mockforge-core' crates/mockforge-http/src/handlers/protocol_contracts.rs

echo "=== HTTP handler imports are unified ==="
grep -nE '^use mockforge_(core|contracts)::contract_drift' crates/mockforge-http/src/handlers/protocol_contracts.rs
```

Expected:
- First grep: only `mockforge-tcp/Cargo.toml:bytes = "1.0"` (intentionally skipped). Anything ELSE is a finding.
- Second grep: empty.
- Third grep: at most one `use mockforge_contracts::contract_drift::*` line (or `use mockforge_contracts::contract_drift::protocol_contracts::*` if the collapsed form needed adjustment).

- [ ] **Step 4: Push the branch**

```bash
git push -u origin refactor/post-604-cleanups
```

- [ ] **Step 5: Open the PR**

```bash
gh pr create --title "refactor: post-#604 cleanups — stale comment + workspace deps (#609)" --body "$(cat <<'EOF'
## Summary

Closes #609. Two small cleanups flagged by the final holistic review of #608:

1. **HTTP handler stale comment + mixed imports** — `mockforge-http/src/handlers/protocol_contracts.rs`'s file-level comment claimed the protocol-contract impls "stay in mockforge-core" but #608 moved them to mockforge-contracts. Comment removed. Imports were also mixed (some from mockforge-contracts directly, some via mockforge-core's forwarding re-exports); collapsed into a single mockforge-contracts import.

2. **Workspace dep promotion** — three deps were pinned at matching versions in multiple crates without using `[workspace.dependencies]`. Promoted `tempfile = "3.27"` and `bytes = "1.5"` to workspace deps; switched matching literal-pin crates (including the 3 `jsonschema` holdouts) to `{ workspace = true }`.

## What changed

### Commit 1 — HTTP handler cleanup
- Deleted 4-line stale comment in `mockforge-http/src/handlers/protocol_contracts.rs:5-8`.
- `#![allow(deprecated)]` [removed because no deprecation warnings surfaced after clippy / kept because <symbol> is still in use — fill in during execution].
- Collapsed two-block `use mockforge_*::contract_drift::*` import into a single `use mockforge_contracts::contract_drift::*` block.

### Commit 2 — Workspace dep promotion
- Added `tempfile = "3.27"` + `bytes = "1.5"` to root `Cargo.toml`'s `[workspace.dependencies]`.
- Switched 5 `tempfile` occurrences across mockforge-amqp, mockforge-contracts, mockforge-plugin-egress, mockforge-plugin-host (×2) to `{ workspace = true }`.
- Switched 3 `bytes` occurrences in mockforge-tunnel, mockforge-contracts, mockforge-core to `{ workspace = true }`.
- Switched 3 `jsonschema` literal-pin holdouts (mockforge-contracts, mockforge-data, mockforge-core) to `{ workspace = true }`.

## Skipped (deferred follow-ups)

- **`prost-reflect`**: `mockforge-grpc` pins `0.14` but core/contracts use `0.16`. Bumping grpc has potential API-break risk — needs its own audit.
- **`mockforge-tcp::bytes`** at `"1.0"` (vs workspace's 1.5): bumping should be safe semver-compatible, but it's a deliberate change worth its own PR.
- **`mockforge-collab::bytes`** at `{ version = "1", optional = true }`: requires figuring out the workspace optional-dep syntax. Out of scope.

## Test plan
- [x] `cargo check --workspace --all-targets`
- [x] `cargo clippy --workspace --all-targets -- -D warnings` (modulo pre-existing desktop-app + cli failures unrelated to this PR)
- [x] `cargo test --workspace --lib --bins`
- [x] `cargo fmt --all --check`
- [x] `cargo tree -d` shows fewer duplicates of the promoted deps
- [x] No leftover stale `mockforge_core::contract_drift::*` reference in the http handler

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
git -C /mnt/projects/mockforge worktree remove /mnt/projects/mockforge-worktrees/brainstorm-609
git -C /mnt/projects/mockforge branch -D refactor/post-604-cleanups
```

---

## Notes for the executing agent

- **`#![allow(deprecated)]` audit is the only "judgment" moment in this PR.** Everything else is mechanical. If clippy fires on a deprecated symbol after the attribute is removed in Task 1 Step 2, restore it with an explanatory comment naming the symbol — don't leave it unexplained.
- **The collapsed import in Task 1 Step 5 assumes `mockforge_contracts::contract_drift::*` re-exports the 6 needed symbols.** If clippy/cargo complains about a missing path, check `crates/mockforge-contracts/src/contract_drift/mod.rs`'s `pub use` block and adjust — e.g., if `compare_contracts` is only at `mockforge_contracts::contract_drift::protocol_contracts::compare_contracts`, the collapsed import block needs to either split or use the longer path.
- **Memory check: rustfmt qualifier collapse.** Run `cargo fmt --all` after any qualifier changes — repo has been bitten before.
- **No new tests.** Pure cleanup. Existing tests are the safety net.
- **Pre-existing CI failures.** desktop-app clippy warnings + mockforge-cli `CARGO_BIN_EXE_mockforge` test-harness failures are not introduced by this PR and don't block auto-merge.
