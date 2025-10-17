#!/bin/bash
set -euo pipefail

if [ $# -ne 1 ]; then
  echo "Usage: $0 <version>" >&2
  exit 1
fi

VERSION="$1"

python3 - "$VERSION" <<'PY'
import re
import sys
from pathlib import Path

version = sys.argv[1]
patterns = [
    (Path("README.md"), r'(mockforge-cli = ")([^"]*)(")'),
    (Path("book/src/README.md"), r'(mockforge-cli = ")([^"]*)(")'),
]

for path, pattern in patterns:
    text = path.read_text()
    new_text, count = re.subn(pattern, lambda m: f"{m.group(1)}{version}{m.group(3)}", text, count=1)
    if count == 0:
        raise SystemExit(f"Could not update version reference in {path}")
    path.write_text(new_text)
PY

