#!/bin/bash
set -euo pipefail

if [ $# -eq 0 ]; then
  echo "Usage: $0 <level|version> [cargo-release-args...]" >&2
  echo "Example: $0 patch --no-push --execute" >&2
  exit 1
fi

LEVEL="$1"
shift

EXECUTE=false
for arg in "$@"; do
  if [ "$arg" = "--execute" ]; then
    EXECUTE=true
    break
  fi
done

if [ "$EXECUTE" = false ]; then
  echo "This wrapper requires --execute so it can create the changelog commit." >&2
  echo "For dry runs, invoke cargo release directly." >&2
  exit 1
fi

if git status --porcelain | grep -Eq '^[^?]'; then
  echo "Working tree must be clean before running the release script." >&2
  exit 1
fi

compute_version() {
  case "$LEVEL" in
    patch|minor|major)
      python3 - "$LEVEL" <<'PY'
import sys
import tomllib
from pathlib import Path

level = sys.argv[1]
data = tomllib.loads(Path("Cargo.toml").read_text())
major, minor, patch = map(int, data["workspace"]["package"]["version"].split("."))
if level == "patch":
    patch += 1
elif level == "minor":
    minor += 1
    patch = 0
elif level == "major":
    major += 1
    minor = 0
    patch = 0
print(f"{major}.{minor}.{patch}")
PY
      ;;
    *)
      echo "$LEVEL"
      ;;
  esac
}

TARGET_VERSION=$(compute_version)

echo "📝 Updating changelog for version $TARGET_VERSION..."
echo "⚠️  REMINDER: Tag all changelog entries with pillars: [Reality], [Contracts], [DevX], [Cloud], [AI]"
echo "   See docs/PILLARS.md for pillar definitions and examples."
echo ""

scripts/update-changelog.sh "$TARGET_VERSION"
scripts/update-version-refs.sh "$TARGET_VERSION"

git add CHANGELOG.md book/src/reference/changelog.md README.md book/src/README.md
if ! git diff --cached --quiet; then
  git commit -m "docs: prepare release $TARGET_VERSION"
fi

scripts/check-changelog.sh

exec cargo release "$LEVEL" "$@"
