#!/usr/bin/env bash
# Build the React Admin UI using the project's declared package manager.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$script_dir/ui"
corepack pnpm run build
printf '%s\n' 'React UI build completed'
