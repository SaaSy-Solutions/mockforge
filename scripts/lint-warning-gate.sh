#!/usr/bin/env bash
set -euo pipefail

# Incremental lint gate:
# - Ignore existing warning debt for now.
# - Fail CI for the currently ratcheted warning classes.
BASE_RUSTFLAGS="${RUSTFLAGS:-}"
if [[ -n "${BASE_RUSTFLAGS}" ]]; then
  export RUSTFLAGS="${BASE_RUSTFLAGS} -Awarnings -Dunused_must_use -Dprivate_interfaces -Dunused_qualifications -Dunused_imports -Dunused_mut"
else
  export RUSTFLAGS="-Awarnings -Dunused_must_use -Dprivate_interfaces -Dunused_qualifications -Dunused_imports -Dunused_mut"
fi

echo "Running incremental warning gate for mockforge-cli and mockforge-ui..."
cargo check -p mockforge-cli --all-targets --all-features --quiet
cargo check -p mockforge-ui --all-targets --all-features --quiet
echo "Warning gate passed."
