# TUI Dashboard

`mockforge tui` is a terminal dashboard you point at a running MockForge
admin server. It's the fastest way to see what's happening: live metrics,
recent requests, chaos events, fixture inventory, error logs — all on one
screen, no browser.

```bash
# Start the server with admin enabled
mockforge serve --spec api.yaml --admin --admin-port 9080

# In another terminal
mockforge tui
```

The TUI talks to the admin server's HTTP API (default
`http://localhost:9080`); set the `--admin-url` flag if it's elsewhere.

## Navigation

| Key | Action |
|---|---|
| `Tab` / `Shift-Tab` | Next / previous panel |
| `1`–`9`, `0` | Jump to panel by number |
| `r` | Force refresh the current panel |
| `q` / `Ctrl-C` | Quit |
| `↑` `↓` `j` `k` | Scroll within a panel |
| `Enter` | Drill into the highlighted row (where applicable) |

Panels auto-refresh every 2 s; `r` forces an immediate fetch.

## Dashboard panel (`1`)

Four quadrants:

```
┌─ Server Status ─────────────────┬─ System Metrics ─────────────────┐
│ ● HTTP    127.0.0.1:3000        │ CPU: 18% (peak 67%)              │
│ ● WS      127.0.0.1:3001        │ Mem: 412 MB (peak 891 MB)        │
│ ● Admin   127.0.0.1:9080        │ Threads: 8  Routes: 25  ...      │
│                                 │                                  │
├─ Request Stats ─────────────────┼─ Recent Logs ────────────────────┤
│ Total: 12.5K  Err Rate: 0.4%    │ 14:23:01 GET    /api/users  200  │
│ (peak 12.3%)                    │ 14:23:01 POST   /api/orders 201  │
│ Avg RT: 34ms                    │ 14:23:00 GET    /api/users  200  │
│ ┃▆█▇▅▆█▇▅▆█▇▅▆█▇▅▆█▇▅           │ ...                              │
└─────────────────────────────────┴──────────────────────────────────┘
```

### What each cell shows

**Server Status** — one row per protocol you've enabled. `●` = up, `○` =
down. Address is what the listener actually bound to (so you see the real
port when you started with `--http-port 0`).

**System Metrics** — current value + lifetime peak (since server start, or
since the last `reset_metric_peaks()` API call):

- `CPU: <current>% (peak <peak>%)` — process CPU usage. Peak helps you
  catch transient spikes during multi-hour soak tests.
- `Mem: <current> MB (peak <peak> MB)` — process RSS. Peak is the
  important number for memory-leak hunting.
- `Threads: N` — count of OS threads in the runtime.
- `Routes: N` — registered HTTP routes.
- `Fixtures: N` — fixture files discovered.

**Request Stats** — total requests served, error rate (5xx + 4xx / total),
peak error rate, average response time, and a sparkline of request rate
over the last 60 ticks (~2 minutes).

**Recent Logs** — last 10 requests (or however many fit), newest at top.
Color-coded by HTTP method and status code.

### Resetting peaks

Peaks accumulate from server start. To reset at the beginning of a new test
run without restarting the server, hit the admin API:

```bash
curl -X POST http://localhost:9080/api/admin/metrics/reset-peaks
```

Or reset programmatically via the SDK.

## Other panels

| Panel | What it shows |
|---|---|
| `Routes` (`2`) | Every HTTP route registered, with hit count and last-request timestamp |
| `Fixtures` (`3`) | Fixture files on disk, sizes, last-modified |
| `Logs` (`4`) | Full request log with filter/search |
| `Metrics` (`5`) | Detailed numeric metrics (response-time distribution, status code histogram) |
| `Chaos` (`6`) | Live chaos engine state — current scenario, fault counters per kind, request matchers |
| `Plugins` (`7`) | Loaded plugins, their status, recent invocations |
| `Recorder` (`8`) | Traffic recordings — start/stop, file sizes, replay status |
| `Health` (`9`) | Health checks, circuit breaker / bulkhead state, dependency status |

Each is also reachable via the admin web UI at `http://localhost:9080`; the
TUI is the same data over the same API, just rendered for the terminal.

## When to use the TUI vs admin web UI

| Use TUI when… | Use admin UI when… |
|---|---|
| You're SSH'd to a remote box | You want to edit fixtures interactively |
| You want to leave it open during a soak test | You're sharing a screenshot |
| Browser is too heavy | You need the visual config builder |
| You want everything on one screen | You're walking someone through a feature |

## CSV log alongside

The TUI peak metrics are in-memory and reset on server restart. For a
permanent record across restarts, set:

```bash
MOCKFORGE_METRICS_LOG_FILE=/var/log/mockforge.csv mockforge serve ...
```

The same numbers the TUI displays (CPU%, memory MB, total requests, error
rate) get appended every 10 s to the CSV file. See the
[Observability chapter](./observability.md#csv-metrics-log-multi-day-soak)
for details.

## Troubleshooting

**"Failed to fetch dashboard"** — the admin server isn't running, or it's
on a different host/port. Use `--admin-url http://host:port` and
retry. Make sure you started `mockforge serve` with `--admin`.

**Metrics show all zeroes** — system monitoring runs in a background task
that takes ~10 s to collect its first sample after server start. Wait a
moment and hit `r` to refresh.

**TUI feels sluggish** — the 2 s refresh fetches a lot. Switch to the
admin web UI for low-traffic panels (Plugins, Fixtures); the TUI is most
useful for high-velocity ones (Dashboard, Logs, Chaos, Metrics).

## Where to go next

- [Observability & Metrics](./observability.md) — Prometheus / OTLP / CSV
  log all in one place
- [Admin UI](./admin-ui.md) — the browser-based equivalent
- [Chaos Engineering](./chaos-engineering.md) — what drives the chaos panel
- [Load Testing](./load-testing.md) — drives the request-rate sparkline
