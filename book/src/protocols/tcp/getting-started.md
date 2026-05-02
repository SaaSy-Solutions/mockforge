# TCP Mocking

MockForge can mock arbitrary TCP services — anything from a custom binary
protocol to a line-delimited text protocol — without you needing to write
a parser. Configure a port, point it at a fixtures directory, and the
server framing-decodes incoming data and replies based on matched fixtures.

This is the right tool when:

- You're testing a client that talks raw TCP (custom protocols, legacy
  systems, line-oriented protocols like Memcached or Redis-style RESP).
- You need protocol-agnostic mocking — drop bytes in, get bytes out.
- You want to validate your client's framing/parsing logic against
  controlled error cases.

For HTTP / WebSocket / gRPC use the dedicated chapters; those have richer
protocol-aware features (route matching, schema validation, streaming).

## Quick Start

```bash
# Listen on the default port 9999, no fixtures
mockforge serve --tcp-port 9999

# Or with the standalone TCP-only command
mockforge tcp serve --port 9999 --fixtures-dir ./fixtures/tcp
```

Point a client at `localhost:9999`:

```bash
nc localhost 9999
> hello
< hello                # echo mode replies with the received bytes
```

## Configuration

```yaml
tcp:
  port: 9999                       # Listener port
  host: "0.0.0.0"                  # Bind address
  fixtures_dir: "./fixtures/tcp"   # Optional: fixture-driven responses
  timeout_secs: 300                # Per-connection idle timeout (5 min)
  max_connections: 100             # Concurrent connection cap
  read_buffer_size: 8192           # Per-read buffer (bytes)
  write_buffer_size: 8192          # Per-write buffer (bytes)
  enable_tls: false                # Wrap connections in TLS
  tls_cert_path: null              # Required when enable_tls: true
  tls_key_path: null               # Required when enable_tls: true
  echo_mode: false                 # Reply with received bytes when no fixture matches
  delimiter: null                  # Frame mode: e.g. [10] for newline-delimited
```

### Echo mode

When `echo_mode: true`, any inbound bytes that don't match a fixture are
echoed back verbatim. Useful as a sanity check or for tests that just need
a roundtrip without specific responses.

```yaml
tcp:
  port: 9999
  echo_mode: true
```

### Stream vs frame mode

| `delimiter` | Behavior |
|---|---|
| `null` (default) | Stream mode — fixtures match on raw byte sequences arriving in any chunking |
| `[10]` | Newline-delimited (LF) — each newline-terminated line is a discrete message |
| `[13, 10]` | CRLF-delimited |
| `[0]` | Null-byte-delimited |
| Any other byte sequence | Custom delimiter |

Frame mode makes fixture authoring much easier for line-oriented protocols
(Memcached, Redis-style RESP, custom telnet-style commands). Stream mode
is the right choice when you control the framing yourself.

### TLS

```yaml
tcp:
  enable_tls: true
  tls_cert_path: "/etc/ssl/certs/mockforge.crt"
  tls_key_path: "/etc/ssl/private/mockforge.key"
```

Self-signed certs are fine for local dev; clients will need to skip
verification (e.g. `openssl s_client -connect host:port -CAfile mockforge.crt`).

## Fixtures

A TCP fixture is a request/response pair, optionally with a wait condition.
Fixture files live in `fixtures_dir` and are picked up at startup.

```yaml
# fixtures/tcp/echo-greeting.yaml
name: echo-greeting
match:
  bytes: "PING\n"          # exact byte match (LF in frame mode)
respond:
  bytes: "PONG\n"
  delay_ms: 5              # optional: simulate latency
```

For binary protocols, use base64:

```yaml
name: handshake
match:
  base64: "SGVsbG8K"       # "Hello\n"
respond:
  base64: "V29ybGQK"       # "World\n"
```

For sequence-based scenarios (multi-step protocols), order matters; the
server walks fixtures top-to-bottom for each connection.

## CLI flags

```bash
# Embedded in `mockforge serve`
mockforge serve --spec api.yaml --tcp-port 9999

# Standalone TCP-only mode
mockforge tcp serve --port 9999 --host 0.0.0.0 \
  --fixtures-dir ./fixtures/tcp \
  --max-connections 50
```

## Environment variables

```bash
MOCKFORGE_TCP_ENABLED=true
MOCKFORGE_TCP_PORT=9999
MOCKFORGE_TCP_HOST=0.0.0.0
```

## Pairing with chaos

For latency, fault injection, or rate limiting on TCP traffic, use the
[chaos engine](../../user-guide/chaos-engineering.md) — it hooks every
protocol on the same listener config surface. The `connection_error_kind:
tcp_reset` knob is especially useful here for testing client reconnect
behavior:

```yaml
observability:
  chaos:
    enabled: true
    fault_injection:
      enabled: true
      connection_errors: true
      connection_error_probability: 0.05
      connection_error_kind: tcp_reset
```

5% of accepted TCP connections will get a kernel-level RST.

## Where to go next

- [Chaos Engineering](../../user-guide/chaos-engineering.md) — fault
  injection on TCP connections
- [Load Testing](../../user-guide/load-testing.md) — drive traffic at the
  TCP server with `mockforge bench-chunked` or a custom hyper client
