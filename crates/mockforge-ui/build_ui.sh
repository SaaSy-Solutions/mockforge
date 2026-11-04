#!/usr/bin/env bash
# Build the React Admin UI using the project's declared package manager.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$script_dir/ui"
if [[ $# -eq 0 ]]; then
  corepack pnpm run build
elif [[ $# -eq 1 ]]; then
  owned_output="$(node - "$1" <<'NODE'
const fs = require('node:fs');
const path = require('node:path');
if (!process.env.OUT_DIR) throw new Error('Cargo OUT_DIR is required for an owned UI output');
const root = fs.realpathSync(process.env.OUT_DIR);
const supplied = path.resolve(process.argv[2]);
const output = path.join(fs.realpathSync(path.dirname(supplied)), path.basename(supplied));
if (output !== path.join(root, 'ui-dist')) throw new Error('UI output must be Cargo OUT_DIR/ui-dist');
const existing = fs.lstatSync(output, { throwIfNoEntry: false });
if (existing && !existing.isDirectory()) throw new Error('Owned UI output must be a directory, not a symlink or file');
process.stdout.write(output);
NODE
)"
  MOCKFORGE_UI_BUILD_OUT_DIR="$owned_output" corepack pnpm run build --outDir "$owned_output" --emptyOutDir
else
  printf '%s\n' 'Usage: build_ui.sh [Cargo OUT_DIR/ui-dist]' >&2
  exit 2
fi
printf '%s\n' 'React UI build completed'
