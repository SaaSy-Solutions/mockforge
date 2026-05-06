#!/bin/sh
# Cloud Plugins sidecar spike entrypoint.
#
# Runs main `mockforge` + a Python plugin-host stub in the same
# container under a single tini PID 1, to measure the memory
# overhead of the multi-process architecture under a 256 MB cgroup
# cap. See Dockerfile.spike for the methodology rationale.

set -eu

cleanup() {
    echo "[spike-entrypoint] received signal — terminating children" >&2
    if [ -n "${MAIN_PID:-}" ]; then
        kill -TERM "$MAIN_PID" 2>/dev/null || true
    fi
    if [ -n "${SIDECAR_PID:-}" ]; then
        kill -TERM "$SIDECAR_PID" 2>/dev/null || true
    fi
    wait
    exit 0
}
trap cleanup TERM INT HUP

echo "[spike-entrypoint] starting main mockforge on :3000 (admin :9080)" >&2
/usr/local/bin/mockforge serve --http-port 3000 --admin --admin-port 9080 &
MAIN_PID=$!

# Stagger so they don't race on the same disk reads.
sleep 1

echo "[spike-entrypoint] starting plugin-host stub" >&2
python3 /usr/local/bin/plugin_host_stub.py &
SIDECAR_PID=$!

echo "[spike-entrypoint] both children up — main=$MAIN_PID sidecar=$SIDECAR_PID" >&2

# Wait for either child to exit; if one dies, kill the other so Fly
# restarts the machine cleanly.
while kill -0 "$MAIN_PID" 2>/dev/null && kill -0 "$SIDECAR_PID" 2>/dev/null; do
    sleep 5
done

echo "[spike-entrypoint] one child exited — tearing down" >&2
cleanup
