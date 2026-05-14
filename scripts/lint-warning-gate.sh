#!/usr/bin/env bash
set -euo pipefail

# Incremental lint gate:
# - Ignore existing warning debt for now.
# - Fail CI for the currently ratcheted warning classes.
#
# Most ratcheted lints are checked on mockforge-cli + mockforge-ui only because
# the backlog hadn't been drained elsewhere when those classes were promoted.
# `unused_qualifications` was fully drained workspace-wide by #500-#511, so it
# graduates here to a workspace-wide check below.
BASE_RUSTFLAGS="${RUSTFLAGS:-}"
if [[ -n "${BASE_RUSTFLAGS}" ]]; then
  export RUSTFLAGS="${BASE_RUSTFLAGS} -Awarnings -Dunused_must_use -Dprivate_interfaces -Dunused_qualifications -Dunused_imports -Dunused_mut"
else
  export RUSTFLAGS="-Awarnings -Dunused_must_use -Dprivate_interfaces -Dunused_qualifications -Dunused_imports -Dunused_mut"
fi

echo "Running incremental warning gate for mockforge-cli and mockforge-ui..."
cargo check -p mockforge-cli --all-targets --all-features --quiet
cargo check -p mockforge-ui --all-targets --all-features --quiet

# Workspace-wide `unused_qualifications` check. The backlog was drained in
# #501/#502/#504/#508/#511; this gate keeps it from re-accumulating. Limited
# to that single lint so the rest of the ratcheted set can keep widening
# independently on cli + ui.
echo "Running workspace-wide unused_qualifications check..."
WORKSPACE_RUSTFLAGS="${BASE_RUSTFLAGS:-} -Awarnings -Dunused_qualifications"
RUSTFLAGS="$WORKSPACE_RUSTFLAGS" cargo check --workspace --all-targets --all-features --quiet

echo "Warning gate passed."
