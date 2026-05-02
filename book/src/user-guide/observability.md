# Observability & Metrics

When MockForge is sitting in front of real client traffic — load tests, soak
tests, integration test suites running for hours — you usually need three
things:

1. **Live numbers.** What's the server doing right now?
2. **Structured metrics.** Pull into Grafana / Datadog / your dashboarding tool.
3. **A persistent record.** When you come back tomorrow, what was the worst
   case overnight?

MockForge ships all three. Pick whichever fits your workflow.

| Surface | Source of truth | When to use |
|---|---|---|
| TUI dashboard | In-process | Local dev, quick eyeballing |
| Prometheus `/metrics` | In-process Prometheus registry | You already run Prometheus / Grafana |
| OpenTelemetry / OTLP | OTLP exporter → collector | You already run Jaeger / Tempo / Honeycomb |
| CSV metrics log | `MOCKFORGE_METRICS_LOG_FILE` env var | Multi-day soak tests; want a flat file |

These aren't mutually exclusive — turn on whichever combination fits.

## TUI dashboard

```bash
mockforge tui
```

Live dashboard in the terminal. Auto-refreshes every 2 s. Shows current and
**lifetime peak** CPU / memory / error rate next to the live numbers, so
you can leave it open during a soak test and glance at it later to see what
the worst case was.

For the full panel layout and keybindings, see the
[TUI Dashboard chapter](./tui-dashboard.md).

## Prometheus metrics

Enable the metrics endpoint when starting the server:

```bash
mockforge serve --spec api.yaml --metrics --metrics-port 9090
```

Scrape `http://localhost:9090/metrics` from Prometheus. Default port is
`9090`; change with `--metrics-port` or in config:

```yaml
observability:
  metrics:
    enabled: true
    port: 9090
    path: "/metrics"
```

### What gets exported

Mock server core:
- `http_requests_total` (counter, labels: `method`, `path`, `status`)
- `http_request_duration_seconds` (histogram, labels: `method`, `path`)
- `active_connections` (gauge, labels: `protocol`)
- `mock_response_generations_total` (counter, labels: `template_type`)

Chaos engine (when `--chaos` is on):
- `chaos_faults_injected_total` (counter, labels: `kind`, `endpoint`) — kind ∈ `http_error`, `connection_error`, `timeout`, `partial_response`, `payload_corruption`
- `chaos_latency_injected_seconds` (histogram, labels: `endpoint`)
- `chaos_rate_limit_violations_total` (counter, labels: `kind`) — kind ∈ `global`, `per_ip`, `per_endpoint`
- `chaos_circuit_breaker_state` (gauge: `0` closed, `1` half-open, `2` open)
- `chaos_bulkhead_concurrent` (gauge)
- `chaos_scenarios_total` (counter, labels: `scenario`, `event`)

Resilience features:
- `chaos_orchestration_step_duration_seconds` (histogram)
- `chaos_orchestration_executions_total` (counter, labels: `status`)
- `chaos_assertion_results_total` (counter, labels: `result`)

### Grafana starter dashboard

A starter Grafana dashboard JSON ships in the repo at
`book/src/assets/mockforge-starter.json` (when present in your install).
Import via Grafana **Dashboards → Import** and pick your Prometheus data
source. The dashboard graphs request rate, p50/p95/p99 latency, error rate,
and per-fault chaos counters by default.

## OpenTelemetry / OTLP

For distributed tracing, MockForge can export OTLP spans to any compatible
collector (Jaeger, Tempo, Honeycomb, Datadog, etc.).

```bash
MOCKFORGE_OTLP_ENDPOINT=http://otel-collector:4317 \
MOCKFORGE_OTLP_SERVICE_NAME=mockforge \
MOCKFORGE_OTLP_SAMPLING_RATE=1.0 \
  mockforge serve --spec api.yaml --tracing
```

YAML form:

```yaml
observability:
  tracing:
    enabled: true
    exporter: otlp                    # otlp | jaeger
    otlp:
      endpoint: "http://otel-collector:4317"
      service_name: "mockforge"
      sampling_rate: 1.0              # 0.0–1.0, 1.0 = trace every request
    # Or, for legacy Jaeger native:
    jaeger:
      endpoint: "http://jaeger:14268/api/traces"
```

### What gets traced

Each incoming HTTP / gRPC / WebSocket request opens a top-level span with
attributes: `http.method`, `http.route`, `http.status_code`, `mockforge.protocol`.
Inner spans cover:

- OpenAPI route matching
- Template expansion
- Plugin invocation (one span per plugin call)
- Chaos middleware decisions (latency injected, faults injected)
- Upstream proxy calls (if you're running in proxy mode)

Sampling is head-based at `sampling_rate`. For chaos / load testing where
you only care about the long tail, set `0.01` and have your collector
upweight on `error=true` spans.

### Pairing with metrics

OTLP and Prometheus aren't either/or. A common pattern:

- **Prometheus** for rate / error / duration RED metrics (cheap, always on).
- **OTLP** for individual request traces (sampled, on for investigations).

Both can run simultaneously. Run a combined demo with:

```bash
MOCKFORGE_OTLP_ENDPOINT=http://localhost:4317 \
  mockforge serve --spec api.yaml \
    --metrics --metrics-port 9090 \
    --tracing
```

## CSV metrics log (multi-day soak)

For long-running test scenarios where you want a flat record that survives
a server restart and chart in any tool:

```bash
MOCKFORGE_METRICS_LOG_FILE=/var/log/mockforge-metrics.csv \
  mockforge serve --spec api.yaml --admin
```

Appends one row every 10 s:

```
timestamp,cpu_pct,mem_mb,total_reqs,err_rate
2026-05-02T12:00:00Z,3.45,412,12345,0.012
2026-05-02T12:00:10Z,4.12,415,12567,0.013
```

Parse with anything (`pandas`, `awk`, Grafana CSV data source, Excel). Header
is written when the file is empty/new; existing files just get appended to,
so multiple runs accumulate.

## Combining all four

For a real chaos / load test you usually want everything at once:

```bash
# Server: chaos + metrics + tracing + CSV log
MOCKFORGE_OTLP_ENDPOINT=http://otel-collector:4317 \
MOCKFORGE_METRICS_LOG_FILE=/var/log/mockforge-soak.csv \
  mockforge serve \
    --spec api.yaml \
    --chaos --chaos-scenario service_instability \
    --metrics --metrics-port 9090 \
    --tracing \
    --admin --admin-port 9080

# Watch live in one terminal
mockforge tui

# Drive load in another
mockforge bench --spec api.yaml --target http://localhost:3000 \
  --duration 6h --vus 50
```

Now you have:
- **Live**: TUI shows current state + peak readouts.
- **Streaming**: Prometheus `/metrics` for Grafana panels.
- **Sampled deep-dives**: OTLP traces for individual problem requests.
- **Persistent**: CSV log survives the run for next-day analysis.

## Where to go next

- [TUI Dashboard](./tui-dashboard.md) — full panel reference
- [Rate Limiting & Traffic Shaping](./rate-limiting.md) — knobs that show up in chaos counters
- [Chaos Engineering](./chaos-engineering.md) — what generates the metric movement
- [Load Testing](./load-testing.md) — how to drive load past it
