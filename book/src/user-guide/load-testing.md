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
| `--raw-body` | Send raw `X` filler for any declared Content-Type (negative testing); preserves request headers |
| `--chunk-interval-ms` | Pause between chunks (0 = back-to-back); the first chunk goes out with the headers |
| `--rps` | Cap on request starts per second, per target (shared by its workers) |
| `--cps` | New TCP/TLS connection per request, so connections/s = requests/s |
| `--rounds` / `--repeat-until` | Re-run the whole pass N times / until a wall-clock duration elapses |
| `--keep-rounds` | Keep only the newest N `round_*` dirs during a campaign |
| `--header` | Extra `Name: Value` header; may be repeated |
| `--insecure` | Skip TLS certificate verification |

**Request body.** With a JSON `Content-Type` (`application/json` or any
`+json` type, from `--header` or a targets-file `headers` entry), each request
body is a valid JSON document of exactly `--total-size-bytes`: the
spec-generated request body for the operation (with `--spec`) plus a
`"_padding"` string field that fills it to size. Any other content type gets
`X` filler bytes, which suits `application/octet-stream` uploads.

**Negative content-type testing.** Add `--raw-body` to send the original
`X` filler even when the targets file or `--header` declares
`application/json` (or any other type). The Content-Type header stays exactly
as configured; only body generation changes. This intentionally produces
invalid JSON for testing WAF rejection. It works with single targets,
`--targets-file`, `--spec`, and campaigns. Omit it to keep automatic JSON
body generation.

```bash
mockforge bench-chunked --targets-file vs_list1.json --spec oas.json \
  --max-concurrency 2 --concurrency 10 --duration 30s \
  --chunk-size-bytes 4096 --total-size-bytes 1048576 --chunk-interval-ms 50 \
  --insecure --raw-body --repeat-until 24h --keep-rounds 2
```

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

### Reading chunked reports

- `campaign_elapsed_seconds` is cumulative wall-clock time since the campaign
  started, recorded after each round. In round 1, 33517 seconds means that
  first pass ended after about 9 hours 19 minutes. In later rounds it includes
  previous rounds; it is not the current round's duration.
- `round_elapsed_seconds` measures just that round, including planning and
  any wait for targets. `--duration` is per operation, not per round. Every
  target runs every POST/PUT/PATCH operation sequentially; active targets run
  in parallel. In-flight uploads may finish after an operation's duration.
  Slow responses, failed connections, and timeouts can extend the pass.
- `rps_per_target` is the configured request-start limit from `--rps`.
  `null` means no limit was supplied, not zero throughput or a rate rounded
  down. `rate_limit_enabled` states whether the limit is active.
- Each target's `measured_rps` is completed request attempts (including
  transport failures and non-2xx responses) divided by `elapsed_seconds`,
  the sum of its operation runtimes. It is numeric even when no limit was
  configured. This average includes time spent finishing in-flight requests;
  it is not the configured start rate or a successful-response rate.
  With `--export-requests`, each operation also has `result.req_per_sec`.
- `start_window_rps` is the same attempt count divided by
  `load_window_seconds`, the sum of each operation's `--duration`: the window
  during which requests could start. This is the number to compare with
  `--rps`; it is not diluted by waiting for in-flight requests, so it stays
  meaningful on 30 or 60 second runs. Exported operations carry the
  equivalent `result.start_rate_per_sec` and `result.load_window_ms`.
- `upload_mbps` is request body bytes sent, in megabits per second over
  `elapsed_seconds`. `bytes_sent` counts the full body of each attempt that
  received an HTTP response.
- `new_connection_per_request` is true only with `--cps`. Without it,
  connections can be reused. With `--cps --rps 1`, each target starts at most
  one request per second, each on a fresh connection. Failed connection
  attempts can mean fewer established connections than request starts;
  these reports do not measure established connections per second.
- `raw_body` records whether the raw filler override was selected.
- `timed_out` counts failed attempts that hit the request timeout (a subset
  of `failed`). `request_timeout_mode` is `auto`, `fixed`, or `off`, with
  `request_timeout_secs` set for `fixed`.
- `overlap_operations` records whether `--overlap-operations` was used. With
  it, `elapsed_seconds` is the pass's wall clock rather than the sum of
  operation runtimes, since operations overlap.
  Existing report fields are retained; these explanatory fields are additive.

For 1 MiB uploads in 4 KiB chunks with 50 ms between chunks, the minimum
upload time is `(256 - 1) × 0.05 = 12.75` seconds. Ten workers can sustain
roughly `10 / 12.75 ≈ 0.78` requests/s per target before response latency.
`--rps 1` is still a ceiling; increase `--concurrency` (at least 13 workers
before allowing for response latency) to approach one request/s. Use
`--operation-id ID` to test rate controls on one operation before running the
whole spec.

### Diagnosing a rate below `--rps`

`--rps 1` caps **request starts per target**, shared by that target's
workers. Two targets can each start one request/s independently; the limit
is not divided between them. It is a ceiling, not a guarantee: workers
waiting for an upload or response cannot start another request.

