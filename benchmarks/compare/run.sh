#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
HERE="$ROOT/benchmarks/compare"
MOCKFORGE_BIN="${MOCKFORGE_BIN:-$ROOT/target/release/mockforge}"
WIREMOCK_VERSION="${WIREMOCK_VERSION:-3.13.0}"
PRISM_VERSION="${PRISM_VERSION:-5.12.0}"
MOCKOON_VERSION="${MOCKOON_VERSION:-9.2.0}"
PORT="${PORT:-4010}"
RUNS="${RUNS:-5}"
CONCURRENCY="${CONCURRENCY:-16}"
DURATION="${DURATION:-15s}"
SPEC="$HERE/specs/petstore.json"

if [[ "${1:-}" == "--dry-run" ]]; then
  printf 'MOCKFORGE_BIN=%q\n' "$MOCKFORGE_BIN"
  printf 'MOCKFORGE_BIN serve --spec %q --http-port %q\n' "$SPEC" "$PORT"
  printf 'docker run --rm -p %q:8080 wiremock/wiremock:%s --global-response-templating\n' "$PORT" "$WIREMOCK_VERSION"
  printf 'npx --yes @stoplight/prism-cli@%s mock -h 0.0.0.0 -p %q %q\n' "$PRISM_VERSION" "$PORT" "$SPEC"
  printf 'npx --yes @mockoon/cli@%s start --data %q --port %q\n' "$MOCKOON_VERSION" "$HERE/generated/mockoon-petstore.json" "$PORT"
  printf 'python3 %q --binary %q --runs %q --concurrency %q --duration %q --port %q --load-client %q\n' "$HERE/measure.py" "$MOCKFORGE_BIN" "$RUNS" "$CONCURRENCY" "$DURATION" "$PORT" "<oha|wrk|python3 load.py>"
  exit 0
fi

for cmd in python3 curl; do command -v "$cmd" >/dev/null || { echo "missing required command: $cmd" >&2; exit 2; }; done
[[ -x "$MOCKFORGE_BIN" ]] || { echo "MockForge binary not executable: $MOCKFORGE_BIN" >&2; exit 2; }
if command -v oha >/dev/null; then LOAD_CLIENT=(oha)
elif command -v wrk >/dev/null; then LOAD_CLIENT=(wrk)
else LOAD_CLIENT=(python3 "$HERE/load.py"); fi

mkdir -p "$HERE/generated" "$HERE/results"
python3 "$HERE/adapters.py" "$HERE/specs" "$HERE/generated"
python3 "$HERE/measure.py" --binary "$MOCKFORGE_BIN" --runs "$RUNS" --concurrency "$CONCURRENCY" --duration "$DURATION" --port "$PORT" --wiremock-version "$WIREMOCK_VERSION" --prism-version "$PRISM_VERSION" --mockoon-version "$MOCKOON_VERSION" --load-client "${LOAD_CLIENT[@]}"
