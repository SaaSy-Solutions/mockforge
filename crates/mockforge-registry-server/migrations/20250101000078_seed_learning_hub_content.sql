-- Seed Learning Hub content: 9 tracks (with lessons) + 18 recipes.
--
-- Each lesson and recipe body links back to the canonical chapter in the
-- book at https://docs.mockforge.dev/ — the Hub is a curated entry-point,
-- not a duplicate of the docs. ON CONFLICT (slug) DO NOTHING keeps the
-- migration idempotent if a track or recipe is later edited in the DB.
--
-- Stable UUIDs are used for tracks so lessons can FK to them without a
-- separate query. UUID format: 11111111-aaaa-aaaa-aaaa-<track-suffix>
--
-- Body fields use PostgreSQL dollar-quoting ($body$...$body$) so markdown
-- doesn't need escaping.

BEGIN;

-- ============================================================================
-- TRACKS
-- ============================================================================

-- Track 1: Getting Started
INSERT INTO learning_tracks (id, slug, title, description, body, is_published, sort_order)
VALUES (
    '11111111-aaaa-aaaa-aaaa-000000000001',
    'getting-started',
    'Your first MockForge in 10 minutes',
    'Install MockForge, load an example OpenAPI spec, hit a route, tour the admin UI, and turn on dynamic templates.',
    $body$Install MockForge, load an example OpenAPI spec, hit a route, tour the admin UI, and turn on dynamic templates. By the end you'll have a running mock that responds with realistic, dynamic data.

**Read more:** [Getting Started chapter](https://docs.mockforge.dev/getting-started/getting-started.html)$body$,
    TRUE,
    10
)
ON CONFLICT (slug) DO NOTHING;

-- Track 2: HTTP & REST
INSERT INTO learning_tracks (id, slug, title, description, body, is_published, sort_order)
VALUES (
    '11111111-aaaa-aaaa-aaaa-000000000002',
    'realistic-http-mocks',
    'Realistic HTTP mocks',
    'Go from a static stub to a dynamic, stateful HTTP mock — paths, headers, latency, and error injection.',
    $body$The basics of HTTP mocking with MockForge: routing, response shaping, dynamic data, and injecting realistic failure modes.

**Read more:** [HTTP Mocking chapter](https://docs.mockforge.dev/user-guide/http-mocking.html)$body$,
    TRUE,
    20
)
ON CONFLICT (slug) DO NOTHING;

-- Track 3: Real-time protocols
INSERT INTO learning_tracks (id, slug, title, description, body, is_published, sort_order)
VALUES (
    '11111111-aaaa-aaaa-aaaa-000000000003',
    'realtime-protocols',
    'Mocking WebSocket, gRPC, and GraphQL together',
    'One MockForge process, three protocols. Drive a real-time client end-to-end without a backend.',
    $body$MockForge runs HTTP, WebSocket, gRPC, and GraphQL listeners side-by-side. This track wires up all three with the same example spec and demonstrates a frontend talking to each.

**Read more:** [WebSocket](https://docs.mockforge.dev/user-guide/websocket-mocking.html), [gRPC](https://docs.mockforge.dev/user-guide/grpc-mocking.html), [GraphQL](https://docs.mockforge.dev/user-guide/graphql-mocking.html).$body$,
    TRUE,
    30
)
ON CONFLICT (slug) DO NOTHING;

-- Track 4: Messaging & async
INSERT INTO learning_tracks (id, slug, title, description, body, is_published, sort_order)
VALUES (
    '11111111-aaaa-aaaa-aaaa-000000000004',
    'messaging-and-async',
    'Mocking event-driven systems',
    'SMTP inboxes, MQTT topics, AMQP queues — and how to drive a consumer test without a real broker.',
    $body$Event-driven systems need event-driven mocks. This track covers the messaging protocols MockForge supports and the patterns for driving consumer tests deterministically.

**Read more:** [SMTP](https://docs.mockforge.dev/protocols/smtp/getting-started.html), [MQTT](https://docs.mockforge.dev/protocols/mqtt/getting-started.html).$body$,
    TRUE,
    40
)
ON CONFLICT (slug) DO NOTHING;

-- Track 5: Chaos & resilience
INSERT INTO learning_tracks (id, slug, title, description, body, is_published, sort_order)
VALUES (
    '11111111-aaaa-aaaa-aaaa-000000000005',
    'chaos-and-resilience',
    'Break things on purpose',
    'Inject latency, errors, partial responses, and timeouts to verify your client behaves under failure.',
    $body$Chaos engineering for clients. Every fault path here is gated on a per-request matcher so you can target one route, one tenant, or one user agent.

**Read more:** [Chaos Engineering chapter](https://docs.mockforge.dev/user-guide/chaos-engineering.html)$body$,
    TRUE,
    50
)
ON CONFLICT (slug) DO NOTHING;

-- Track 6: Performance & load testing
INSERT INTO learning_tracks (id, slug, title, description, body, is_published, sort_order)
VALUES (
    '11111111-aaaa-aaaa-aaaa-000000000006',
    'performance-and-load-testing',
    'From spec to k6 script with mockforge-bench',
    'Generate a k6 load test from any OpenAPI spec, then iterate on scenarios, thresholds, and auth.',
    $body$`mockforge bench` is OpenAPI-driven k6 generation: every operation in the spec becomes a request in a scenario you can shape (constant, ramp-up, spike, stress, soak).

**Read more:** [Load Testing chapter](https://docs.mockforge.dev/user-guide/load-testing.html)$body$,
    TRUE,
    60
)
ON CONFLICT (slug) DO NOTHING;

-- Track 7: Plugins & extensibility
INSERT INTO learning_tracks (id, slug, title, description, body, is_published, sort_order)
VALUES (
    '11111111-aaaa-aaaa-aaaa-000000000007',
    'plugins-and-extensibility',
    'Author your first MockForge plugin',
    'Build, sign, and distribute a plugin using the mockforge-plugin-sdk. From "hello world" to marketplace.',
    $body$Plugins extend MockForge with custom response transforms, data sources, auth providers, and protocol handlers. This track walks the full lifecycle: scaffold → build → sign → publish.

**Read more:** [Plugin System chapter](https://docs.mockforge.dev/user-guide/plugins.html)$body$,
    TRUE,
    70
)
ON CONFLICT (slug) DO NOTHING;

-- Track 8: CI/CD
INSERT INTO learning_tracks (id, slug, title, description, body, is_published, sort_order)
VALUES (
    '11111111-aaaa-aaaa-aaaa-000000000008',
    'mockforge-in-ci',
    'MockForge in CI',
    'Run MockForge as a service container in GitHub Actions, gate merges on contract verification.',
    $body$Move from "works on my laptop" to "verified in CI". Covers the official Docker image, GitHub Actions service containers, and contract verification against a published spec.$body$,
    TRUE,
    80
)
ON CONFLICT (slug) DO NOTHING;

-- Track 9: Advanced patterns
INSERT INTO learning_tracks (id, slug, title, description, body, is_published, sort_order)
VALUES (
    '11111111-aaaa-aaaa-aaaa-000000000009',
    'advanced-patterns',
    'Multi-tenant mocks with the registry server',
    'Branch mocks per environment, share scenarios across teams, and run a self-hosted registry.',
    $body$The registry server turns MockForge from a single-developer CLI into a team-wide platform: workspaces, RBAC, scenario marketplace, and federated registries.

**Read more:** [Cloud Workspaces](https://docs.mockforge.dev/user-guide/cloud-workspaces.html), [Scenario Marketplace](https://docs.mockforge.dev/user-guide/scenario-marketplace.html).$body$,
    TRUE,
    90
)
ON CONFLICT (slug) DO NOTHING;

-- ============================================================================
-- LESSONS  (track-by-track; sort_order is intra-track)
-- ============================================================================

-- Track 1 lessons (Getting Started)

INSERT INTO learning_lessons (track_id, slug, title, body, sort_order) VALUES
('11111111-aaaa-aaaa-aaaa-000000000001', 'install', 'Install MockForge',
$body$Pick the install method that matches your environment. The Cargo install is the most reliable on developer machines; Docker is the easiest for CI.

```bash
# Cargo (recommended)
cargo install mockforge-cli

# Docker
docker pull ghcr.io/saasy-solutions/mockforge:latest

# Pre-built binaries
# https://github.com/SaaSy-Solutions/mockforge/releases
```

Verify:

```bash
mockforge --version
```

**Read more:** [Installation](https://docs.mockforge.dev/getting-started/installation.html).$body$,
10),

('11111111-aaaa-aaaa-aaaa-000000000001', 'serve-the-demo-spec', 'Serve the demo OpenAPI spec',
$body$MockForge ships with a demo spec at `examples/openapi-demo.json`. Point `serve` at it and you have a live mock in seconds.

```bash
git clone https://github.com/SaaSy-Solutions/mockforge.git
cd mockforge

mockforge serve \
  --spec examples/openapi-demo.json \
  --http-port 3000
```

Every operation in the spec becomes a route. MockForge synthesizes a response from the schema, so even before you customize anything you get realistic-looking JSON.

**Read more:** [Five-Minute API](https://docs.mockforge.dev/getting-started/five-minute-api.html).$body$,
20),

('11111111-aaaa-aaaa-aaaa-000000000001', 'make-your-first-request', 'Make your first request',
$body$With the server running, hit a route from another terminal:

```bash
curl http://localhost:3000/users
curl http://localhost:3000/users/42
```

You should see synthesized JSON shaped by the spec. The response body is logged in the `mockforge serve` terminal — that log line is the fastest debugging surface.

For interactive exploration, the [admin UI](https://docs.mockforge.dev/user-guide/admin-ui.html) exposes a route inspector, request log, and live config editor. Add `--admin --admin-port 9080` to the serve command and open <http://localhost:9080>.$body$,
30),

('11111111-aaaa-aaaa-aaaa-000000000001', 'turn-on-dynamic-templates', 'Turn on dynamic templates',
$body$Out of the box, responses come from the OpenAPI examples. Enable template expansion to make every response unique:

```bash
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true \
  mockforge serve --spec examples/openapi-demo.json --http-port 3000
```

In your spec or config, you can now write things like:

```yaml
body: |
  {
    "id": "{{uuid}}",
    "createdAt": "{{now}}",
    "name": "{{faker.name}}"
  }
```

Each request gets a fresh UUID, a fresh timestamp, and a faker-generated name.

**Read more:** [Templating reference](https://docs.mockforge.dev/reference/templating.html).$body$,
40),

('11111111-aaaa-aaaa-aaaa-000000000001', 'where-to-go-next', 'Where to go next',
$body$You now have a running, dynamic mock. Where you go next depends on what you''re building:

- Adding routes by hand → **Realistic HTTP mocks** track
- Driving a WebSocket / gRPC / GraphQL client → **Real-time protocols** track
- Verifying client behavior under failure → **Break things on purpose** (chaos) track
- Loading the API for capacity testing → **From spec to k6 script** track
- Integrating with CI → **MockForge in CI** track

Each track is short — pick the one closest to your current task and come back for the others later.$body$,
50)
ON CONFLICT (track_id, slug) DO NOTHING;


-- Track 2 lessons (HTTP & REST)

INSERT INTO learning_lessons (track_id, slug, title, body, sort_order) VALUES
('11111111-aaaa-aaaa-aaaa-000000000002', 'routes-and-matchers', 'Routes and matchers',
$body$Routes match on path + method. Path params (`/users/{id}`), query strings, and headers are all available to your response body via `{{request.path.id}}`, `{{request.query.limit}}`, `{{request.headers.x-trace-id}}`.

```yaml
http:
  port: 3000
  routes:
    - path: /users/{id}
      method: GET
      response:
        status: 200
        body: |
          { "id": "{{request.path.id}}", "name": "Alice" }
```

If multiple routes could match, MockForge picks the most specific one (literal path > path with one param > path with two params).

**Read more:** [HTTP Mocking](https://docs.mockforge.dev/user-guide/http-mocking.html).$body$,
10),

('11111111-aaaa-aaaa-aaaa-000000000002', 'shaping-responses', 'Shaping responses',
$body$Static responses get boring fast. Three knobs to make them dynamic:

1. **Templates** — `{{uuid}}`, `{{now}}`, `{{faker.name}}`, `{{request.body.email}}`. Enable with `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true`.
2. **Override rules** — JSON patches against an OpenAPI-generated response, no spec edits needed.
3. **Custom headers** — set `Cache-Control`, CORS headers, custom auth challenges per route.

```yaml
overrides:
  - targets: ["path:/users"]
    patch:
      - op: replace
        path: "/responses/200/content/application~1json/example"
        value:
          users:
            - id: "{{uuid}}"
              name: "{{faker.name}}"
```

**Read more:** [Custom Responses](https://docs.mockforge.dev/user-guide/http-mocking/custom-responses.html).$body$,
20),

('11111111-aaaa-aaaa-aaaa-000000000002', 'inject-latency', 'Inject latency',
$body$Real APIs aren''t instant. Add latency so your client''s loading states actually run.

```bash
MOCKFORGE_LATENCY_ENABLED=true \
MOCKFORGE_LATENCY_FIXED_MS=250 \
MOCKFORGE_LATENCY_JITTER_PERCENT=20 \
  mockforge serve --spec api.yaml --http-port 3000
```

That gives you 250ms ± 20% on every response. For per-route latency, configure it in the chaos block (see the chaos track).

**Read more:** [Advanced Behavior](https://docs.mockforge.dev/user-guide/advanced-behavior.html).$body$,
30),

('11111111-aaaa-aaaa-aaaa-000000000002', 'inject-errors', 'Inject errors',
$body$Half the bugs in a web app come from forgetting to handle the error path. Make MockForge return errors deterministically:

```yaml
http:
  routes:
    - path: /payments
      method: POST
      response:
        status: 503
        body: |
          { "error": "downstream_timeout" }
```

Or probabilistically via the chaos block:

```yaml
chaos:
  fault_injection:
    enabled: true
    error_rate: 0.1     # 10% of /payments requests
    error_status: 502
    targets: ["path:/payments"]
```

Now your retry / circuit breaker / fallback code finally gets exercised.$body$,
40),

('11111111-aaaa-aaaa-aaaa-000000000002', 'state-across-requests', 'State across requests',
$body$Static stubs can''t do POST → GET round-trips. The world-state engine fixes that: a POST writes into a tiny in-memory store, and GET reads from it.

```yaml
http:
  routes:
    - path: /todos
      method: POST
      response:
        status: 201
        state:
          set: "todos[{{uuid}}]"
          to: "{{request.body}}"
        body: "{{state.last_set}}"

    - path: /todos
      method: GET
      response:
        status: 200
        body: "{{state.todos | values}}"
```

Now your frontend round-trip works without a real DB. State persists for the lifetime of the `serve` process (or longer, if you snapshot it).

**Read more:** [World State Engine](https://docs.mockforge.dev/user-guide/advanced-features/world-state-engine.html).$body$,
50)
ON CONFLICT (track_id, slug) DO NOTHING;


-- Track 3 lessons (Real-time protocols)

INSERT INTO learning_lessons (track_id, slug, title, body, sort_order) VALUES
('11111111-aaaa-aaaa-aaaa-000000000003', 'three-listeners-one-process', 'Three listeners, one process',
$body$MockForge''s `serve` command can bind HTTP, WebSocket, and gRPC ports at the same time, sharing a single config file. There''s no separate "websocket mode" or "grpc mode" — they''re all on by default if you give them a port.

```bash
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true \
MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl \
MOCKFORGE_PROTO_DIR=examples/grpc-protos \
  mockforge serve \
    --spec examples/openapi-demo.json \
    --http-port 3000 \
    --ws-port 3001 \
    --grpc-port 50051 \
    --admin --admin-port 9080
```

That gives you HTTP at :3000, WebSocket at :3001/ws, gRPC at :50051, and an admin UI at :9080 — all from one process.$body$,
10),

('11111111-aaaa-aaaa-aaaa-000000000003', 'websocket-replay', 'WebSocket replay basics',
$body$The simplest WebSocket mock is a JSONL file. Each line is a message; `ts` is the millisecond offset from connection start; `dir` is `"out"` (server → client) or `"in"` (expected from client).

```jsonl
{"ts":0,"dir":"out","text":"Welcome to MockForge"}
{"ts":500,"dir":"out","text":"Type 'hello' to start","waitFor":"hello"}
{"ts":100,"dir":"out","text":"Session {{uuid}} started at {{now}}"}
{"ts":2000,"dir":"out","text":"Goodbye"}
```

`waitFor` is **substring** matching, not regex (despite older docs).

```bash
mockforge serve --ws-port 3001 --ws-replay-file ws-scenario.jsonl
```

Connect with any client to `ws://localhost:3001/ws` and the script runs.

**Read more:** [WebSocket Replay Mode](https://docs.mockforge.dev/user-guide/websocket-mocking/replay.html).$body$,
20),

('11111111-aaaa-aaaa-aaaa-000000000003', 'grpc-from-proto', 'gRPC from a proto file',
$body$Drop `.proto` files into a directory and MockForge discovers them on startup — no codegen step.

```
proto/
  user_service.proto
  subdir/
    analytics.proto    # discovered recursively
```

```bash
MOCKFORGE_PROTO_DIR=proto mockforge serve --grpc-port 50051
```

Every method (unary, server-streaming, client-streaming, bidirectional) is registered automatically. Responses are synthesized from the message types — same realism story as HTTP.

If you don''t have proto files, gRPC reflection still works (the next recipe shows the reflection-only path).

**Read more:** [gRPC Mocking](https://docs.mockforge.dev/user-guide/grpc-mocking.html).$body$,
30),

('11111111-aaaa-aaaa-aaaa-000000000003', 'graphql-quickstart', 'GraphQL quickstart',
$body$MockForge can mock a GraphQL endpoint from an SDL schema. Drop the schema in a config and you have a working `/graphql` endpoint.

```yaml
graphql:
  port: 4000
  schema_file: schema.graphql
```

Where `schema.graphql` is your real schema:

```graphql
type Query {
  user(id: ID!): User
}
type User {
  id: ID!
  name: String!
  email: String!
}
```

Every field is mocked from its type by default; override individual resolvers in config when you need a specific value.

**Read more:** [GraphQL Mocking](https://docs.mockforge.dev/user-guide/graphql-mocking.html).$body$,
40),

('11111111-aaaa-aaaa-aaaa-000000000003', 'driving-a-real-client', 'Driving a real client',
$body$With all three listeners running, point a real client at them. A common shape: an SPA hitting HTTP for CRUD, WebSocket for live updates, gRPC for backend-to-backend calls (via grpc-web).

A minimal smoke test:

```bash
# HTTP
curl http://localhost:3000/users

# WebSocket (using websocat)
echo "hello" | websocat ws://localhost:3001/ws

# gRPC (using grpcurl, leverages reflection)
grpcurl -plaintext localhost:50051 list
grpcurl -plaintext -d '{"id":"1"}' localhost:50051 user.UserService/GetUser
```

If all three respond, your client integration tests have everything they need to run end-to-end without a backend.$body$,
50)
ON CONFLICT (track_id, slug) DO NOTHING;


-- Track 4 lessons (Messaging & async)

INSERT INTO learning_lessons (track_id, slug, title, body, sort_order) VALUES
('11111111-aaaa-aaaa-aaaa-000000000004', 'why-mock-messaging', 'Why mock messaging?',
$body$Real message brokers are slow to spin up, hard to clean up between tests, and add an external dependency to every CI job. MockForge replaces them with in-process listeners that speak the wire protocol — your producers and consumers don''t know the difference.

| Protocol | Port (default) | Status |
|---|---|---|
| SMTP   | 1025 | Stable |
| MQTT   | 1883 | Stable |
| Kafka  | 9092 | Stable |
| AMQP   | 5672 | Beta |

All run in the same `mockforge serve` process; turn on what you need in config.$body$,
10),

('11111111-aaaa-aaaa-aaaa-000000000004', 'smtp-quickstart', 'SMTP quickstart',
$body$Inbound SMTP is the fastest message protocol to mock — no broker concept, no consumer groups, just an in-memory mailbox.

```yaml
smtp:
  enabled: true
  port: 1025
  hostname: "mockforge-smtp"
```

```bash
mockforge serve --config config.yaml
# 📧 SMTP server listening on localhost:1025
```

Send a test email with `swaks` or any SMTP client; MockForge stores the message in memory and exposes it via the admin API.

**Read more:** [SMTP getting started](https://docs.mockforge.dev/protocols/smtp/getting-started.html).$body$,
20),

('11111111-aaaa-aaaa-aaaa-000000000004', 'mqtt-quickstart', 'MQTT quickstart',
$body$The MQTT broker mocks pub/sub for IoT-style workloads.

```yaml
mqtt:
  enabled: true
  port: 1883
  max_connections: 1000
  keep_alive_secs: 60
```

```bash
mockforge serve --config config.yaml
# 📡 MQTT broker listening on localhost:1883

# In another terminal:
mosquitto_sub -h localhost -p 1883 -t "sensors/+" &
mosquitto_pub -h localhost -p 1883 -t "sensors/temperature" -m "25.5"
```

Wildcard subscriptions, retained messages, and QoS 0/1/2 all work as the spec defines.

**Read more:** [MQTT getting started](https://docs.mockforge.dev/protocols/mqtt/getting-started.html).$body$,
30),

('11111111-aaaa-aaaa-aaaa-000000000004', 'kafka-broker', 'A Kafka broker without ZooKeeper',
$body$The Kafka mock implements the wire protocol directly — no broker process, no ZooKeeper, no Docker container.

```yaml
kafka:
  enabled: true
  port: 9092
```

Then point any rdkafka / Sarama / kafka-go client at `localhost:9092`. Topics are created on-demand; consumer groups and offset management work as the protocol expects.

For pre-seeded topics, use a fixtures file:

```yaml
kafka:
  fixtures:
    - topic: "orders"
      partitions: 3
      messages:
        - key: "order-1"
          value: '{"id":"order-1","total":42.10}'
```

**Read more:** [`mockforge-kafka` crate README](https://github.com/SaaSy-Solutions/mockforge/blob/main/crates/mockforge-kafka/README.md).$body$,
40),

('11111111-aaaa-aaaa-aaaa-000000000004', 'consumer-test-pattern', 'The consumer-test pattern',
$body$Mocking a broker only helps if your consumer test is deterministic. The shape that works:

1. **Start MockForge** with a fixtures file pre-seeding the topic.
2. **Start your consumer** with `bootstrap.servers=localhost:9092` (or equivalent).
3. **Assert** on the consumer''s behavior — what it wrote to its database, what HTTP it called, what it logged.
4. **Tear down** by stopping the MockForge process; in-memory state evaporates.

```bash
mockforge serve --config config.yaml &
MF_PID=$!

cargo test --test consumer_integration -- --nocapture

kill $MF_PID
```

The next recipe (Kafka consumer test) shows the full repeatable pattern.$body$,
50)
ON CONFLICT (track_id, slug) DO NOTHING;


-- Track 5 lessons (Chaos & resilience)

INSERT INTO learning_lessons (track_id, slug, title, body, sort_order) VALUES
('11111111-aaaa-aaaa-aaaa-000000000005', 'predefined-scenarios', 'Predefined chaos scenarios',
$body$Five built-in scenarios cover the most common failure shapes. Each is a single flag away.

| Scenario | What it does |
|---|---|
| `network_degradation` | High latency + packet loss |
| `service_instability` | Random 5xx errors + timeouts |
| `cascading_failure`   | Latency + errors + connection drops + rate limits |
| `peak_traffic`        | Aggressive rate limiting (per endpoint) |
| `slow_backend`        | Consistent 2s latency on every request |

```bash
mockforge serve --chaos --chaos-scenario cascading_failure
```

Start with one of these before writing custom configs — they''re tuned to surface real client bugs.

**Read more:** [Chaos Engineering](https://docs.mockforge.dev/user-guide/chaos-engineering.html).$body$,
10),

('11111111-aaaa-aaaa-aaaa-000000000005', 'latency-and-jitter', 'Latency and jitter',
$body$Add fixed delay, plus jitter to randomize it slightly so client timing assumptions don''t silently rely on uniform response times.

```yaml
chaos:
  latency:
    enabled: true
    fixed_delay_ms: 250
    jitter_percent: 20
    probability: 0.5      # 50% of requests
```

Probability 0.5 with 20% jitter is a good "everything works most of the time, but slow" baseline. Crank `probability` to 1.0 for pure slow-backend simulation, or to 0.05 to find timeout bugs that only fire on a few percent of requests.$body$,
20),

('11111111-aaaa-aaaa-aaaa-000000000005', 'errors-and-timeouts', 'Errors and timeouts',
$body$Three distinct failure modes — pick the one that actually breaks your client.

```yaml
chaos:
  fault_injection:
    enabled: true

    # 1) HTTP errors
    http_errors: [500, 503]
    http_error_probability: 0.1

    # 2) Real server-side hang followed by 504
    timeout_errors: true
    timeout_ms: 5000
    timeout_probability: 0.05

    # 3) Truncated response body
    partial_responses: true
    partial_response_probability: 0.05
```

Each is independent — combine to recreate the failure modes you''ve seen in production.$body$,
30),

('11111111-aaaa-aaaa-aaaa-000000000005', 'tcp-level-faults', 'TCP-level faults',
$body$Some bugs only show up below HTTP. The `connection_error_kind` knob picks the wire-level behavior:

| Kind | Client sees |
|---|---|
| `http_503` (default) | HTTP 503 |
| `tcp_reset`          | `ECONNRESET` (RST at accept) |
| `tcp_close`          | EOF before any HTTP response (FIN at accept) |

```yaml
chaos:
  fault_injection:
    enabled: true
    connection_errors: true
    connection_error_probability: 0.05
    connection_error_kind: tcp_reset
```

TCP-level injection runs at accept time, so it''s per-connection — every pipelined request on that socket is affected. Plain HTTP only; TLS path doesn''t yet support TCP-level injection.$body$,
40),

('11111111-aaaa-aaaa-aaaa-000000000005', 'targeted-faults', 'Targeted faults',
$body$Probability is blunt. `request_matcher` lets you target faults at one client, one endpoint, or one shape of request — leaving the rest of your traffic clean.

```yaml
fault_injection:
  enabled: true
  http_errors: [503]
  http_error_probability: 0.5
  request_matcher:
    source_ips:
      - "10.0.0.0/8"
    headers:
      - name: "x-test"
        value: "yes"
    min_body_size_bytes: 1048576
    chunked_only: true
```

Semantics: AND across fields, OR within a list. Empty matcher matches every request. So you can add chaos to *just* `X-Test: yes` traffic from one CI runner, while real users see no faults.$body$,
50)
ON CONFLICT (track_id, slug) DO NOTHING;


-- Track 6 lessons (Performance & load testing)

INSERT INTO learning_lessons (track_id, slug, title, body, sort_order) VALUES
('11111111-aaaa-aaaa-aaaa-000000000006', 'spec-to-k6', 'Spec → k6 in one command',
$body$`mockforge bench` walks an OpenAPI spec, generates a k6 script with realistic request bodies, and runs it.

```bash
mockforge bench \
  --spec api.yaml \
  --target http://localhost:3000 \
  --duration 60 --vus 10 \
  --scenario ramp-up
```

Output:
- A k6 script in `bench-results/`
- `summary.json` with `http_req_duration`, `http_req_failed`, and per-endpoint p95/p99
- A printed pass/fail against your thresholds

Works against any HTTP target — your real backend, a MockForge mock, or a hosted environment.

**Read more:** [Load Testing](https://docs.mockforge.dev/user-guide/load-testing.html).$body$,
10),

('11111111-aaaa-aaaa-aaaa-000000000006', 'load-shapes', 'Choosing a load shape',
$body$Five built-in scenarios cover the canon:

| Scenario | Shape | Use case |
|---|---|---|
| `constant`  | Flat at `--vus`               | Steady-state throughput |
| `ramp-up`   | 0 → vus → 0                   | Find capacity ceiling |
| `spike`     | Sudden burst                  | Test autoscaling response |
| `stress`    | Aggressive ramp past `--vus`  | Find breaking point |
| `soak`      | Long duration at moderate load | Memory leaks / degradation |

Pick `ramp-up` first — it produces the cleanest "where did latency start to climb?" curve. `soak` is the highest-value one for production: many bugs only show up after 30+ minutes.$body$,
20),

('11111111-aaaa-aaaa-aaaa-000000000006', 'auth-and-filtering', 'Auth and operation filtering',
$body$Real APIs need auth and you usually only want to load-test a subset of endpoints.

```bash
mockforge bench --spec api.yaml --target https://api.example.com \
  --auth "Bearer $TOKEN" \
  --headers "X-Tenant: acme,X-Region: us-east-1" \
  --operations "GET /users,POST /orders"
```

Operation filtering supports:
- Exact: `"GET /users"`
- By method: `"GET"`
- Wildcards: `"* /api/v1/*"`

For per-tenant or per-user load shapes, generate one bench config per tenant and run them in parallel via `--targets-file`.$body$,
30),

('11111111-aaaa-aaaa-aaaa-000000000006', 'chunked-bodies', 'Chunked-body load with bench-chunked',
$body$`mockforge bench` falls back to `Content-Length` for some body types — Go''s HTTP transport decides chunking automatically. When you need *guaranteed* `Transfer-Encoding: chunked` traffic, use the native generator:

```bash
mockforge bench-chunked \
  --target http://localhost:3000/upload \
  --concurrency 10 --duration 60 \
  --chunk-size-bytes 4096 \
  --total-size-bytes 10485760 \
  --chunk-interval-ms 50
```

Use this to:
- Test chunked-body parsing on the server
- Reproduce slow-upload bugs (`--chunk-interval-ms 500`)
- Find max-body memory limits (`--total-size-bytes 1073741824`)$body$,
40),

('11111111-aaaa-aaaa-aaaa-000000000006', 'pairing-with-chaos', 'Pairing load with chaos',
$body$Throughput numbers without failure injection lie. Pair `bench` with `serve --chaos` to find out what your client does when the server slows down under load.

```bash
# Terminal 1
mockforge serve --spec api.yaml --chaos --chaos-scenario cascading_failure

# Terminal 2
mockforge bench --spec api.yaml --target http://localhost:3000 \
  --duration 120 --vus 50 --scenario ramp-up
```

Now your load test exercises retries, circuit breakers, timeouts, and partial-response handling at scale. The interesting question stops being "how many RPS" and becomes "does the client survive degraded service."$body$,
50)
ON CONFLICT (track_id, slug) DO NOTHING;


-- Track 7 lessons (Plugins & extensibility)

INSERT INTO learning_lessons (track_id, slug, title, body, sort_order) VALUES
('11111111-aaaa-aaaa-aaaa-000000000007', 'plugin-types', 'Plugin types: pick the right shape',
$body$Plugins are WebAssembly modules that run in a sandbox alongside the MockForge process. Each one declares a *type* — that determines which interface it implements and where in the request lifecycle it runs.

| Type | Interface | Use case |
|---|---|---|
| `template`   | template fn       | Domain-specific data generators |
| `auth`       | `AuthProvider`    | JWT/OAuth/custom auth |
| `response`   | `ResponseGenerator` | Dynamic body generation |
| `datasource` | `DataSourceConnector` | CSV / DB / external API as the source of truth |
| `webhook`    | webhook trigger   | Fire outbound HTTP from a route |
| `chaos`      | chaos pattern     | Custom failure mode or latency curve |

Pick the simplest type that does what you need. `template` and `response` cover ~80% of real plugin needs.

**Read more:** [Plugin System](https://docs.mockforge.dev/user-guide/plugins.html).$body$,
10),

('11111111-aaaa-aaaa-aaaa-000000000007', 'scaffold', 'Scaffold a plugin in 30 seconds',
$body$The plugin CLI generates a full project — Cargo manifest, plugin.yaml, source skeleton, tests, README.

```bash
# One-time setup
rustup target add wasm32-unknown-unknown
cargo install mockforge-plugin-cli

# Scaffold
mockforge plugin init my-plugin --plugin-type template
cd my-plugin
```

You get:

```
my-plugin/
├── Cargo.toml
├── plugin.yaml         # manifest: name, version, capabilities
├── src/lib.rs          # plugin entrypoint
└── tests/
```

**Read more:** [Plugin Starter Guide](https://docs.mockforge.dev/tutorials/plugin-starter.html).$body$,
20),

('11111111-aaaa-aaaa-aaaa-000000000007', 'build-and-test', 'Build and test',
$body$Plugins compile to WASM and run in the MockForge sandbox.

```bash
# Build
cargo build --target wasm32-unknown-unknown --release

# The artifact:
# target/wasm32-unknown-unknown/release/my_plugin.wasm

# Local install
mockforge plugin install ./target/wasm32-unknown-unknown/release/my_plugin.wasm

# Use it
mockforge plugin enable my-plugin
mockforge serve --config mockforge.yaml
```

For unit tests, the SDK ships a host harness so plugins are testable as plain Rust code without a full MockForge process — `cargo test` does what you''d expect.$body$,
30),

('11111111-aaaa-aaaa-aaaa-000000000007', 'sign-and-publish', 'Sign and publish',
$body$Plugins can be signed with a Ed25519 key and published to the marketplace, where users install them by name.

```bash
# One-time: register a public key with your registry account
mockforge plugin keys generate --output ./my-key
mockforge plugin keys register --public-key ./my-key.pub

# Sign and publish
mockforge plugin publish ./my-plugin.wasm \
  --signing-key ./my-key \
  --version 0.1.0
```

The marketplace verifies the signature and refuses unsigned (or differently-signed) updates. Users install with:

```bash
mockforge plugin install my-plugin
```

**Read more:** [Plugin Marketplace](https://github.com/SaaSy-Solutions/mockforge/blob/main/docs/PLUGIN_MARKETPLACE_IMPLEMENTATION.md).$body$,
40),

('11111111-aaaa-aaaa-aaaa-000000000007', 'capabilities-and-egress', 'Capabilities and egress',
$body$Plugins run in a deny-by-default sandbox. To make a network call or read a file, declare it in `plugin.yaml`:

```yaml
name: my-plugin
version: 0.1.0
type: response
capabilities:
  network:
    egress:
      - host: "api.example.com"
        ports: [443]
  filesystem:
    read:
      - path: "/var/lib/mockforge/data/*.csv"
```

Egress goes through MockForge''s HTTP CONNECT proxy (per-plugin allowlist enforced at the proxy, not in the plugin), so a misbehaving plugin can''t silently exfiltrate to an unlisted host.

**Read more:** [Plugin egress recipe](#sandboxed-plugin-egress).$body$,
50)
ON CONFLICT (track_id, slug) DO NOTHING;


-- Track 8 lessons (CI/CD & automation)

INSERT INTO learning_lessons (track_id, slug, title, body, sort_order) VALUES
('11111111-aaaa-aaaa-aaaa-000000000008', 'official-docker-image', 'The official Docker image',
$body$The fastest way to run MockForge in CI is the official image — published on every release.

```bash
docker pull ghcr.io/saasy-solutions/mockforge:latest
```

Run a mock from a spec mounted at `/spec`:

```bash
docker run --rm -p 3000:3000 \
  -v "$PWD/api.yaml:/spec/api.yaml" \
  ghcr.io/saasy-solutions/mockforge:latest \
  serve --spec /spec/api.yaml --http-port 3000
```

Image is multi-arch (amd64 + arm64), digest-pinnable, and stays small enough to pull on every CI job.$body$,
10),

('11111111-aaaa-aaaa-aaaa-000000000008', 'github-actions', 'GitHub Actions: as a service container',
$body$The cleanest pattern: declare MockForge as a service container so it''s ready before your test step runs.

```yaml
jobs:
  test:
    runs-on: ubuntu-latest
    services:
      mockforge:
        image: ghcr.io/saasy-solutions/mockforge:latest
        ports:
          - 3000:3000
        options: >-
          --health-cmd "curl -f http://localhost:3000/__mockforge/health || exit 1"
          --health-interval 5s --health-retries 5
    steps:
      - uses: actions/checkout@v4
      - name: Run integration tests
        run: npm test
        env:
          API_BASE_URL: http://localhost:3000
```

If your spec lives in the repo, run MockForge as a step instead so you can mount the spec from `$GITHUB_WORKSPACE`. The `guides/guide01/ci_snippets.md` in the repo has both shapes.

**Read more:** [Tutorial: Mock OpenAPI in CI](https://github.com/SaaSy-Solutions/mockforge/blob/main/guides/guide01/ci_snippets.md).$body$,
20),

('11111111-aaaa-aaaa-aaaa-000000000008', 'conformance-testing', 'Contract verification with --conformance',
$body$Mocks are only useful if they match the spec. `mockforge bench --conformance` verifies that any HTTP API (your mock, or your real backend) implements its OpenAPI spec correctly — 47 features across 11 categories.

```bash
# Test your real API
mockforge bench --conformance \
  --spec api.yaml \
  --target https://api.staging.example.com \
  --conformance-header "Authorization: Bearer $TOKEN"
```

Outputs `conformance-report.json`. Wire it into CI as a fail-the-build gate: if the spec and the implementation drift apart, the build breaks.

**Read more:** [Conformance Testing Guide](https://github.com/SaaSy-Solutions/mockforge/blob/main/docs/CONFORMANCE_TESTING.md).$body$,
30),

('11111111-aaaa-aaaa-aaaa-000000000008', 'pin-and-cache', 'Pin and cache for fast pipelines',
$body$Two cheap wins:

**Pin the image by digest** so a transient registry change doesn''t break a green build:

```yaml
services:
  mockforge:
    image: ghcr.io/saasy-solutions/mockforge@sha256:abcd1234...
```

**Cache the spec parse** when it''s big. MockForge accepts `--spec-cache-dir`; set it to `~/.cache/mockforge` and add the path to your CI cache. Saves 1–3 seconds per job, which adds up over thousands of CI runs a month.

For self-hosted runners, also pre-pull the image into the runner''s Docker cache via a startup script — first job on a cold node otherwise spends 30s pulling.$body$,
40),

('11111111-aaaa-aaaa-aaaa-000000000008', 'review-app-pattern', 'Review apps: one mock per PR',
$body$For PR-level integration testing, spin up a fresh MockForge per PR using the cloud or a per-PR namespace.

```yaml
jobs:
  preview:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Push mock to cloud
        run: |
          mockforge cloud push \
            --project pr-${{ github.event.number }} \
            --spec api.yaml
        env:
          MOCKFORGE_CLOUD_TOKEN: ${{ secrets.MOCKFORGE_CLOUD_TOKEN }}
      - name: Comment URL on PR
        uses: peter-evans/create-or-update-comment@v3
        with:
          issue-number: ${{ github.event.number }}
          body: "Mock preview: https://pr-${{ github.event.number }}.mocks.mockforge.dev"
```

Reviewers can hit the PR''s mock directly from a web preview, no local setup. The cloud workspace garbage-collects old PR mocks automatically.

**Read more:** [Cloud Workspaces](https://docs.mockforge.dev/user-guide/cloud-workspaces.html).$body$,
50)
ON CONFLICT (track_id, slug) DO NOTHING;


-- Track 9 lessons (Advanced patterns)

INSERT INTO learning_lessons (track_id, slug, title, body, sort_order) VALUES
('11111111-aaaa-aaaa-aaaa-000000000009', 'cloud-workspaces-overview', 'Cloud workspaces overview',
$body$Cloud Workspaces is the team-share layer on top of MockForge: shared workspace records, cloud-synced configs, role-based collaborator management. Same mock format as the local CLI — this is the "we''ve outgrown one-laptop-one-mock" upgrade.

Run once per developer:

```bash
mockforge cloud login
mockforge org list
mockforge org use my-org
```

```bash
mockforge cloud workspace create my-workspace --name "My Workspace"
mockforge cloud workspace link . my-workspace
mockforge cloud sync start --workspace my-workspace --watch
```

Now everyone with access to the workspace edits the same mock — changes flow through the cloud, not through ad-hoc Slack DMs.

**Read more:** [Cloud Workspaces](https://docs.mockforge.dev/user-guide/cloud-workspaces.html).$body$,
10),

('11111111-aaaa-aaaa-aaaa-000000000009', 'rbac-and-roles', 'RBAC and roles',
$body$Workspaces enforce role-based access: owners can manage members and rotate keys, editors can change mocks, viewers can only read. Map this to your team:

```bash
# Invite a collaborator
mockforge cloud workspace member invite my-workspace alice@example.com --role editor

# Promote
mockforge cloud workspace member set-role my-workspace alice@example.com --role owner

# Remove
mockforge cloud workspace member remove my-workspace alice@example.com
```

For multi-tenant scenarios (e.g. mocks served to external partners), use the Org-level RBAC layer to scope visibility — partners see only the workspaces their org membership grants.

**Read more:** [RBAC Guide](https://github.com/SaaSy-Solutions/mockforge/blob/main/docs/RBAC_GUIDE.md).$body$,
20),

('11111111-aaaa-aaaa-aaaa-000000000009', 'scenario-state-machines', 'Scenario state machines',
$body$Static mocks can''t represent "the API behaves differently after the user has accepted the ToS." State machines model that.

```yaml
name: order_workflow
initial_state: pending
states:
  - name: pending
    response:
      status_code: 200
      body: ''{"order_id":"{{resource_id}}","status":"pending"}''
  - name: processing
    response:
      status_code: 200
      body: ''{"order_id":"{{resource_id}}","status":"processing"}''
  - name: shipped
    response:
      status_code: 200
      body: ''{"order_id":"{{resource_id}}","status":"shipped"}''

transitions:
  - from: pending
    to: processing
    condition: ''method == "PUT" && path == "/api/orders/{id}/process"''
  - from: processing
    to: shipped
    condition: ''method == "PUT" && path == "/api/orders/{id}/ship"''
```

`GET /api/orders/{id}` returns the response from whatever state the machine is currently in. Multi-step flows that used to need a real backend now fit in 30 lines of YAML.

**Read more:** [Scenario State Machines](https://docs.mockforge.dev/user-guide/scenario-state-machines.html).$body$,
30),

('11111111-aaaa-aaaa-aaaa-000000000009', 'recording-real-traffic', 'Recording real traffic',
$body$The Flight Recorder runs MockForge as a transparent proxy in front of a real backend, captures every request/response into SQLite, and replays them later as deterministic mocks.

```bash
mockforge serve \
  --upstream https://api.staging.example.com \
  --recorder \
  --recorder-db ./recordings.db \
  --http-port 3000
```

Hit `localhost:3000`, get the real response, with a copy stored in `recordings.db`. Stop the proxy when you''ve captured enough.

To replay:

```bash
mockforge serve --replay ./recordings.db --http-port 3000
```

Now `localhost:3000` returns the recorded responses, byte-identical, every run — perfect for "reproduce yesterday''s production bug locally."$body$,
40),

('11111111-aaaa-aaaa-aaaa-000000000009', 'self-hosted-registry', 'Self-hosted registry',
$body$For air-gapped environments or strict compliance, run your own registry server. Same code as `app.mockforge.dev`, deployed in your VPC.

```bash
docker run --rm -p 8080:8080 \
  -e DATABASE_URL=postgres://... \
  -e MOCKFORGE_REGISTRY_BASE_URL=https://registry.internal.example.com \
  ghcr.io/saasy-solutions/mockforge-registry:latest
```

Point developers at it:

```bash
mockforge cloud login --server https://registry.internal.example.com
```

Plugin marketplace, scenario marketplace, workspace sync, RBAC — all the cloud features run against your own Postgres. Federation lets multiple registries cross-reference each other for orgs that span air-gap boundaries.

**Read more:** [`mockforge-registry-server` README](https://github.com/SaaSy-Solutions/mockforge/tree/main/crates/mockforge-registry-server).$body$,
50)
ON CONFLICT (track_id, slug) DO NOTHING;


-- ============================================================================
-- RECIPES
-- ============================================================================

-- Recipes: Getting Started bucket

INSERT INTO learning_recipes (slug, title, description, body, tags, is_published) VALUES
('openapi-spec-to-mock-one-command',
'From OpenAPI spec to live mock in one command',
'Point `mockforge serve --spec` at any OpenAPI/Swagger document and get a working mock instantly.',
$body$If you have an OpenAPI 3.x or Swagger 2.0 spec, you don''t need a config file. `mockforge serve --spec` walks the document, registers a route per operation, and synthesizes responses from the schemas.

```bash
mockforge serve \
  --spec ./openapi.yaml \
  --http-port 3000
```

Works with:
- Local files (`.json` or `.yaml`)
- Remote URLs: `--spec https://api.example.com/openapi.json`
- Multi-file specs with `$ref` (relative refs are resolved from the spec''s directory)

By default the response body uses the schema''s `example` (or first `examples` entry). Turn on template expansion with `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true` to fill in dynamic fields. Combine with `--admin --admin-port 9080` to inspect what got loaded.

**Tip:** if the spec uses authentication, MockForge ignores it for inbound mocks unless you explicitly enable [security policies](https://docs.mockforge.dev/user-guide/security.html).

**Read more:** [HTTP Mocking → OpenAPI Integration](https://docs.mockforge.dev/user-guide/http-mocking/openapi.html).$body$,
ARRAY['getting-started', 'http', 'openapi'],
TRUE),

('cli-vs-desktop-vs-cloud',
'Choosing CLI vs. desktop vs. cloud-hosted MockForge',
'Three runtimes, one mock format. Pick the right one for your workflow.',
$body$MockForge runs in three places. The mock format is identical across all three — you can move a project between them without rewriting anything.

| Runtime | Best for | Trade-offs |
|---|---|---|
| **CLI (`mockforge serve`)** | Local dev, CI, single-developer projects | Lives only on the machine that runs it; sharing means re-running on each box. |
| **Desktop app** | Solo devs who prefer a GUI; offline demos | Same engine as the CLI plus a built-in admin UI; not aimed at team sharing. |
| **Cloud (app.mockforge.dev)** | Teams; review apps; review-time bug repros | Persistent URLs, RBAC, scenario marketplace; needs a paid plan past the free tier. |

**Quick decision guide:**

- "I''m the only one using this mock and it lives on my laptop." → CLI.
- "I want a UI but no team-share." → Desktop.
- "My QA / FE / PM all need to hit the same URL." → Cloud.

You can always start in the CLI and promote a project to cloud later: `mockforge cloud push --project my-mock`.

**Read more:** [Cloud Workspaces](https://docs.mockforge.dev/user-guide/cloud-workspaces.html), [Admin UI](https://docs.mockforge.dev/user-guide/admin-ui.html).$body$,
ARRAY['getting-started', 'cloud', 'desktop'],
TRUE)
ON CONFLICT (slug) DO NOTHING;

-- Recipes: HTTP & REST bucket

INSERT INTO learning_recipes (slug, title, description, body, tags, is_published) VALUES
('handlebars-conditional-responses',
'Conditional responses with Handlebars templates',
'Branch a response body on the request — different payload for premium vs. free users.',
$body$Templates aren''t just `{{uuid}}` placeholders. The full Handlebars syntax is available, including `{{#if}}`, `{{#each}}`, and helpers — so a single route can return different shapes based on the request.

Enable template expansion:

```bash
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true \
  mockforge serve --config mockforge.yaml
```

Then:

```yaml
http:
  routes:
    - path: /users/{id}
      method: GET
      response:
        status: 200
        body: |
          {
            "id": "{{request.path.id}}",
            "name": "{{faker.name}}",
            {{#if (eq request.headers.x-tier "premium")}}
            "features": ["sso", "audit-log", "priority-support"],
            "rate_limit": 10000
            {{else}}
            "features": ["basic"],
            "rate_limit": 100
            {{/if}}
          }
```

Send `X-Tier: premium` and you get the premium payload; without it you get the free one. Same route, two clients, no duplication.

Available helpers include `eq`, `ne`, `lt`, `gt`, `or`, `and`, `not`, `lookup`, `with`, plus MockForge-specific ones like `{{faker.X}}`, `{{rand.int min max}}`, `{{state.X}}`.

**Read more:** [Templating reference](https://docs.mockforge.dev/reference/templating.html).$body$,
ARRAY['http', 'templates', 'handlebars'],
TRUE),

('stateful-crud-without-a-db',
'Stateful CRUD with world-state — no real DB needed',
'POST creates, GET reads back, DELETE removes. Real round-trip, in-memory storage.',
$body$You don''t need Postgres to test that "create user → list users" actually works. The world-state engine gives you a request-scoped key-value store you can write to and read from across routes.

```yaml
http:
  routes:
    - path: /widgets
      method: POST
      response:
        status: 201
        state:
          set: "widgets[{{uuid}}]"
          to:
            id: "{{state.last_key}}"
            name: "{{request.body.name}}"
            createdAt: "{{now}}"
        body: "{{state.last_set}}"

    - path: /widgets
      method: GET
      response:
        status: 200
        body: "{{state.widgets | values}}"

    - path: /widgets/{id}
      method: DELETE
      response:
        status: 204
        state:
          delete: "widgets[{{request.path.id}}]"
```

Now `curl -X POST .../widgets -d '{"name":"foo"}'` writes to state, `curl .../widgets` lists it, and `curl -X DELETE .../widgets/{id}` removes it.

State lives for the lifetime of the `serve` process by default. If you need it to survive restarts, snapshot to disk:

```bash
mockforge serve --state-snapshot ./state.json --state-snapshot-interval 30s
```

**Read more:** [World State Engine](https://docs.mockforge.dev/user-guide/advanced-features/world-state-engine.html), [CRUD Simulation](https://github.com/SaaSy-Solutions/mockforge/blob/main/docs/CRUD_SIMULATION.md).$body$,
ARRAY['http', 'state', 'crud'],
TRUE)
ON CONFLICT (slug) DO NOTHING;

-- Recipes: Real-time protocols bucket

INSERT INTO learning_recipes (slug, title, description, body, tags, is_published) VALUES
('replay-recorded-websocket-session',
'Replay a recorded WebSocket session from a .jsonl file',
'Capture a real WebSocket transcript and replay it deterministically — perfect for E2E tests of streaming clients.',
$body$When you''re testing a client that consumes a WebSocket stream (chat, live prices, telemetry), the most realistic mock is a real recording played back at real speed.

**Step 1: capture a session.** With `mockforge-recorder` (or any tool that emits the JSONL frame format), record a connection against the real backend:

```bash
mockforge record \
  --target ws://staging.example.com/feed \
  --output recordings/feed.jsonl
```

That produces a file like:

```jsonl
{"ts":0,"dir":"in","text":"{\"type\":\"subscribe\",\"channel\":\"ticker\"}"}
{"ts":42,"dir":"out","text":"{\"type\":\"ack\",\"channel\":\"ticker\"}"}
{"ts":1037,"dir":"out","text":"{\"type\":\"tick\",\"price\":42.10}"}
{"ts":2055,"dir":"out","text":"{\"type\":\"tick\",\"price\":42.13}"}
```

**Step 2: replay it.**

```bash
mockforge serve \
  --ws-port 3001 \
  --ws-replay-file recordings/feed.jsonl
```

Your test client connects to `ws://localhost:3001/ws` and gets the exact same byte stream, with the exact same timing, every run. That makes flaky test reproduction a one-liner.

**Tip:** if you need to fuzz the recording (different prices, randomized timing), edit the `text` field to use templates: `"text":"{\"price\":{{rand.float 40.0 45.0}}}"`. Combine with `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true`.

**Read more:** [WebSocket Replay](https://docs.mockforge.dev/user-guide/websocket-mocking/replay.html).$body$,
ARRAY['websocket', 'replay', 'real-time'],
TRUE),

('grpc-reflection-without-proto',
'gRPC mocks via reflection — no .proto rebuilds',
'Use server reflection to mock a service when you don''t own the proto files.',
$body$If you''re consuming a third-party gRPC service and don''t have the proto files, MockForge can still front it: enable server reflection and the client gets enough metadata to drive calls.

The MockForge gRPC server enables reflection by default. Probe it with `grpcurl`:

```bash
# List services exposed by the mock
grpcurl -plaintext localhost:50051 list

# Describe a method (auto-discovered from the service)
grpcurl -plaintext localhost:50051 describe user.UserService.GetUser

# Make a call
grpcurl -plaintext \
  -d '{"id":"123"}' \
  localhost:50051 user.UserService/GetUser
```

To register a service without bringing your own protos, drop a minimal `.proto` stub matching the public surface you need:

```protobuf
// proto/external.proto — copies the upstream method shapes you call
syntax = "proto3";
package external;

service AccountService {
  rpc GetAccount (GetAccountReq) returns (Account);
}
message GetAccountReq { string id = 1; }
message Account { string id = 1; string email = 2; double balance = 3; }
```

```bash
MOCKFORGE_PROTO_DIR=proto mockforge serve --grpc-port 50051
```

MockForge synthesizes responses from the message schema, and reflection lets your real client (or `grpcurl`) discover the API live — no codegen step in your build pipeline.

**Read more:** [gRPC Mocking](https://docs.mockforge.dev/user-guide/grpc-mocking.html).$body$,
ARRAY['grpc', 'reflection', 'real-time'],
TRUE)
ON CONFLICT (slug) DO NOTHING;

-- Recipes: Messaging & async bucket

INSERT INTO learning_recipes (slug, title, description, body, tags, is_published) VALUES
('smtp-inbox-for-password-reset',
'Faking an SMTP inbox for password-reset E2E tests',
'Capture the password-reset email, parse the link, and click it — all without leaving CI.',
$body$Password-reset flows are awkward to E2E-test because they cross the email boundary. With a mocked SMTP server you can bring that boundary back inside the test process.

**1. Run MockForge SMTP alongside your app:**

```yaml
# config.yaml
smtp:
  enabled: true
  port: 1025
  hostname: "mockforge-smtp"
```

Point your app at it (e.g. `SMTP_HOST=localhost SMTP_PORT=1025`).

**2. Trigger a password reset from the test:**

```javascript
await page.goto('/forgot-password');
await page.fill('input[name=email]', 'user@example.com');
await page.click('button[type=submit]');
```

**3. Pull the captured email from the admin API:**

```javascript
const res = await fetch('http://localhost:9080/api/v1/smtp/messages?to=user@example.com');
const [email] = await res.json();
const link = email.body.match(/https?:\/\/[^\s"]+\/reset\/[a-z0-9]+/i)[0];
```

**4. Continue the flow with the captured link:**

```javascript
await page.goto(link);
await page.fill('input[name=password]', 'newPassword123');
await page.click('button[type=submit]');
```

The whole reset flow runs in CI in seconds, with zero dependency on a real SMTP relay or test mailbox provider. MockForge''s in-memory inbox auto-expires when the process exits, so cleanup is free.

**Read more:** [SMTP getting started](https://docs.mockforge.dev/protocols/smtp/getting-started.html), [SMTP fixtures](https://docs.mockforge.dev/protocols/smtp/fixtures.html).$body$,
ARRAY['smtp', 'e2e', 'testing'],
TRUE),

('deterministic-kafka-consumer-test',
'Driving consumer tests with a deterministic Kafka mock',
'Pre-seed a topic, run your consumer, assert. No ZooKeeper, no Docker, no flakes.',
$body$The shape that makes Kafka consumer tests reliable:

**1. Pre-seed messages via a fixtures file** so you don''t race the consumer''s startup:

```yaml
# kafka-fixtures.yaml
kafka:
  enabled: true
  port: 9092
  fixtures:
    - topic: "orders"
      partitions: 3
      messages:
        - key: "order-1"
          value: '{"id":"order-1","total":42.10}'
        - key: "order-2"
          value: '{"id":"order-2","total":99.00}'
        - key: "order-3"
          value: '{"id":"order-3","total":12.34}'
```

**2. Boot MockForge before the test process:**

```bash
mockforge serve --config kafka-fixtures.yaml &
MF_PID=$!

# Wait for the broker to be ready
until nc -z localhost 9092; do sleep 0.1; done
```

**3. Run your consumer test against `localhost:9092`:**

```rust
let consumer: StreamConsumer = ClientConfig::new()
    .set("bootstrap.servers", "localhost:9092")
    .set("group.id", "test-consumer")
    .set("auto.offset.reset", "earliest")
    .create()?;

consumer.subscribe(&["orders"])?;

let mut total = 0.0;
let mut count = 0;
while count < 3 {
    let msg = consumer.recv().await?;
    let order: Order = serde_json::from_slice(msg.payload().unwrap())?;
    total += order.total;
    count += 1;
}
assert_eq!(total, 153.44);
```

**4. Tear down:**

```bash
kill $MF_PID
```

Three messages, in order, every run. No `--retry` on flakes, no test pollution, no broker process to clean up. Add per-message timing controls if you need to test backpressure or rebalancing.

**Read more:** [`mockforge-kafka` README](https://github.com/SaaSy-Solutions/mockforge/blob/main/crates/mockforge-kafka/README.md).$body$,
ARRAY['kafka', 'testing', 'consumers'],
TRUE)
ON CONFLICT (slug) DO NOTHING;

-- Recipes: Chaos & resilience bucket

INSERT INTO learning_recipes (slug, title, description, body, tags, is_published) VALUES
('per-route-chaos',
'Per-route chaos with mockforge-route-chaos',
'Inject latency on `/payments` while leaving `/health` clean — chaos that maps to real outage shapes.',
$body$"All endpoints fail equally" isn''t how production breaks. The `mockforge-route-chaos` crate lets you scope every fault to a path pattern, so you can recreate "the payment service is slow but auth is fine."

```yaml
route_chaos:
  enabled: true
  rules:
    # Slow payments — third-party processor degraded
    - matches: "POST /payments/*"
      latency_ms: 1500
      latency_jitter_percent: 20
      probability: 1.0

    # Sporadic 503s on pricing — cache layer flapping
    - matches: "GET /pricing/*"
      http_errors: [503]
      probability: 0.2

    # Health checks always healthy — load balancer keeps the host in rotation
    - matches: "GET /health"
      probability: 0.0
```

Run it:

```bash
mockforge serve --config chaos.yaml --http-port 3000
```

Now your client''s circuit breaker for `/payments` opens, the pricing retry kicks in, and the load balancer doesn''t evict the node. That''s the failure mode you were trying to test — without taking down a real downstream.

**Pattern syntax:** glob (`*`, `**`) on path; method prefix is optional. Multiple rules are evaluated in order; first match wins.

**Read more:** [Chaos Engineering → request matchers](https://docs.mockforge.dev/user-guide/chaos-engineering.html).$body$,
ARRAY['chaos', 'route', 'resilience'],
TRUE),

('reproducing-flaky-timeout-bugs',
'Reproducing a flaky timeout bug deterministically',
'Pin the failure to a specific request, run the test 100 times, and watch it fail every time.',
$body$Flaky timeouts are the worst class of bug: they fail in CI once a week, repro never, and the stack trace blames the wrong place. The fix is to make the failure deterministic, then debug it like any other bug.

**Step 1: capture the failing request shape.** Look at logs from a real failure and note: which endpoint, which user, which payload size, which header.

**Step 2: write a chaos rule that fires on exactly that shape:**

```yaml
fault_injection:
  enabled: true
  timeout_errors: true
  timeout_ms: 30000              # match the client''s real timeout
  timeout_probability: 1.0       # 100% — but only on matching requests
  request_matcher:
    headers:
      - name: "x-trace-id"
        value: "repro-bug-1234"  # only this exact request
```

**Step 3: drive the failing path with that header.**

```bash
mockforge serve --config repro.yaml --http-port 3000 &

for i in $(seq 1 100); do
  curl -s -m 35 \
    -H "x-trace-id: repro-bug-1234" \
    http://localhost:3000/api/v1/charge \
    -d @payload.json
done
```

Every request hangs for 30s and returns 504. Your client''s retry / circuit breaker / dead-letter logic is now exercised on every iteration, not 1% of the time. Fix it, re-run, see the failure rate drop to zero, ship.

**Pro tip:** combine with `partial_responses` to repro "request times out *with* truncated body" — a classically nasty combo that breaks JSON-streaming clients.

**Read more:** [Chaos Engineering → timeouts](https://docs.mockforge.dev/user-guide/chaos-engineering.html).$body$,
ARRAY['chaos', 'debugging', 'timeouts'],
TRUE)
ON CONFLICT (slug) DO NOTHING;

-- Recipes: Performance & load testing bucket

INSERT INTO learning_recipes (slug, title, description, body, tags, is_published) VALUES
('wafbench-against-mocked-api',
'Running WAFBench against a mocked API',
'Establish a security-payload baseline before you ship — every request gets injected with classic OWASP attacks.',
$body$WAFBench-style testing fires thousands of malicious payloads (SQLi, XSS, command injection, path traversal, …) at every endpoint. Run it against a MockForge mock first to establish a baseline of what your client will send, *before* the real backend is even built.

```bash
mockforge bench \
  --spec api.yaml \
  --target http://localhost:3000 \
  --duration 300 --vus 5 \
  --wafbench-dir /usr/share/mockforge/wafbench/ \
  --wafbench-cycle-all
```

What this does:

1. Loads OWASP-style payload corpora from the WAFBench directory.
2. Generates a k6 script that walks every operation in `api.yaml`.
3. Substitutes attack payloads into every parameter, header, and body field.
4. Cycles through ALL payloads (`--wafbench-cycle-all`) rather than random-sampling — so the run is deterministic and reproducible.

Use the resulting `summary.json` as a regression baseline. When a real backend ships, run the same command against the staging URL and diff the response codes — anything that changes from "blocked / 4xx" to "200 OK" is a new vulnerability.

**Mock-side benefit:** because MockForge synthesizes responses from the spec, it returns 200 to *every* malformed payload. That gives you a pristine "what the client tries" trace, separate from "what the WAF blocks" — two distinct dimensions.

**Read more:** [Penetration Testing Guide](https://github.com/SaaSy-Solutions/mockforge/blob/main/docs/PENETRATION_TESTING_GUIDE.md).$body$,
ARRAY['performance', 'security', 'wafbench'],
TRUE),

('comparing-response-shapes-under-load',
'Comparing two response shapes under load',
'A/B two API designs without writing two clients — bench script generation does it for you.',
$body$Before you commit to a response shape, find out which one is actually faster end-to-end (not just on the wire — also through your client''s parser, validator, and rendering layer).

**Step 1: define both shapes** in the same MockForge config under different paths:

```yaml
http:
  routes:
    - path: /v1/orders
      method: GET
      response:
        body: |
          { "orders": [{{#repeat 50}}{ "id":"{{uuid}}", "total":{{rand.float 10 500}} }{{#unless @last}},{{/unless}}{{/repeat}}] }

    - path: /v2/orders   # flatter shape, fewer keys
      method: GET
      response:
        body: |
          [{{#repeat 50}}["{{uuid}}",{{rand.float 10 500}}]{{#unless @last}},{{/unless}}{{/repeat}}]
```

**Step 2: bench both, with identical load shapes:**

```bash
mockforge bench --target http://localhost:3000 \
  --operations "GET /v1/orders" --duration 120 --vus 25 \
  --threshold-percentile p95 --threshold-ms 100 \
  --output bench-results/v1.json

mockforge bench --target http://localhost:3000 \
  --operations "GET /v2/orders" --duration 120 --vus 25 \
  --threshold-percentile p95 --threshold-ms 100 \
  --output bench-results/v2.json
```

**Step 3: diff p95 / p99 / payload size** between `v1.json` and `v2.json`. Now you''re comparing payload designs with hard numbers, not vibes — and you haven''t built either backend.

For a tighter loop, wrap this in a `criterion`-style benchmark and chart the trend over commits.

**Read more:** [Load Testing](https://docs.mockforge.dev/user-guide/load-testing.html).$body$,
ARRAY['performance', 'benchmarking', 'design'],
TRUE)
ON CONFLICT (slug) DO NOTHING;

-- Recipes: Plugins & extensibility bucket

INSERT INTO learning_recipes (slug, title, description, body, tags, is_published) VALUES
('graphql-response-transform-plugin',
'A response-transform plugin (GraphQL example)',
'Take a generated GraphQL response and post-process it — pagination cursors, field filtering, derived fields.',
$body$The `examples/plugins/response-graphql` directory in the repo is a working response-type plugin you can copy as a starting point.

**What it does:** intercepts the synthesized GraphQL response, walks the result tree, and applies transforms — pagination cursors, field-level redaction, derived fields like `fullName = firstName + " " + lastName`.

**Skeleton:**

```rust
use mockforge_plugin_sdk::*;

#[plugin_export]
pub fn transform_response(req: Request, res: Response) -> Response {
    let mut value: serde_json::Value = serde_json::from_slice(&res.body).unwrap();

    // Add pagination cursor
    if let Some(arr) = value.pointer_mut("/data/users").and_then(|v| v.as_array_mut()) {
        let last_id = arr.last().and_then(|u| u["id"].as_str()).unwrap_or("");
        value["data"]["pageInfo"] = serde_json::json!({
            "endCursor": base64::encode(last_id),
            "hasNextPage": arr.len() == 50,
        });
    }

    Response {
        body: serde_json::to_vec(&value).unwrap().into(),
        ..res
    }
}
```

**Build + try it:**

```bash
cd examples/plugins/response-graphql
cargo build --target wasm32-unknown-unknown --release
mockforge plugin install ./target/wasm32-unknown-unknown/release/response_graphql.wasm
mockforge serve --config mockforge.yaml --enable-plugin response-graphql
```

Now every GraphQL response gets the cursor injected. The same shape works for HTTP/REST — swap `transform_response` for the `Response` interface and you''ve got a pre-flight content rewriter for any path.

**Read more:** [Plugin Development Guide](https://docs.mockforge.dev/development/plugin-development.html), [example source](https://github.com/SaaSy-Solutions/mockforge/tree/main/examples/plugins/response-graphql).$body$,
ARRAY['plugins', 'graphql', 'extensibility'],
TRUE),

('sandboxed-plugin-egress',
'Sandboxed plugin egress with the HTTP CONNECT proxy',
'Let a plugin call exactly two third-party APIs — and nothing else. Egress allowlisted at the proxy.',
$body$Plugins are sandboxed: by default they have no network access. To call out to an external API (rate-limit lookup, webhook target, ML inference endpoint, …), declare an egress allowlist.

**1. Declare in `plugin.yaml`:**

```yaml
name: rate-limit-lookup
version: 0.1.0
type: response
capabilities:
  network:
    egress:
      - host: "ratelimit.example.com"
        ports: [443]
      - host: "internal-quota.acme.io"
        ports: [443]
```

**2. Make HTTP calls from the plugin** using the SDK''s `host_fetch` helper, which routes through MockForge''s HTTP CONNECT proxy:

```rust
use mockforge_plugin_sdk::host_fetch;

let resp = host_fetch::get("https://ratelimit.example.com/v1/check?key=abc")
    .header("authorization", "Bearer ...")
    .send()?;
```

**3. The host enforces the allowlist at the proxy boundary**, not inside the plugin. A plugin trying to reach `evil.example.org` gets a connection refused at the wire level — even if the plugin is later patched to ignore its own manifest.

This is the safe pattern for any plugin that needs the outside world: declare the hosts up front, the host enforces, and the manifest is your audit trail.

**Why this matters:** without an egress allowlist, a malicious plugin update could silently start exfiltrating data. With it, the worst case is "plugin can call the hosts you already approved" — the same blast radius your application code has.

**Read more:** [`mockforge-plugin-egress` crate](https://github.com/SaaSy-Solutions/mockforge/tree/main/crates/mockforge-plugin-egress).$body$,
ARRAY['plugins', 'security', 'sandbox'],
TRUE)
ON CONFLICT (slug) DO NOTHING;

-- Recipes: CI/CD & automation bucket

INSERT INTO learning_recipes (slug, title, description, body, tags, is_published) VALUES
('mockforge-as-test-container',
'Spin up MockForge as a Postgres-style test container',
'Use Testcontainers (or any container runtime) to start a fresh MockForge per test, just like you would with Postgres.',
$body$The Testcontainers pattern works for MockForge: each test gets a fresh container, gets a randomly-allocated port, and tears down on completion. No port-conflict races, no leaked state between tests.

**Java / Kotlin (Testcontainers):**

```java
@Container
GenericContainer<?> mockforge = new GenericContainer<>(
        DockerImageName.parse("ghcr.io/saasy-solutions/mockforge:latest"))
    .withCopyFileToContainer(MountableFile.forClasspathResource("api.yaml"), "/spec/api.yaml")
    .withCommand("serve", "--spec", "/spec/api.yaml", "--http-port", "3000")
    .withExposedPorts(3000)
    .waitingFor(Wait.forHttp("/__mockforge/health").forPort(3000));

@Test
void itPostsToOrders() {
    String url = "http://" + mockforge.getHost() + ":" + mockforge.getMappedPort(3000);
    // ... HTTP-test against url ...
}
```

**Node.js (testcontainers package):**

```javascript
import { GenericContainer } from "testcontainers";

const mockforge = await new GenericContainer("ghcr.io/saasy-solutions/mockforge:latest")
  .withCopyFilesToContainer([{ source: "api.yaml", target: "/spec/api.yaml" }])
  .withCommand(["serve", "--spec", "/spec/api.yaml", "--http-port", "3000"])
  .withExposedPorts(3000)
  .start();

const baseUrl = `http://${mockforge.getHost()}:${mockforge.getMappedPort(3000)}`;
// run tests against baseUrl
await mockforge.stop();
```

**Rust (`testcontainers` crate):**

```rust
let mockforge = GenericImage::new("ghcr.io/saasy-solutions/mockforge", "latest")
    .with_exposed_port(3000)
    .with_volume("./api.yaml", "/spec/api.yaml")
    .with_cmd(vec!["serve", "--spec", "/spec/api.yaml", "--http-port", "3000"]);
let container = docker.run(mockforge);
let port = container.get_host_port_ipv4(3000);
```

Each test gets isolation equivalent to a fresh database. Combine with template expansion and world-state snapshots if you need pre-seeded data.

**Read more:** [Tutorial → CI snippets](https://github.com/SaaSy-Solutions/mockforge/blob/main/guides/guide01/ci_snippets.md).$body$,
ARRAY['ci', 'testcontainers', 'integration-testing'],
TRUE),

('versioned-mocks-per-branch',
'Versioned mocks per branch using the registry server',
'Match the mock version to the branch — `main` gets prod-ready mocks, feature branches get their own variant.',
$body$When multiple branches modify the same API surface, you want each branch''s tests to hit *its* version of the mock — not whatever happens to be deployed centrally. The registry server handles this with workspaces and tags.

**1. On each branch, push a tagged mock:**

```bash
# .github/workflows/push-mock.yml — runs on every PR + main push
mockforge cloud push \
  --project my-api \
  --tag "${{ github.head_ref || github.ref_name }}" \
  --spec api.yaml
```

That creates a versioned snapshot like `my-api:feature/new-orders` or `my-api:main`.

**2. Tests pull the matching mock:**

```bash
# in CI
export MF_TAG="${GITHUB_HEAD_REF:-${GITHUB_REF_NAME}}"
mockforge cloud pull --project my-api --tag "$MF_TAG" --output mock.yaml
mockforge serve --config mock.yaml --http-port 3000 &
```

Now `feature/new-orders` runs against the mock as it exists on that branch — no cross-contamination from other in-flight branches. When the branch merges, its tag is auto-promoted to `main`.

**3. Production parity check** (optional but worth it): on every PR, also run tests against the `main`-tagged mock. If they fail, your branch has introduced a contract regression.

```bash
# parity job
mockforge cloud pull --project my-api --tag main --output baseline.yaml
mockforge serve --config baseline.yaml --http-port 3001 &
API_BASE_URL=http://localhost:3001 npm run test:contract
```

The result is a contract-verification gate that''s as automatic as your unit tests.

**Read more:** [Cloud Workspaces](https://docs.mockforge.dev/user-guide/cloud-workspaces.html), [Scenario Marketplace](https://docs.mockforge.dev/user-guide/scenario-marketplace.html).$body$,
ARRAY['ci', 'cloud', 'versioning'],
TRUE)
ON CONFLICT (slug) DO NOTHING;

-- Recipes: Advanced patterns bucket

INSERT INTO learning_recipes (slug, title, description, body, tags, is_published) VALUES
('scenario-branching',
'Scenario branching with mockforge-scenarios',
'Same endpoint, different reality — switch between "happy path", "user-not-found", and "service-degraded" with one header.',
$body$A single mock can serve many scenarios. The `mockforge-scenarios` crate exposes a header (`X-Mock-Scenario` by default) that routes to a named scenario folder.

**1. Lay out scenarios on disk:**

```
scenarios/
├── happy/
│   └── users-get.json
├── user-not-found/
│   └── users-get.json
└── service-degraded/
    └── users-get.json
```

Each file is a normal MockForge route override.

**2. Configure scenario routing:**

```yaml
scenarios:
  enabled: true
  base_dir: ./scenarios
  selector:
    type: header
    name: X-Mock-Scenario
  default: happy
```

**3. Switch reality from the client:**

```bash
# Happy path — default
curl http://localhost:3000/users/42

# Force a 404
curl -H "X-Mock-Scenario: user-not-found" http://localhost:3000/users/42

# Force a 503
curl -H "X-Mock-Scenario: service-degraded" http://localhost:3000/users/42
```

Now your QA team can drive the same E2E test through three failure modes by toggling one header — no environment swap, no separate mock instance, no race conditions.

**Pairs well with:** scenario marketplace, where you can publish a scenario set (`acme-corp/payments-failure-modes@1.0`) and let other teams `mockforge scenario install` it.

**Read more:** [Scenario State Machines](https://docs.mockforge.dev/user-guide/scenario-state-machines.html), [Scenario Marketplace](https://docs.mockforge.dev/user-guide/scenario-marketplace.html).$body$,
ARRAY['scenarios', 'advanced', 'qa'],
TRUE),

('record-and-replay-real-traffic',
'Recording real traffic and replaying it',
'Capture a session against staging, replay it locally as a deterministic mock — bug repros made trivial.',
$body$The Flight Recorder is the answer to "I can''t reproduce this bug locally." Run MockForge as a transparent proxy in front of a real backend, replay later.

**Step 1: record.** Point MockForge at the real upstream, drive the failing flow through it, and every request/response lands in a SQLite file.

```bash
mockforge serve \
  --upstream https://api.staging.example.com \
  --recorder \
  --recorder-db ./bug-1234.db \
  --http-port 3000

# In another terminal — drive the failing flow
curl -X POST http://localhost:3000/api/v1/checkout -d @cart.json
# ... continue until the bug reproduces ...
```

The recorder captures full request + response, including headers, body, status, and timing.

**Step 2: replay.** Stop the proxy. Restart in replay mode against the same DB:

```bash
mockforge serve --replay ./bug-1234.db --http-port 3000
```

Now `localhost:3000` returns the exact recorded responses. Run your client against it, attach a debugger, iterate. The bug is now deterministic — you can step through the failing branch as many times as you need.

**Step 3: ship the recording with the bug report.** `bug-1234.db` is small (a few hundred KB for most flows) and self-contained. Anyone can:

```bash
git clone bug-repo
mockforge serve --replay ./bug-1234.db --http-port 3000
npm test
```

…and reproduce the failure on the first try.

**Tip:** the recorder also captures a normalized request fingerprint, so downstream tools can dedupe replays, redact secrets in headers, or generate scenario files automatically from the recorded session.

**Read more:** [`mockforge-recorder` crate](https://github.com/SaaSy-Solutions/mockforge/tree/main/crates/mockforge-recorder).$body$,
ARRAY['recorder', 'debugging', 'advanced'],
TRUE)
ON CONFLICT (slug) DO NOTHING;


COMMIT;
