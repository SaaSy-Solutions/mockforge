#!/usr/bin/env bash
# Guard: every `FROM rust:<version>` builder stage must match the workspace
# toolchain pinned in rust-toolchain.toml.
#
# Why this exists: the Dockerfiles drifted to 1.75 / 1.90 / 1.91 while
# rust-toolchain.toml (and CI) moved to 1.96.0. Nothing caught it, because a
# stale builder keeps working right up until a transitive dependency raises its
# MSRV past the pinned version. On 2026-07-30 that happened:
#
#   error: rustc 1.91.1 is not supported by the following package:
#     aws-smithy-types@1.6.1 requires rustc 1.94.1
#
# which broke `flyctl deploy --config fly.registry.toml` — i.e. production
# registry deploys — with a failure that has nothing to do with the change
# being deployed. The failure mode is silent and time-delayed, so it needs a
# guard rather than vigilance.
#
# Usage: scripts/check-dockerfile-rust-drift.sh

set -euo pipefail

unset CDPATH
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." >/dev/null && pwd)"
cd "$REPO_ROOT" >/dev/null

TOOLCHAIN_FILE="rust-toolchain.toml"
[ -f "$TOOLCHAIN_FILE" ] || { echo "error: $TOOLCHAIN_FILE not found" >&2; exit 1; }

# channel = "1.96.0"  ->  1.96.0
pinned="$(grep -E '^\s*channel\s*=' "$TOOLCHAIN_FILE" | head -1 | sed -E 's/.*"([^"]+)".*/\1/')"
[ -n "$pinned" ] || { echo "error: could not parse channel from $TOOLCHAIN_FILE" >&2; exit 1; }

# Docker tags are usually major.minor (rust:1.96-slim) while the toolchain is
# major.minor.patch. Compare on major.minor so `rust:1.96-slim` satisfies
# `channel = "1.96.0"`.
pinned_mm="$(cut -d. -f1,2 <<<"$pinned")"

fail=0
found=0
while IFS= read -r line; do
  file="${line%%:*}"
  rest="${line#*:}"
  lineno="${rest%%:*}"
  version="$(sed -E 's/.*FROM rust:([0-9]+(\.[0-9]+)*).*/\1/' <<<"$line")"
  found=$((found + 1))
  version_mm="$(cut -d. -f1,2 <<<"$version")"
  if [ "$version_mm" != "$pinned_mm" ]; then
    echo "DRIFT  $file:$lineno  FROM rust:$version  (rust-toolchain.toml pins $pinned)"
    fail=1
  fi
done < <(grep -rnE '^FROM rust:[0-9]' --include='Dockerfile*' . 2>/dev/null || true)

if [ "$found" -eq 0 ]; then
  echo "warning: no 'FROM rust:<version>' stages found — did the Dockerfiles move?" >&2
fi

if [ "$fail" -ne 0 ]; then
  cat <<EOF

Docker builder images have drifted from the workspace toolchain.

Bump the offending 'FROM rust:<version>' lines to rust:$pinned_mm so container
builds use the same compiler as CI and local development. A stale builder does
not fail immediately — it fails whenever a dependency raises its MSRV, and it
fails on whatever change happens to be deploying at that moment.
EOF
  exit 1
fi

echo "OK: all $found rust builder stage(s) match rust-toolchain.toml ($pinned)"
