# Chaos Engineering

MockForge ships a runtime chaos engineering surface that injects controlled
failures, delays, and resource constraints into your mock or proxy server.
Use it to test how clients behave when the real backend gets slow, returns
errors, drops the connection, sends partial responses, or rate-limits you.

> **Two related but distinct features:** this chapter covers the **YAML /
> runtime config** (loaded by `mockforge serve`). For the in-app
> [Chaos Lab](./chaos-lab.md) UI and predefined network profiles, see the
> separate page. Many users want both — the YAML config for headless test
> runs and the UI for interactive exploration.

The full reference (every config knob, every API endpoint, every predefined
scenario) lives at
[docs/CHAOS_ENGINEERING.md](https://github.com/SaaSy-Solutions/mockforge/blob/main/docs/CHAOS_ENGINEERING.md).
This chapter focuses on the patterns most teams reach for first.

## What you can inject

| Category | Examples |
|---|---|
| **Latency** | Fixed delay, random range, jitter, probability-based |
| **HTTP errors** | Any status code, configurable probability, error pattern (burst / random / sequential) |
| **Connection errors** | HTTP 503 (default), TCP RST, TCP FIN — at the wire level |
| **Timeouts** | Real `tokio::sleep` then `504 Gateway Timeout` |
| **Partial responses** | Body truncation; preserves Content-Length on non-chunked, drops the terminator on chunked |
| **Payload corruption** | Random bytes, bit-flips, truncation |
| **Rate limiting** | Global, per-IP, per-endpoint with burst |
| **Traffic shaping** | Bandwidth throttle, packet loss, max connections |

Every fault path can be gated on a per-request matcher (source IP / CIDR,
header, body size, `Transfer-Encoding: chunked`).

## Quick start

### Enable from the CLI

```bash
mockforge serve --chaos --chaos-scenario service_instability
```

`service_instability` is one of five predefined scenarios. The others are
`network_degradation`, `cascading_failure`, `peak_traffic`, `slow_backend`.

### Enable from a config profile

```yaml
profiles:
  flaky:
    observability:
      chaos:
        enabled: true
        latency:
          enabled: true
          fixed_delay_ms: 250
          jitter_percent: 20
          probability: 0.5
        fault_injection:
          enabled: true
          http_errors: [500, 503]
          http_error_probability: 0.1
```

Run with `mockforge serve --profile flaky`.

## Recipes: varied 5xx and slow responses

These are the patterns most teams reach for first when exercising client
timeout / retry behaviour. Each recipe is copy-pasteable; tune the
probabilities to your spec.

### Mix of 4xx and 5xx statuses across the whole spec

Global error injection via `mockforge serve --chaos`. `--chaos-http-errors`
takes a comma-separated list; the injector picks one at the configured
probability:

```bash
mockforge serve --spec api.yaml --http-port 3000 \
  --chaos --chaos-http-errors 500,502,503,504,429,408 \
  --chaos-http-error-probability 0.10
```

`0.10` means roughly 10% of matching requests get a chaos-status response.
The remaining 90% flow through your spec unchanged. Mix 4xx and 5xx as
needed: clients should retry-with-backoff on 503/429/504/502 but treat
500/400/422 as terminal.

### Slow backend (every request hangs N ms)

Useful for exercising the client's request-level timeout. Sleeps before
the upstream handler runs, so the client experiences a true server-side
hang followed by a normal 2xx (or the configured status).

```bash
mockforge serve --spec api.yaml --http-port 3000 \
  --chaos --chaos-latency-ms 2000 --chaos-latency-probability 1.0
```

Pair with `--chaos-latency-range 500-5000` to randomise:

```bash
mockforge serve --spec api.yaml --http-port 3000 \
  --chaos --chaos-latency-range 500-5000 --chaos-latency-probability 0.30
```

### Real timeout (server hangs, returns 504)

Different from a slow response: this fires when you want the *server* to
report timeout, not just go slow. Configure via the YAML chaos profile:

```yaml
fault_injection:
  enabled: true
  timeout_errors: true
  timeout_ms: 30000             # 30 s sleep then 504 Gateway Timeout
  timeout_probability: 0.05     # 5% of matching requests
```

Run with `mockforge serve --spec api.yaml --config chaos.yaml`.

### Per-endpoint chaos (slow this route, error that one)

Global flags hit every route equally. To exercise *one* endpoint's
timeout while keeping the rest healthy, use per-route chaos:

```yaml
# routes-with-chaos.yaml
routes:
  - path: /api/slow-report
    method: GET
    response:
      status: 200
      body: { ok: true }
    latency:
      enabled: true
      probability: 1.0
      fixed_delay_ms: 8000        # always hang 8s on this route
  - path: /api/flaky-write
    method: POST
    response:
      status: 201
      body: { id: 1 }
    fault_injection:
      enabled: true
      probability: 0.25           # 25% of POSTs to this route fail
      fault_types:
        - type: http_error
          status_code: 503
          message: "Backend warming up"
        - type: http_error
          status_code: 504
        - type: http_error
          status_code: 429
```

Mockforge picks one `fault_types` entry uniformly when the probability
fires, so a list of three statuses gives you ~equal weight across the
three. Add more entries to bias toward a status (entries are picked
uniformly, so two `503` entries plus one `504` entry gives 2:1 odds).

`latency.distribution` also supports `normal { mean_ms, std_dev_ms }` and
`exponential { lambda }` for non-uniform spreads — handy when you want
"P95 = 8s but a long tail."

### Exercise client retry / backoff behaviour

Combine the two above. The pattern: ~30% of requests get a transient
status (503 / 504 / 429), the rest succeed. A well-written client with
exponential backoff should eventually succeed on retries; a buggy client
either gives up immediately or hammers the server flat.

```yaml
fault_injection:
  enabled: true
  http_errors: true
  http_error_codes: [503, 504, 429]
  http_error_probability: 0.30
  latency_ms: 250                 # mild latency baseline
  latency_probability: 1.0
```

### Verify what fired

The chaos config exposes counters at `GET /api/chaos/status` (when admin
is enabled). The TUI Chaos screen also surfaces real-time injection
counts. Use these to confirm your bench / test client actually saw the
intended faults rather than guessing from the response log.

## Per-request fault matchers

By default, fault probabilities apply to every request. Add a `request_matcher`
to gate fault injection on properties of the incoming request — useful for
targeting specific clients, headers, body sizes, or chunked-encoded traffic
without affecting baseline traffic.

```yaml
fault_injection:
  enabled: true
  http_errors: [503]
  http_error_probability: 0.5     # 50% of *matching* requests get 503
  request_matcher:
    source_ips:
      - "10.0.0.0/8"              # CIDR range; bare IP works too
      - "192.168.1.42"
    headers:
      - name: "x-test"            # case-insensitive
        value: "yes"              # exact value; omit `value` for presence-only
    min_body_size_bytes: 1048576  # only requests with body >= 1 MB
    max_body_size_bytes: 10485760 # AND <= 10 MB (omit either side)
    chunked_only: true            # only Transfer-Encoding: chunked requests
```

Semantics: AND across fields, OR within a list. Empty matcher matches every
request (preserves prior behavior). Applies to all five fault paths
(HTTP errors, timeouts, partial responses, payload corruption,
connection errors).

## TCP-level connection errors

The `connection_error_kind` knob picks the wire-level behavior:

| Kind | What clients see |
|---|---|
| `http_503` (default) | HTTP 503 on a healthy connection — application-layer only |
| `tcp_reset` | TCP RST sent at accept time. Clients see `ECONNRESET`. |
| `tcp_close` | TCP FIN at accept time. Clients see EOF before any HTTP response. |

```yaml
fault_injection:
  enabled: true
  connection_errors: true
  connection_error_probability: 0.05
  connection_error_kind: tcp_reset    # http_503 | tcp_reset | tcp_close
```

The TCP-level kinds (`tcp_reset`, `tcp_close`) wrap the listener with a chaos
accept loop, so the fault is per-connection (every request that would have
pipelined onto that socket is affected). The wrapper installs automatically
when chaos is enabled and the kind is not `http_503`. Plain HTTP only — TLS
path doesn't yet support TCP-level injection.

## Timeouts and partial responses

When `timeout_errors: true` fires, MockForge sleeps for `timeout_ms` and then
returns `504 Gateway Timeout`. The sleep happens *before* the upstream
handler runs, so the client experiences a true server-side hang followed
by a 504. Applies uniformly to chunked and non-chunked requests.

When `partial_responses: true` fires, MockForge truncates the response body:

- **Non-chunked response** — preserves the original `Content-Length` header,
  so clients perceive an unexpected EOF (they expect N bytes, receive fewer).
- **Chunked response** — cuts before the terminating chunk, surfacing as a
  real protocol violation (`IncompleteMessage` / `ChunkedDecoderError` in
  most HTTP clients).

```yaml
fault_injection:
  enabled: true
  timeout_errors: true
  timeout_ms: 5000
  timeout_probability: 0.05      # 5% of matching requests hang then 504
  partial_responses: true
  partial_response_probability: 0.05  # 5% get a truncated body
```

## Predefined scenarios

| Scenario | Profile |
|---|---|
| `network_degradation` | High latency + packet loss |
| `service_instability` | Random 5xx errors + timeouts |
| `cascading_failure` | Combined latency, errors, connection drops, rate limits |
| `peak_traffic` | Aggressive rate limiting (per-endpoint) |
| `slow_backend` | Consistent 2 s latency on every request |

```bash
mockforge serve --chaos --chaos-scenario cascading_failure
```

You can also start, stop, and combine scenarios at runtime via the management
API (`POST /api/chaos/scenarios/{name}`, `DELETE /api/chaos/scenarios/{name}`).
See the [reference doc](https://github.com/SaaSy-Solutions/mockforge/blob/main/docs/CHAOS_ENGINEERING.md#management-api)
for the full API surface.

## Hot-reload

The chaos config is read on every request. Update it via `PUT /api/chaos/config`
and changes apply immediately to subsequent requests — no restart needed.

## Observability while testing

When chaos is on, you usually want to watch what's happening:

- **TUI dashboard** (in `mockforge-tui`): shows current and lifetime peak CPU,
  memory, and error rate. The error-rate peak is especially useful for
  multi-hour soak tests where you might miss a spike.
- **CSV metrics log**: set `MOCKFORGE_METRICS_LOG_FILE=/var/log/mockforge.csv`
  to append `timestamp,cpu_pct,mem_mb,total_reqs,err_rate` every 10 s.
  Survives restarts; chartable in Grafana / spreadsheets / anything.
- **Prometheus metrics**: expose `/metrics` and scrape with your usual
  observability stack (`mockforge serve --metrics --metrics-port 9090`).

## Pairing with load testing

Chaos is most useful when you're driving real traffic at the server. Combine
with [`mockforge bench`](./load-testing.md) to generate that traffic:

```bash
# Terminal 1 — server with chaos
mockforge serve --spec api.yaml --chaos --chaos-scenario service_instability

# Terminal 2 — drive load
mockforge bench --spec api.yaml --target http://localhost:3000 \
  --vus 50 --duration 5m
```

For chunked-encoding traffic specifically (e.g. to exercise the
`chunked_only: true` matcher), use the native generator:

```bash
mockforge bench-chunked \
  --target http://localhost:3000/upload \
  --concurrency 10 --duration 60 \
  --total-size-bytes 10485760
```

## Where to go next

- [Reference: full chaos config schema](https://github.com/SaaSy-Solutions/mockforge/blob/main/docs/CHAOS_ENGINEERING.md)
- [Chaos Lab UI](./chaos-lab.md) — the in-app interactive view
- [Load Testing](./load-testing.md) — generate traffic to exercise chaos
- [Configuration reference](../configuration/files.md) — every config field
