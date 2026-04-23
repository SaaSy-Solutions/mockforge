#!/usr/bin/env bash
#
# Publish every MockForge workspace crate to crates.io at the current
# `[workspace.package]` version, in dependency order.
#
# Why not just `cargo publish --workspace`? Because:
#   1. crates.io rejects path-only deps (`path = "..."` without `version = ...`)
#      but our local Cargo.toml files use path-only so workspace builds pick
#      the in-repo source. We patch the Cargo.toml files temporarily before
#      each publish and `git checkout` them after — this is the surgery.
#   2. crates.io rate-limits new-crate creation (~10 / 10 min window). We
#      parse the `Retry-After` header on 429s and wait automatically.
#   3. Some internal crates are marked `publish = false` but still need to
#      ship whenever a workspace dependent references them (e.g.
#      `mockforge-registry-core`). We flip that to `true` in the temp edit.
#   4. crates.io indexing is asynchronous — the HTTP 200 on upload doesn't
#      mean the next `cargo publish` can resolve the version yet. We poll.
#
# Usage:
#   export CARGO_REGISTRY_TOKEN=<token>   # or use `cargo login` first
#   ./scripts/publish-crates.sh           # publish everything
#   ./scripts/publish-crates.sh --dry-run # pack only, no upload
#   ./scripts/publish-crates.sh --only mockforge-foundation,mockforge-core
#
# The crate order below was computed by a topological sort over the
# workspace's `mockforge-*` path deps (see scripts/compute-publish-order.py
# or just re-run the Python snippet in the commit message of this file).
# Re-sort when the dep graph changes — the script does NOT derive the
# order at runtime because that would require parsing TOML inside bash.

set -uo pipefail

VERSION=$(
  python3 - <<'PY'
import tomllib
from pathlib import Path
print(tomllib.loads(Path("Cargo.toml").read_text())["workspace"]["package"]["version"])
PY
)

DRY_RUN=false
ONLY_LIST=""
INDEX_TIMEOUT=120

while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) DRY_RUN=true; shift ;;
    --only)    ONLY_LIST="$2"; shift 2 ;;
    --index-timeout) INDEX_TIMEOUT="$2"; shift 2 ;;
    -h|--help)
      sed -n 's/^# \{0,1\}//;3,/^$/p' "$0" | head -40
      exit 0 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

if [ "$DRY_RUN" = false ] && [ -z "${CARGO_REGISTRY_TOKEN:-}" ] && [ -z "${CRATES_IO_TOKEN:-}" ]; then
  echo "warn: no CARGO_REGISTRY_TOKEN / CRATES_IO_TOKEN in env; falling back to ~/.cargo/credentials.toml" >&2
fi
if [ -n "${CRATES_IO_TOKEN:-}" ] && [ -z "${CARGO_REGISTRY_TOKEN:-}" ]; then
  export CARGO_REGISTRY_TOKEN="$CRATES_IO_TOKEN"
fi

# Dep-ordered publish list. Regenerate when adding new workspace crates.
CRATES=(
  mockforge-analytics
  mockforge-config
  mockforge-data
  mockforge-plugin-cli
  mockforge-plugin-core
  mockforge-plugin-registry
  mockforge-registry-core
  mockforge-security-core
  mockforge-template-expansion
  mockforge-tracing
  mockforge-tui
  mockforge-tunnel
  mockforge-foundation
  mockforge-observability
  mockforge-plugin-loader
  mockforge-plugin-sdk
  mockforge-contracts
  mockforge-openapi
  mockforge-core
  mockforge-amqp
  mockforge-federation
  mockforge-ftp
  mockforge-graphql
  mockforge-grpc
  mockforge-import
  mockforge-intelligence
  mockforge-kafka
  mockforge-mqtt
  mockforge-performance
  mockforge-proxy
  mockforge-route-chaos
  mockforge-runtime-daemon
  mockforge-scenarios
  mockforge-smtp
  mockforge-tcp
  mockforge-workspace
  mockforge-world-state
  mockforge-ws
  mockforge-pipelines
  mockforge-recorder
  mockforge-bench
  mockforge-chaos
  mockforge-collab
  mockforge-http
  mockforge-k8s-operator
  mockforge-registry-server
  mockforge-reporting
  mockforge-sdk
  mockforge-test
  mockforge-vbr
  mockforge-ui
  mockforge-cli
)

blue()  { printf '\033[0;34m%s\033[0m\n' "$*"; }
green() { printf '\033[0;32m%s\033[0m\n' "$*"; }
yellow(){ printf '\033[1;33m%s\033[0m\n' "$*"; }
red()   { printf '\033[0;31m%s\033[0m\n' "$*"; }

# Query crates.io for the latest version of a given crate.
# Prints `NONE` when the crate doesn't exist, `QUERYFAIL` on network errors.
crate_latest_version() {
  local crate=$1
  curl -sS -m 10 "https://crates.io/api/v1/crates/$crate" 2>/dev/null | \
    python3 -c "import sys,json
try:
  d=json.load(sys.stdin)
  print(d.get('crate',{}).get('max_version') or 'NONE')
except Exception:
  print('QUERYFAIL')" 2>/dev/null || echo "QUERYFAIL"
}

