# Rate Limiting & Traffic Shaping

Rate limiting and traffic shaping live in the chaos engine but they're
useful well beyond chaos scenarios. This chapter covers them on their own
because most teams reach for them outside of fault injection — to rehearse
client-side backoff, throttle local dev environments, or simulate flaky
backhaul during integration tests.

## Rate Limiting

Cap the request rate at the HTTP layer. Three scopes you can combine:

| Scope | Use case |
|---|---|
| **Global** | "Cap everything at 100 RPS" — load shedding |
| **Per-IP** | "Each client gets 10 RPS" — abuse mitigation |
| **Per-endpoint** | "`/login` gets 5 RPS" — per-route policy |

YAML config:

```yaml
observability:
  chaos:
    enabled: true
    rate_limit:
      enabled: true
      requests_per_second: 100        # token-bucket refill rate
      burst_size: 10                  # additional tokens for short spikes
      per_ip: true                    # token bucket per source IP
      per_endpoint: false             # token bucket per (method, path)
```

Quick CLI form:

```bash
mockforge serve --spec api.yaml --chaos --chaos-rate-limit 100
```

Rejected requests return `429 Too Many Requests` with body
`Rate limit exceeded`. Counts as a `chaos_rate_limit_violations_total` metric
labeled by scope (`global` / `per_ip` / `per_endpoint`) so dashboards can
distinguish.

### Disabling for trusted clients

```bash
# CLI flag
mockforge serve --spec api.yaml --no-rate-limit

# Or env var (preferred for SDK / Docker / k8s setups)
MOCKFORGE_RATE_LIMIT_ENABLED=false mockforge serve --spec api.yaml
```

For a related case where requests still hit the limit middleware but get
exempted, see the **per-request matchers** in
[Chaos Engineering](./chaos-engineering.md) — you can scope rate limits to
only requests with certain headers or source IPs.

### Burst behavior

Rate limiting uses token-bucket semantics, not a hard request-per-second
cap:

- `requests_per_second: 100, burst_size: 10` allows 110 requests in the
  first second (refill + initial burst), then 100 RPS sustained.
- Use a small burst (e.g. 10–20% of RPS) for "smooth client traffic with
  occasional spikes." Use a large burst (e.g. 2× RPS) for "I want to
  observe what happens when traffic spikes briefly."

## Traffic Shaping

Sub-application-layer constraints. Rate limiting throttles requests; traffic
shaping throttles bytes and connections.

```yaml
observability:
  chaos:
    enabled: true
    traffic_shaping:
      enabled: true
      bandwidth_limit_bps: 1000000     # 1 MB/s, 0 = unlimited
      packet_loss_percent: 2.0         # randomly drop 2% of "packets"
      max_connections: 100             # reject when accept queue > 100
      connection_timeout_ms: 30000     # drop idle connections after 30s
```

### Bandwidth throttling

Caps bytes/sec going through the HTTP layer. Useful for:

- Simulating slow clients (3G, dial-up, satellite).
- Pairing with `mockforge bench --duration 60` to see how the client
  handles low-bandwidth conditions.
- Stress-testing buffer / backpressure logic.

CLI:

```bash
mockforge serve --spec api.yaml --chaos --chaos-bandwidth-limit 100000
```

(100 KB/s.)

### Packet loss

Randomly returns `408 Request Timeout` for the configured percentage of
requests. Not real packet loss at the network layer — that requires `tc` or
`iptables`. The HTTP-layer simulation is enough to test client-side retry /
timeout logic in most cases.

```bash
mockforge serve --spec api.yaml --chaos --chaos-packet-loss 5
```

### Connection limits

Caps the number of concurrent connections the listener accepts. New
connections beyond `max_connections` get rejected with `503 Service
Unavailable`. Useful for testing connection-pool exhaustion in clients.

## Predefined network profiles

For common real-world conditions, MockForge ships built-in network profiles
that combine latency + bandwidth + connection-error rates:

| Profile | Latency | Bandwidth | Notes |
|---|---|---|---|
| `slow_3g` | 400 ms | 400 KB/s | + 1% packet loss |
| `fast_3g` | 150 ms | 1.5 MB/s | + 0.5% packet loss |
| `flaky_wifi` | 50 ms | unlimited | + 5% packet loss + 3% disconnects |
| `cable` | 20 ms | 10 MB/s | clean |
| `dialup` | 2 s | 50 KB/s | + 2% packet loss |

```bash
mockforge serve --spec api.yaml --chaos-profile slow_3g
```

Or list and apply via the management API:

```bash
curl http://localhost:3000/api/chaos/profiles            # list
curl -X POST http://localhost:3000/api/chaos/profiles/slow_3g/apply
```

## Example: rehearse client backoff

Question you want to answer: *does my client handle 429s with exponential
backoff?*

```bash
# Server: 10 RPS cap, return 429 above that
mockforge serve --spec api.yaml --chaos --chaos-rate-limit 10

# Drive 50 RPS at it
mockforge bench --spec api.yaml --target http://localhost:3000 \
  --vus 50 --duration 30 --scenario constant
```

Watch the bench output for `http_req_failed` rate. A correctly-implemented
client with backoff will show error rate climbing initially then settling
near 80% (40 RPS rejected). A client without backoff will show error rate
flat at ~80% for the whole run.

## Example: low-bandwidth soak

```bash
mockforge serve --spec api.yaml \
  --chaos --chaos-bandwidth-limit 100000 \
  --chaos --chaos-packet-loss 1.0 \
  --metrics --metrics-port 9090

mockforge bench --spec api.yaml --target http://localhost:3000 \
  --vus 5 --duration 1h --scenario soak
```

Now scrape Prometheus / watch `mockforge tui` for memory growth — slow
clients tend to surface buffering bugs.

## Where to go next

- [Chaos Engineering](./chaos-engineering.md) — full chaos config (combine
  rate-limit / traffic-shaping with fault injection)
- [Observability & Metrics](./observability.md) — `chaos_rate_limit_violations_total`
  labels and dashboards
- [Load Testing](./load-testing.md) — driving load through these limits
- [Reference: full config schema](../reference/config-schema.md)
