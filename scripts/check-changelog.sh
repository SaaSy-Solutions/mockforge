#!/bin/bash
set -euo pipefail

# Ensure working tree is clean before release (cargo-release requirement)
if git status --porcelain | grep -Eq '^[^?]'; then
  echo "Working tree is dirty. Commit or stash changes before releasing." >&2
  exit 1
fi

# Pick a "what changed in this change-set" diff range that survives both call
# sites. Two contexts:
#
#   1. CI on a pull_request event. `actions/checkout@v6` checks out the
#      synthetic merge commit at `refs/pull/<N>/merge`. `git diff-tree -r HEAD`
#      on a merge commit returns the *combined* diff (files where the two
#      parents disagreed and the merge had to choose) — which is empty for any
#      clean merge. So this script was previously failing CI runs whose
#      CHANGELOG.md edit lived in a non-merge-touching commit (e.g. the most
#      recent push amended *only* a different file), even though the PR
#      clearly modified CHANGELOG.md across its full diff.
#
#   2. Local cargo-release via `scripts/release.sh`. HEAD is the actual
#      version-bump commit on a regular branch; `diff-tree -r HEAD` returns
#      its real file list.
#
# GitHub Actions exports $GITHUB_BASE_REF in pull_request contexts (and only
# there), so we use it as the signal to switch modes.
if [ -n "${GITHUB_BASE_REF:-}" ]; then
  changed_files=$(git diff --name-only "origin/${GITHUB_BASE_REF}...HEAD")
else
  changed_files=$(git diff-tree --no-commit-id --name-only -r HEAD)
fi

if ! grep -qx 'CHANGELOG.md' <<<"$changed_files"; then
  echo "CHANGELOG.md was not modified in this change. Add the changelog entry first." >&2
  exit 1
fi

if ! grep -qx 'book/src/reference/changelog.md' <<<"$changed_files"; then
  echo "book/src/reference/changelog.md was not modified. Keep the docs in sync before releasing." >&2
  exit 1
fi

# Validate that changelog sections have at least one pillar tag
# Pillars: [Reality], [Contracts], [DevX], [Cloud], [AI]
check_pillar_tags() {
  local changelog_file="$1"
  local has_errors=0

  # Check [Unreleased] section
  local unreleased_section=$(awk '/^## \[Unreleased\]/{p=1} p{print} /^## \[[0-9]/ && p{exit}' "$changelog_file" 2>/dev/null || true)

  if [ -n "$unreleased_section" ]; then
    # Check if Unreleased section has entries (not just headers)
    local has_entries=$(echo "$unreleased_section" | grep -qE '^### (Added|Changed|Deprecated|Removed|Fixed|Security)' || echo "")

    if [ -n "$has_entries" ]; then
      # Check if the Unreleased section contains at least one pillar tag
      if ! echo "$unreleased_section" | grep -qE '\[(Reality|Contracts|DevX|Cloud|AI)\]'; then
        echo "ERROR: The [Unreleased] section in $changelog_file has no pillar tags!" >&2
        echo "       All changelog entries must be tagged with at least one pillar:" >&2
        echo "       [Reality], [Contracts], [DevX], [Cloud], or [AI]" >&2
        echo "       See docs/PILLARS.md for pillar definitions and examples." >&2
        echo "" >&2
        echo "       Example format:" >&2
        echo "       - **[Reality] Feature description**" >&2
        echo "       - **[Contracts][DevX] Multi-pillar feature**" >&2
        echo "" >&2
        has_errors=1
      fi
    fi
  fi

  # Extract the first version section (most recent release)
  # Skip the [Unreleased] section and get the first actual version.
  # The previous awk used `NR>1` to detect "this is the second version heading",
  # but that exits immediately when the first version heading is itself past
  # line 1 (e.g. book/src/reference/changelog.md has a preamble blockquote).
  # Track whether we already opened a section instead.
  local version_section=$(awk '/^## \[[0-9]/{ if (in_section) exit; in_section=1 } in_section{print}' "$changelog_file" 2>/dev/null || true)

  if [ -z "$version_section" ]; then
    # If no version section found, might be a new file or only Unreleased section
    # This is okay if we already checked Unreleased
    if [ $has_errors -eq 0 ]; then
      return 0
    else
      return 1
    fi
  fi

  # Check if the version section contains at least one pillar tag
  if ! echo "$version_section" | grep -qE '\[(Reality|Contracts|DevX|Cloud|AI)\]'; then
    echo "ERROR: The latest version section in $changelog_file has no pillar tags!" >&2
    echo "       All changelog entries must be tagged with at least one pillar:" >&2
    echo "       [Reality], [Contracts], [DevX], [Cloud], or [AI]" >&2
    echo "       See docs/PILLARS.md for pillar definitions and examples." >&2
    echo "" >&2
    echo "       Example format:" >&2
    echo "       - **[Reality] Feature description**" >&2
    echo "       - **[Contracts][DevX] Multi-pillar feature**" >&2
    echo "" >&2
    echo "       For more information, see:" >&2
    echo "       - docs/PILLARS.md - Complete pillar documentation" >&2
    echo "       - docs/contributing/PILLAR_TAGGING.md - How to tag features" >&2
    has_errors=1
  fi

  if [ $has_errors -eq 1 ]; then
    return 1
  fi

  return 0
}

# Check both changelog files
if ! check_pillar_tags "CHANGELOG.md"; then
  exit 1
fi

if ! check_pillar_tags "book/src/reference/changelog.md"; then
  exit 1
fi

echo "✅ Changelog validation passed: pillar tags found in new version sections."
