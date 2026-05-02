# MQTT Configuration Reference

This document covers MockForge's actual MQTT broker configuration surface
in v0.3.x. The MQTT implementation supports MQTT 3.1.1 and 5.0 protocol
features (publish, subscribe, QoS 0/1/2, retained messages, wildcards),
but the configuration surface itself is intentionally narrow — broader
concerns (TLS, auth, ACL, rate limiting) are handled at the platform
level rather than by per-protocol fields.

> **Status legend used in this chapter:**
> - **Implemented** (default) — fields under "Configuration" below.
> - **Roadmap** — TLS-with-cert-paths, authentication / authorization,
>   topic ACLs, JWT/OAuth2, persistent sessions with custom expiry, and
>   per-client message-flow tuning. Not yet wired into `MqttConfig`.
> - **Use the platform alternative** — for cross-cutting concerns (rate
>   limiting, max connections, latency injection, fault injection),
>   configure the [chaos engine](../../user-guide/chaos-engineering.md)
>   instead. It hooks every protocol on the same listener config surface.

## Configuration

The full set of YAML config knobs `MqttConfig` accepts in v0.3.x:

```yaml
mqtt:
  enabled: true                    # Start the MQTT broker
  port: 1883                       # Listener port (TCP, plaintext)
  host: "0.0.0.0"                  # Bind address
  max_connections: 1000            # Reject new clients beyond this count
  max_packet_size: 268435456       # 256 MiB — packet-size cap
  keep_alive_secs: 60              # PINGREQ deadline default
  fixtures_dir: "./fixtures/mqtt"  # Optional: directory of MQTT fixture files
  enable_retained_messages: true   # Honor RETAIN flag on PUBLISH
  max_retained_messages: 10000     # Total retained-message cap across all topics
```

That's the entire struct. Anything else you find in older docs is roadmap.

### CLI flags

```bash
mockforge serve --spec api.yaml --mqtt-port 1883
```

### Environment variables

```bash
MOCKFORGE_MQTT_ENABLED=true
MOCKFORGE_MQTT_PORT=1883
MOCKFORGE_MQTT_HOST=0.0.0.0
MOCKFORGE_MQTT_FIXTURES_DIR=./fixtures/mqtt
```

These are the only MQTT-specific env vars currently honored by the config
loader. Setting `MOCKFORGE_MQTT_TLS_ENABLED` / `MOCKFORGE_MQTT_AUTH_*`
won't error — they'll just be silently ignored, because the corresponding
config fields don't exist yet.

## QoS support

All three QoS levels work and are honored by the broker:

- **QoS 0** — At most once delivery (fire and forget).
- **QoS 1** — At least once delivery (PUBACK roundtrip).
- **QoS 2** — Exactly once delivery (full PUBREC / PUBREL / PUBCOMP).

QoS is a per-message attribute set by publishing clients. Fixtures can
also set QoS levels on auto-published messages — see
[fixtures.md](./fixtures.md).

## Retained messages and wildcards

Both work as defined by the MQTT specification:

- `enable_retained_messages: true` (default) makes MockForge honor the
  `RETAIN` flag on PUBLISH and replay the latest retained message on each
  topic to new subscribers.
- Subscriptions support `+` (single-level wildcard) and `#` (multi-level
  wildcard). A subscription to `sensors/+/temperature` matches
  `sensors/garage/temperature` but not `sensors/garage/humidity/inside`.
- Subscriptions to `sensors/#` match every topic under `sensors/`.

## Cross-cutting concerns

For features the per-protocol config doesn't expose, use the chaos engine.
Examples:

- **Rate limiting**: `chaos.rate_limit.requests_per_second` caps publishes
  / subscribes per client.
- **Max connections at a finer scope**: `chaos.traffic_shaping.max_connections`
  applies platform-wide.
- **Latency injection**: `chaos.latency.fixed_delay_ms` adds delay to every
  protocol response — useful for slow-broker simulations.
- **Fault injection**: drop random messages, force disconnects (TCP-level
  via `chaos.fault_injection.connection_error_kind: tcp_reset`).

See the [chaos engineering chapter](../../user-guide/chaos-engineering.md)
for the full surface.

## Roadmap (not yet implemented)

These appeared in older drafts of this chapter; they're tracked but not
landed. Configuring them in `mqtt:` will be silently ignored:

- TLS with cert / key / CA paths (only the bare `tls_enabled` field exists
  on AMQP today; MQTT doesn't have any TLS fields)
- Authentication: basic / JWT / OAuth2 (`auth_*` fields)
- Topic ACLs (`topic_acl` block)
- Persistent sessions with custom expiry (`persistent_sessions`,
  `session_expiry_secs`)
- Per-client message-flow tuning (`max_inflight_messages`,
  `max_queued_messages`)
- Worker-thread / socket buffer tuning

If you need any of these in the meantime, raise an issue describing your
use case so it can be prioritized.

## Configuration Examples

### Local development

```yaml
mqtt:
  enabled: true
  port: 1883
  host: "127.0.0.1"
  max_connections: 100
```

### IoT-style (lots of slow clients)

```yaml
mqtt:
  enabled: true
  port: 1883
  host: "0.0.0.0"
  max_connections: 5000
  max_packet_size: 524288         # 512 KiB — typical sensor payload
  keep_alive_secs: 300            # 5 min for battery-powered devices
  enable_retained_messages: true
  max_retained_messages: 50000
```

### With chaos for stress testing

```yaml
mqtt:
  enabled: true
  port: 1883
  max_connections: 1000

observability:
  chaos:
    enabled: true
    latency:
      fixed_delay_ms: 100
    fault_injection:
      enabled: true
      connection_errors: true
      connection_error_probability: 0.01
      connection_error_kind: tcp_close
```

## Troubleshooting

**Connection rejected immediately** — check `max_connections`. The broker
hard-caps; clients beyond the cap get refused at TCP accept time.

**Clients can connect but RETAIN is ignored** — confirm
`enable_retained_messages: true` and that `max_retained_messages` hasn't
been exhausted.

**TLS / auth config silently ignored** — see the Roadmap section. These
fields aren't wired yet; configure auth at your reverse-proxy / ingress
layer for now.

## Next Steps

- [Getting Started](./getting-started.md) — basic MQTT setup
- [Fixtures](./fixtures.md) — define MQTT mock scenarios
- [Examples](./examples.md) — real-world usage examples
- [Chaos Engineering](../../user-guide/chaos-engineering.md) — for
  cross-cutting concerns the per-protocol config doesn't yet cover
