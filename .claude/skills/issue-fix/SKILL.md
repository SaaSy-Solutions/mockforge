---
user-invocable: true
allowed-tools: [Bash, Read, Edit, Write, Glob, Grep, Task]
description: Address a GitHub issue end-to-end — fresh worktree, fix, verify with the real binary, ship before replying
argument-hint: "<issue-number or description>"
---

# /issue-fix — Issue-Driven Workflow

Standardizes how issues get fixed in this repo: isolate in a worktree, fix,
prove the fix by running the actual binary (not just tests), then ship (merge +
release) BEFORE telling the user to try anything. Encodes the issue-workflow and
always-use-worktrees memories.

## Steps

### 1. Fetch context before branching
```bash
git fetch
gh issue view <N>                          # the actual ask + repro
gh pr list --state merged --limit 10       # don't branch from stale main / dup work
```
Read the issue fully. If it names a file/flag/function, confirm it still exists.

### 2. Spawn an isolated worktree off fresh main
Never branch in the primary checkout (it often holds concurrent-refactor WIP).
Use the `using-git-worktrees` superpower or:
```bash
git worktree add ../mockforge-issue-<N> -b fix/issue-<N> origin/main
```
Watch disk: `/mnt/projects` fills up; prune merged-PR worktrees first if needed
(see local-disk-pressure memory).

### 3. Reproduce, then fix
- Reproduce the bug first (a failing test or a real binary invocation).
- Implement the fix. Match surrounding code style.
- Prefer TDD where it fits (superpowers:test-driven-development).

### 4. Verify with the REAL binary
Tests alone are not enough for an issue fix. Build and run the actual CLI to
confirm the reported behavior is gone:
```bash
cargo build -p mockforge-cli            # add --features all-protocols for ws/grpc/mqtt e2e
cargo run -p mockforge-cli -- <repro command from the issue>
```
Note: default features exclude ws/grpc/mqtt; protocol e2e needs
`--features all-protocols` + `MOCKFORGE_TEST_BINARY` (see memory). Beware the
stale `$PATH` binary — run via cargo, not a loose `mockforge` on PATH.

### 5. Verify the change set
Run `/verify` (scoped to affected crates) or directly:
`cargo fmt --all --check`, `cargo clippy -p <crate> --all-targets -- -D warnings`,
`cargo test -p <crate>`. If bench templates changed, run `/template-check`.
If auth/protocol surfaces changed, dispatch `auth-sentinel` / `protocol-parity`.

### 6. Ship before replying
Per memory: publish before telling the user to try anything.
- `/commit-push-pr` (or `/ship-release` if a release is warranted), reference
  `(#<N>)` in the commit.
- Auto-merge: `gh pr merge <#> --auto --squash`.
- If the fix needs a release to reach users, cut it now via `/ship-release`.

### 7. Reply
Close the loop on the issue with what changed and how to verify, in which
version. No em dashes in the reply (memory) — `/reply-lint` guards this.

## Rules
- Worktree always; never commit-producing work in the primary checkout.
- Real-binary verification is mandatory, not optional.
- Ship (merge + release if needed) BEFORE asking the user to test.