# Rewrite a crate's Cargo.toml so it can be published:
#   - Adds `version = "<VER>"` to every `mockforge-X = { path = "..." }` that
#     lacks one (crates.io rejects path-only deps).
#   - Flips `publish = false` -> `publish = true` so internal crates that
#     are needed transitively can still ship.
# The changes are reverted by `git checkout` after publish.
rewrite_cargo_toml_for_publish() {
  python3 - "$1" "$VERSION" <<'PY'
import re, sys
from pathlib import Path
p = Path(sys.argv[1]); v = sys.argv[2]
text = p.read_text()

# 1) Inline-table path deps: `mockforge-X = { ... path = ... }` without version.
pattern = re.compile(
    r'^(?P<ind>\s*)(?P<name>mockforge-[a-z0-9-]+)\s*=\s*\{(?P<body>[^}]*path\s*=[^}]*)\}\s*$',
    re.MULTILINE,
)
def add_version(m):
  body = m.group('body')
  if 'version' in body:
    return m.group(0)
  body = body.strip().lstrip(',').strip()
  return f'{m.group("ind")}{m.group("name")} = {{ version = "{v}", {body} }}'
text = pattern.sub(add_version, text)

# 2) `publish = false` -> `publish = true`
text = re.sub(
    r'^(publish\s*=\s*)false(\s*(?:#.*)?)$',
    r'\1true\2',
    text,
    flags=re.MULTILINE,
)

p.write_text(text)
PY
}

# Wait for crates.io to serve a specific version of a crate. Used between
# publishes because the upload returning 200 doesn't mean dependents can
# resolve it yet.
wait_for_indexing() {
  local crate=$1
  local deadline=$(( $(date +%s) + INDEX_TIMEOUT ))
  while [ "$(date +%s)" -lt "$deadline" ]; do
    if [ "$(crate_latest_version "$crate")" = "$VERSION" ]; then
      echo "  indexed on crates.io"
      return 0
    fi
    sleep 5
  done
  yellow "  WARN: $crate@$VERSION not visible after ${INDEX_TIMEOUT}s; continuing"
}

# Parse a `Retry-After: <seconds>` line or `try again after <GMT date>` from
# cargo publish output. Returns 0 and the seconds to wait, or 0 if unknown.
# We use `until <GMT>` because crates.io's error message formats that way.
retry_after_seconds() {
  local log=$1
  # crates.io form: "Please try again after Thu, 23 Apr 2026 18:46:50 GMT"
  local when
  when=$(grep -oE 'try again after [A-Za-z]{3}, [0-9]{1,2} [A-Za-z]{3} [0-9]{4} [0-9:]{8} GMT' "$log" | head -1 | sed 's/try again after //')
  if [ -n "$when" ]; then
    local target
    target=$(date -u -d "$when" +%s 2>/dev/null || echo 0)
    if [ "$target" -gt 0 ]; then
      local now; now=$(date -u +%s)
      local diff=$(( target - now + 10 ))  # 10s grace
      [ "$diff" -lt 0 ] && diff=0
      echo "$diff"
      return
    fi
  fi
  echo 0
}

# Publish one crate. Returns 0 on success or if already at target version.
publish_one() {
  local crate=$1
  local cargo_toml="crates/$crate/Cargo.toml"

  if [ ! -f "$cargo_toml" ]; then
    yellow "  skip: $cargo_toml does not exist"
    return 0
  fi

  # Skip if we're filtering and this crate isn't in the list.
  if [ -n "$ONLY_LIST" ] && [[ ! ",$ONLY_LIST," == *",$crate,"* ]]; then
    return 0
  fi

  local current; current=$(crate_latest_version "$crate")
  if [ "$current" = "$VERSION" ]; then
    green "  skip: already at $VERSION"
    return 0
  fi
  blue "  crates.io: $current  ->  publishing $VERSION"

  rewrite_cargo_toml_for_publish "$cargo_toml"

  local log=/tmp/publish-$$-$crate.log
  local flags="--no-verify --allow-dirty"
  if [ "$DRY_RUN" = true ]; then flags="$flags --dry-run"; fi

  # Retry up to 3 times on rate-limit (429). On other errors, give up (the
  # per-crate error lands in the run summary; other crates can still ship).
  local attempts=0
  while : ; do
    attempts=$(( attempts + 1 ))
    if cargo publish $flags -p "$crate" 2>&1 | tee "$log" | sed 's/^/    /'; then
      break
    fi
    if grep -q '429 Too Many Requests' "$log"; then
      local wait_s; wait_s=$(retry_after_seconds "$log")
      if [ "$wait_s" -gt 0 ] && [ "$attempts" -le 3 ]; then
        yellow "  429 rate limit; sleeping ${wait_s}s (attempt $attempts/3)"
        sleep "$wait_s"
        continue
      fi
    fi
    red "  publish failed"
    rm -f "$log"
    git checkout -- "$cargo_toml" 2>/dev/null
    return 1
  done
  rm -f "$log"
  git checkout -- "$cargo_toml" 2>/dev/null

  [ "$DRY_RUN" = true ] && return 0
  sleep 2
  wait_for_indexing "$crate"
}

# Snapshot & final summary
summary() {
  echo
  echo "=== summary ==="
  local ok=0 stale=0
  for crate in "${CRATES[@]}"; do
    local v; v=$(crate_latest_version "$crate")
    local flag="OK"
    if [ "$v" != "$VERSION" ]; then
      flag="STALE"
      stale=$(( stale + 1 ))
    else
      ok=$(( ok + 1 ))
    fi
    printf "  %-32s %-12s %s\n" "$crate" "$v" "$flag"
  done
  echo
  green "  $ok/${#CRATES[@]} at $VERSION"
  [ "$stale" -gt 0 ] && yellow "  $stale remain stale; re-run after fixing"
}

cd "$(dirname "$0")/.."

blue "publishing MockForge workspace @ $VERSION"
[ "$DRY_RUN" = true ] && yellow "DRY RUN — no uploads"
[ -n "$ONLY_LIST" ]   && yellow "only: $ONLY_LIST"

for crate in "${CRATES[@]}"; do
  echo
  blue "=== $crate ==="
  publish_one "$crate" || true
done

summary
