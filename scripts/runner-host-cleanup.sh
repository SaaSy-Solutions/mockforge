#!/usr/bin/env bash
#
# Source-of-truth for /usr/local/bin/runner-host-cleanup.sh on the shared
# self-hosted runner host (#706 item 5 — the host copy previously lived
# only on the machine, so changes were unreviewed and undocumented).
#
# Deploy: install this file at /usr/local/bin/runner-host-cleanup.sh
# (root-owned, 0755) and enable scripts/systemd/runner-host-cleanup.timer.
#
# Reclaims disk on a host running ~12 GitHub Actions runner services whose
# Rust `target/` dirs are 20–40 GB each.
#
# Pressure tiers:
#   < 70% used          : nothing but logging.
#   >= SOFT (70%)       : soft cache trims ONLY (~/.cache/* — uv, pnpm,
#                         pip, grype, ms-playwright). Never touches
#                         _work/.../target.
#   >= AGGRESSIVE (73%): also wipes IDLE runners' _work directories.
#
# Active-job guard, defense in depth (#706 item 1):
#   1. pgrep -f "<runner>/bin..*Runner.Worker" — process match, as before.
#   2. NEW backstop: skip any candidate `_work` whose target/ mtime is
#      younger than ACTIVE_MTIME_MIN minutes. An active build touches
#      target/ constantly, so this holds even if the runner layout or
#      process name ever changes out from under the pgrep pattern.
#
# Aggressive events are LOUD (#706 item 6): every aggressive run logs a
# dedicated marker line and exits 4 so monitoring can alert on it — an
# aggressive wipe means disk pressure is already risking builds.
#
# Exit codes: 0 = clean / soft-only, 3 = a worktree was skipped as
# "don't know" (guard ambiguity), 4 = aggressive wipe fired.

set -uo pipefail

SOFT_THRESHOLD=70
AGGRESSIVE_THRESHOLD=73
ACTIVE_MTIME_MIN=15
WORK_DIR="${WORK_DIR:-/home/actions/actions-runner/_work}"
LOG_TAG="runner-host-cleanup"
AGGRESSIVE_FIRED=false

log() {
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ) [${LOG_TAG}] $*"
}

pct_used() {
    df --output=pcent "$WORK_DIR" | tail -1 | tr -dc '0-9'
}

# Returns 0 when the runner dir is SAFE to wipe (idle), 1 when active,
# 2 when ambiguous (treated as active — never delete on "don't know").
runner_is_idle() {
    local runner_dir="$1" runner_name
    runner_name=$(basename "$runner_dir")

    # Guard 1: live Runner.Worker process for this runner.
    if pgrep -f "${runner_name}/bin\..*Runner\.Worker" >/dev/null 2>&1; then
        return 1
    fi

    # Guard 2 (#706): recent target/ activity means a build is warm even
    # if the worker process isn't matched (layout change, between-jobs
    # artifact writing, etc.).
    local target_dir="$runner_dir/_work/*/target" # shellcheck disable=SC2086
    if find $target_dir -maxdepth 0 -mmin -"${ACTIVE_MTIME_MIN}" 2>/dev/null | grep -q .; then
        return 1
    fi

    return 0
}

soft_cache_trim() {
    # Only ~/.cache content — never _work. Safe under any pressure tier.
    for cache in uv pnpm pip grype ms-playwright; do
        local dir="/home/actions/.cache/$cache"
        [ -d "$dir" ] || continue
        case "$cache" in
            uv)        find "$dir" -type d -name '*' -mtime +7 -exec rm -rf {} + 2>/dev/null ;;
            pnpm)      pnpm store prune >/dev/null 2>&1 ;;
            *)         find "$dir" -depth -mtime +14 -exec rm -rf {} + 2>/dev/null ;;
        esac
    done
    log "soft cache trim complete"
}

main() {
    local used
    used=$(pct_used)
    log "disk ${used}% used (soft=${SOFT_THRESHOLD}%, aggressive=${AGGRESSIVE_THRESHOLD}%)"

    if [ "$used" -lt "$SOFT_THRESHOLD" ]; then
        exit 0
    fi

    soft_cache_trim
    used=$(pct_used)
    if [ "$used" -lt "$AGGRESSIVE_THRESHOLD" ]; then
        log "post-trim ${used}% — below aggressive threshold"
        exit 0
    fi

    AGGRESSIVE_FIRED=true
    log "AGGRESSIVE WIPE ARMED at ${used}% — wiping idle runners' _work"

    local skipped_unknown=0 wiped=0
    for runner_dir in "$(dirname "$WORK_DIR")"/*/; do
        [ -d "$runner_dir/_work" ] || continue
        runner_is_idle "$runner_dir"
        case $? in
            0)
                rm -rf "${runner_dir:?}/_work/target" 2>/dev/null \
                    && { wiped=$((wiped + 1)); log "wiped targets under $runner_dir"; } ;;
            1) : ;; # active — guarded
            *) skipped_unknown=$((skipped_unknown + 1));;
        esac
    done

    if [ "$skipped_unknown" -gt 0 ]; then
        log "$skipped_unknown runner(s) skipped with ambiguous state — NOT deleted"
    fi
    log "aggressive pass complete: $wiped runner(s) reclaimed"
}

main
if [ "$AGGRESSIVE_FIRED" = true ]; then
    exit 4
fi
exit 0
