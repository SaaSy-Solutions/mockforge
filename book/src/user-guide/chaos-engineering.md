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
