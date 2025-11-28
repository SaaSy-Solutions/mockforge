#!/bin/bash
set -euo pipefail

if [ $# -ne 1 ]; then
  echo "Usage: $0 <version>" >&2
  exit 1
fi

VERSION="$1"
DATE="$(date -u +%Y-%m-%d)"

LAST_RELEASE_COMMIT=$(git rev-list --grep='^chore: release ' HEAD --max-count=1 || true)
if [ -z "$LAST_RELEASE_COMMIT" ]; then
  LAST_RELEASE_COMMIT=$(git rev-list --max-parents=0 HEAD)
fi

LOG_OUTPUT=$(git log --no-merges --pretty=format:%s "$LAST_RELEASE_COMMIT"..HEAD)
FILTERED_CHANGES=()
while IFS= read -r line; do
  case "$line" in
    "docs: update changelog"*) continue ;;
    "chore: release "*) continue ;;
    "") continue ;;
  esac
  FILTERED_CHANGES+=("- $line")
done <<< "$LOG_OUTPUT"

if [ ${#FILTERED_CHANGES[@]} -eq 0 ]; then
  echo "No changes found since last release to populate changelog." >&2
  exit 1
fi

CHANGE_BLOCK=$(printf '%s\n' "${FILTERED_CHANGES[@]}")
export CHANGE_BLOCK VERSION DATE

python3 - "$VERSION" "$DATE" <<'PY'
import re
import sys
import os
from pathlib import Path

version, date = sys.argv[1:3]
change_block = os.environ["CHANGE_BLOCK"]

stub = """## [Unreleased]\n\n### Added\n\n- Nothing yet.\n\n### Changed\n\n- Nothing yet.\n\n### Deprecated\n\n- Nothing yet.\n\n### Removed\n\n- Nothing yet.\n\n### Fixed\n\n- Nothing yet.\n\n### Security\n\n- Nothing yet.\n\n"""

# NOTE: Remember to tag changelog entries with pillars!
# Format: - **[Pillar] Feature description** or - **[Pillar1][Pillar2] Feature description**
# Pillars: [Reality], [Contracts], [DevX], [Cloud], [AI]
# See docs/PILLARS.md for pillar definitions and examples.
section = f"## [{version}] - {date}\n\n### Changes\n\n{change_block}\n\n"

for path_str in ("CHANGELOG.md", "book/src/reference/changelog.md"):
    path = Path(path_str)
    text = path.read_text()
    m = re.search(r"## \[Unreleased\](.*?)(?=^## \[|\Z)", text, re.S | re.M)
    if not m:
        raise SystemExit(f"Unable to locate Unreleased section in {path}")
    rest = text[m.end():].lstrip('\n')
    new_text = stub + section + rest
    path.write_text(new_text)
PY
