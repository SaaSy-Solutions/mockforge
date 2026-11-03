#!/usr/bin/env bash
# Print `CARGO_TARGET_DIR=<dir>` for one job class on a self-hosted runner.
#
#   bash scripts/ci/runner-cargo-target.sh <class> >> "$GITHUB_ENV"
#
# WHY: GitHub Actions cache storage is billed to the org, and the org runs out
# of free storage early in each month. Swatinem/rust-cache and actions/cache
# kept ~10 GB of target/registry tarballs in GitHub storage and re-uploaded
# 1.0-1.3 GB per job class whenever a key missed. Self-hosted runners already
# have a persistent disk, so the warm target lives there instead and nothing is
# written to GitHub storage. The registry is already persistent through the
# per-runner CARGO_HOME=/tmp/cargo-mockforge-<runner> set by each job.
#
# LAYOUT: /tmp/cargo-mockforge-target-<class>-<runner>. A runner executes one
# job at a time and the path carries the runner name, so no two jobs ever build
# in the same dir. The /tmp/cargo-* prefix keeps the shared host's existing
# cleanup in charge of idle dirs (saas-platform
# infrastructure/github-runners/host-shared/runner-host-cleanup.sh: protected
# while a live process has it as CARGO_TARGET_DIR, swept after 60 idle min).
#
# SELF-BOUNDING, because a warm dir is never reset by checkout:
#   * the dir for this class is dropped when it exceeds MAX_KB (30 GiB, the
#     size at which check_rust_cache.py starts warning);
#   * when the filesystem is at or above PRESSURE_PCT, this runner's OTHER
#     class dirs are dropped, least recently used first. They cannot be in use:
#     this runner is busy with the current job. Other runners' dirs and
#     anything not matching this runner's exact prefix are never touched.
# Removal is mv-aside then rm, so an interrupted run leaves a
# `*.reclaiming.*` leftover that the host cleanup reaps, never a half-deleted
# live dir. Symlinks are refused, never followed.
set -euo pipefail

ROOT="${MOCKFORGE_RUNNER_TARGET_ROOT:-/tmp}"
MAX_KB="${MOCKFORGE_RUNNER_TARGET_MAX_KB:-31457280}"
PRESSURE_PCT="${MOCKFORGE_RUNNER_TARGET_PRESSURE_PCT:-70}"

valid_name() {
  case "$1" in
    '' | *[!A-Za-z0-9._-]*) return 1 ;;
  esac
}

class="${1:-}"
if ! valid_name "$class"; then
  echo "usage: $0 <class>   (class: letters, digits, . _ - only)" >&2
  exit 2
fi
runner="${RUNNER_NAME:-}"
if ! valid_name "$runner"; then
  echo "::error::RUNNER_NAME is missing or has unexpected characters" >&2
  exit 2
fi

prefix="$ROOT/cargo-mockforge-target-"
suffix="-$runner"
dir="$prefix$class$suffix"

reclaim() { # <dir>
  local doomed="${1:?}.reclaiming.$$"
  mv "$1" "$doomed"
  rm -rf "${doomed:?}"
}

used_pct() {
  if [ -n "${MOCKFORGE_RUNNER_TARGET_USED_PCT:-}" ]; then
    echo "$MOCKFORGE_RUNNER_TARGET_USED_PCT"
    return
  fi
  df --output=pcent "$ROOT" | tail -1 | tr -dc '0-9'
}

if [ -L "$dir" ] || { [ -e "$dir" ] && [ ! -d "$dir" ]; }; then
  echo "::error::$dir exists and is not a plain directory; refusing to use it" >&2
  exit 1
fi

if [ -d "$dir" ]; then
  kb=$(du -sxk "$dir" | awk '{print $1}')
  if [ "${kb:-0}" -gt "$MAX_KB" ]; then
    echo "::notice::$dir is ${kb} KB (over ${MAX_KB} KB); starting this class cold" >&2
    reclaim "$dir"
  fi
fi

if [ "$(used_pct)" -ge "$PRESSURE_PCT" ]; then
  while IFS= read -r sibling; do
    [ "$(used_pct)" -ge "$PRESSURE_PCT" ] || break
    [ "$sibling" = "$dir" ] && continue
    [ -d "$sibling" ] && [ ! -L "$sibling" ] || continue
    case "$sibling" in
      "$prefix"*"$suffix") ;;
      *) continue ;;
    esac
    # A class name may itself end in "-<runner>"-like text; require that the
    # part between prefix and suffix is a valid class name for THIS runner.
    middle="${sibling#"$prefix"}"
    middle="${middle%"$suffix"}"
    valid_name "$middle" || continue
    echo "::notice::filesystem at or above ${PRESSURE_PCT}%; dropping idle $sibling" >&2
    reclaim "$sibling"
  done < <(find "$ROOT" -mindepth 1 -maxdepth 1 -type d -name "cargo-mockforge-target-*$suffix" \
             -printf '%T@ %p\n' 2> /dev/null | sort -n | cut -d' ' -f2-)
fi

mkdir -p "$dir"
# The host cleanup judges idleness by nested mtime; mark the dir as in use now.
touch "$dir"
echo "CARGO_TARGET_DIR=$dir"
