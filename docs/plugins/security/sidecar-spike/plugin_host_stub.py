#!/usr/bin/env python3
"""Plugin-host stand-in for the cloud-plugins sidecar spike.

Approximates the memory footprint of a real plugin-host: a Rust
binary with `wasmtime::Engine` + N `Store`s loaded, each carrying
a small WASM module + the `MemoryTracker` from
`mockforge-plugin-loader::memory_tracking`.

Empirical anchors for the 50 MB number:
  - Bare Wasmtime engine + WASI ctx: ~10 MB resident
  - Each loaded Store + a small (~100 KB) WASM module: ~5 MB
  - Headroom for the plugin-host's own state + IPC buffers: ~10 MB

So 10 + 6×5 + 10 ≈ 50 MB for a hosted-mock with 6 attached plugins —
the Team-tier limit suggested in the trust RFC's pricing table.
A real-world Pro-tier deployment (≤5 plugins) would be smaller.
"""

import os
import signal
import socket
import sys
import threading
import time

SIDECAR_RESERVE_BYTES = 50 * 1024 * 1024  # 50 MiB
SOCKET_PATH = "/tmp/plugin-host.sock"


def reserve_memory(nbytes: int) -> bytearray:
    """Allocate and *touch* every page so the kernel actually commits
    the memory. A bare `bytearray(nbytes)` may stay zero-faulted and
    not show up in RSS until written to."""
    buf = bytearray(nbytes)
    page = 4096
    for offset in range(0, nbytes, page):
        buf[offset] = 1
    return buf


def serve_unix_socket() -> None:
    """Mirror the IPC topology of the future plugin-host: listen on a
    Unix socket and echo a short greeting to anyone who connects.
    Validates that the main + sidecar can reach each other through
    the filesystem boundary."""
    try:
        os.unlink(SOCKET_PATH)
    except FileNotFoundError:
        pass

    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.bind(SOCKET_PATH)
    os.chmod(SOCKET_PATH, 0o666)
    sock.listen(8)

    print(f"[plugin-host-stub] listening on {SOCKET_PATH}", flush=True)
    while True:
        try:
            conn, _ = sock.accept()
        except OSError:
            return
        with conn:
            conn.sendall(b"plugin-host-stub: ok\n")


def handle_term(signum, frame):
    print(f"[plugin-host-stub] received signal {signum} — exiting", flush=True)
    try:
        os.unlink(SOCKET_PATH)
    except FileNotFoundError:
        pass
    sys.exit(0)


def main() -> None:
    signal.signal(signal.SIGTERM, handle_term)
    signal.signal(signal.SIGINT, handle_term)

    print(
        f"[plugin-host-stub] reserving {SIDECAR_RESERVE_BYTES // (1024 * 1024)} MB "
        "to approximate wasmtime + plugin Store footprint",
        flush=True,
    )
    _reserved = reserve_memory(SIDECAR_RESERVE_BYTES)
    # Hold a reference so the GC can't reclaim it while we idle.
    globals()["_reserved"] = _reserved

    sock_thread = threading.Thread(target=serve_unix_socket, daemon=True)
    sock_thread.start()

    print("[plugin-host-stub] idle — waiting for SIGTERM", flush=True)
    while True:
        time.sleep(60)


if __name__ == "__main__":
    main()
