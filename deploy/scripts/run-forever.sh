#!/usr/bin/env bash
# Lightweight supervisor for `mockforge serve` — restarts the binary
# whenever it exits, until the wrapper itself is killed (Ctrl-C or
# `kill <pid>`). Use this on hosts without systemd or when you don't
# want to write a service unit.
#
# Issue #79: under heavy chunked traffic the server can be killed by
# the kernel OOM killer or by an in-process panic. This wrapper makes
# the recovery automatic so a single crash doesn't end the test.
#
# Usage:
#   ./run-forever.sh <serve args...>
#
# Examples:
#   ./run-forever.sh --spec api.json --http-port 80 --https-port 443 \
#     --tls-cert server.pem --tls-key key.pem --no-rate-limit
#
#   MOCKFORGE_BIN=/usr/local/bin/mockforge \
#     RESTART_BACKOFF_SECS=10 RESTART_LOG=/var/log/mockforge-supervisor.log \
#     ./run-forever.sh --spec api.json --admin --admin-port 9080
#
# Environment overrides:
#   MOCKFORGE_BIN          path to the binary (default: `mockforge` on $PATH)
#   RESTART_BACKOFF_SECS   sleep between restart attempts (default: 5)
#   RESTART_LOG            file to append restart events to (default: stderr only)
#
# Send SIGINT/SIGTERM to this script (Ctrl-C, `kill`) to stop cleanly —
# the running mockforge child gets the signal forwarded.

set -u

BIN="${MOCKFORGE_BIN:-mockforge}"
BACKOFF="${RESTART_BACKOFF_SECS:-5}"
LOG="${RESTART_LOG:-}"

if ! command -v "$BIN" >/dev/null 2>&1 && ! [ -x "$BIN" ]; then
  echo "run-forever: $BIN not found on PATH and not executable" >&2
  exit 127
fi

log() {
  local msg="[$(date -u +%Y-%m-%dT%H:%M:%SZ)] $*"
  echo "$msg" >&2
  if [ -n "$LOG" ]; then
    echo "$msg" >> "$LOG"
  fi
}

stop_requested=0
child_pid=0

forward_signal() {
  local sig="$1"
  stop_requested=1
  if [ "$child_pid" -ne 0 ] && kill -0 "$child_pid" 2>/dev/null; then
    log "supervisor received SIG$sig — forwarding to child PID $child_pid"
    kill -s "$sig" "$child_pid" 2>/dev/null || true
  fi
}

trap 'forward_signal INT' INT
trap 'forward_signal TERM' TERM

attempt=0
while [ "$stop_requested" -eq 0 ]; do
  attempt=$((attempt + 1))
  log "starting mockforge serve (attempt $attempt): $BIN serve $*"
  set +e
  "$BIN" serve "$@" &
  child_pid=$!
  wait "$child_pid"
  rc=$?
  set -e
  child_pid=0

  if [ "$stop_requested" -eq 1 ]; then
    log "supervisor exiting after explicit stop request (last child rc=$rc)"
    exit "$rc"
  fi

  log "mockforge exited with rc=$rc; restarting in ${BACKOFF}s"
  # Use a single sleep that responds to signals so Ctrl-C during the
  # backoff window stops the supervisor immediately.
  sleep "$BACKOFF" &
  wait $! 2>/dev/null || true
done
