---
user-invocable: true
allowed-tools: [Bash, Read, Edit, Glob, Grep]
description: Scaffold a new mockforge-* crate already wired into every required registration point
argument-hint: "<name> [--desc \"...\"] [--bin] [--publish]"
---

# /new-crate — Scaffold a Fully-Wired Crate

Create a new `mockforge-*` crate that is registered everywhere a crate has to be,
so it cannot become an orphan (#796) or a publish-list drift (#795). Every
problem fixed in those issues traced to a crate being created but missing one of
its registration points; this closes that gap at creation time.

## Process

### 1. Run the scaffolder
```bash
scripts/new-crate.sh <name> [--desc "one-line description"] [--bin] [--publish]
```
It creates `crates/mockforge-<name>/` (Cargo.toml with workspace inheritance +
`[lints] workspace = true`, `src/lib.rs` or `src/main.rs`, `README.md`), adds the
crate to root `Cargo.toml` `[workspace].members`, and — with `--publish` — inserts
it into `scripts/publish-crates.sh` before `mockforge-cli`. It then verifies the
crate is a workspace member and compiles.

Defaults: **library** crate, **`publish = false`** (most new crates have no
published consumer yet). Pass `--bin` for a binary, `--publish` to ship it now.

### 2. Decide publish posture (the one judgment call)
- **`publish = false`** (default) is correct when nothing published depends on
  the crate yet. The pre-push drift guard / `release-guardian` stay green.
- **`--publish`** only when a published crate will depend on it (or it ships
  standalone). If you use `--publish`, double-check its slot in the
  `scripts/publish-crates.sh` CRATES list: it must come AFTER every
  `mockforge-*` crate it depends on. The script inserts before `mockforge-cli`,
  which is safe for a leaf crate but not if an earlier crate depends on it.

### 3. Wire dependencies + code
- Add workspace deps to `[dependencies]` (`dep.workspace = true`).
- Replace the placeholder in `src/`.
- If the crate exposes public items, keep doc comments (workspace
  `missing_docs` lint).

### 4. Verify
```bash
cargo clippy -p mockforge-<name> --all-targets -- -D warnings
cargo test -p mockforge-<name>
```
If this crate will ship, run `/release-check` before the next release so the
guardian confirms it's listed (or correctly `publish = false`).

## Rules
- Never hand-create a crate dir without registering it in `[workspace].members`
  — that is exactly the orphan bug from #796.
- Use `[lints] workspace = true` (the scaffolder does), not a hand-rolled
  `[lints.rust]` block, so the crate inherits the curated workspace lint set.
- A `publish = false` crate must still be a workspace member (so it's built and
  linted); it just doesn't ship.
