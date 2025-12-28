# MockForge Environment Variables Reference

This document provides a comprehensive reference for all environment variables used across the MockForge ecosystem.

## Quick Reference

| Category | Variables | Required |
|----------|-----------|----------|
| [Registry Server](#registry-server) | 30+ | DATABASE_URL, JWT_SECRET |
| [Core Configuration](#core-configuration) | 50+ | None |
| [HTTP Server](#http-server) | 20+ | None |
| [CLI](#cli) | 10+ | None |
| [AI/RAG Features](#airag-features) | 15+ | None (API keys for features) |

---

## Registry Server

These variables configure the MockForge Plugin Registry server.

### Required Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `DATABASE_URL` | Database connection URL (PostgreSQL/SQLite) | `postgres://user:pass@localhost/mockforge` |
| `JWT_SECRET` | Secret key for JWT token signing | `your-secure-secret-key-min-32-chars` |

### Server Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `8080` | Server port |
| `CORS_ALLOWED_ORIGINS` | None | Comma-separated CORS allowed origins |
| `MAX_REQUEST_BODY_SIZE` | None | Maximum request body size |
| `SHUTDOWN_TIMEOUT_SECS` | `30` | Graceful shutdown timeout in seconds |
| `ENVIRONMENT` | None | Deployment environment (dev/staging/prod) |

### Database

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_MAX_CONNECTIONS` | None | Maximum database connection pool size |
| `TEST_DATABASE_URL` | None | Test database URL for integration tests |
| `ANALYTICS_DB_PATH` | None | Path to analytics SQLite database |

### Storage (S3/MinIO)

| Variable | Default | Description |
|----------|---------|-------------|
| `S3_BUCKET` | `mockforge-plugins` | S3 bucket for plugin storage |
| `S3_REGION` | `us-east-1` | AWS S3 region |
| `S3_ENDPOINT` | None | Custom S3 endpoint (MinIO compatible) |
| `AWS_ACCESS_KEY_ID` | None | AWS S3 access key |
| `AWS_SECRET_ACCESS_KEY` | None | AWS S3 secret access key |
| `MAX_PLUGIN_SIZE` | `52428800` (50MB) | Maximum plugin upload size in bytes |

### Caching

| Variable | Default | Description |
|----------|---------|-------------|
| `REDIS_URL` | None | Redis connection URL for caching |

### Security Features

| Variable | Default | Description |
|----------|---------|-------------|
| `TWO_FACTOR_ENABLED` | `false` | Enable two-factor authentication (requires Redis) |

### Rate Limiting

| Variable | Default | Description |
|----------|---------|-------------|
| `RATE_LIMIT_PER_MINUTE` | `60` | Global rate limit per minute |
| `RATE_LIMIT_PER_USER` | None | Per-user rate limit |
| `RATE_LIMIT_CLEANUP_INTERVAL_SECS` | None | Rate limit cleanup interval |

### Email Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `EMAIL_PROVIDER` | `disabled` | Email provider (`postmark`, `brevo`, `smtp`, `disabled`) |
| `EMAIL_FROM` | `noreply@mockforge.dev` | From email address |
| `EMAIL_FROM_NAME` | `MockForge` | From email display name |
| `EMAIL_API_KEY` | None | Email service API key (Postmark/Brevo) |
| `SMTP_HOST` | None | SMTP server hostname |
| `SMTP_PORT` | `587` | SMTP server port |
| `SMTP_USERNAME` | None | SMTP username |
| `SMTP_PASSWORD` | None | SMTP password |

### Deployment

| Variable | Default | Description |
|----------|---------|-------------|
| `APP_BASE_URL` | `https://app.mockforge.dev` | Base URL for app links in emails |
| `MOCKFORGE_BASE_URL` | None | MockForge API base URL |
| `FLYIO_API_TOKEN` | None | Fly.io API token for deployments |
| `FLYIO_ORG_SLUG` | None | Fly.io organization slug |
| `MOCKFORGE_DOCKER_IMAGE` | None | Docker image for deployments |

---

## Core Configuration

These variables configure the core MockForge mock server functionality.

### Server Ports

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_HTTP_PORT` | `3000` | HTTP server port |
| `MOCKFORGE_HTTP_HOST` | `0.0.0.0` | HTTP server bind host |
| `MOCKFORGE_WS_PORT` | None | WebSocket server port |
| `MOCKFORGE_GRPC_PORT` | None | gRPC server port |
| `MOCKFORGE_ADMIN_PORT` | None | Admin interface port |
| `MOCKFORGE_TCP_PORT` | None | TCP proxy port |
| `MOCKFORGE_TCP_HOST` | None | TCP proxy bind host |

### Protocol Enable Flags

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_TCP_ENABLED` | `false` | Enable TCP proxy |
| `MOCKFORGE_ADMIN_ENABLED` | `false` | Enable admin interface |
| `MOCKFORGE_ADMIN_API_ENABLED` | `false` | Enable admin API endpoints |
| `MOCKFORGE_CUSTOM_FIXTURES_ENABLED` | `false` | Enable custom fixtures |

### SMTP Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_SMTP_ENABLED` | `false` | Enable SMTP server |
| `MOCKFORGE_SMTP_PORT` | None | SMTP server port |
| `MOCKFORGE_SMTP_HOST` | None | SMTP server host |
| `MOCKFORGE_SMTP_HOSTNAME` | None | SMTP hostname for MAIL FROM |

### Traffic Control

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_LATENCY_ENABLED` | `false` | Enable latency injection |
| `MOCKFORGE_FAILURES_ENABLED` | `false` | Enable failure injection |
| `MOCKFORGE_OVERRIDES_ENABLED` | `false` | Enable response overrides |
| `MOCKFORGE_TRAFFIC_SHAPING_ENABLED` | `false` | Enable traffic shaping |

### Bandwidth Control

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_BANDWIDTH_ENABLED` | `false` | Enable bandwidth limiting |
| `MOCKFORGE_BANDWIDTH_MAX_BYTES_PER_SEC` | None | Max bandwidth in bytes/sec |
| `MOCKFORGE_BANDWIDTH_BURST_CAPACITY_BYTES` | None | Burst capacity in bytes |

### Packet Loss Simulation

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_BURST_LOSS_ENABLED` | `false` | Enable packet loss bursts |
| `MOCKFORGE_BURST_LOSS_PROBABILITY` | None | Probability of loss burst (0-1) |
| `MOCKFORGE_BURST_LOSS_DURATION_MS` | None | Burst loss duration in ms |
| `MOCKFORGE_BURST_LOSS_RATE` | None | Packet loss rate (0-1) |
| `MOCKFORGE_BURST_LOSS_RECOVERY_MS` | None | Recovery time after burst loss |

### Response Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND` | `false` | Expand response templates |
| `MOCKFORGE_RESPONSE_SELECTION_MODE` | None | Response selection strategy |
| `MOCKFORGE_REALITY_LEVEL` | None | Mock response realism (0-100) |

### Validation

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_REQUEST_VALIDATION` | None | Request validation mode |
| `MOCKFORGE_RESPONSE_VALIDATION` | `false` | Validate responses |
| `MOCKFORGE_AGGREGATE_ERRORS` | `false` | Aggregate validation errors |
| `MOCKFORGE_VALIDATION_STATUS` | None | Validation status code |
| `MOCKFORGE_VALIDATION_OVERRIDES_JSON` | None | Validation overrides as JSON |

### File Paths

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_MOCK_FILES_DIR` | `mock-files` | Directory for mock files |
| `MOCKFORGE_FIXTURES_DIR` | `fixtures` | Directory for test fixtures |
| `MOCKFORGE_SNAPSHOT_DIR` | None | Snapshot storage directory |
| `MOCKFORGE_HTTP_OVERRIDES_GLOB` | None | Glob pattern for override files |
| `MOCKFORGE_COVERAGE_UI_PATH` | None | Coverage UI path |

### Logging

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_LOG_LEVEL` | None | Log level (debug/info/warn/error) |
| `RUST_LOG` | None | Rust logging level (standard) |

---

## HTTP Server

### Rate Limiting

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_RATE_LIMIT_RPM` | None | Requests per minute rate limit |
| `MOCKFORGE_RATE_LIMIT_BURST` | None | Burst rate limit |

### Management API

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_MANAGEMENT_API_URL` | `http://localhost:3000/management` | Management API URL |

### Recording

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | None | Database URL for recording |
| `RECORDER_DATABASE_PATH` | None | Recorder SQLite database path |
| `BEHAVIORAL_CLONING_ENABLED` | `false` | Enable behavioral cloning |

### WebSocket

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_WS_REPLAY_FILE` | None | WebSocket replay file path |
| `MOCKFORGE_WS_HOTRELOAD` | `false` | Enable hot reload for WS |
| `MOCKFORGE_WS_PROXY_UPSTREAM_URL` | None | Upstream WebSocket URL to proxy |

---

## AI/RAG Features

### RAG Provider Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_RAG_PROVIDER` | `openai` | RAG provider (`openai`/`anthropic`/`ollama`) |
| `MOCKFORGE_RAG_API_ENDPOINT` | Provider-specific | RAG API endpoint |
| `MOCKFORGE_RAG_API_KEY` | None | RAG service API key |
| `MOCKFORGE_RAG_MODEL` | Provider-specific | RAG model name |
| `MOCKFORGE_RAG_MAX_TOKENS` | None | Max tokens for RAG |
| `MOCKFORGE_RAG_TEMPERATURE` | None | RAG temperature parameter |
| `MOCKFORGE_RAG_TIMEOUT` | None | RAG request timeout |
| `MOCKFORGE_RAG_CONTEXT_WINDOW` | None | RAG context window size |
| `MOCKFORGE_RAG_TIMEOUT_SECONDS` | None | RAG timeout in seconds |
| `MOCKFORGE_RAG_MAX_RETRIES` | None | Max RAG retries |

### AI Generation

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_AI_PROVIDER` | `openai` | AI provider for generation |
| `MOCKFORGE_AI_API_KEY` | None | AI service API key |
| `MOCKFORGE_AI_MODEL` | Provider-specific | AI model name |
| `MOCKFORGE_AI_ENDPOINT` | Provider-specific | AI API endpoint |
| `MOCKFORGE_AI_TEMPERATURE` | None | AI temperature |
| `MOCKFORGE_AI_MAX_TOKENS` | None | Max AI response tokens |
| `OPENAI_API_KEY` | None | OpenAI API key (fallback) |

### Semantic Search

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_SEMANTIC_SEARCH` | `false` | Enable semantic search |
| `MOCKFORGE_EMBEDDING_PROVIDER` | `openai` | Embedding provider |
| `MOCKFORGE_EMBEDDING_MODEL` | `text-embedding-3-small` | Embedding model |
| `MOCKFORGE_EMBEDDING_ENDPOINT` | None | Embedding API endpoint |
| `MOCKFORGE_SIMILARITY_THRESHOLD` | `0.7` | Similarity threshold (0-1) |
| `MOCKFORGE_MAX_CHUNKS` | None | Max context chunks |

---

## CLI

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_MANAGEMENT_URL` | None | Management API URL |
| `MOCKFORGE_API_KEY` | None | MockForge cloud API key |
| `MOCKFORGE_REGISTRY_TOKEN` | None | Registry authentication token |
| `MOCKFORGE_RECORDER_DB` | `recordings.db` | Recorder database path |
| `USER` | `unknown` | Current user for authoring |

### Speech-to-Text (Voice Commands)

| Variable | Default | Description |
|----------|---------|-------------|
| `OPENAI_API_KEY` | None | OpenAI API key for STT |
| `GOOGLE_CLOUD_API_KEY` | None | Google Cloud API key for STT |
| `GOOGLE_APPLICATION_CREDENTIALS` | None | Google Cloud credentials file |
| `VOSK_MODEL_PATH` | None | Vosk model directory |

---

## Collaboration Server

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_JWT_SECRET` | `change-me-in-production` | JWT signing secret |
| `MOCKFORGE_DATABASE_URL` | `sqlite://mockforge-collab.db` | Database URL |
| `MOCKFORGE_BIND_ADDRESS` | `127.0.0.1:8080` | Server bind address |

---

## Tunnel Server

| Variable | Default | Description |
|----------|---------|-------------|
| `TUNNEL_SERVER_PORT` | None | Tunnel server port |
| `TUNNEL_SERVER_BIND` | None | Tunnel server bind address |
| `TUNNEL_DATABASE_PATH` | None | Tunnel database file path |
| `TUNNEL_USE_IN_MEMORY_STORAGE` | `false` | Use in-memory storage |
| `TUNNEL_TLS_CERT` | None | TLS certificate path |
| `TUNNEL_TLS_KEY` | None | TLS private key path |
| `TUNNEL_RATE_LIMIT_ENABLED` | `false` | Enable rate limiting |
| `TUNNEL_RATE_LIMIT_RPM` | None | Tunnel rate limit per minute |
| `TUNNEL_AUDIT_LOG_ENABLED` | `false` | Enable audit logging |
| `TUNNEL_AUDIT_LOG_PATH` | None | Audit log file path |

---

## Runtime Daemon

Auto-generation features for development workflows.

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_RUNTIME_DAEMON_ENABLED` | `false` | Enable runtime daemon |
| `MOCKFORGE_RUNTIME_DAEMON_AUTO_CREATE_ON_404` | `true` | Auto-create mocks on 404 |
| `MOCKFORGE_RUNTIME_DAEMON_AI_GENERATION` | `false` | Enable AI mock generation |
| `MOCKFORGE_RUNTIME_DAEMON_GENERATE_TYPES` | `false` | Generate TypeScript types |
| `MOCKFORGE_RUNTIME_DAEMON_GENERATE_CLIENT_STUBS` | `false` | Generate client stubs |
| `MOCKFORGE_RUNTIME_DAEMON_UPDATE_OPENAPI` | `false` | Auto-update OpenAPI spec |
| `MOCKFORGE_RUNTIME_DAEMON_CREATE_SCENARIO` | `false` | Auto-create scenarios |
| `MOCKFORGE_RUNTIME_DAEMON_WORKSPACE_DIR` | None | Workspace directory |
| `MOCKFORGE_RUNTIME_DAEMON_EXCLUDE_PATTERNS` | `/health,/metrics,/__mockforge` | Comma-separated exclusion patterns |

---

## Notifications (Slack)

| Variable | Default | Description |
|----------|---------|-------------|
| `SLACK_METHOD` | `disabled` | Slack integration method |
| `SLACK_WEBHOOK_URL` | None | Slack webhook URL |
| `SLACK_BOT_TOKEN` | None | Slack bot token |
| `SLACK_DEFAULT_CHANNEL` | None | Default Slack channel |
| `SLACK_CHANNEL` | None | Alternative Slack channel var |

---

## Kubernetes Operator

| Variable | Default | Description |
|----------|---------|-------------|
| `WATCH_NAMESPACE` | None | Kubernetes namespace to watch |

---

## Docker Detection

| Variable | Description |
|----------|-------------|
| `DOCKER_CONTAINER` | Set to indicate running in Docker (affects defaults) |
| `container` | Alternative Docker detection variable |

---

## Example .env File

```bash
# Required for Registry Server
DATABASE_URL=postgres://mockforge:password@localhost:5432/mockforge
JWT_SECRET=your-secure-jwt-secret-at-least-32-characters

# Server Configuration
PORT=8080
MOCKFORGE_HTTP_PORT=3000

# Optional: Redis for caching
REDIS_URL=redis://localhost:6379

# Optional: S3 for plugin storage
S3_BUCKET=my-plugins-bucket
S3_REGION=us-west-2
AWS_ACCESS_KEY_ID=your-access-key
AWS_SECRET_ACCESS_KEY=your-secret-key

# Optional: AI Features
MOCKFORGE_RAG_PROVIDER=openai
OPENAI_API_KEY=sk-your-openai-key

# Optional: Email
EMAIL_PROVIDER=smtp
SMTP_HOST=smtp.example.com
SMTP_PORT=587
SMTP_USERNAME=user
SMTP_PASSWORD=pass

# Logging
MOCKFORGE_LOG_LEVEL=info
RUST_LOG=info
```

---

*Last Updated: 2024-12-27*
