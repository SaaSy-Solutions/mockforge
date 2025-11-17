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

# Validate that new version sections have at least one pillar tag
# Pillars: [Reality], [Contracts], [DevX], [Cloud], [AI]
check_pillar_tags() {
  local changelog_file="$1"

  # Extract the first version section (most recent release)
  # Skip the [Unreleased] section and get the first actual version
  local version_section=$(awk '/^## \[[0-9]/{p=1} p{print} /^## \[[0-9]/ && NR>1{exit}' "$changelog_file" 2>/dev/null || true)

  if [ -z "$version_section" ]; then
    # If no version section found, might be a new file or only Unreleased section
    # This is okay, skip validation
    return 0
  fi

  # Check if the version section contains at least one pillar tag
  if echo "$version_section" | grep -qE '\[(Reality|Contracts|DevX|Cloud|AI)\]'; then
    return 0
  else
    echo "ERROR: The new version section in $changelog_file has no pillar tags!" >&2
    echo "       All changelog entries must be tagged with at least one pillar:" >&2
    echo "       [Reality], [Contracts], [DevX], [Cloud], or [AI]" >&2
    echo "       See docs/PILLARS.md for pillar definitions and examples." >&2
    echo "" >&2
    echo "       Example format:" >&2
    echo "       - **[Reality] Feature description**" >&2
    echo "       - **[Contracts][DevX] Multi-pillar feature**" >&2
    return 1
  fi
}

# Check both changelog files
if ! check_pillar_tags "CHANGELOG.md"; then
  exit 1
fi

if ! check_pillar_tags "book/src/reference/changelog.md"; then
  exit 1
fi

echo "✅ Changelog validation passed: pillar tags found in new version sections."
