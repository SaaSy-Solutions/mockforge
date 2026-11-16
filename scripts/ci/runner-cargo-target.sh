#!/usr/bin/env bash
# Print CARGO_TARGET_DIR for one job class in a disjoint runner namespace.
# Runner disks retain build output without uploading GitHub cache archives.
# This selector never deletes caches; disk reclamation happens in
# guard-runner-cargo-targets.sh, the separate, explicitly authorized
# operation, which runs at job start immediately before this selector.
# Per-runner/per-class components avoid
# ambiguity between runner-a and other-runner-a, even when class names overlap.
set -euo pipefail

ROOT="${MOCKFORGE_RUNNER_TARGET_ROOT:-/tmp}"

valid_name() {
  case "$1" in
    '' | . | .. | *[!A-Za-z0-9._-]*) return 1 ;;
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

runner_dir="$ROOT/cargo-mockforge-target-$runner"
dir="$runner_dir/$class"
for path in "$runner_dir" "$dir"; do
  if [ -L "$path" ] || { [ -e "$path" ] && [ ! -d "$path" ]; }; then
    echo "::error::$path exists and is not a plain directory; refusing to use it" >&2
    exit 1
  fi
done
mkdir -p "$dir"
echo "CARGO_TARGET_DIR=$dir"
