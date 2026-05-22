# Deferred dep cleanups from #609: prost-reflect promotion, tcp dead-dep deletion, collab optional bytes

- **Status**: Approved
- **Date**: 2026-05-22
- **Follows**: [#609](https://github.com/SaaSy-Solutions/mockforge/issues/609) / [#612](https://github.com/SaaSy-Solutions/mockforge/pull/612). #612 explicitly deferred these 3 items as separate concerns.

## Context

PR #612 (closing #609) promoted `tempfile` + `bytes` to workspace deps and unified literal `jsonschema`/`tempfile`/`bytes` pins. It deferred three items because each had a different kind of risk that needed individual evaluation:

1. **`prost-reflect 0.14` in `mockforge-grpc` vs `0.16` in core/contracts** — needed to verify whether the 0.14 → 0.16 bump would break grpc's 20+ call sites.
2. **`bytes = "1.0"` in `mockforge-tcp`** — needed to verify whether it's actually used (could be silent semver-compatible bump to 1.5, or could be dead code).
3. **`bytes = { version = "1", optional = true }` in `mockforge-collab`** — needed to confirm workspace deps support `{ workspace = true, optional = true }` syntax.

Investigation findings (recorded in this spec so the implementation can lean on them):

### Item 1 — prost-reflect bump verdict: **safe**

- `mockforge-grpc` uses these prost-reflect types: `DescriptorPool`, `DynamicMessage`, `FieldDescriptor`, `Kind`, `MapKey`, `MessageDescriptor`, `MethodDescriptor`, `ReflectMessage`, `Value`. All are core public-API types that exist unchanged in both 0.14 and 0.16.
- Per prost-reflect's GitHub release notes (`https://github.com/andrewhickman/prost-reflect/releases`), the changes between 0.14 → 0.16 were:
  - **0.15.x**: descriptor updates for protobuf 25.4 well-known types + docs.rs metadata fixes. No public API changes.
  - **0.16.0**: internal prost dep bumped from 0.13 → 0.14. (`mockforge-grpc` already uses `prost = "0.14"` on line 24 of its Cargo.toml.)
  - **0.16.1, 0.16.2, 0.16.3**: pure additions (`OneofDescriptor::is_synthetic`, `FieldDescriptor::is_required`, `default_value` methods, prost 0.14.2 update).
- `Cargo.lock` currently contains BOTH `prost-reflect 0.14.7` AND `prost-reflect 0.16.3` because of the version pin mismatch. Bumping grpc to 0.16 deduplicates this.

### Item 2 — tcp `bytes` verdict: **dead code, delete**

- `crates/mockforge-tcp/Cargo.toml:34` declares `bytes = "1.0"`.
- `git grep -rE 'use bytes|bytes::' crates/mockforge-tcp/src/` returns **zero matches**.
- The `bytes`-related references in tcp's src/ (`exact_bytes`, `throttle_bytes_per_sec`, `partial_data_bytes`) are field names on config structs, not uses of the `bytes` crate.
- Conclusion: the dep is dead. Delete it rather than bump it.

### Item 3 — collab optional `bytes` verdict: **standard syntax**

- `crates/mockforge-collab/Cargo.toml:84` declares `bytes = { version = "1", optional = true }`.
- Cargo natively supports `{ workspace = true, optional = true }` for workspace-inherited optional deps. This is documented behavior since Cargo 1.64.
- The dep is used inside feature-gated code (`use bytes::Bytes` in `src/backup.rs:894` under some feature flag).
- Switching to `{ workspace = true, optional = true }` inherits the workspace's `bytes = "1.5"` and keeps the optional gating intact.

## Decision

Single PR with three coupled changes:

1. **Bump `mockforge-grpc::prost-reflect` to align with workspace** (and promote `prost-reflect` itself to workspace deps).
2. **Delete the dead `bytes` dep from `mockforge-tcp`**.
3. **Switch `mockforge-collab::bytes` to workspace + optional**.

After this PR, `Cargo.lock` should show only `prost-reflect 0.16.x` (deduped) and only `bytes 1.5.x` (deduped from the workspace-inherited deps). No more direct literal pins of either crate's version remain in any first-party crate.

## Components

### 1. Add `prost-reflect = "0.16"` to root `[workspace.dependencies]`

In `Cargo.toml`, add a line near the existing dep promotions from #609 (e.g. next to `bytes = "1.5"`):

```toml
prost-reflect = "0.16"
```

### 2. Switch `crates/mockforge-grpc/Cargo.toml` to workspace = true

Change line 25 from:

```toml
prost-reflect = "0.14"
```

to:

```toml
prost-reflect = { workspace = true }
```

### 3. Switch `crates/mockforge-core/Cargo.toml` to workspace = true

Change line 73 from:

```toml
prost-reflect = "0.16"
```

to:

```toml
prost-reflect = { workspace = true }
```

### 4. Switch `crates/mockforge-contracts/Cargo.toml` to workspace = true

Change the matching line from:

```toml
prost-reflect = "0.16"
```

to:

```toml
prost-reflect = { workspace = true }
```

### 5. Delete the dead `bytes` line from `crates/mockforge-tcp/Cargo.toml`

Remove line 34 (`bytes = "1.0"`) entirely. Audit to confirm no other line in the file declares `bytes` (e.g. in `[dev-dependencies]` or behind a feature gate).

### 6. Switch `crates/mockforge-collab/Cargo.toml` to workspace + optional

Change line 84 from:

```toml
bytes = { version = "1", optional = true }
```

to:

```toml
bytes = { workspace = true, optional = true }
```

## Testing strategy

Pure Cargo-config change. No new tests; existing tests are the safety net.

**Compile-time gates**
- `cargo check --workspace --all-targets` — clean.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean (modulo pre-existing desktop-app + cli failures unrelated to this PR).
- `cargo fmt --all --check` — clean.

**Per-crate tests** (the three crates affected by the changes)
- `cargo test -p mockforge-grpc --lib` — **most important verification**. Exercises the 20+ call sites that use prost-reflect's API. If any of `DescriptorPool`/`DynamicMessage`/`FieldDescriptor`/`Kind`/`MapKey`/`MessageDescriptor`/`MethodDescriptor`/`ReflectMessage`/`Value` shifted incompatibly between 0.14 and 0.16, this surface will catch it.
- `cargo test -p mockforge-tcp --lib` — confirms the dead-dep deletion didn't break anything.
- `cargo test -p mockforge-collab --lib` — confirms the optional workspace dep works.

**Dep-tree verification**
- `cargo tree -d` — confirms `prost-reflect` is no longer duplicated. Was: `0.14.7` + `0.16.3`. After: just `0.16.x`.
- `cargo tree -d` for `bytes` — confirms tcp's deletion removed a copy if one existed. (Possibly the workspace was already deduped to 1.5; verify the diff.)

**Cargo.lock spot-check**
- Some lock churn is expected:
  - `prost-reflect 0.14.7` entry removed
  - `prost-reflect 0.16.x` may pick up a newer patch
  - `bytes` entries unchanged (collab and tcp both go through workspace 1.5)
- If a MAJOR version bump appears in Cargo.lock that wasn't expected (e.g. prost 0.14 → 0.15), surface as a finding.

## Risks

- **prost-reflect call-site break in grpc** (low risk, but the largest surface). Mitigation: `cargo test -p mockforge-grpc --lib` runs the test suite. If a specific call site fails:
  - Read the error carefully. Most likely cause: a method moved, a field changed visibility, or an enum variant renamed.
  - Often fixable in 1–3 lines (e.g. import a missing trait, rename a method).
  - If the fix is non-obvious, fall back to keeping grpc on prost-reflect = "0.14" via per-crate pin (revert just step 2 of the spec), and document the deferral.
- **Hidden `bytes` use in tcp** (low risk). Mitigation: cargo check fails if some feature-gated code (e.g. behind `#[cfg(feature = "foo")]`) imports `bytes`. The pre-commit hook runs cargo check on the full feature matrix.
- **Workspace optional dep syntax** (low risk; standard Cargo). Mitigation: cargo check catches any syntax error.

## Out of scope

Nothing further deferred. This PR closes the 3 items from #609.

## Approval

Single PR, conservative scope. Confirmed via brainstorming session 2026-05-22.
