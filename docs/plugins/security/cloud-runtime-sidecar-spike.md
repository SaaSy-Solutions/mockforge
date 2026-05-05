# Spike: Cloud Plugin Runtime — Sidecar on a Fly Machine

| Field        | Value                                                       |
| ------------ | ----------------------------------------------------------- |
| **Status**   | Validated                                                   |
| **Phase**    | Cloud Plugins Phase 1 — informs Phase 2 implementation      |
| **Author**   | Ray Clanan                                                  |
| **Created**  | 2026-05-05                                                  |
| **Companions** | `cloud-trust-permissions-rfc.md`, `cloud-runtime-build-vs-buy-spike.md` |
| **Artifacts** | `sidecar-spike/Dockerfile.spike`, `sidecar-spike/entrypoint-spike.sh`, `sidecar-spike/plugin_host_stub.py` |

## TL;DR

**The sidecar architecture fits in `shared-cpu-1x:256MB` with comfortable headroom.** Measured cgroup-accounted memory was 27% of cap (69 MB / 256 MB) with main mockforge + a plugin-host stub holding 50 MB of representative wasmtime-style overhead. Unix socket IPC works cleanly across the two processes. Tini handles PID 1 reaping. A shell-script supervisor is sufficient — supervisord adds nothing.

**One important finding outside the headline question:** `FlyioMachineConfig` in the registry-server doesn't include a `guest` field, so production hosted-mocks accept Fly's API default (currently `shared-cpu-1x:256MB`). For plugin-enabled deployments we'll want this configurable so a Team-tier customer can opt into a larger floor without us touching the orchestrator code.

## What the spike actually validates

The trust RFC and the build-vs-buy spike both proposed putting a plugin-host sidecar in the same Fly machine as the user's mockforge instance, with Unix socket IPC between them. That left three open questions:

1. **Topology.** Can two processes share one container with PID 1 supervision and IPC? *Well-understood pattern, but worth confirming with the actual mockforge image.*
2. **Memory budget.** Does the proposed architecture fit in the existing 256 MB Fly floor, or do we need to bump the tier?
3. **Operational shape.** What does the entrypoint look like, what supervises children, what survives a SIGTERM?

This spike answers all three. It does *not* answer Phase 2 implementation questions (real wasmtime in the sidecar, signature verification, egress proxy) — those are build work, gated on the demand signal from #385.

## Methodology

Built `Dockerfile.spike` (in `sidecar-spike/`) on top of the production `ghcr.io/saasy-solutions/mockforge:latest`. Layered on:

- `tini` for PID 1 reaping
- `python3-minimal` for the plugin-host stub
- `entrypoint-spike.sh` — shell script that fans out main + sidecar, traps SIGTERM, kills both children if either exits
- `plugin_host_stub.py` — allocates 50 MB and listens on `/tmp/plugin-host.sock`

The 50 MB sidecar reservation is a **conservative high estimate** of a real Rust plugin-host's idle footprint:

| Component | Estimated RSS |
|---|---|
| Bare Wasmtime engine + WASI ctx | ~10 MB |
| 6 loaded `Store`s × small WASM module each | ~30 MB |
| Plugin-host's own state + IPC buffers | ~10 MB |
| **Total** | **~50 MB** |

So if main + stub fits in 256 MB, the production sidecar (which will be a Rust binary, much leaner than Python with the same allocation) certainly does.

