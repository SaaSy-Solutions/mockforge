#!/usr/bin/env bash
# RLS coverage gate (#832 / #960, design step (b)).
#
# Runs the registry-server E2E suite with the request-path pool bound to a
# NOSUPERUSER / NOBYPASSRLS role, so the RLS policies from migration
# `20250101000082_rls_tenant_isolation` are ACTIVE for every request-path
# query. Any covered-table query (projects, audit_logs, hosted_mocks,
# templates, scenarios) that runs without `app.current_org_id` bound
# fail-closes to 0 rows and the E2E assertion that depends on it fails.
#
# That is the point: this script is the forcing function for
# request-path RLS coverage. A pre-cutover audit found ~20 covered-table
# queries the store delegates to model methods on the shared pool with no
# GUC — activating the runtime role in production without covering them
# would have fail-closed live org queries. This gate makes that class of
# gap a test failure instead of an outage.
#
# Usage:
#   scripts/rls-e2e-gate.sh all            # up + test + down (CI shape)
#   scripts/rls-e2e-gate.sh up             # bring the stack up, leave it running
#   scripts/rls-e2e-gate.sh test [args..]  # run the suite against a live stack
#   scripts/rls-e2e-gate.sh control        # re-point the server at the owner pool
#                                          #   (RLS inert) to attribute a failure
#   scripts/rls-e2e-gate.sh rearm          # put it back on the NOBYPASSRLS role
#   scripts/rls-e2e-gate.sh psql-app       # psql shell as the NOBYPASSRLS role
#   scripts/rls-e2e-gate.sh down           # tear everything down
#
# Attribution matters more than it sounds: several of these E2E suites are
# `#[ignore]`d and never run in CI, so they can carry pre-existing failures.
# `up -> test` vs `control -> test` is how you tell "RLS broke this" from
# "this was already broken".
#
# `up` + `test` is the dev loop: bring the stack up once, then re-run `test`
# after each coverage change without paying the docker/migration cost again.
#
# Requires: docker (compose v2), cargo, curl, psql.

set -euo pipefail

# `unset CDPATH` + redirect: with CDPATH set (common in interactive shells that
# export it), `cd` echoes the resolved directory on stdout, which would end up
# concatenated into REPO_ROOT via the command substitution.
unset CDPATH
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." >/dev/null && pwd)"
cd "$REPO_ROOT" >/dev/null

# ---------------------------------------------------------------------------
# Configuration. This gate runs its OWN stack (docker-compose.rls-gate.yml) so
# it can never contend with the required "Registry E2E" job's stack.
# ---------------------------------------------------------------------------
# Deliberately NOT the ports in docker-compose.e2e.yml. The required
# "Registry E2E" job binds 55432 / 59000 / 58080 with fixed container names, and
# the self-hosted host runs several runners at once, so a shared stack lets this
# gate knock over the check that gates every PR (it did, on #965:
# "failed to bind host port 0.0.0.0:55432/tcp: address already in use").
# See docker-compose.rls-gate.yml.
COMPOSE_FILE_GATE="${COMPOSE_FILE_GATE:-docker-compose.rls-gate.yml}"

# Keep concurrent runs of THIS job apart. The CI host runs several runners side
# by side, so two PRs can be in this script at the same moment. Anything global
# to the docker daemon (a fixed container name) or to the host (a fixed port) is
# a collision waiting to happen — the first version of this gate used both and
# knocked over a run with `No such container` when one teardown raced another's
# `up`.
# Ports are ephemeral, so `up` must hand them to the later `test` / `down`
# invocations — otherwise each process picks different ones and `test` looks for
# a server that isn't there. `up`/`all` allocate fresh and write this file;
# every other subcommand reads it.
STATE_FILE="${STATE_FILE:-$REPO_ROOT/.rls-gate-state}"
__SUBCMD="${1:-all}"
if [ "$__SUBCMD" != "up" ] && [ "$__SUBCMD" != "all" ] && [ -f "$STATE_FILE" ]; then
  # shellcheck disable=SC1090
  . "$STATE_FILE"
fi

export COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-rlsgate-$$-${RUNNER_NAME:-local}}"

