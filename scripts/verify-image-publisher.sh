#!/usr/bin/env bash
# Fail closed before GHCR login on a dedicated trusted publisher VM.
set -euo pipefail

expected_user=mockforge-image-publish

die() { echo "::error title=Image runner isolation::${*}" >&2; exit 1; }

actual_user="$(id -un)"
[[ "$actual_user" == "$expected_user" ]] || die "expected $expected_user, got $actual_user"
uid="$(id -u)"
[[ "$uid" != 0 ]] || die 'root must not execute image jobs'
[[ ",$(id -nG | tr ' ' ',')," != *,docker,* ]] || die 'Docker group grants host root access'
if command -v sudo >/dev/null 2>&1 && sudo -n true >/dev/null 2>&1; then
  die 'passwordless sudo grants host root access'
fi

runtime="/run/user/${uid}"
[[ "${XDG_RUNTIME_DIR:-}" == "$runtime" ]] || die 'XDG_RUNTIME_DIR is not the private user runtime'
[[ "${DOCKER_HOST:-}" == "unix://${runtime}/docker.sock" ]] || die 'DOCKER_HOST must use the private rootless socket'
[[ -d "$runtime" ]] || die 'private runtime directory is missing'
[[ -S "$runtime/docker.sock" ]] || die 'private rootless Docker socket is missing'
for path in "$runtime" "$runtime/docker.sock"; do
  [[ "$(stat -c %u "$path")" == "$uid" ]] || die "$path is not owned by $expected_user"
  mode="$(stat -c %a "$path")"
(( (8#$mode & 077) == 0 )) || die "$path is accessible by another Unix user"
done

security_options="$(docker info --format '{{json .SecurityOptions}}')" || die 'rootless Docker is unavailable'
[[ "$security_options" == *'name=rootless'* ]] || die 'Docker daemon is not rootless'

[[ -n "${RUNNER_TEMP:-}" ]] || die 'RUNNER_TEMP is unset'
[[ "$(stat -c %u "$RUNNER_TEMP")" == "$uid" ]] || die 'runner temp is not owned by image user'
[[ "$RUNNER_TEMP" != /home/actions/* ]] || die 'shared actions workspace is forbidden'
[[ "$(hostname -s)" != saasy-ci-fsn-02 ]] || die 'publisher cannot share fsn-02 with rootful PR runners'
[[ "$(hostname -s)" != saasy-ci-runner ]] || die 'publisher cannot share the old CI host with rootful PR runners'
marker=/etc/mockforge-image-publish/isolated-host
[[ ! -L "$marker" ]] || die 'publisher attestation must not be a symlink'
  marker_dir="$(dirname "$marker")"
  [[ "$(stat -c %u "$marker_dir")" == 0 ]] || die 'publisher attestation directory must be root owned'
  dir_mode="$(stat -c %a "$marker_dir")"
  (( (8#$dir_mode & 022) == 0 )) || die 'publisher attestation directory is writable by another user'
  [[ -f "$marker" ]] || die 'publisher host isolation has not been attested'
[[ "$(stat -c %u "$marker")" == 0 ]] || die 'publisher attestation must be root owned'
mode="$(stat -c %a "$marker")"
(( (8#$mode & 022) == 0 )) || die 'publisher attestation is writable by another user'
[[ "$(cat "$marker")" == 'SaaSy-Solutions/mockforge:mockforge-image-publish' ]] || die 'wrong publisher host attestation'
echo "Image publisher isolation verified: user=$expected_user socket=$DOCKER_HOST"
