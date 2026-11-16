#!/usr/bin/env bash
# Reclaim disk from oversized runner-persistent cargo target namespaces.
# Without a cap these grow without bound (one runner reached 86G and filled
# its disk to 99% within 3 hours, killing every product's CI on that host;
# saas-platform#14178). runner-cargo-target.sh only ever selects a target
# dir and never deletes; this guard is the separate, explicitly authorized
# reclamation operation. Run it at job start, in the same workflow step and
# immediately before the selector.
# A runner executes one job at a time and the target namespace is private
# to that runner (keyed by RUNNER_NAME), so at job start no concurrent job
# holds these directories; reclaiming the whole namespace is safe.
set -euo pipefail

valid_name() {
  case "$1" in
    '' | . | .. | *[!A-Za-z0-9._-]*) return 1 ;;
  esac
}

runner="${RUNNER_NAME:-}"
if ! valid_name "$runner"; then
  echo "::error::RUNNER_NAME is missing or has unexpected characters" >&2
  exit 2
fi

root="${MOCKFORGE_RUNNER_TARGET_ROOT:-/tmp}"
runner_dir="$root/cargo-mockforge-target-$runner"

# Fresh runner (or first job after a boot): nothing to reclaim.
if [ ! -e "$runner_dir" ] && [ ! -L "$runner_dir" ]; then
  exit 0
fi
# A symlinked root or namespace would redirect the delete into another
# directory tree; refuse rather than remove anything.
if [ -L "$root" ] || { [ -e "$root" ] && [ ! -d "$root" ]; }; then
  echo "::error::$root is a symlink or not a plain directory; refusing to reclaim" >&2
  exit 1
fi
if [ -L "$runner_dir" ] || [ ! -d "$runner_dir" ]; then
  echo "::error::$runner_dir is a symlink or not a plain directory; refusing to reclaim" >&2
  exit 1
fi

# ${param-default} (no colon): an empty value is a set-but-invalid cap, not
# the default; only a fully unset variable falls back to 40.
max_gb="${MOCKFORGE_RUNNER_TARGET_MAX_GB-40}"
case "$max_gb" in
  '' | *[!0-9]*) echo "::error::MOCKFORGE_RUNNER_TARGET_MAX_GB must be a non-negative integer" >&2; exit 2 ;;
esac

# Measurement problems must never fail the job; only the unsafe-path
# refusals above exit non-zero.
if ! size_kb=$(du -sk -- "$runner_dir" | awk '{print $1}'); then
  echo "::warning::could not measure $runner_dir; skipping reclamation" >&2
  exit 0
fi
max_kb=$((max_gb * 1024 * 1024))
echo "Runner target namespace size before guard: ${size_kb} KiB (cap: ${max_gb} GiB)"
if [ "$size_kb" -le "$max_kb" ]; then
  echo "Runner target namespace is under cap; keeping it."
  exit 0
fi

# Remove immediate children only, enumerated explicitly: never a glob in
# rm, never the namespace itself, never its parent or the filesystem root.
# Anything whose basename is not a valid class name, or that is not a plain
# directory, is skipped with a warning.
while IFS= read -r -d '' child; do
  name="${child##*/}"
  if ! valid_name "$name"; then
    echo "::warning::skipping $child: basename is not a valid class name" >&2
    continue
  fi
  if [ -L "$child" ] || [ ! -d "$child" ]; then
    echo "::warning::skipping $child: not a plain directory" >&2
    continue
  fi
  rm -rf -- "${child:?}" || echo "::warning::failed to remove $child" >&2
done < <(find "$runner_dir" -mindepth 1 -maxdepth 1 -print0)

if ! after_kb=$(du -sk -- "$runner_dir" | awk '{print $1}'); then
  echo "::warning::could not re-measure $runner_dir after reclamation" >&2
else
  echo "Runner target namespace size after guard: ${after_kb} KiB"
fi
