#!/bin/bash
set -euo pipefail

if [ $# -eq 0 ]; then
  echo "Usage: $0 <cargo-release-args...>" >&2
  echo "Example: $0 patch --no-push --execute" >&2
  exit 1
fi

scripts/check-changelog.sh

exec cargo release "$@"
