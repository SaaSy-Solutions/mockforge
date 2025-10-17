#!/bin/bash
set -euo pipefail

# Ensure working tree is clean before release (cargo-release requirement)
if git status --porcelain | grep -Eq '^[^?]'; then
  echo "Working tree is dirty. Commit or stash changes before releasing." >&2
  exit 1
fi

changed_files=$(git diff-tree --no-commit-id --name-only -r HEAD)

if ! grep -qx 'CHANGELOG.md' <<<"$changed_files"; then
  echo "The latest commit does not update CHANGELOG.md. Add the changelog entry first." >&2
  exit 1
fi

if ! grep -qx 'book/src/reference/changelog.md' <<<"$changed_files"; then
  echo "The latest commit does not update book/src/reference/changelog.md. Keep the docs in sync before releasing." >&2
  exit 1
fi
