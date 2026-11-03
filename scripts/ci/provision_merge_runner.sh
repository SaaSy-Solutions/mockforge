#!/usr/bin/env bash
# Run over SSH as root on the existing FSN02 host; registers separately.
set -euo pipefail
case "${1:-}" in 01|02) slot="$1" ;; *) echo 'Expected slot 01 or 02' >&2; exit 2 ;; esac
root="/tmp/mockforge-merge-ci-$slot"
account="mfmerge$slot"
if id "$account" >/dev/null 2>&1 || test -e "$root"; then
  echo 'Runner account or directory already exists; inspect before retrying.' >&2
  exit 1
fi
dpkg -s protobuf-compiler libfontconfig1-dev libglib2.0-dev libgtk-3-dev libsoup2.4-dev libasound2-dev libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev >/dev/null
for tool in docker curl jq python3 psql; do command -v "$tool"; done
useradd --system --home-dir "$root" --create-home --shell /bin/bash --groups docker "$account"
install -d -o "$account" -g "$account" "$root/.cargo/bin" "$root/tmp"
install -o "$account" -g "$account" /home/actions/.cargo/bin/rustup "$root/.cargo/bin/rustup"
runuser -u "$account" -- env HOME="$root" CARGO_HOME="$root/.cargo" RUSTUP_HOME="$root/.rustup" PATH="$root/.cargo/bin:/usr/local/bin:/usr/bin:/bin" bash -s -- "$root" <<'SETUP'
set -euo pipefail
cd "$1"
curl -fL --retry 3 https://github.com/actions/runner/releases/download/v2.338.2/actions-runner-linux-x64-2.338.2.tar.gz -o runner.tar.gz
echo 'baebf204307bac6193480852732f0aba2ae97b6c5597c244f4d5558390147c40  runner.tar.gz' | sha256sum --check
tar -xzf runner.tar.gz
rustup toolchain install 1.96.0 --profile minimal --component clippy --component rustfmt
for name in cargo rustc rustdoc rustfmt cargo-fmt cargo-clippy clippy-driver; do
  if ! test -x ".cargo/bin/$name"; then ln -s rustup ".cargo/bin/$name"; fi
done
rustup default 1.96.0
docker version --format '{{.Server.Version}}'
SETUP
echo "Prepared $root; registration and bounded service startup are separate."
