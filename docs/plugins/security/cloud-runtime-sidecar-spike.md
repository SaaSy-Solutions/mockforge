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

## Addendum: real Fly microVM measurement (2026-05-06)

Closing the "did NOT validate" gap above by deploying the spike image to an actual Fly machine (`mockforge-cp-spike` in `iad`, since destroyed) and re-measuring. **The local docker numbers were too optimistic; 256 MB is *not* sufficient for plugin-enabled deployments.**

### Measurements

```
$ flyctl ssh console --app mockforge-cp-spike --command 'ps -eo pid,rss,comm --sort=-rss; cat /sys/fs/cgroup/memory.current'

  PID   RSS COMMAND
  657 61804 python3        ← plugin-host stub
  644 52424 mockforge      ← main mockforge
  636 20448 hallpass       ← Fly's SSH/RPC daemon
    1  6444 init           ← Fly's microVM init
  667  1816 sh
  643  1728 entrypoint-spik
  635  1452 tini

cgroup memory: 174153728 bytes  (166 MB)
MemTotal:      212236 kB        (Fly reserves ~44 MB on a 256 MB tier for kernel)
MemAvailable:   72492 kB        (only 71 MB free)
```

After scaling the same machine to `shared-cpu-1x:512MB`:

```
cgroup memory: 202166272 bytes  (193 MB / 512 MB = 38%)
MemAvailable: 323664 kB         (316 MB free)
```

### What changed vs. local docker

| Tier | Local docker (cgroup) | Real Fly (cgroup) | Δ |
|---|---:|---:|---:|
| 256 MB | 69 MB / 27% | 166 MB / 65% | **+97 MB** |
| 512 MB | (not measured) | 193 MB / 38% | — |

The +97 MB delta is **larger than the +30 MB** I'd estimated. Sources:
- `init` (Fly microVM PID 1, before our tini): ~6 MB
- `hallpass` (Fly's exec/SSH daemon): ~20 MB
- Kernel reserve: 256 MB tier reports `MemTotal: 212 MB`, so ~44 MB is reserved before user space sees it
- Cgroup accounting differences between docker and Fly's microVM

### Revised tier table

The original headroom analysis assumed local-docker numbers; here are the corrected per-Fly-tier numbers:

| Tier | Idle cgroup | Headroom | Fits ≤5 plugins (Pro)? | Fits ≤25 plugins (Team)? |
|---|---:|---:|---|---|
| `shared-cpu-1x:256MB` | 166 MB / 65% | 71 MB | tight; small plugins only | no |
| `shared-cpu-1x:512MB` | 193 MB / 38% | 316 MB | yes, comfortably | yes, with care |
| `shared-cpu-1x:1024MB` | (extrapolated ~205 MB / 20%) | ~800 MB | yes | yes, comfortably |

### Revised recommendation

**Pro tier with cloud plugins requires `shared-cpu-1x:512MB` (not 256MB).** The original "Pro stays on 256MB" note from the docker-only measurement was incorrect — the on-Fly headroom isn't enough for plugin runtime growth without OOM-killing mockforge.

| Plan | Plugins enabled? | Floor |
|---|---|---|
| Free | (plugins not available) | `shared-cpu-1x:256MB` |
| Pro | no | `shared-cpu-1x:256MB` |
| Pro | yes | `shared-cpu-1x:512MB` |
| Team | yes | `shared-cpu-1x:1024MB` |
| Future Enterprise | yes | `shared-cpu-2x:2048MB` |

The pricing implication is real: a Pro-tier customer who attaches plugins is on a more expensive Fly machine. Budget that into the cloud-plugins pricing table from the build-vs-buy spike.

### Action taken

`FlyioGuest::for_hosted_mock(plan, plugins_enabled)` ships in this PR with these numbers as defaults. Existing hosted-mocks (no plugins) keep `guest: None` → Fly's API default, so this is non-disruptive.

## Decision log

| Date | Decision | By |
| ---- | -------- | -- |
| 2026-05-05 | Sidecar architecture validated for `shared-cpu-1x:256MB` (local docker). Pro tier proceeds with current floor; Team tier needs `512MB`; `FlyioMachineConfig` needs a `guest` field. | Ray Clanan |
| 2026-05-06 | Real Fly deploy revised the floor: Pro w/ plugins needs `512MB`, Team w/ plugins needs `1024MB`. `FlyioGuest::for_hosted_mock` codifies this. | Ray Clanan |
