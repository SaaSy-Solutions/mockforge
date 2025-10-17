#!/bin/bash
set -euo pipefail

if ! git diff --cached --name-only | grep -q '^CHANGELOG\.md$'; then
  echo "Changelog not staged; aborting release." >&2
  exit 1
fi

if ! git diff --cached --name-only | grep -q '^book/src/reference/changelog\.md$'; then
  echo "Book changelog not staged; aborting release." >&2
  exit 1
fi