A short run reports a much lower `measured_rps` even when starts are paced
correctly. `--duration` ends the window for starting requests, then
in-flight requests finish. For example, 26 attempts started during a
30-second window and finished after 76.28 seconds report
`measured_rps = 26 / 76.28 = 0.34`, while `start_window_rps` reports
`26 / 30 = 0.87` attempts/s. The console shows both:

```text
  RPS (started):  0.87 req/s over the 30.0s load window
  RPS (overall):  0.34 req/s over 76.3s, incl. in-flight drain
  Upload:         2.86 Mbps
```

Use `RPS (started)` / `start_window_rps` to check a `--rps` cap. That is a window average,
not proof of evenly spaced starts; verify start timestamps at the receiver
or in a packet capture. Even a healthy target with 12.75-second uploads can
report roughly `30 / (29 + 12.75) = 0.72` requests/s on a 30-second run.

With `--spec`, every operation prints its own block, and those lines end in
`(this operation only)`. They are not the rate for the whole pass. With
`--overlap-operations` the next operation starts while the previous one's
last uploads finish, so the pass takes the sum of the load windows plus one
final drain, not the sum of the per-operation times. The pass summary at the
end of each pass gives the aggregate, with the inputs used to compute it:

```text
Multi-target chunked summary (whole pass per target, all operations):
  target_1   https://10.0.0.1:443  requests=15150 ok=15150 failed=0 bytes=15885926400 start_rps=1.00 rps=1.00 load_window=15150.0s elapsed=15162.1s
```

Here `start_rps = requests / load_window = 15150 / 15150.0` and
`rps = requests / elapsed = 15150 / 15162.1 = 0.9992`. The same values, unrounded,
are `start_window_rps`, `measured_rps`, `load_window_seconds` and
`elapsed_seconds` in `chunked-multi-target-summary.json` and in each
`round-summaries/round_*.json`. A single `--target` run with `--spec` prints
the same figures on a `Pass summary (all operations):` line.

`--repeat-until` is checked before each new pass. A pass that is already
running when the time is reached still finishes, so a campaign can run past
`--repeat-until` by up to one pass.

To investigate:

1. Run one longer operation (for example, `--duration 300s`) without campaign
   repetition first. Compare a capped run with an uncapped run using the
   same body, pacing, and concurrency. A longer window reduces the effect
   of waiting for the last in-flight requests.
2. Check `failed` and `status_counts`. `failed` counts transport errors;
   HTTP 503 responses appear separately in `status_counts`. The existing
   `successful` counter counts attempts that received an HTTP response,
   including non-2xx responses; it does not mean every response was 2xx.
   Neither low throughput nor a 503 alone identifies the failing component.
3. Save the console output: it includes latency percentiles and sampled
   non-2xx response headers/body excerpts. With `--spec --export-requests`,
   those diagnostics also appear per operation in `chunked-requests.json`.
   `--export-requests` requires a spec; it does not export no-spec runs.
4. Check proxy/WAF/backend logs for the same interval. If uncapped traffic
   also stays below the requested cap, investigate upload/response latency,
   errors, and available workers before increasing the limit. The worker
   estimate must use the **whole request time**, including response latency,
   not just the minimum chunk-upload time.

`--cps` changes connection reuse, not the request-rate ceiling. Add it only
when testing a new connection per request; request starts are not a measure
of successfully established TCP/TLS connections.

### Long campaigns and stalled requests

**Where `measured_rps` falls short of `--rps`.** With `--spec`, operations
run one after another and each waits for its in-flight uploads before the
next starts. Starts pause for about one request time (13 s for the 1 MiB
example above) at every operation boundary, so a campaign of 30-second
operations averages roughly `30 / (30 + 13) ≈ 0.70` requests/s even though
each window starts at exactly `--rps 1`. Raising `--rps` to compensate
overshoots the rate inside each window. Instead, add
`--overlap-operations`: each operation starts as soon as the previous one's
`--duration` window closes, so starts stay continuous at `--rps` across the
whole pass. Workers of consecutive operations overlap for one request time,
so up to twice `--concurrency` connections can be open briefly. Rounds of a
`--repeat-until` campaign still run one after another, so each round ends
with one drain.

**Without `--rps`,** each worker sends requests back to back, so the steady
rate is `concurrency / request time` (20 workers and 13.07 s uploads give
1.53 requests/s). Short windows round up: all workers start at once, so a
30-second window fits three starts per worker (`60 / 30 = 2.00`).

**Request timeout.** Each worker sends one whole request at a time on its
own connection; a stalled request blocks only that worker. `--request-timeout
SECS` bounds a request end to end (connect, upload, response). It defaults to
twice the minimum upload time plus 60 s (85.5 s for the 1 MiB example). A
request that misses it counts as failed and `timed_out`, and its worker moves
on; the console shows `Failed: N (M timed out after Xs)`. `--request-timeout
0` disables it, leaving a silent drop to the OS TCP timeouts (minutes).

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
