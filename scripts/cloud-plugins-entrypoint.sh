#!/bin/sh
# Cloud Plugins multi-process entrypoint.
#
# Starts main mockforge + plugin-host sidecar + egress proxy
# under tini PID 1, traps SIGTERM, kills all children if any
# child exits unexpectedly so Fly restarts the machine cleanly.
#
# Mirrors the architecture validated in PR #397 (the sidecar
# spike) but with real Rust binaries instead of the Python stub.
# tini handles PID 1 reaping; this script is the supervisor.
#
# Process roles:
#   main          — the user-facing mockforge (HTTP, admin UI,
#                   gRPC, etc.). Reads plugin-enabled config from
#                   env vars.
#   plugin-host   — Wasmtime sidecar. Listens on the Unix socket;
#                   loads/invokes WASM plugins on behalf of main.
#   egress-proxy  — HTTP CONNECT proxy on loopback. Plugins reach
#                   the internet only via HTTP_PROXY pointed here.

set -eu

# ─── Configurable defaults ─────────────────────────────────────────
PLUGIN_HOST_SOCKET="${MOCKFORGE_PLUGIN_HOST_SOCKET:-/run/mockforge/plugin-host.sock}"
EGRESS_LISTEN="${MOCKFORGE_PLUGIN_EGRESS_LISTEN:-127.0.0.1:8125}"

# Make sure the plugin-host's Unix socket directory exists with
# the right ownership before either process tries to bind.
PLUGIN_HOST_SOCKET_DIR="$(dirname "$PLUGIN_HOST_SOCKET")"
mkdir -p "$PLUGIN_HOST_SOCKET_DIR"
chown mockforge:mockforge "$PLUGIN_HOST_SOCKET_DIR" 2>/dev/null || true

cleanup() {
    echo "[cloud-plugins-entrypoint] received signal — terminating children" >&2
    if [ -n "${MAIN_PID:-}" ]; then
        kill -TERM "$MAIN_PID" 2>/dev/null || true
    fi
    if [ -n "${PLUGIN_HOST_PID:-}" ]; then
        kill -TERM "$PLUGIN_HOST_PID" 2>/dev/null || true
    fi
    if [ -n "${EGRESS_PID:-}" ]; then
        kill -TERM "$EGRESS_PID" 2>/dev/null || true
    fi
    wait
    exit 0
}
trap cleanup TERM INT HUP

# ─── Start order ──────────────────────────────────────────────────
# Egress proxy first — main and plugin-host both depend on it
# being up before they accept user traffic.
echo "[cloud-plugins-entrypoint] starting egress proxy on $EGRESS_LISTEN" >&2
MOCKFORGE_PLUGIN_EGRESS_LISTEN="$EGRESS_LISTEN" \
    su mockforge -s /bin/sh -c '/usr/local/bin/mockforge-plugin-egress' &
EGRESS_PID=$!

# Plugin host next so main can connect to its socket on first
# request. Brief stagger to avoid disk-read contention.
sleep 1
echo "[cloud-plugins-entrypoint] starting plugin-host on $PLUGIN_HOST_SOCKET" >&2
MOCKFORGE_PLUGIN_HOST_SOCKET="$PLUGIN_HOST_SOCKET" \
    su mockforge -s /bin/sh -c '/usr/local/bin/mockforge-plugin-host' &
PLUGIN_HOST_PID=$!

# Wait for the plugin-host socket to actually appear before
# starting main — main's request handler will try to dial it on
# the first plugin invocation, and a missing socket = 503 stamp.
WAIT_TRIES=0
while [ ! -S "$PLUGIN_HOST_SOCKET" ] && [ "$WAIT_TRIES" -lt 50 ]; do
    sleep 0.2
    WAIT_TRIES=$((WAIT_TRIES + 1))
done
if [ ! -S "$PLUGIN_HOST_SOCKET" ]; then
    echo "[cloud-plugins-entrypoint] plugin-host socket failed to appear at $PLUGIN_HOST_SOCKET" >&2
    cleanup
fi

# Finally main. Default args are appropriate for hosted-mock
# deployments (admin UI, all protocols enabled). Operators can
# override the entrypoint or pass extra args via the COMMAND.
echo "[cloud-plugins-entrypoint] starting main mockforge" >&2
su mockforge -s /bin/sh -c '/usr/local/bin/mockforge serve --admin' &
MAIN_PID=$!

echo "[cloud-plugins-entrypoint] all children up — main=$MAIN_PID host=$PLUGIN_HOST_PID egress=$EGRESS_PID" >&2

# Wait for any child to exit. If any of the three dies, take
# them all down — Fly restarts the whole machine, which is the
# correct response to a partial failure: we can't run plugins
# without all three processes alive.
while kill -0 "$MAIN_PID" 2>/dev/null \
   && kill -0 "$PLUGIN_HOST_PID" 2>/dev/null \
   && kill -0 "$EGRESS_PID" 2>/dev/null; do
    sleep 5
done

echo "[cloud-plugins-entrypoint] one child exited — tearing down" >&2
cleanup
