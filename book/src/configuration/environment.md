# Environment Variables

MockForge supports extensive configuration through environment variables. This page documents all available environment variables, their purposes, and usage examples.

## Core Functionality

### Server Control

- `MOCKFORGE_LATENCY_ENABLED=true|false` (default: `true`)
  - Enable/disable response latency simulation
  - When disabled, responses are immediate

- `MOCKFORGE_FAILURES_ENABLED=true|false` (default: `false`)
  - Enable/disable failure injection
  - When enabled, can simulate HTTP errors and timeouts

- `MOCKFORGE_LOG_LEVEL=debug|info|warn|error` (default: `info`)
  - Set the logging verbosity level
  - Available: `debug`, `info`, `warn`, `error`

### Recording and Replay

- `MOCKFORGE_RECORD_ENABLED=true|false` (default: `false`)
  - Enable recording of HTTP requests as fixtures
  - Recorded fixtures can be replayed later

- `MOCKFORGE_REPLAY_ENABLED=true|false` (default: `false`)
  - Enable replay of recorded fixtures
  - When enabled, serves recorded responses instead of generating new ones

- `MOCKFORGE_PROXY_ENABLED=true|false` (default: `false`)
  - Enable proxy mode for forwarding requests
  - Useful for testing against real APIs

## HTTP Server Configuration

### Server Settings

- `MOCKFORGE_HTTP_PORT=3000` (default: `3000`)
  - Port for the HTTP server to listen on

- `MOCKFORGE_HTTP_HOST=127.0.0.1` (default: `0.0.0.0`)
  - Host address for the HTTP server to bind to

- `MOCKFORGE_CORS_ENABLED=true|false` (default: `true`)
  - Enable/disable CORS headers in responses

- `MOCKFORGE_REQUEST_TIMEOUT_SECS=30` (default: `30`)
  - Timeout for HTTP requests in seconds

### OpenAPI Integration

- `MOCKFORGE_HTTP_OPENAPI_SPEC=path/to/spec.json`
  - Path to OpenAPI specification file
  - Enables automatic endpoint generation from OpenAPI spec

### Validation and Templating

- `MOCKFORGE_REQUEST_VALIDATION=enforce|warn|off` (default: `enforce`)
  - Level of request validation
  - `enforce`: Reject invalid requests with error
  - `warn`: Log warnings but allow requests
  - `off`: Skip validation entirely

- `MOCKFORGE_RESPONSE_VALIDATION=true|false` (default: `false`)
  - Enable validation of generated responses
  - Useful for ensuring response format compliance

- `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true|false` (default: `false`)
  - Enable template expansion in responses
  - Allows use of `{{uuid}}`, `{{now}}`, etc. in responses

- `MOCKFORGE_AGGREGATE_ERRORS=true|false` (default: `true`)
  - Aggregate multiple validation errors into a single response
  - When enabled, returns all validation errors at once

- `MOCKFORGE_VALIDATION_STATUS=400|422` (default: `400`)
  - HTTP status code for validation errors
  - `400`: Bad Request (general)
  - `422`: Unprocessable Entity (validation-specific)

## WebSocket Server Configuration

### Server Settings

- `MOCKFORGE_WS_PORT=3001` (default: `3001`)
  - Port for the WebSocket server to listen on

- `MOCKFORGE_WS_HOST=127.0.0.1` (default: `0.0.0.0`)
  - Host address for the WebSocket server to bind to

- `MOCKFORGE_WS_CONNECTION_TIMEOUT_SECS=300` (default: `300`)
  - WebSocket connection timeout in seconds

### Replay Configuration

- `MOCKFORGE_WS_REPLAY_FILE=path/to/replay.jsonl`
  - Path to WebSocket replay file
  - Enables scripted WebSocket message sequences

## gRPC Server Configuration

### Server Settings

- `MOCKFORGE_GRPC_PORT=50051` (default: `50051`)
  - Port for the gRPC server to listen on

- `MOCKFORGE_GRPC_HOST=127.0.0.1` (default: `0.0.0.0`)
  - Host address for the gRPC server to bind to

## Admin UI Configuration

### Server Settings

- `MOCKFORGE_ADMIN_ENABLED=true|false` (default: `false`)
  - Enable/disable the Admin UI
  - When enabled, provides web interface for management

- `MOCKFORGE_ADMIN_PORT=9080` (default: `9080`)
  - Port for the Admin UI server to listen on

- `MOCKFORGE_ADMIN_HOST=127.0.0.1` (default: `127.0.0.1`)
  - Host address for the Admin UI server to bind to

### UI Configuration

- `MOCKFORGE_ADMIN_MOUNT_PATH=/admin` (default: none)
  - Mount path for embedded Admin UI
  - When set, Admin UI is available under HTTP server

- `MOCKFORGE_ADMIN_API_ENABLED=true|false` (default: `true`)
  - Enable/disable Admin UI API endpoints
  - Controls whether `/__mockforge/*` endpoints are available

## Data Generation Configuration

### Faker Control

