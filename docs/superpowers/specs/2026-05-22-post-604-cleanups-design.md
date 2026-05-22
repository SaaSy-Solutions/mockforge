# Post-#604 cleanups: stale http handler comment + workspace dep promotion

- **Status**: Approved
- **Date**: 2026-05-22
- **Issue**: [#609](https://github.com/SaaSy-Solutions/mockforge/issues/609)
- **Follows**: [#608](https://github.com/SaaSy-Solutions/mockforge/pull/608) — both #609 items are minor artifacts of #604's migration that the final holistic review flagged as out-of-scope follow-ups.

## Context

Two small cleanup items surfaced during #608's final review:

1. **`mockforge-http/src/handlers/protocol_contracts.rs` has a stale comment block** (lines 5–9) that claims `GrpcContract` / `KafkaContract` / `MqttContract` / `WebSocketContract` / `ProtocolContractRegistry` "stay in `mockforge-core` because they hold compiled jsonschema validators" — but #608 moved all of those types to `mockforge-contracts`. The file's imports are also inconsistent: `compare_contracts` + `ProtocolContractRegistry` come from `mockforge_contracts::contract_drift::protocol_contracts::*` directly, while the 4 contract impls come from `mockforge_core::contract_drift::*` (through #608's forwarding re-exports).

2. **Several deps are pinned at the same version in multiple crates without using the workspace `[workspace.dependencies]` table.** Specifically `tempfile`, `bytes`, `jsonschema`. (#608 added `prost-reflect`, `bytes`, `tempfile` to `mockforge-contracts/Cargo.toml` matching `mockforge-core`'s versions.) Audit findings:

| Dep | Versions across crates | Promote action |
|---|---|---|
| `jsonschema` | already workspace-deps at `"0.33"`; 3 crates (`mockforge-contracts`, `mockforge-data`, `mockforge-core`) still pin literal `"0.33"`/`"0.33.0"` | **Yes** — switch the 3 holdouts to `{ workspace = true }` |
| `tempfile` | 4 crates at loose `"3"`, `mockforge-contracts` at pinned `"3.27"` | **Yes** — add `tempfile = "3.27"` to workspace deps; switch all 5 crates to `{ workspace = true }` |
| `bytes` | 3 crates at `"1.5"` (`mockforge-tunnel`, `mockforge-contracts`, `mockforge-core`); `mockforge-tcp` at `"1.0"`; `mockforge-collab` at `{ version = "1", optional = true }` | **Partial** — add `bytes = "1.5"` to workspace deps; switch only the 3 matching crates. Skip tcp (different major-compatible version) + collab (different shape with optional) |
| `prost-reflect` | `mockforge-core` + `mockforge-contracts` at `"0.16"`; `mockforge-grpc` at `"0.14"` | **Skip** — bumping grpc from 0.14 → 0.16 is a separate change with potential API-break risk. Track as a follow-up. |

## Decision

Single PR with two coupled cleanups:

1. **HTTP handler cleanup**: rewrite the stale comment block, repoint the 4 mixed imports for consistency, and audit whether the `#![allow(deprecated)]` attribute is still needed.
2. **Workspace dep promotion**: promote `tempfile` + `bytes` to workspace deps. Switch existing literal `jsonschema` / `tempfile` / `bytes` declarations to `{ workspace = true }` where versions match. Skip `prost-reflect` + tcp + collab outliers; document them as follow-ups.

## Components

### 1. `crates/mockforge-http/src/handlers/protocol_contracts.rs`

**Current state (lines 1–10):**

```rust
//! Protocol contract management handlers
//!
//! This module provides HTTP handlers for managing protocol contracts (gRPC, WebSocket, MQTT, Kafka).

// Per-protocol contract impls (GrpcContract, KafkaContract, MqttContract,
// WebSocketContract, ProtocolContractRegistry) stay in mockforge-core because
// they hold compiled jsonschema validators. Allow here until a future phase
// extracts them to mockforge-contracts.
#![allow(deprecated)]
```

**Target state**: delete the stale 4-line comment block. Keep the file-level `//!` docstring. Decide on `#![allow(deprecated)]` during execution:

- Try removing it. Run `cargo clippy -p mockforge-http --all-targets -- -D warnings`.
- If clippy stays clean: remove the attribute entirely.
- If clippy fires on a deprecated symbol: keep the attribute, add a one-line comment naming the specific deprecated symbol it allows (so future maintainers know what it's covering). Don't keep an unexplained `#![allow(deprecated)]`.

**Imports** (lines ~17–22 currently):

```rust
use mockforge_contracts::contract_drift::protocol_contracts::{
    compare_contracts, ProtocolContractRegistry,
};
use mockforge_core::contract_drift::{
    GrpcContract, KafkaContract, MqttContract, WebSocketContract,
};
```

**Target**: collapse into a single `mockforge_contracts::contract_drift::*` import block. Read each type's actual home (via `mockforge_contracts::contract_drift::mod.rs`'s re-exports) to determine the canonical path. Likely:

```rust
use mockforge_contracts::contract_drift::{
    compare_contracts, GrpcContract, KafkaContract, MqttContract,
    ProtocolContractRegistry, WebSocketContract,
};
```

(Verify that `compare_contracts` and `ProtocolContractRegistry` are re-exported from `mockforge_contracts::contract_drift::*` at the top level. If not, keep the longer `mockforge_contracts::contract_drift::protocol_contracts::*` path for the registry but unify with the impls.)

### 2. Root `Cargo.toml` — `[workspace.dependencies]` additions

Add to the existing `[workspace.dependencies]` block (preserve alphabetical or logical grouping with adjacent deps):

```toml
tempfile = "3.27"
bytes = "1.5"
```

The existing `jsonschema = "0.33"` line stays.

Do NOT add `prost-reflect` (deferred — version mismatch with `mockforge-grpc`).

### 3. Per-crate `Cargo.toml` updates — switch to `{ workspace = true }`

**`jsonschema`** — 3 crates (currently literal):
- `crates/mockforge-contracts/Cargo.toml`: `jsonschema = "0.33.0"` → `jsonschema = { workspace = true }`
- `crates/mockforge-data/Cargo.toml`: `jsonschema = "0.33"` → `jsonschema = { workspace = true }`
- `crates/mockforge-core/Cargo.toml`: `jsonschema = "0.33.0"` → `jsonschema = { workspace = true }`

The existing `mockforge-bench` + `mockforge-openapi` `{ workspace = true }` lines stay unchanged.

**`tempfile`** — 5 crates:
- `crates/mockforge-amqp/Cargo.toml`: `tempfile = "3"` → `tempfile = { workspace = true }`
- `crates/mockforge-contracts/Cargo.toml`: `tempfile = "3.27"` → `tempfile = { workspace = true }`
- `crates/mockforge-plugin-egress/Cargo.toml`: `tempfile = "3"` → `tempfile = { workspace = true }`
- `crates/mockforge-plugin-host/Cargo.toml`: 2 lines `tempfile = "3"` → `tempfile = { workspace = true }` (both occurrences)

Audit for any other `tempfile` consumers via `grep -rnE '^tempfile' crates/*/Cargo.toml` and update them too.

**`bytes`** — 3 crates (matching version):
- `crates/mockforge-tunnel/Cargo.toml`: `bytes = "1.5"` → `bytes = { workspace = true }`
- `crates/mockforge-contracts/Cargo.toml`: `bytes = "1.5"` → `bytes = { workspace = true }`
- `crates/mockforge-core/Cargo.toml`: `bytes = "1.5"` → `bytes = { workspace = true }`

**Do NOT touch**:
- `crates/mockforge-tcp/Cargo.toml`: keeps `bytes = "1.0"` (different version; harmonizing is a separate concern)
- `crates/mockforge-collab/Cargo.toml`: keeps `bytes = { version = "1", optional = true }` (different shape with `optional`; harmonizing requires the optional-dep dance and is a separate concern)

## Testing strategy

Pure Cargo-config + imports cleanup. No new tests.

**Compile-time gates**
- `cargo check --workspace --all-targets` — clean.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean. Pre-existing desktop-app / cli failures unrelated to this PR.
- `cargo fmt --all --check` — clean (no source-formatting changes expected; Cargo.toml isn't formatted by rustfmt).

**Dep-tree verification**
- `cargo tree -d 2>&1 | head` — confirm `jsonschema`, `tempfile`, `bytes` no longer show duplicate copies (or at minimum, fewer dupes than before).

**Compile inputs**
- After all Cargo.toml changes, run `cargo update` is NOT needed. Workspace deps don't require lockfile changes if the resolved version is the same; if it changes, that's an unintended side-effect to surface.

**The http handler cleanup specifically**
- `cargo check -p mockforge-http --lib` — clean.
- `cargo clippy -p mockforge-http --all-targets -- -D warnings` — clean (regardless of whether the `#![allow(deprecated)]` is kept or removed).

## Out of scope (file follow-up tickets if not already tracked)

- **`prost-reflect` workspace promotion** — requires bumping `mockforge-grpc` from `0.14` → `0.16`. Potential API break; needs its own audit + targeted PR.
- **`mockforge-tcp::bytes` version harmonization** — bumping from `"1.0"` to `"1.5"` (or whatever the workspace pins) is usually safe (semver-compatible) but should be a deliberate change with cargo-tree verification.
- **`mockforge-collab::bytes` optional-dep harmonization** — could declare it as `{ workspace = true, optional = true }` if Cargo allows that syntax for path-style workspace deps. Check syntax + verify behavior before promoting.

## Risks

- **Workspace promotion downstream resolution**: switching a crate from literal `"0.33.0"` to `{ workspace = true }` (resolving to `"0.33"`) is semver-compatible — Cargo will pick the same major.minor. But if the workspace's `"0.33"` resolves to a different patch in `Cargo.lock` than the previous `"0.33.0"` pin, that's a (probably-fine) side-effect to verify.
- **Hidden version drift via `Cargo.lock`**: after the Cargo.toml updates, inspect `cargo update --dry-run` (or `cargo tree -d`) for any surprising resolution differences.
- **`#![allow(deprecated)]` removal might re-surface a clippy warning**: handle during execution — keep the attribute if needed, document what it allows.

## Approval

Scope: conservative — fix http handler + promote tempfile/bytes/jsonschema (where versions match). Skip prost-reflect + tcp + collab. Confirmed via brainstorming session 2026-05-22.
