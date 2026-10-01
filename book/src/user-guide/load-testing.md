# Load Testing

MockForge ships first-class load-testing tools that work against any HTTP API
— your real backend, a MockForge mock, or a hosted test environment. Two
commands, two purposes:

| Command | When to use |
|---|---|
| **`mockforge bench`** | OpenAPI-driven k6 script generation. Best for ramp/spike/soak/constant scenarios across realistic, schema-aware traffic. |
| **`mockforge bench-chunked`** | Native Rust generator for guaranteed `Transfer-Encoding: chunked` request bodies — chunk size, total size, inter-chunk delay all controllable. |

The full reference lives at
[docs/bench-command-examples.md](https://github.com/SaaSy-Solutions/mockforge/blob/main/docs/bench-command-examples.md)
and
[docs/LOAD_TESTING_GUIDE.md](https://github.com/SaaSy-Solutions/mockforge/blob/main/docs/LOAD_TESTING_GUIDE.md).
This chapter covers the patterns most teams reach for first.

## `mockforge bench` — OpenAPI-driven k6

### Quick start

```bash
mockforge bench \
  --spec api.yaml \
  --target http://localhost:3000 \
  --duration 60 --vus 10 \
  --scenario ramp-up
```

That generates a k6 script from `api.yaml`, writes it to `bench-results/`,
and runs it. The script:

- Walks every operation in the spec
- Generates realistic placeholder bodies / headers / path params
- Tracks `http_req_duration` and `http_req_failed` thresholds
- Prints a summary at the end (and writes `summary.json`)

### Load scenarios

| Scenario | Shape | Use case |
|---|---|---|
| `constant` | Flat at `--vus` | Steady-state throughput |
| `ramp-up` | 0 → vus → 0 | Find capacity ceiling |
| `spike` | Sudden burst | Test autoscaling response |
| `stress` | Aggressive ramp past vus | Find breaking point |
| `soak` | Long duration at moderate load | Memory leaks / degradation |

### Auth and custom headers

```bash
mockforge bench --spec api.yaml --target https://api.example.com \
  --auth "Bearer $TOKEN" \
  --headers "X-Tenant: acme,X-Region: us-east-1"
```

### Operation filtering

Test only specific endpoints:

```bash
# Specific operation
mockforge bench --spec api.yaml --target ... \
  --operations "GET /users,POST /orders"

# By method
mockforge bench --spec api.yaml --target ... \
  --operations "GET"

# Wildcards
mockforge bench --spec api.yaml --target ... \
  --operations "* /api/v1/*"
```

### Generate-only (don't run)

```bash
mockforge bench --spec api.yaml --target ... \
  --generate-only --script-output my-load-test.js
```

Edit the script by hand, then run with k6 directly. Useful for CI/CD where
the test infrastructure runs k6 on its own.

### Multi-target

```bash
mockforge bench --spec api.yaml --targets-file targets.txt \
  --max-concurrency 5 --results-format both
```

`targets.txt` lists one URL per line; bench runs against all of them in
parallel and produces per-target + aggregated reports.

## `mockforge bench-chunked` — native chunked traffic

When you need real `Transfer-Encoding: chunked` request bodies on the wire,
`mockforge bench` (k6/Go-based) may fall back to `Content-Length` because
Go's HTTP transport decides chunking from body type. `bench-chunked` bypasses
that — it builds a streaming body via hyper's `Body::wrap_stream` with no
declared length, so the wire is **always** chunked.

```bash
mockforge bench-chunked \
  --target http://localhost:3000/upload \
  --concurrency 10 --duration 60 \
  --chunk-size-bytes 4096 \
  --total-size-bytes 10485760 \
  --chunk-interval-ms 50 \
  --header "Authorization: Bearer $TOKEN"
```

| Flag | Meaning |
|---|---|
| `--target` | URL to POST chunked bodies at |
| `--targets-file` | Bench every target in the file in parallel instead of one `--target` |
| `--max-concurrency` | Max targets running at once with `--targets-file` (default 10) |
| `--method` | `POST` (default), `PUT`, or `PATCH` |
| `--concurrency` | Concurrent uploads **per target**: each worker sends one chunked request at a time and starts the next as soon as it finishes |
| `--duration` | Run length (`30s`, `15m`, ...); **per operation** with `--spec` |
| `--chunk-size-bytes` | Bytes per chunk emitted into the body stream |
| `--total-size-bytes` | Total body size per request |
| `--chunk-interval-ms` | Pause between chunks (0 = back-to-back); the first chunk goes out with the headers |
| `--rps` | Cap on request starts per second, per target (shared by its workers) |
| `--cps` | New TCP/TLS connection per request, so connections/s = requests/s |
| `--rounds` / `--repeat-until` | Re-run the whole pass N times / until a wall-clock duration elapses |
| `--keep-rounds` | Keep only the newest N `round_*` dirs during a campaign |
| `--header` | Extra `Name: Value` header; may be repeated |
| `--insecure` | Skip TLS certificate verification |

### Common patterns

**Slow upload simulation** — high `--chunk-interval-ms` keeps the connection
open for minutes per request, stressing the server's idle/slow-connection
handling:

```bash
mockforge bench-chunked --target http://server/upload \
  --concurrency 50 --duration 300 \
  --chunk-size-bytes 256 --total-size-bytes 65536 \
  --chunk-interval-ms 500
```

**Large body soak** — find the server's max-body-size limits and memory
behavior:

```bash
mockforge bench-chunked --target http://server/upload \
  --concurrency 5 --duration 180 \
  --chunk-size-bytes 1048576 --total-size-bytes 1073741824
```

**Multiple targets** — send chunked traffic to every server in a targets
file at once. The file format is the same as `mockforge bench --targets-file`
(one URL per line, or JSON with per-target `auth`, `headers` and `spec`).
Without `--spec` each entry is a full endpoint URL; with `--spec` each entry
is a base URL and every POST/PUT/PATCH operation runs against it:

```bash
mockforge bench-chunked --targets-file targets.txt --spec api.yaml \
  --max-concurrency 5 --concurrency 10 --duration 10m \
  --chunk-size-bytes 4096 --total-size-bytes 1048576 --export-requests
```

Output lines are prefixed with `[target_N]`. Per-target files go to
`<output>/target_N/`, and `<output>/chunked-multi-target-summary.json` rolls up
request, byte and status-code totals for every target.

**How the numbers combine.** `--max-concurrency` × `--concurrency` is the
peak number of uploads in flight (e.g. 5 × 10 = 50). With `--spec`, every
target gets every POST/PUT/PATCH operation; operations run one after another
per target, each for `--duration`. If the targets file has more entries than
`--max-concurrency`, the extra targets wait for a running one to finish all of
its operations. Each request lasts at least
`(total_size_bytes / chunk_size_bytes - 1) × chunk_interval_ms`, so without
`--rps` the per-target request rate is roughly `concurrency / request_seconds`
and the per-target bandwidth is roughly
`concurrency × chunk_size_bytes / chunk_interval_ms`. A request in flight when
`--duration` expires is allowed to finish.

**Fixed rate and longevity** — `--rps` paces request starts per target,
`--cps` makes each one a new connection, and `--repeat-until` loops the whole
pass (each round in `<output>/round_N/`, per-round stats in
`<output>/campaign.jsonl` and `<output>/round-summaries/`):

```bash
mockforge bench-chunked --targets-file targets.txt --spec api.yaml \
  --concurrency 20 --rps 2 --cps --duration 15m \
  --repeat-until 24h --keep-rounds 3 --export-requests
```

`--rps` is a ceiling: with 128 s uploads and `--concurrency 10`, a target can
only start about 10 / 128 ≈ 0.08 requests per second, so raise
`--concurrency` to reach higher rates.

**Chunked + chaos matching** — pair with `chunked_only: true` in
`fault_injection.request_matcher` (see
[Chaos Engineering](./chaos-engineering.md)) to inject faults *only* on
chunked traffic:

```yaml
# server config
fault_injection:
  enabled: true
  http_errors: [503]
  http_error_probability: 0.5
  request_matcher:
    chunked_only: true
```

```bash
# in another terminal
mockforge bench-chunked --target http://localhost:3000/upload \
  --concurrency 10 --duration 60 \
  --total-size-bytes 1048576
```

## Pairing with chaos

Load testing alone tells you the *throughput* your server handles. Pair it
with [chaos engineering](./chaos-engineering.md) to test how it behaves
when things go wrong:

```bash
# Terminal 1
mockforge serve --spec api.yaml --chaos --chaos-scenario cascading_failure

# Terminal 2
mockforge bench --spec api.yaml --target http://localhost:3000 \
  --duration 5m --vus 50
```

Watch the TUI dashboard to see CPU / memory / error-rate spike as chaos
fires. Set `MOCKFORGE_METRICS_LOG_FILE=metrics.csv` for a persistent record
of multi-hour runs.

## Watching the run

While a bench is running, you usually want to monitor:

- **`mockforge-tui`** — live dashboard with current and peak CPU, memory,
  error rate. Auto-refreshes every 2 s.
- **CSV metrics log** — `MOCKFORGE_METRICS_LOG_FILE=path` appends a row per
  10 s. Persistent record for soak tests.
- **k6 stdout / `summary.json`** — at the end of the bench run.
- **Prometheus** — `mockforge serve --metrics --metrics-port 9090`, scrape
  with your usual stack.

## CI/CD

```bash
#!/bin/bash
set -e

# Start MockForge against staging
mockforge serve --spec api.yaml &
SERVER=$!
trap "kill $SERVER" EXIT

# Wait for ready
until curl -sf http://localhost:3000/__health > /dev/null; do sleep 1; done

# Run bench with thresholds enforced
mockforge bench --spec api.yaml \
  --target http://localhost:3000 \
  --duration 30 --vus 20 \
  --threshold-percentile 95 \
  --threshold-ms 250 \
  --max-error-rate 0.01

# Exit code propagates from k6 — fails the build if thresholds break
```

## Where to go next

- [Reference: bench command examples](https://github.com/SaaSy-Solutions/mockforge/blob/main/docs/bench-command-examples.md)
- [Reference: load testing guide](https://github.com/SaaSy-Solutions/mockforge/blob/main/docs/LOAD_TESTING_GUIDE.md)
- [Chaos Engineering](./chaos-engineering.md) — pair load tests with fault injection
- [k6 Documentation](https://k6.io/docs/) — k6 itself
