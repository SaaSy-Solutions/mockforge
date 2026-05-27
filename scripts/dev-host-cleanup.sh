#!/usr/bin/env bash
#
# Dev-host disk pressure cleanup for the mockforge worktree pattern.
#
# Sibling of the runner-host cleanup script that lives at
# /usr/local/bin/runner-host-cleanup.sh on the Hetzner CI runner —
# different paths, different "safe to delete" heuristic (we use GitHub
# PR state via `gh`), same pressure-tier idea.
#
# Each `Agent`-style coding session in this repo creates a git worktree
# under `<repo>/.claude/worktrees/<branch>/`, with its own
# `target/` directory that can be 15–35 GB. After a couple of sessions
# the disk fills up and cargo dies mid-build with "No space left on
# device". This script reclaims the target/ dirs on worktrees whose
# branch has already merged (or, under heavier pressure, closed),
# without touching source files or active worktrees.
#
# Usage:
#   dev-host-cleanup.sh                                  # dry-run on default repo
#   dev-host-cleanup.sh --force                          # actually delete
#   dev-host-cleanup.sh --repo /path/to/repo --force
#   dev-host-cleanup.sh --mount /mnt/projects --force    # pressure check on a specific mount
#
# Thresholds (matched to the runner's, lowered 2026-05-23):
#   soft pressure   ≥ 65%  → delete target/ on worktrees with MERGED PRs
#   aggressive      ≥ 80%  → also delete target/ on worktrees with CLOSED PRs
#
# Safety:
# - Only touches *.claude/worktrees/<name>/target/ directories.
# - Never removes the worktree itself, never touches source files,
#   never runs `git clean` (which could nuke local edits).
# - Skips worktrees whose branch has no PR (assume in-flight work).
# - Skips worktrees whose branch has an OPEN PR.
# - A failed `gh` lookup (transient 401 / rate-limit) is treated as
#   "don't know" — the worktree is skipped, NOT deleted — and the run
#   exits 3 so a partial cleanup never looks like a clean no-op.
# - Logs everything to stderr and to /var/tmp/dev-host-cleanup.log.
#
# Exit codes: 0 = clean, 1 = setup error (no gh auth / unknown repo),
# 2 = bad args, 3 = ran but ≥1 worktree unclassifiable (gh API error).

set -uo pipefail

REPO="${HOME}/dev/projects/work/mockforge"
# Fallback: most callers will be running from inside the project tree.
if [ ! -d "$REPO" ]; then
  REPO="/mnt/projects/mockforge"
fi
MOUNT="/mnt/projects"
SOFT_THRESHOLD=65
AGGRESSIVE_THRESHOLD=80
FORCE=false
LOG_FILE="/var/tmp/dev-host-cleanup.log"

while [ $# -gt 0 ]; do
  case "$1" in
    --force) FORCE=true; shift ;;
    --repo) REPO="$2"; shift 2 ;;
    --mount) MOUNT="$2"; shift 2 ;;
    --soft) SOFT_THRESHOLD="$2"; shift 2 ;;
    --aggressive) AGGRESSIVE_THRESHOLD="$2"; shift 2 ;;
    -h|--help)
      sed -n '3,/^$/p' "$0" | sed 's/^# \{0,1\}//'
      exit 0 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

log() {
  local ts
  ts=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  echo "[$ts] $*" | tee -a "$LOG_FILE" >&2
}

if [ ! -d "$REPO/.claude/worktrees" ]; then
  log "no worktrees dir at $REPO/.claude/worktrees — nothing to do"
  exit 0
fi

# Disk pressure tier. df reports as a percentage string with a trailing %.
USAGE=$(df --output=pcent "$MOUNT" | tail -1 | tr -d ' %' || echo 0)
TIER="ok"
if [ "$USAGE" -ge "$AGGRESSIVE_THRESHOLD" ]; then
  TIER="aggressive"
elif [ "$USAGE" -ge "$SOFT_THRESHOLD" ]; then
  TIER="soft"
fi
log "disk usage $USAGE% on $MOUNT — tier=$TIER (soft≥${SOFT_THRESHOLD}%, aggressive≥${AGGRESSIVE_THRESHOLD}%)"
if [ "$TIER" = "ok" ]; then
  log "no pressure; exiting"
  exit 0
fi
[ "$FORCE" = false ] && log "DRY-RUN — pass --force to actually delete"

# Reconcile gh auth before we walk: a stale token gives every PR as
# "UNKNOWN" and the script would skip everything. Fail loud rather
# than silently no-op.
if ! gh auth status >/dev/null 2>&1; then
  log "ERROR: gh CLI is not authenticated. Run 'gh auth login' on this host."
  exit 1
fi

# Resolve the repo's GitHub slug once (e.g. SaaSy-Solutions/mockforge).
# Saves one network call per worktree.
REPO_SLUG=$(cd "$REPO" && gh repo view --json nameWithOwner --jq .nameWithOwner 2>/dev/null || true)
if [ -z "$REPO_SLUG" ]; then
  log "ERROR: could not determine GitHub slug for $REPO"
  exit 1
fi
log "repo=$REPO_SLUG"