- `MOCKFORGE_RAG_ENABLED=true|false` (default: `false`)
  - Enable Retrieval-Augmented Generation for data
  - Requires additional setup for LLM integration

- `MOCKFORGE_FAKE_TOKENS=true|false` (default: `true`)
  - Enable/disable faker token expansion
  - Controls whether `{{faker.email}}` etc. work

### RAG / LLM Provider

- `MOCKFORGE_RAG_PROVIDER=openai|anthropic|ollama` (default: `openai`)
  - LLM provider for retrieval-augmented generation.
- `MOCKFORGE_RAG_API_KEY=<key>` — provider API key (falls back to `OPENAI_API_KEY` when unset).
- `MOCKFORGE_RAG_API_ENDPOINT=<url>` — custom endpoint, overrides the provider default.
- `MOCKFORGE_RAG_MODEL=<name>` — e.g. `gpt-4`, `claude-3-5-sonnet`, `llama3`.
- `MOCKFORGE_RAG_TEMPERATURE=<float>` (default: `0.7`).
- `MOCKFORGE_RAG_MAX_TOKENS=<int>` (default: `2048`).
- `MOCKFORGE_RAG_CONTEXT_WINDOW=<int>` (default: `4096`).
- `MOCKFORGE_RAG_TIMEOUT_SECONDS=<int>` (default: `30`).
- `MOCKFORGE_RAG_MAX_RETRIES=<int>` (default: `3`).
- `OPENAI_API_KEY=<key>` — fallback when `MOCKFORGE_RAG_API_KEY` is unset.

### Embedding (vector search)

- `MOCKFORGE_EMBEDDING_PROVIDER=openai|local|ollama` — provider for embeddings used by RAG.
- `MOCKFORGE_EMBEDDING_MODEL=<name>` — e.g. `text-embedding-3-small`.
- `MOCKFORGE_EMBEDDING_ENDPOINT=<url>` — custom endpoint.
- `MOCKFORGE_SIMILARITY_THRESHOLD=<float>` (`0.0`–`1.0`) — minimum match score.

## Registry / Marketplace

- `MOCKFORGE_REGISTRY_TOKEN=<token>` — auth token for publishing scenarios or plugins to the registry. Required for `mockforge scenario publish` and similar commands.
- `MOCKFORGE_PLUGIN_REGISTRY_URL=<url>` — registry endpoint used by `mockforge plugin install` / `publish` (default: the public mockforge.dev registry).

## OpenAPI / Spec Loading

- `MOCKFORGE_OPENAPI_SPEC_URL=<url>` — alternative to `--spec`; load the OpenAPI spec from a URL at startup. Useful in containerized deployments where the spec lives behind a static CDN.

## Distributed Tracing (OpenTelemetry / OTLP)

- `MOCKFORGE_OTLP_ENDPOINT=<url>` — OTLP collector endpoint (e.g. `http://jaeger:4317` or `http://otel-collector:4318`).
- `MOCKFORGE_OTLP_SERVICE_NAME=<name>` — service name attached to spans (default: `mockforge`).
- `MOCKFORGE_OTLP_SAMPLING_RATE=<float>` — `0.0`–`1.0`. `1.0` = trace every request.

## Rate Limiting

- `MOCKFORGE_RATE_LIMIT_ENABLED=true|false` — toggle the HTTP middleware rate limiter.
- `MOCKFORGE_RATE_LIMIT_DISABLED=true|false` — explicit opt-out (overrides config-file `enabled: true`). Useful for `--no-rate-limit` parity in containers.
- `MOCKFORGE_RATE_LIMIT_RPM=<int>` — global requests-per-minute cap.

## Kafka Protocol

- `MOCKFORGE_KAFKA_ENABLED=true|false` — start the Kafka mock listener.
- `MOCKFORGE_KAFKA_ADVERTISED_PORT=<int>` — port advertised in metadata responses (defaults to the bind port).
- `MOCKFORGE_KAFKA_FIXTURES_DIR=path/to/kafka-fixtures` — directory of pre-recorded Kafka topic fixtures.

## Federation

- `MOCKFORGE_FEDERATION_POLL_URL=<url>` — upstream MockForge instance to poll for shared workspaces.
- `MOCKFORGE_FEDERATION_POLL_TOKEN=<token>` — auth token for the upstream poll.
- `MOCKFORGE_FEDERATION_POLL_INTERVAL_SECS=<int>` — poll cadence (default: 60).
- `MOCKFORGE_FEDERATION_WORKSPACE_ID=<uuid>` — workspace this instance federates into.

## Encryption / Secret Storage

