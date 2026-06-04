#!/usr/bin/env bash
#
# Publish-list drift guard (model-independent enforcement of the release-guardian
# agent's check #1). Wired as a pre-push hook so a state where a publishable
# workspace member is missing from scripts/publish-crates.sh cannot reach the
# remote, where it would later break `publish-crates.sh` (a downstream crate
# would fail to resolve the missing dependency on crates.io). See #584 / #795.
#
# Drift = a publishable member absent from the CRATES list  -> FAIL (blocks push).
# Orphan = an on-disk crate that is not a workspace member  -> WARN (exit 0).
#
# "Publishable" is derived from `cargo metadata` workspace membership, NOT a
# directory glob: a crates/mockforge-*/ dir that is not a workspace member is an
# orphan (never built, never published), not drift. (See #796.)
#
# Usage: scripts/check-publish-drift.sh
set -uo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT" || exit 0

if ! command -v cargo >/dev/null 2>&1; then
  echo "publish-drift: cargo not found, skipping" >&2
  exit 0
fi

# Publishable members (publish != false). In `cargo metadata`, publish == []
# means publish=false; null means publishable.
publishable=$(cargo metadata --no-deps --format-version 1 2>/dev/null | python3 -c "
import sys, json
try:
    pkgs = json.load(sys.stdin)['packages']
except Exception:
    sys.exit(0)
for p in pkgs:
    if p['name'].startswith('mockforge-') and p.get('publish') != []:
        print(p['name'])
" | sort -u)

# CRATES list block in publish-crates.sh.
listed=$(sed -n '64,120p' scripts/publish-crates.sh 2>/dev/null | grep -oE 'mockforge-[a-z0-9-]+' | sort -u)

drift=$(comm -23 <(printf '%s\n' "$publishable") <(printf '%s\n' "$listed"))

# Orphans: on-disk mockforge-* crate dirs that are not workspace members.
members=$(cargo metadata --no-deps --format-version 1 2>/dev/null | python3 -c "
import sys, json
try:
    print('\n'.join(p['name'] for p in json.load(sys.stdin)['packages']))
except Exception:
    pass
" | sort -u)
ondisk=$(ls -d crates/mockforge-*/ 2>/dev/null | sed 's#crates/##;s#/##' | sort -u)
orphans=$(comm -23 <(printf '%s\n' "$ondisk") <(printf '%s\n' "$members"))

if [ -n "$orphans" ]; then
  echo "publish-drift: WARN orphan crates (on disk, not workspace members; see #796):" >&2
  printf '  - %s\n' $orphans >&2
fi

if [ -n "$drift" ]; then
  echo "publish-drift: FAIL — publishable members missing from scripts/publish-crates.sh:" >&2
  printf '  - %s\n' $drift >&2
  echo "Fix: add each to the CRATES list (in dependency order), or set publish=false if it must not ship." >&2
  exit 1
fi

echo "publish-drift: OK — every publishable member is in the publish list"
exit 0
