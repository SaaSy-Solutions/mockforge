---
user-invocable: true
allowed-tools: [Bash, Read, Edit, Glob, Grep, Task]
description: Cut a MockForge release end-to-end — bump, CHANGELOG, gate, publish, tag, verify
argument-hint: "[patch|minor|major|X.Y.Z] (default: patch)"
---

# /ship-release — End-to-End Release

Cuts a release the way this repo expects: version bump, CHANGELOG with pillar
tags, a hard pre-publish gate, publish, tag, and verification against the
*published* artifact. Do the whole chain without prompting per step (the user
has standing approval to cut releases end-to-end), but STOP if the guardian
returns NO-GO.

This codifies the recurring release flow and the lessons from the 0.3.142 yank,
the #584 dep-list drift, and the caret poison-pill.

## Steps

### 1. Confirm clean starting state
```bash
git status --porcelain        # must be empty (ignore untracked)
git rev-parse --abbrev-ref HEAD
```
Never start a release on a dirty tree or mid another release. Work on a branch,
not directly mutating `main` if a bump is racing another PR.

### 2. Decide the version
Argument is `patch` (default), `minor`, `major`, or an explicit `X.Y.Z`.
Read the current `[workspace.package] version` in root `Cargo.toml`. Compute the
target. **Never** bump while a publish could be in flight (it reads the working
tree — see memory: publish version-bump race).

### 3. Bump + CHANGELOG
- Bump `[workspace.package] version` in root `Cargo.toml`.
- Prepend a `## [<version>] - <today>` entry to `CHANGELOG.md` with bullets
  tagged by pillar: `[Reality]`, `[Contracts]`, `[DevX]`, `[Cloud]`, `[AI]`.
  Summarize what changed since the last entry (read recent commits).
- If concurrent PRs also bumped the version / CHANGELOG, resolve mechanically:
  keep the higher version, concatenate both CHANGELOG entries (newer first).

### 4. Pre-publish GATE (mandatory)
Run the install smoke-test, then dispatch the guardian:
```bash
scripts/smoke-test-install.sh        # or --fast for build-only iteration
```
Then launch the **`release-guardian`** agent (haiku). If it returns **NO-GO**,
STOP and fix the FAIL rows. Do not publish on NO-GO. Only proceed on **GO**.

### 5. Commit
Commit the bump + CHANGELOG on a branch:
`chore(release): v<version>` (CHANGELOG body is exempt from the no-em-dash rule).
Run `cargo fmt --all --check` first — qualifier shortenings can let rustfmt
collapse lines and break CI (see memory). Do NOT use `--no-verify`.

### 6. Publish
```bash
export CARGO_REGISTRY_TOKEN=...      # or rely on ~/.cargo/credentials.toml
scripts/publish-crates.sh            # publishes every crate in dep order
```
Do NOT bump the version or switch branches while this runs.

### 7. Tag + verify against the published binary
- Tag `v<version>` and push the tag.
- Verify the published artifact, not the local build:
  `cargo install --locked mockforge-cli@<version>` in a scratch dir and run
  `mockforge --version`.
- Confirm crates.io shows the version, the git tag exists, and the Release
  workflow ran (avoid a double-run — see memory: a held release may already be
  published by another agent).

### 8. Report
Summarize: version, crates published, tag URL, smoke-test result. Note anything
skipped and why.

## Rules
- NO-GO from `release-guardian` is a hard stop.
- Per memory: prefer local/self-hosted runners; don't trigger costly GHA.
- Per memory: whenever a new `crates/mockforge-*/` dir exists, ensure it's in
  `scripts/publish-crates.sh` CRATES list (or marked `publish = false`) BEFORE
  publishing — the guardian checks this, but fix it here.
- Auto-merge any release PR you open (`gh pr merge <#> --auto --squash`).
