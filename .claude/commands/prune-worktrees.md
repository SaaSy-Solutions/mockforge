---
allowed-tools: Bash, Read
description: Remove git worktrees whose PR has merged, to recover disk (dry-run first)
---

# /prune-worktrees — Reclaim Disk from Merged Worktrees

`/mnt/projects` fills up because each worktree carries ~15-26G of build
artifacts. This removes worktrees whose branch's PR has already merged.

Squash-merge aware: because PRs land as squash commits, `git branch --merged`
can't see them, so merged-ness is matched against `gh pr list --state merged`
head branches (the correct signal for this repo — see the disk-pressure memory).

## Steps

1. **Dry-run first** (always):
   ```bash
   scripts/prune-worktrees.sh
   ```
   Shows each worktree it would remove (with size), which it would skip for
   uncommitted changes, and which it keeps (main, current, unmerged, detached).

2. **Review the plan with the user.** Confirm nothing in the "would remove" list
   is still wanted. The script never touches the main checkout, the current
   worktree, `main`, or any worktree with uncommitted changes.

3. **Execute** once confirmed:
   ```bash
   scripts/prune-worktrees.sh --execute
   ```
   Removes only merged + clean worktrees (never `--force`), then
   `git worktree prune`. Reports how many were removed / skipped / kept.

## Rules
- Dry-run before `--execute`, every time. Show the plan; don't surprise-delete.
- The script refuses to remove dirty worktrees (no `--force`) — if one is dirty
  but you know it's disposable, the human handles it manually.
- Safe to run often; it's idempotent.
