#!/usr/bin/env bash
# Remove an oversized runner-persistent release target after a release job.
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
expected="$root/cargo-mockforge-target-$runner/release"
target="${CARGO_TARGET_DIR:-}"
if [ -z "$target" ]; then
  echo "::error::CARGO_TARGET_DIR is empty; refusing to trim" >&2
  exit 1
fi
if [ "$target" != "$expected" ]; then
  echo "::error::CARGO_TARGET_DIR does not match expected runner release target; refusing to trim" >&2
  exit 1
fi
if [ -L "$target" ] || [ ! -d "$target" ]; then
  echo "::error::$target is not a plain directory; refusing to trim" >&2
  exit 1
fi

max_gb="${MOCKFORGE_RELEASE_TARGET_MAX_GB:-40}"
case "$max_gb" in
  '' | *[!0-9]*) echo "::error::MOCKFORGE_RELEASE_TARGET_MAX_GB must be a non-negative integer" >&2; exit 2 ;;
esac
size_kb=$(du -sk -- "$target" | awk '{print $1}')
max_kb=$((max_gb * 1024 * 1024))
echo "Runner release target size before trim: ${size_kb} KiB (cap: ${max_gb} GiB)"
if [ "$size_kb" -le "$max_kb" ]; then
  echo "Runner release target is under cap; keeping it."
  exit 0
fi

echo "Runner release target exceeds cap; removing $target"
rm -rf -- "${target:?}"
if [ -e "$target" ] || [ -L "$target" ]; then
  echo "::error::target still exists after removal" >&2
  exit 1
fi
echo "Runner release target size after trim: 0 KiB (removed)"