- `MOCKFORGE_ENCRYPTION_KEY=<base64-key>` — base64-encoded AES-256 key used to encrypt config/data at rest.
- `MOCKFORGE_MASTER_KEY=<base64-key>` — master KEK that wraps per-workspace data encryption keys.
- `MOCKFORGE_BYOK_ENCRYPTION_KEY=<base64-key>` — bring-your-own-key override for workspace encryption.
- `MOCKFORGE_KMS_PROVIDER=aws|gcp|azure|vault` — when set, MockForge fetches the master key from a managed KMS instead of using `MOCKFORGE_MASTER_KEY`.
- `MOCKFORGE_KMS_REGION=<region>` — region for the KMS provider (e.g. `us-east-1`).
- `MOCKFORGE_VAULT_ADDR=<url>` — Vault server address when using `MOCKFORGE_KMS_PROVIDER=vault`.
- `MOCKFORGE_VAULT_TOKEN=<token>` — Vault auth token.

## Database (registry server / collab)

These apply to the multi-tenant registry server and the collab workspace
backend; the OSS local mock server doesn't need them.

- `MOCKFORGE_DB_TYPE=sqlite|postgres` — backing store for the registry / collab.
- `MOCKFORGE_DB_CONNECTION=<url>` — connection string (e.g. `postgres://user:pass@host/db` or `sqlite:./mockforge.db`).

## Fixtures and Testing

### Fixtures Configuration

- `MOCKFORGE_FIXTURES_DIR=path/to/fixtures` (default: `./fixtures`)
  - Directory where fixtures are stored
  - Used for recording and replaying HTTP requests

- `MOCKFORGE_RECORD_GET_ONLY=true|false` (default: `false`)
  - When recording, only record GET requests
  - Reduces fixture file size for read-only APIs

## Observability

### Metrics CSV Log

- `MOCKFORGE_METRICS_LOG_FILE=path/to/metrics.csv`
  - When set, the admin server's system-monitoring task appends one CSV row
    every 10 s with `timestamp,cpu_pct,mem_mb,total_reqs,err_rate`.
  - Survives restarts; chartable in any spreadsheet, Grafana, or
    dashboarding tool.
  - Example: `MOCKFORGE_METRICS_LOG_FILE=/var/log/mockforge-metrics.csv`
  - The TUI dashboard also tracks lifetime peak CPU%, memory MB, and
    error rate in-memory and renders them as `current (peak X)` next to
    the live values.

## Configuration Files

### Configuration Loading

- `MOCKFORGE_CONFIG_FILE=path/to/config.yaml`
  - Path to YAML configuration file
  - Alternative to environment variables

## Usage Examples

### Basic HTTP Server with OpenAPI

```bash
export MOCKFORGE_HTTP_OPENAPI_SPEC=examples/openapi-demo.json
export MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true
export MOCKFORGE_ADMIN_ENABLED=true
cargo run -p mockforge-cli -- serve --http-port 3000 --admin-port 9080
```

### Full WebSocket Support

```bash
export MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl
export MOCKFORGE_WS_PORT=3001
export MOCKFORGE_HTTP_OPENAPI_SPEC=examples/openapi-demo.json
export MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true
cargo run -p mockforge-cli -- serve --admin
```

### Development Setup

```bash
export MOCKFORGE_LOG_LEVEL=debug
export MOCKFORGE_LATENCY_ENABLED=false
export MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true
export MOCKFORGE_ADMIN_ENABLED=true
export MOCKFORGE_HTTP_OPENAPI_SPEC=examples/openapi-demo.json
cargo run -p mockforge-cli -- serve
```

### Production Setup

```bash
export MOCKFORGE_LOG_LEVEL=warn
export MOCKFORGE_LATENCY_ENABLED=true
export MOCKFORGE_FAILURES_ENABLED=false
export MOCKFORGE_REQUEST_VALIDATION=enforce
export MOCKFORGE_ADMIN_ENABLED=false
export MOCKFORGE_HTTP_OPENAPI_SPEC=path/to/production-spec.json
cargo run -p mockforge-cli -- serve --http-port 80
```

## Environment Variable Priority

Environment variables override configuration file settings. CLI flags take precedence over both. The priority order is:

1. CLI flags (highest priority)
2. Environment variables
3. Configuration file settings
4. Default values (lowest priority)

## Security Considerations

- Be careful with `MOCKFORGE_ADMIN_ENABLED=true` in production
- Consider setting restrictive host bindings (`127.0.0.1`) for internal use
- Use `MOCKFORGE_FAKE_TOKENS=false` for deterministic testing
- Review `MOCKFORGE_CORS_ENABLED` settings for cross-origin requests

## Troubleshooting

### Common Issues

1. **Environment variables not taking effect**
   - Check variable names for typos
   - Ensure variables are exported before running the command
   - Use `env | grep MOCKFORGE` to verify variables are set

2. **Port conflicts**
   - Use different ports via `MOCKFORGE_HTTP_PORT`, `MOCKFORGE_WS_PORT`, etc.
   - Check what processes are using ports with `netstat -tlnp`

3. **OpenAPI spec not loading**
   - Verify file path in `MOCKFORGE_HTTP_OPENAPI_SPEC`
   - Ensure JSON/YAML syntax is valid
   - Check file permissions

4. **Template expansion not working**
   - Set `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true`
   - Verify token syntax (e.g., `{{uuid}}` not `{uuid}`)

For more detailed configuration options, see the [Configuration Files](files.md) documentation.