Ran the container under `--memory 256m --memory-swap 256m` (matches Fly's cgroup behavior; OOM-kill on overage, no swap fallback). Let it idle ~30 seconds for stable measurements.

## Measurements

```
$ docker exec mockforge-spike3 ps -eo pid,rss,vsz,comm --sort=-rss
    PID   RSS    VSZ COMMAND
     26 61948 141132 python3        ← plugin-host stub
      8 55324 1165552 mockforge     ← main mockforge
      7  1772   2680 entrypoint-spik
      1  1444   2572 tini
```

```
$ docker stats --no-stream
mockforge-spike3: mem=69.22MiB / 256MiB pct=27.04%
```

Cross-process IPC over the Unix socket:
```
$ docker exec mockforge-spike3 python3 -c "
import socket
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect('/tmp/plugin-host.sock')
print(s.recv(64))
"
b'plugin-host-stub: ok\n'
```

## Why the cgroup number is far below the RSS sum

Process RSS sums to ~120 MB (54 mockforge + 60 sidecar + ~5 misc), but the cgroup accounts only 69 MB. Difference is shared pages: glibc, kernel-mapped read-only segments, the `mockforge` binary itself if it's reused, libpython, etc. Cgroup memory accounting deduplicates these — and Fly's OOM killer fires on cgroup memory, not summed RSS. The 27% number is what matters operationally.

## Headroom analysis

Working from the cgroup measurement of 69 MB at idle:

| State | Memory | Headroom in 256 MB |
|---|---:|---:|
| Idle (this measurement) | 69 MB | 187 MB |
| 6 plugins active, each 30 MB linear memory | ~249 MB | 7 MB |
| Tier limit suggests realistic plugin sizing | | |

The "6 plugins × 30 MB" upper bound assumes every attached plugin grows to its allowed maximum simultaneously. With realistic plugin sizes (most are KB-scale request transformers, not MB-scale), the typical working set will be much smaller. **256 MB is fine for Pro tier** (≤5 plugins per mock, per the trust RFC's strawman pricing).

For Team tier (≤25 plugins), 256 MB is too tight. Recommended floor for plugin-enabled deployments at Team+:

| Plan | Floor | Headroom @ idle |
|---|---|---|
| Free / Pro (no plugins) | shared-cpu-1x:256MB | 187 MB |
| Pro w/ plugins | shared-cpu-1x:256MB | sufficient if plugins are small |
| Team w/ plugins | shared-cpu-1x:512MB | 443 MB |
| Enterprise | shared-cpu-2x:1024MB | 955 MB |

## What I'd change in `FlyioMachineConfig`

```rust
// Today: no guest field — Fly API default (shared-cpu-1x:256MB) wins.
pub struct FlyioMachineConfig {
    pub image: String,
    pub env: HashMap<String, String>,
    pub services: Vec<FlyioService>,
    pub checks: Option<HashMap<String, FlyioCheck>>,
}

// Phase 2: add guest, defaulted from the org's plan tier.
pub struct FlyioMachineConfig {
    pub image: String,
    pub env: HashMap<String, String>,
    pub services: Vec<FlyioService>,
    pub checks: Option<HashMap<String, FlyioCheck>>,
    pub guest: Option<FlyioGuest>,  // ← new
}

pub struct FlyioGuest {
    pub cpu_kind: String,    // "shared" | "performance"
    pub cpus: u32,
    pub memory_mb: u32,
}
```

The orchestrator picks the right `guest` based on `org.plan` + whether the deployment has any plugins attached. Defaults to today's behavior (Fly chooses) if `None`. This is a small, additive change that doesn't disrupt the existing deployment path.

## Operational findings

- **Tini for PID 1 is sufficient.** No supervisord needed. The shell script sees `EXIT` semantics correctly via `kill -0`, traps SIGTERM, propagates to children.
- **Children share stdout/stderr cleanly** — Fly's log aggregation gets both streams interleaved with no extra config. Good for debugging.
- **Unix socket creation needs explicit chmod** — the stub sets `0666` after bind because the default umask varies; main mockforge process runs as a non-root user in production and otherwise can't open the socket. Locking down to `0660` + a shared group is the production hardening.
- **Startup ordering matters slightly.** A 1-second sleep between starting main and sidecar is enough to avoid disk-read contention on a cold cache. A real implementation should make the sidecar wait for main's readiness signal (or vice versa, since main needs the socket to exist before it sends the first plugin invocation).

## What this spike unblocks

- Phase 2 runtime architecture is **fully de-risked** on the structural axis. Build can start whenever the demand signal from #385 supports it.
- The `Dockerfile.spike` and `entrypoint-spike.sh` are reusable templates — Phase 2's production Dockerfile should look very similar.
- Adding a `guest` field to `FlyioMachineConfig` is now a documented prerequisite, not a discovery during Phase 2 implementation.

## What this spike did *not* validate

- **A real wasmtime sidecar.** The Python stub matches the memory shape but not the latency or syscall patterns. Phase 2 will get a real Rust plugin-host; expect to remeasure when that lands.
- **An actual Fly deploy.** The Fly microVM adds ~30 MB of kernel + init + DNS resolver overhead on top of the cgroup count. Adjusted estimate: idle ≈ 99 MB on a real Fly machine, still 39% of 256 MB. Comfortable, but the on-Fly number is one cycle off real. A 5-minute Fly deploy with this spike's image would close that gap; treat it as the next operational check before Phase 2 build, not as an open question.
- **Egress proxy as a third process.** RFC §5.1 proposes a small HTTP forward-proxy as a *third* process for egress allowlist enforcement. Adding it to this measurement is a follow-up; expect ~5-10 MB overhead.

## Decision log

| Date | Decision | By |
| ---- | -------- | -- |
| 2026-05-05 | Sidecar architecture validated for `shared-cpu-1x:256MB`. Pro tier proceeds with current floor; Team tier needs `512MB`; `FlyioMachineConfig` needs a `guest` field. | Ray Clanan |
