#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
HERE="$ROOT/benchmarks/compare"
MOCKFORGE_BIN="${MOCKFORGE_BIN:-$ROOT/target/release/mockforge}"
WIREMOCK_VERSION="${WIREMOCK_VERSION:-3.13.2}"
PRISM_VERSION="${PRISM_VERSION:-5.16.0}"
MOCKOON_VERSION="${MOCKOON_VERSION:-9.10.0}"
OHA_VERSION="1.16.0"
LOAD_CLIENT="${LOAD_CLIENT:-oha}"
PORT="${PORT:-4010}"
RUNS="${RUNS:-5}"
CONCURRENCY="${CONCURRENCY:-16}"
DURATION="${DURATION:-15s}"
WARMUP="${WARMUP:-1}"
WARMUP_DURATION="${WARMUP_DURATION:-2s}"
SPEC="$HERE/specs/petstore.json"
OHA_BIN="$HERE/.tools/bin/oha"
if [[ "$LOAD_CLIENT" == "python" ]]; then
  LOAD_CLIENT_CMD=("$HERE/.venv/bin/python" "$HERE/load.py")
elif [[ "$LOAD_CLIENT" == "oha" ]]; then
  LOAD_CLIENT_CMD=("$OHA_BIN")
else
  echo 'LOAD_CLIENT must be oha or python' >&2
  exit 2
fi

if [[ "${1:-}" == "--dry-run" ]]; then
  printf 'MOCKFORGE_BIN=%q\n' "$MOCKFORGE_BIN"
  printf 'MOCKFORGE_BIN serve --spec %q --http-port %q\n' "$SPEC" "$PORT"
  printf 'docker run --rm -p %q:8080 wiremock/wiremock:%s --global-response-templating\n' "$PORT" "$WIREMOCK_VERSION"
  printf 'npx --yes @stoplight/prism-cli@%s mock -h 0.0.0.0 -p %q %q\n' "$PRISM_VERSION" "$PORT" "$SPEC"
  printf 'npx --yes @mockoon/cli@%s start --data %q --port %q\n' "$MOCKOON_VERSION" "$HERE/specs/petstore.json" "$PORT"
  if [[ "$LOAD_CLIENT" == "oha" ]]; then
    printf 'oha version: %s; install if missing: cargo install oha --version %s --locked --root %q\n' "$OHA_VERSION" "$OHA_VERSION" "$HERE/.tools"
  fi
  printf 'fixtures: petstore, large_api_100_endpoints; load client: %s\n' "$LOAD_CLIENT"
  printf 'python3 %q --binary %q --runs %q --warmup %q --warmup-duration %q --concurrency %q --duration %q --port %q --load-client %q' "$HERE/measure.py" "$MOCKFORGE_BIN" "$RUNS" "$WARMUP" "$WARMUP_DURATION" "$CONCURRENCY" "$DURATION" "$PORT" "${LOAD_CLIENT_CMD[0]}"
  if [[ "${#LOAD_CLIENT_CMD[@]}" -gt 1 ]]; then printf ' %q' "${LOAD_CLIENT_CMD[@]:1}"; fi
  printf '\n'
  exit 0
fi

for cmd in python3 curl; do command -v "$cmd" >/dev/null || { echo "missing required command: $cmd" >&2; exit 2; }; done
[[ -x "$MOCKFORGE_BIN" ]] || { echo "MockForge binary not executable: $MOCKFORGE_BIN" >&2; exit 2; }

mkdir -p "$HERE/generated" "$HERE/results"
if [[ "$LOAD_CLIENT" != "python" && ! -x "$OHA_BIN" ]]; then
  cargo install oha --version "$OHA_VERSION" --locked --root "$HERE/.tools"
fi
python3 -m venv "$HERE/.venv"
"$HERE/.venv/bin/python" -m pip install --disable-pip-version-check -r "$HERE/requirements.txt"
"$HERE/.venv/bin/python" -c 'import jsonschema' || { echo 'required dependency jsonschema is unavailable in benchmark venv' >&2; exit 2; }
"$HERE/.venv/bin/python" "$HERE/adapters.py" "$HERE/specs" "$HERE/generated"
"$HERE/.venv/bin/python" "$HERE/measure.py" --binary "$MOCKFORGE_BIN" --runs "$RUNS" --warmup "$WARMUP" --warmup-duration "$WARMUP_DURATION" --concurrency "$CONCURRENCY" --duration "$DURATION" --port "$PORT" --wiremock-version "$WIREMOCK_VERSION" --prism-version "$PRISM_VERSION" --mockoon-version "$MOCKOON_VERSION" --load-client "${LOAD_CLIENT_CMD[@]}"
