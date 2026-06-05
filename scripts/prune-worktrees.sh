#!/usr/bin/env bash
#
# Prune git worktrees whose branch's PR has already been merged, to recover disk
# (/mnt/projects fills up; each worktree is ~15-26G of build artifacts).
#
# Squash-merge aware: because PRs land as squash commits, `git branch --merged`
# does NOT detect a merged branch (its commits never become ancestors of main).
# So merged-ness is determined by matching each worktree's branch against
# `gh pr list --state merged` head branches — the correct signal for this repo.
#
# Safety:
#   - never touches the main checkout or the current worktree
#   - never removes a worktree with uncommitted changes (no --force)
#   - DEFAULTS TO DRY-RUN: prints the plan; pass --execute to actually remove
#
# Usage:
#   scripts/prune-worktrees.sh            # dry-run: show what would be removed
#   scripts/prune-worktrees.sh --execute  # actually remove merged, clean worktrees
set -uo pipefail

EXECUTE=false
[ "${1:-}" = "--execute" ] && EXECUTE=true

command -v gh >/dev/null 2>&1 || { echo "error: gh CLI required (merged-PR detection)" >&2; exit 2; }

# Merged head branches (squash-merge aware signal).
mapfile -t MERGED < <(gh pr list --state merged --limit 300 --json headRefName -q '.[].headRefName' 2>/dev/null | sort -u)
is_merged() { local b="$1"; for m in "${MERGED[@]}"; do [ "$m" = "$b" ] && return 0; done; return 1; }

MAIN_WT="$(git worktree list --porcelain | awk '/^worktree /{print $2; exit}')"
CUR_WT="$(git rev-parse --show-toplevel 2>/dev/null || echo "")"

removed=0 skipped_dirty=0 kept=0 freed_note=""
path="" branch=""
flush() {
  [ -z "$path" ] && return
  if [ "$path" = "$MAIN_WT" ] || [ "$path" = "$CUR_WT" ]; then kept=$((kept+1)); path=""; branch=""; return; fi
  if [ -z "$branch" ]; then kept=$((kept+1)); path=""; branch=""; return; fi   # detached HEAD
  if [ "$branch" = "main" ]; then kept=$((kept+1)); path=""; branch=""; return; fi
  if is_merged "$branch"; then
    if [ -n "$(git -C "$path" status --porcelain 2>/dev/null)" ]; then
      echo "  SKIP (uncommitted changes): $path [$branch]"; skipped_dirty=$((skipped_dirty+1))
    else
      local sz; sz="$(du -sh "$path" 2>/dev/null | cut -f1)"
      if [ "$EXECUTE" = true ]; then
        if git worktree remove "$path" 2>/dev/null; then echo "  REMOVED ($sz): $path [$branch]"; removed=$((removed+1));
        else echo "  FAILED to remove (left intact): $path [$branch]"; fi
      else
        echo "  would remove ($sz): $path [$branch]"; removed=$((removed+1))
      fi
    fi
  else
    kept=$((kept+1))
  fi
  path=""; branch=""
}

while IFS= read -r line; do
  case "$line" in
    "worktree "*) flush; path="${line#worktree }" ;;
    "branch "*)   branch="${line#branch refs/heads/}" ;;
    "detached")   branch="" ;;
    "")           flush ;;
  esac
done < <(git worktree list --porcelain; echo "")
flush

echo ""
if [ "$EXECUTE" = true ]; then
  git worktree prune
  echo "Pruned $removed worktree(s); skipped $skipped_dirty dirty; kept $kept."
else
  echo "DRY-RUN: would prune $removed; would skip $skipped_dirty dirty; keep $kept."
  echo "Re-run with --execute to remove."
fi
