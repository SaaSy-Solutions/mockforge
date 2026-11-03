#!/usr/bin/env bash
set -euo pipefail

# Incremental lint gate plus the next-ratchet preview, in one build (#1450):
# - Ignore existing warning debt for now.
# - Fail CI for the currently ratcheted warning classes.
# - Report, without failing, what the next candidate (unused_variables) finds.
#
# Most ratcheted lints are checked on mockforge-cli + mockforge-ui only because
# the backlog hadn't been drained elsewhere when those classes were promoted.
# `unused_qualifications` was fully drained workspace-wide by #500-#511, so it
# graduates here to a workspace-wide check below.
#
# Lint levels go through `cargo clippy -- <flags>` rather than RUSTFLAGS.
# Clippy applies them to workspace members only, so dependencies are compiled
# once with the job's ordinary flags and shared by every step (and with the
# runner's persistent target dir) instead of being rebuilt per lint set. All
# clippy-specific lints are allowed so this stays a rustc-lint gate.
# `--force-warn` lets the preview lint escape `-A warnings` without failing.
GATE_LINTS=(-Awarnings -Aclippy::all -Dunused_must_use -Dprivate_interfaces
  -Dunused_qualifications -Dunused_imports -Dunused_mut)
PREVIEW_LINT=unused_variables

log="$(mktemp "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/warning-gate-XXXXXXXX")"
trap 'rm -f -- "${log:?}"' EXIT

echo "Running incremental warning gate for mockforge-cli and mockforge-ui..."
for package in mockforge-cli mockforge-ui; do
  cargo clippy -p "$package" --all-targets --all-features --quiet -- \
    "${GATE_LINTS[@]}" --force-warn "$PREVIEW_LINT" 2>&1 | tee -a "$log"
done

# Workspace-wide `unused_qualifications` check. The backlog was drained in
# #501/#502/#504/#508/#511; this gate keeps it from re-accumulating. Limited
# to that single lint so the rest of the ratcheted set can keep widening
# independently on cli + ui.
echo "Running workspace-wide unused_qualifications check..."
cargo clippy --workspace --all-targets --all-features --quiet -- \
  -Awarnings -Aclippy::all -Dunused_qualifications

echo "Warning gate passed."

# Ratchet preview (non-blocking): count what promoting the next lint would hit.
preview_count="$(grep -c "^warning: unused variable" "$log" || true)"
if [[ "$preview_count" -gt 0 ]]; then
  echo "::warning title=Warning ratchet preview::${preview_count} ${PREVIEW_LINT} warning(s) would fail the next ratchet."
else
  echo "Ratchet preview passed (no ${PREVIEW_LINT} warnings)."
fi
