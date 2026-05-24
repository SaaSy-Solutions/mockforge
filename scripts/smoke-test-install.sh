#!/usr/bin/env bash
#
# Smoke-test the `cargo install mockforge-cli` path before publishing.
#
# Why this exists: v0.3.142 published 53 crates with a broken `mockforge-cli`
# because `cargo build --workspace` unified features across workspace members
# and masked an ungated `use crate::database::Database;` in `mockforge-http`
# (PR #611). End-users running `cargo install mockforge-cli` got a hard compile
# error on the default-features build — 18 crates were yanked, v0.3.143 was a
# hotfix. `cargo build --workspace` and `cargo publish --dry-run` did not catch
# it because workspace feature unification activated the missing `database`
# feature transitively.
#
# This script reproduces exactly what `cargo install mockforge-cli` does, so
# the same compile error aborts the release BEFORE the tag is published.
#
# Usage:
#   scripts/smoke-test-install.sh             # full check (build + install)
#   scripts/smoke-test-install.sh --fast      # build-only (skip the fresh-
#                                             # target-dir install simulation)

set -euo pipefail

FAST=false
while [ $# -gt 0 ]; do
  case "$1" in
    --fast) FAST=true; shift ;;
    -h|--help)
      sed -n '3,/^$/p' "$0" | sed 's/^# \{0,1\}//'
      exit 0 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

cd "$(dirname "$0")/.."

blue()  { printf '\033[0;34m%s\033[0m\n' "$*"; }
green() { printf '\033[0;32m%s\033[0m\n' "$*"; }
red()   { printf '\033[0;31m%s\033[0m\n' "$*" >&2; }

# Stage 1: build mockforge-cli with --locked and ONLY its default features.
# `-p mockforge-cli` (no --workspace) prevents feature unification across
# workspace members, so cargo resolves features the same way `cargo install`
# does. This is the cheapest reproduction of the v0.3.142 failure mode.
blue "==> [1/2] cargo build -p mockforge-cli --locked --release"
if ! cargo build -p mockforge-cli --locked --release; then
  red "FAIL: mockforge-cli default-features build failed."
  red "      This is the v0.3.142 failure mode — fix before tagging a release."
  red "      Check for crate-internal items used without a feature gate."
  exit 1
fi

if ! ./target/release/mockforge --version; then
  red "FAIL: target/release/mockforge --version did not exit cleanly."
  exit 1
fi
green "  default-features build + --version OK"

if [ "$FAST" = true ]; then
  green "smoke-test (--fast) PASSED"
  exit 0
fi

# Stage 2: simulate `cargo install --locked mockforge-cli` from local path in
# a fresh target dir. This catches issues that the workspace target dir could
# mask (e.g. cached artifacts compiled with a wider feature set during an
# earlier `cargo build --workspace`).
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

blue "==> [2/2] cargo install --locked --path crates/mockforge-cli --root $TMP --force"
if ! cargo install --locked --path crates/mockforge-cli --root "$TMP" --force; then
  red "FAIL: cargo install --locked --path failed."
  red "      Users running 'cargo install mockforge-cli' would hit the same error."
  exit 1
fi

if ! "$TMP/bin/mockforge" --version; then
  red "FAIL: installed binary did not exit cleanly on --version."
  exit 1
fi

green "smoke-test PASSED"