# Convert a worktree branch name back to the original branch.
# EnterWorktree creates branches like `worktree-<sanitised-name>` —
# the worktree dir itself is named after the user-supplied slug,
# with `+` substituted for `/`. There's also the case of a worktree
# entered via `path` against an already-existing branch (no rename).
worktree_branch() {
  local wt_path="$1"
  # Prefer the actual branch name from git; fall back to the dir name.
  git -C "$wt_path" symbolic-ref --short HEAD 2>/dev/null || basename "$wt_path"
}

# One `gh pr list` lookup for a single head ref, with retries.
# `gh` (and the GitHub API behind it) intermittently 401s or rate-limits
# even when `gh auth status` reports a healthy token — bursting a call
# per worktree is enough to trip a secondary limit. We MUST distinguish
# "the API said this branch has no PR" (empty, safe to treat as
# in-flight) from "the call itself failed" (don't know — must NOT treat
# as no-PR, or a transient blip silently leaves merged target/ dirs
# unreclaimed and the cron run looks like a clean no-op).
#
# Echoes the PR state ("" if genuinely no PR) on success, or the literal
# sentinel "API_ERROR" if every attempt failed.
gh_pr_state() {
  local head="$1"
  local out attempt
  for attempt in 1 2 3; do
    # `--jq` is processed inside gh, so this is a single command and its
    # exit code is gh's own (0 even for an empty result, non-zero on
    # auth/network/API failure).
    if out=$(gh pr list --repo "$REPO_SLUG" --state all --head "$head" \
               --limit 1 --json state --jq '.[0].state // ""' 2>/dev/null); then
      echo "$out"
      return 0
    fi
    sleep $((attempt * 2))
  done
  echo "API_ERROR"
  return 1
}

# Try to find a PR whose head ref matches the worktree's branch.
# Returns the PR state (MERGED|CLOSED|OPEN), "" if no PR exists, or
# "API_ERROR" if the lookup could not be completed.
pr_state_for_branch() {
  local branch="$1"
  # Strip the EnterWorktree `worktree-` prefix when present so we
  # find the real PR. Both `<slug>` and `worktree-<slug>` variants
  # can exist depending on whether the user committed under the
  # session branch or rebased onto a `feat/...` branch later.
  local stripped="${branch#worktree-}"
  # The actual PR branch usually uses `/` separators where the
  # worktree path used `+`. Map back.
  local candidate="${stripped//+/\/}"

  local state
  state=$(gh_pr_state "$candidate")
  # Only fall through to the raw branch name on a genuine empty result,
  # not on an API error (retrying the same failing call is pointless and
  # we'd lose the error signal).
  if [ -z "$state" ] && [ "$candidate" != "$branch" ]; then
    state=$(gh_pr_state "$branch")
  fi
  echo "$state"
}

freed_bytes=0
checked=0
deleted=0
skipped_open=0
skipped_unknown=0
skipped_error=0

for wt_target in "$REPO/.claude/worktrees"/*/target; do
  [ -d "$wt_target" ] || continue
  checked=$((checked + 1))

  wt_dir="$(dirname "$wt_target")"
  wt_name="$(basename "$wt_dir")"
  branch=$(worktree_branch "$wt_dir")
  state=$(pr_state_for_branch "$branch")
  size=$(du -sb "$wt_target" 2>/dev/null | cut -f1 || echo 0)
  size_h=$(numfmt --to=iec --suffix=B "$size" 2>/dev/null || echo "${size}B")

  case "$state" in
    MERGED)
      eligible=true
      reason="MERGED"
      ;;
    CLOSED)
      if [ "$TIER" = "aggressive" ]; then
        eligible=true
        reason="CLOSED (aggressive tier)"
      else
        eligible=false
        reason="CLOSED (skipped at soft tier — open --aggressive to clean)"
      fi
      ;;
    OPEN)
      eligible=false
      reason="OPEN PR — actively in use"
      skipped_open=$((skipped_open + 1))
      ;;
    API_ERROR)
      eligible=false
      reason="gh API error after retries — skipping to be safe (NOT a no-PR result)"
      skipped_error=$((skipped_error + 1))
      ;;
    "")
      eligible=false
      reason="no PR found — assuming in-flight"
      skipped_unknown=$((skipped_unknown + 1))
      ;;
    *)
      eligible=false
      reason="unknown PR state '$state'"
      skipped_unknown=$((skipped_unknown + 1))
      ;;
  esac

  if [ "$eligible" = true ]; then
    if [ "$FORCE" = true ]; then
      log "DELETE  $wt_name  ($size_h, $reason)"
      rm -rf "$wt_target"
      deleted=$((deleted + 1))
      freed_bytes=$((freed_bytes + size))
    else
      log "WOULD   $wt_name  ($size_h, $reason)"
      freed_bytes=$((freed_bytes + size))
    fi
  else
    log "skip    $wt_name  ($size_h, $reason)"
  fi
done

freed_h=$(numfmt --to=iec --suffix=B "$freed_bytes" 2>/dev/null || echo "${freed_bytes}B")
log "done — checked=$checked deleted=$deleted skipped_open=$skipped_open skipped_unknown=$skipped_unknown skipped_error=$skipped_error freed=$freed_h"

# Exit non-zero when a transient gh failure prevented us from classifying
# one or more worktrees. We may still have reclaimed space from the ones
# that resolved, but a partial run under disk pressure shouldn't look
# like a clean success in cron logs / monitoring.
if [ "$skipped_error" -gt 0 ]; then
  log "WARNING: $skipped_error worktree(s) could not be classified due to gh API errors; rerun once gh is healthy"
  exit 3
fi
