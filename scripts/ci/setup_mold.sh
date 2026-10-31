#!/usr/bin/env bash
# Link Rust test and binary targets with mold when the runner provides it (#1451).
#
# Linking is most of the PR test lane (14.5 of 18.2 min compiling+linking test
# binaries). mold is selected only after a probe link proves it works with the
# runner's C compiler, so a missing or broken mold never fails the job; it just
# keeps the default linker. Self-hosted runners are never modified here: mold is
# operator-provisioned (`apt-get install mold`), GitHub-hosted runners install
# it with rui314/setup-mold before this script runs.
#
# The flag is exported through RUSTFLAGS so every Cargo command in the job (and
# every job sharing the runner's persistent target dir) sees identical flags.
set -euo pipefail

probe_dir="$(mktemp -d "${RUNNER_TEMP:-/tmp}/mold-probe-XXXXXXXX")"
trap 'rm -rf -- "${probe_dir:?}"' EXIT
printf 'int main(void) { return 0; }\n' > "$probe_dir/probe.c"
cc_bin="${CC:-cc}"

uses_mold() { readelf -p .comment "$1" 2>/dev/null | grep -q mold; }

if ! command -v mold >/dev/null 2>&1; then
  echo "::notice::mold is not installed on this runner; using the default linker."
  exit 0
fi
mold --version

# Already the default linker (rui314/setup-mold replaces /usr/bin/ld).
if "$cc_bin" "$probe_dir/probe.c" -o "$probe_dir/default" && uses_mold "$probe_dir/default"; then
  echo "mold is already the default linker."
  exit 0
fi

flags=""
libexec="$(dirname "$(dirname "$(command -v mold)")")/libexec/mold"
if [[ -x "$libexec/ld" ]] && "$cc_bin" "$probe_dir/probe.c" -o "$probe_dir/b" "-B$libexec" && uses_mold "$probe_dir/b"; then
  flags="-C link-arg=-B$libexec"
elif "$cc_bin" "$probe_dir/probe.c" -o "$probe_dir/fuse" -fuse-ld=mold && uses_mold "$probe_dir/fuse"; then
  flags="-C link-arg=-fuse-ld=mold"
fi

if [[ -z "$flags" ]]; then
  echo "::notice::mold is installed but the C compiler cannot drive it; using the default linker."
  exit 0
fi
echo "Linking with mold: $flags"
echo "RUSTFLAGS=${RUSTFLAGS:+$RUSTFLAGS }$flags" >> "${GITHUB_ENV:?}"
