#!/usr/bin/env bash
#
# PostToolUse hook: auto-format Rust files immediately after Edit/Write/MultiEdit.
#
# Why this exists: qualifier shortenings (e.g. `std::time::Duration::` ->
# `Duration::`) can let rustfmt collapse a multi-line call onto one line, which
# then breaks CI's `cargo fmt --all --check` step. That has burned us repeatedly
# (#585->#589, #591/#592->#594). Formatting on write makes the working tree
# always-formatted, so the class of failure cannot reach CI.
#
# Contract: reads the PostToolUse JSON envelope on stdin, formats the touched
# file in place if it is a .rs file, and ALWAYS exits 0 (never blocks the tool).
set -uo pipefail

input=$(cat 2>/dev/null || true)
[ -z "$input" ] && exit 0

file=$(printf '%s' "$input" | python3 -c "import json,sys
try:
    d=json.load(sys.stdin)
    print(d.get('tool_input',{}).get('file_path',''))
except Exception:
    print('')" 2>/dev/null || true)

case "$file" in
  *.rs)
    if command -v rustfmt >/dev/null 2>&1 && [ -f "$file" ]; then
      # Edition matches workspace.package.edition (2021). Silent + best-effort:
      # a syntactically incomplete intermediate edit should never surface noise.
      rustfmt --edition 2021 "$file" >/dev/null 2>&1 || true
    fi
    ;;
esac

exit 0