# Free ephemeral ports, chosen now, rather than fixed ones. Explicit
# PG_PORT/MINIO_PORT/REGISTRY_PORT still win so a local run can pin them.
pick_free_port() {
  python3 - <<'PYPORT'
import socket
s = socket.socket()
s.bind(('', 0))
print(s.getsockname()[1])
s.close()
PYPORT
}
PG_PORT="${PG_PORT:-$(pick_free_port)}"
MINIO_PORT="${MINIO_PORT:-$(pick_free_port)}"
MINIO_CONSOLE_PORT="${MINIO_CONSOLE_PORT:-$(pick_free_port)}"
REGISTRY_PORT="${REGISTRY_PORT:-$(pick_free_port)}"
export PG_PORT MINIO_PORT MINIO_CONSOLE_PORT

PG_SUPERUSER="postgres"
PG_SUPERPASS="password"
PG_DB="mockforge_registry"

# The unprivileged request-path role. Mirrors the prod role name in the #960
# rollout plan and the constant in tests/tenant_isolation_rls.rs.
APP_ROLE="${APP_ROLE:-mockforge_app}"
APP_ROLE_PASSWORD="${APP_ROLE_PASSWORD:-rls_app_pw}"

OWNER_URL="postgres://${PG_SUPERUSER}:${PG_SUPERPASS}@localhost:${PG_PORT}/${PG_DB}"
APP_URL="postgres://${APP_ROLE}:${APP_ROLE_PASSWORD}@localhost:${PG_PORT}/${PG_DB}"

STORAGE_PATH="${STORAGE_PATH:-${TMPDIR:-/tmp}/mf-rls-gate-storage}"
SERVER_LOG="${SERVER_LOG:-$REPO_ROOT/rls-gate-registry-server.log}"
SERVER_PID_FILE="$REPO_ROOT/rls-gate-registry-server.pid"

# Honor a shared CARGO_TARGET_DIR (worktrees commonly point at one target dir
# to avoid recompiling every registry dependency per checkout).
TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
SERVER_BIN="$TARGET_DIR/debug/mockforge-registry-server"

# The E2E targets the required "Registry E2E (Postgres + MinIO)" check runs.
# Kept in sync with the `Run E2E tests` step in .github/workflows/registry-e2e.yml.
E2E_TESTS=(
  --test workspace_content_e2e
  --test signup_flow_e2e
  --test marketplace_e2e
  --test cloud_verification_e2e
  --test cloud_ai_contract_diff_e2e
  --test cloud_conformance_e2e
  --test paid_flow_e2e
)

# Server + test env. Values are literals from registry-e2e.yml; none are secret.
export DATABASE_URL="$OWNER_URL"
export JWT_SECRET="e2e-jwt-secret-do-not-use-in-prod"
export S3_BUCKET="mockforge-plugins"
export S3_REGION="us-east-1"
export S3_ENDPOINT="http://localhost:${MINIO_PORT}"
export AWS_ACCESS_KEY_ID="minioadmin"
export AWS_SECRET_ACCESS_KEY="minioadmin"
export REGISTRY_URL="http://localhost:${REGISTRY_PORT}"
export MOCKFORGE_INTERNAL_API_TOKEN="e2e-internal-token-do-not-use-in-prod"
export STRIPE_WEBHOOK_SECRET="whsec_e2e_test_secret"
export RATE_LIMIT_PER_MINUTE="100000"
export RATE_LIMIT_PER_USER="100000"

log()  { printf '\n\033[1;36m==> %s\033[0m\n' "$*"; }
warn() { printf '\033[1;33mwarn: %s\033[0m\n' "$*" >&2; }
die()  { printf '\033[1;31merror: %s\033[0m\n' "$*" >&2; exit 1; }

psql_owner() { PGPASSWORD="$PG_SUPERPASS" psql -h localhost -p "$PG_PORT" -U "$PG_SUPERUSER" -d "$PG_DB" "$@"; }

# ---------------------------------------------------------------------------

