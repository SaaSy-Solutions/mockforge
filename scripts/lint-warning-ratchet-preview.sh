#!/usr/bin/env bash
set -euo pipefail

# Non-blocking preview for the next warning ratchet candidate.
BASE_RUSTFLAGS="${RUSTFLAGS:-}"
if [[ -n "${BASE_RUSTFLAGS}" ]]; then
  export RUSTFLAGS="${BASE_RUSTFLAGS} -Awarnings -Dunused_must_use -Dprivate_interfaces -Dunused_qualifications -Dunused_imports -Dunused_mut -Dunused_variables"
else
  export RUSTFLAGS="-Awarnings -Dunused_must_use -Dprivate_interfaces -Dunused_qualifications -Dunused_imports -Dunused_mut -Dunused_variables"
fi

echo "Running warning ratchet preview (adds unused_variables)..."
cargo check -p mockforge-cli --all-targets --all-features --quiet
cargo check -p mockforge-ui --all-targets --all-features --quiet
echo "Ratchet preview passed."