require_tools() {
  local missing=()
  for t in docker cargo curl psql; do
    command -v "$t" >/dev/null 2>&1 || missing+=("$t")
  done
  [ ${#missing[@]} -eq 0 ] || die "missing required tools: ${missing[*]}"
}

# Health of a compose SERVICE, resolved through the project rather than a fixed
# container name.
svc_health() {
  local cid
  cid="$(docker compose -f "$COMPOSE_FILE_GATE" ps -q "$1" 2>/dev/null | head -1)"
  [ -n "$cid" ] || { echo "missing"; return; }
  docker inspect --format='{{.State.Health.Status}}' "$cid" 2>/dev/null || echo "unknown"
}

write_state() {
  cat >"$STATE_FILE" <<EOF
COMPOSE_PROJECT_NAME="$COMPOSE_PROJECT_NAME"
PG_PORT="$PG_PORT"
MINIO_PORT="$MINIO_PORT"
MINIO_CONSOLE_PORT="$MINIO_CONSOLE_PORT"
REGISTRY_PORT="$REGISTRY_PORT"
EOF
}

compose_up() {
  log "Starting Postgres + MinIO ($COMPOSE_FILE_GATE)"
  # Same leftover-state guard as registry-e2e.yml: a killed prior run can hold
  # the fixed host ports.
  # Scoped to THIS compose project, so it cannot reach a sibling run's stack.
  docker compose -f "$COMPOSE_FILE_GATE" down --remove-orphans --volumes >/dev/null 2>&1 || true
  docker compose -f "$COMPOSE_FILE_GATE" up -d

  local i
  for i in $(seq 1 60); do
    if [ "$(svc_health db)" = "healthy" ]; then
      break
    fi
    sleep 1
  done
  [ "$(svc_health db)" = "healthy" ] || die "postgres never became healthy"

  for i in $(seq 1 60); do
    if [ "$(svc_health minio)" = "healthy" ]; then
      break
    fi
    sleep 1
  done
  docker compose -f "$COMPOSE_FILE_GATE" logs minio-init 2>/dev/null | tail -3 || true
}

build_server() {
  log "Building mockforge-registry-server (debug)"
  cargo build -p mockforge-registry-server
}

stop_server() {
  if [ -f "$SERVER_PID_FILE" ]; then
    kill "$(cat "$SERVER_PID_FILE")" 2>/dev/null || true
    rm -f "$SERVER_PID_FILE"
  fi
  # Deliberately NOT a blanket `pkill -f mockforge-registry-server`: that would
  # kill the server belonging to a concurrent run of this same gate. The recorded
  # pid plus our (now unique) port are enough.
  fuser -k "${REGISTRY_PORT}/tcp" 2>/dev/null || true
  sleep 1
}

# Boot the server and wait for /health. $1 = human label. Any extra env the
# caller exports (notably APP_DATABASE_URL) is inherited.
boot_server() {
  local label="$1"
  mkdir -p "$STORAGE_PATH"
  log "Booting registry-server ($label)"
  PORT="$REGISTRY_PORT" HOST="0.0.0.0" STORAGE_PATH="$STORAGE_PATH" \
    nohup "$SERVER_BIN" >"$SERVER_LOG" 2>&1 &
  local pid=$!
  echo "$pid" >"$SERVER_PID_FILE"

  local i
  for i in $(seq 1 60); do
    if ! kill -0 "$pid" 2>/dev/null; then
      tail -60 "$SERVER_LOG" || true
      die "registry-server (pid $pid) exited during startup"
    fi
    if curl -sfS "http://localhost:${REGISTRY_PORT}/health" >/dev/null 2>&1; then
      echo "registry-server healthy (pid $pid)"
      return 0
    fi
    sleep 1
  done
  tail -60 "$SERVER_LOG" || true
  die "registry-server never became healthy"
}

# Create the NOSUPERUSER / NOBYPASSRLS request-path role and grant it CRUD on
# everything the migrations created. Idempotent.
#
# Must run AFTER migrations: `GRANT .. ON ALL TABLES` only covers tables that
# exist at grant time. `ALTER DEFAULT PRIVILEGES` additionally covers anything
# a later migration adds within the same stack.
provision_app_role() {
  log "Provisioning $APP_ROLE (NOSUPERUSER NOBYPASSRLS) + grants"
  psql_owner -v ON_ERROR_STOP=1 <<SQL
DO \$\$ BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = '${APP_ROLE}') THEN
    CREATE ROLE ${APP_ROLE} LOGIN PASSWORD '${APP_ROLE_PASSWORD}' NOSUPERUSER NOBYPASSRLS;
  END IF;
END \$\$;

-- Re-assert attributes in case a stale role survived a prior run. Without
-- NOBYPASSRLS the role silently ignores every policy and the gate false-passes.
ALTER ROLE ${APP_ROLE} NOSUPERUSER NOBYPASSRLS;

GRANT USAGE ON SCHEMA public TO ${APP_ROLE};
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO ${APP_ROLE};
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO ${APP_ROLE};
ALTER DEFAULT PRIVILEGES FOR ROLE ${PG_SUPERUSER} IN SCHEMA public
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO ${APP_ROLE};
ALTER DEFAULT PRIVILEGES FOR ROLE ${PG_SUPERUSER} IN SCHEMA public
  GRANT USAGE, SELECT ON SEQUENCES TO ${APP_ROLE};
SQL
}

# Fail loudly if the gate is not actually armed. Without these checks a
# misprovisioned role (BYPASSRLS, or policies missing because the migration
# didn't run) produces a GREEN run that proves nothing.
verify_gate_armed() {
  log "Verifying the gate is armed"

  local bypass
  bypass="$(psql_owner -tAc "SELECT rolbypassrls FROM pg_roles WHERE rolname = '${APP_ROLE}'")"
  [ "$bypass" = "f" ] || die "$APP_ROLE has rolbypassrls=$bypass — RLS would be bypassed and the gate would false-pass"

  local covered
  covered="$(psql_owner -tAc "SELECT count(*) FROM pg_class WHERE relrowsecurity AND relname IN ('projects','audit_logs','hosted_mocks','templates','scenarios')")"
  [ "$covered" = "5" ] || die "expected RLS enabled on 5 tables, found $covered — did migration 20250101000082 run?"

  local forced
  forced="$(psql_owner -tAc "SELECT count(*) FROM pg_class WHERE relforcerowsecurity AND relname IN ('projects','audit_logs','hosted_mocks','templates','scenarios')")"
  [ "$forced" = "5" ] || die "expected FORCE RLS on 5 tables, found $forced"

  # Positive control: as the app role with no GUC bound, a covered table must
  # return 0 rows. If this returns rows the policies are not doing anything.
  local leaked
  leaked="$(PGPASSWORD="$APP_ROLE_PASSWORD" psql -h localhost -p "$PG_PORT" -U "$APP_ROLE" -d "$PG_DB" -tAc \
    "SELECT count(*) FROM projects")"
  [ "$leaked" = "0" ] || die "app role saw $leaked projects rows with no org GUC bound — RLS is not enforcing"

  echo "gate armed: $APP_ROLE is NOBYPASSRLS, 5/5 covered tables ENABLE+FORCE RLS, unbound reads see 0 rows"
}

cmd_up() {
  require_tools
  write_state
  compose_up
  build_server

  # Phase 1: boot on the owner pool only. This applies the migrations (the
  # server runs them at startup) and creates every table the grants below need.
  # APP_DATABASE_URL is deliberately unset here — `Database::new` then aliases
  # runtime_pool to the owner pool, i.e. today's production behavior.
  unset APP_DATABASE_URL
  boot_server "phase 1: owner pool, applying migrations"
  stop_server

  provision_app_role
  verify_gate_armed

  # Phase 2: the real thing. Request-path queries now run as NOBYPASSRLS.
  export APP_DATABASE_URL="$APP_URL"
  boot_server "phase 2: request path on $APP_ROLE (RLS ACTIVE)"

  cat <<EOF

Stack is up with RLS ACTIVE on the request path.
  registry:  $REGISTRY_URL
  owner DB:  $OWNER_URL
  app DB:    $APP_URL
  log:       $SERVER_LOG

Next: scripts/rls-e2e-gate.sh test          (run the full suite)
      scripts/rls-e2e-gate.sh test --test marketplace_e2e   (one target)
      scripts/rls-e2e-gate.sh down
EOF
}

# Re-point the ALREADY-RUNNING stack's server at the owner pool (RLS inert),
# keeping the same database and data. This is the control for attribution:
#
#   up      -> test   : does the suite pass with RLS ACTIVE?
#   control -> test   : does it pass with RLS INACTIVE?
#
# A test that fails in BOTH is a pre-existing bug, not an RLS coverage gap.
# Without this, every pre-existing failure in a suite that CI never runs
# (the marketplace ones are `#[ignore]`) looks like it was caused by RLS.
cmd_control() {
  require_tools
  stop_server
  unset APP_DATABASE_URL
  boot_server "control: owner pool, RLS INACTIVE"
  echo
  echo "Control stack up (RLS inert). Run: scripts/rls-e2e-gate.sh test [args]"
  echo "Re-arm RLS with: scripts/rls-e2e-gate.sh rearm"
}

# Put the running stack back on the NOBYPASSRLS role without redoing docker or
# migrations.
cmd_rearm() {
  require_tools
  stop_server
  verify_gate_armed
  export APP_DATABASE_URL="$APP_URL"
  boot_server "re-armed: request path on $APP_ROLE (RLS ACTIVE)"
}

cmd_test() {
  require_tools
  curl -sfS "http://localhost:${REGISTRY_PORT}/health" >/dev/null 2>&1 \
    || die "no registry-server on :${REGISTRY_PORT} — run 'scripts/rls-e2e-gate.sh up' first"

  # Confirm the running server is actually the RLS-active one. A stale server
  # from a non-gate run would make this pass for the wrong reason.
  if ! grep -q "APP_DATABASE_URL set" "$SERVER_LOG" 2>/dev/null; then
    warn "server log has no 'APP_DATABASE_URL set' line — the running server may not be RLS-active"
  fi

  local targets=()
  if [ "$#" -gt 0 ]; then
    targets=("$@")
  else
    targets=("${E2E_TESTS[@]}")
  fi

  log "Running E2E suite against the RLS-active server"
  # --test-threads=1 for the same reason as CI: the suites create orgs with
  # slugs that can collide when run concurrently.
  #
  # --no-fail-fast is important for a COVERAGE gate specifically: without it
  # cargo stops at the first failing test target, so one uncovered query hides
  # every other one behind it and you discover the gaps one slow run at a time.
  # We want the complete list in a single pass.
  cargo test --no-fail-fast -p mockforge-registry-server "${targets[@]}" \
    -- --ignored --nocapture --test-threads=1
}

cmd_down() {
  log "Tearing down"
  stop_server
  docker compose -f "$COMPOSE_FILE_GATE" down -v >/dev/null 2>&1 || true
  rm -f "$STATE_FILE"
  echo "done"
}

cmd_psql_app() {
  log "psql as $APP_ROLE (NOBYPASSRLS). Bind an org with:"
  echo "  SELECT set_config('app.current_org_id', '<uuid>', false);"
  PGPASSWORD="$APP_ROLE_PASSWORD" psql -h localhost -p "$PG_PORT" -U "$APP_ROLE" -d "$PG_DB"
}

cmd_all() {
  local rc=0
  cmd_up
  cmd_test || rc=$?
  if [ "$rc" -ne 0 ]; then
    log "E2E failed under RLS — dumping server log tail"
    tail -200 "$SERVER_LOG" || true
  fi
  cmd_down
  return "$rc"
}

case "${1:-all}" in
  up)       shift; cmd_up "$@" ;;
  test)     shift; cmd_test "$@" ;;
  control)  shift; cmd_control "$@" ;;
  rearm)    shift; cmd_rearm "$@" ;;
  down)     shift; cmd_down "$@" ;;
  psql-app) shift; cmd_psql_app "$@" ;;
  all)      shift; cmd_all "$@" ;;
  *)        die "unknown subcommand '$1' (expected: all|up|test|control|rearm|down|psql-app)" ;;
esac
