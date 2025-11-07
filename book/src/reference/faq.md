# Frequently Asked Questions (FAQ)

Quick answers to common questions about MockForge.

## General Questions

### What is MockForge?

MockForge is a comprehensive multi-protocol mocking framework for APIs. It allows you to create realistic mock servers for HTTP/REST, gRPC, WebSocket, and GraphQL without writing code. Perfect for frontend development, integration testing, and parallel team development.

### Is MockForge free?

Yes, MockForge is completely free and open-source under MIT/Apache-2.0 licenses. There are no premium tiers, paid features, or usage limits.

### What protocols does MockForge support?

MockForge supports:
- **HTTP/REST**: OpenAPI/Swagger-based mocking with full validation
- **gRPC**: Dynamic service discovery from `.proto` files with HTTP Bridge
- **WebSocket**: Replay mode, interactive mode, and AI event generation
- **GraphQL**: Schema-based mocking with automatic resolver generation

### How does MockForge compare to WireMock, Mockoon, or MockServer?

See our [detailed comparison table](https://github.com/SaaSy-Solutions/mockforge#-why-mockforge) in the README. Key differentiators:
- **Multi-protocol** in a single binary
- **AI-powered** mock generation and data drift
- **WASM plugin system** for extensibility
- **gRPC HTTP Bridge** for REST access to gRPC services
- **Built-in encryption** for sensitive data
- **Rust performance** with native compilation
- **Multi-language SDKs** - Native support for 6 languages vs WireMock's Java-first approach

For detailed ecosystem comparison, see [Ecosystem Comparison Guide](../../docs/ECOSYSTEM_COMPARISON.md).

### Can I use MockForge in production?

Yes! MockForge is production-ready with:
- Comprehensive test coverage
- Security audits
- Performance benchmarks
- Docker deployment support
- Observability (Prometheus metrics, tracing)

However, it's primarily designed for **development and testing**. For production API mocking, ensure proper security configurations.

---

## Getting Started

### How do I install MockForge?

Three options:

```bash
# 1. From crates.io (requires Rust)
cargo install mockforge-cli

# 2. From source
git clone https://github.com/SaaSy-Solutions/mockforge
cd mockforge && make setup && make install

# 3. Using Docker
docker pull ghcr.io/saasy-solutions/mockforge:latest
```

See the [Installation Guide](../getting-started/installation.md) for details.

### What's the fastest way to get started?

Follow our **[5-Minute Tutorial](../getting-started/five-minute-api.md)**:

1. `cargo install mockforge-cli`
2. `mockforge init my-project`
3. `mockforge serve --config mockforge.yaml`
4. Test with `curl`

### Do I need to know Rust to use MockForge?

**No.** MockForge is a CLI tool you can use without Rust knowledge. You only need Rust if:
- Building from source
- Developing custom plugins
- Embedding MockForge as a library

### What programming languages are supported?

MockForge provides native SDKs for 6 languages:
- **Rust** - Native SDK with zero-overhead embedding
- **Node.js/TypeScript** - Full TypeScript support
- **Python** - Context manager support with type hints
- **Go** - Idiomatic Go API
- **Java** - Maven/Gradle integration
- **.NET/C#** - NuGet package

All SDKs support embedded mock servers in your test suites. See [SDK Documentation](../../sdk/README.md) for examples.

### Can I use MockForge from Python/Node.js/Go/etc.?

Yes! MockForge provides native SDKs for multiple languages. You can embed mock servers directly in your test code:

**Python**:
```python
from mockforge_sdk import MockServer

with MockServer(port=3000) as server:
    server.stub_response('GET', '/api/users/123', {'id': 123})
    # Your test code here
```

**Node.js**:
```typescript
import { MockServer } from '@mockforge/sdk';

const server = await MockServer.start({ port: 3000 });
await server.stubResponse('GET', '/api/users/123', { id: 123 });
```

**Go**:
```go
server := mockforge.NewMockServer(mockforge.MockServerConfig{Port: 3000})
server.Start()
defer server.Stop()
```

See [Ecosystem & Use Cases Guide](../../docs/ECOSYSTEM_AND_USE_CASES.md) for complete examples in all languages.

### How do I create my first mock API?

```bash
# 1. Initialize a project
mockforge init my-api

# 2. Edit the generated mockforge.yaml
vim mockforge.yaml

# 3. Start the server
mockforge serve --config mockforge.yaml

# 4. Test it
curl http://localhost:3000/your-endpoint
```

Or use an existing OpenAPI spec:

```bash
mockforge serve --spec your-api.json
```

---

## Configuration & Setup

### How do I configure MockForge?

Three ways (in order of priority):

1. **CLI flags**: `mockforge serve --http-port 3000`
2. **Environment variables**: `export MOCKFORGE_HTTP_PORT=3000`
3. **Config file**: `mockforge serve --config config.yaml`

See the [Configuration Guide](../configuration/files.md) and [Complete Config Template](https://github.com/SaaSy-Solutions/mockforge/blob/main/config.template.yaml).

### Where should I put my configuration file?

MockForge looks for config files in this order:

1. Path specified with `--config`
2. `MOCKFORGE_CONFIG_FILE` environment variable
3. `./mockforge.yaml` or `./mockforge.yml` in current directory
4. Auto-discovered in parent directories

### Can I use environment variables for all settings?

Yes! Every config option can be set via environment variables using the `MOCKFORGE_` prefix:

```bash
export MOCKFORGE_HTTP_PORT=3000
export MOCKFORGE_ADMIN_ENABLED=true
export MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true
```

### How do I validate my configuration?

```bash
mockforge config validate
mockforge config validate --config my-config.yaml
```

See the [Configuration Validation Guide](config-validation.md).

---

## OpenAPI & HTTP Mocking

### Can I use Swagger/OpenAPI specs?

Yes! Both OpenAPI 3.0 and Swagger 2.0 are supported:

```bash
mockforge serve --spec openapi.json
mockforge serve --spec swagger.yaml
```

MockForge automatically generates mock endpoints from your specification.

### How does request validation work?

Three modes:

- **`off`**: No validation (accept all requests)
- **`warn`**: Log validation errors but accept requests
- **`enforce`**: Reject invalid requests with 400/422

```bash
mockforge serve --validation enforce --spec api.json
```

### Why aren't my template variables working?

Template expansion must be **explicitly enabled**:

```bash
# Via CLI
mockforge serve --response-template-expand

# Via environment
export MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true

# Via config
http:
  response_template_expand: true
```

This is a security feature to prevent accidental template processing.

### What template variables are available?

```
{{uuid}}          - Random UUID v4
{{now}}           - Current timestamp (ISO 8601)
{{now+2h}}        - Timestamp 2 hours from now
{{now-30m}}       - Timestamp 30 minutes ago
{{randInt 1 100}} - Random integer 1-100
{{rand.float}}    - Random float
{{faker.email}}   - Fake email address
{{faker.name}}    - Fake person name
{{request.body.field}}   - Access request data
{{request.path.id}}      - Path parameters
{{request.header.Auth}}  - Request headers
```

See the [Templating Reference](templating.md) for complete details.

### Can I override specific endpoints?

Yes! Define custom routes in your config that override OpenAPI spec:

```yaml
http:
  openapi_spec: api.json
  routes:
    - path: /custom/endpoint
      method: GET
      response:
        status: 200
        body: '{"custom": "response"}'
```

---

## gRPC Mocking

### Do I need to compile my proto files?

**No.** MockForge dynamically parses `.proto` files at runtime. Just:

1. Put `.proto` files in `./proto` directory
2. Start MockForge: `mockforge serve --grpc-port 50051`
3. Services are automatically discovered and mocked

### How do I access gRPC services via HTTP?

Enable the **HTTP Bridge**:

```yaml
grpc:
  dynamic:
    enabled: true
    http_bridge:
      enabled: true
      base_path: "/api"
```

Now access gRPC services as REST APIs:
```bash
# gRPC
grpcurl -d '{"id": "123"}' localhost:50051 UserService/GetUser

# HTTP (via bridge)
curl -X POST http://localhost:8080/api/userservice/getuser \
  -d '{"id": "123"}'
```

### Can I use gRPC reflection?

Yes, it's enabled by default:

```bash
# List services
grpcurl -plaintext localhost:50051 list

# Describe a service
grpcurl -plaintext localhost:50051 describe UserService
```

### Does MockForge support gRPC streaming?

Yes, all four streaming modes:
- Unary (single request → single response)
- Server streaming (single request → stream of responses)
- Client streaming (stream of requests → single response)
- Bidirectional streaming (stream ↔ stream)

---

## WebSocket Mocking

### How do I create WebSocket replay files?

Use JSON Lines (JSONL) format:

```json
{"ts":0,"dir":"out","text":"Welcome!","waitFor":"^CLIENT_READY$"}
{"ts":100,"dir":"out","text":"{{uuid}}"}
{"ts":200,"dir":"in","text":"ACK"}
```

- `ts`: Milliseconds timestamp
- `dir`: "in" (received) or "out" (sent)
- `text`: Message content (supports templates)
- `waitFor`: Optional regex/JSONPath pattern

See [WebSocket Replay Mode](../user-guide/websocket-mocking/replay.md).

### Can I match JSON messages?

Yes, use JSONPath in `waitFor`:

```json
{"waitFor": "$.type", "text": "Matched type field"}
{"waitFor": "$.user.id", "text": "Matched user ID"}
```

See [README-websocket-jsonpath.md](https://github.com/SaaSy-Solutions/mockforge/blob/main/examples/README-websocket-jsonpath.md).

### What's AI event generation?

Generate realistic WebSocket event streams from narrative descriptions:

```bash
mockforge serve --ws-ai-enabled \
  --ws-ai-narrative "Simulate 5 minutes of stock trading" \
  --ws-ai-event-count 20
```

Perfect for testing real-time features without manually scripting events.

---

## AI Features

### Do I need an API key for AI features?

Not necessarily. Three options:

1. **Ollama (Free, Local)**: No API key needed
   ```bash
   ollama pull llama2
   mockforge serve --ai-enabled --rag-provider ollama
   ```

2. **OpenAI (Paid)**: ~$0.01 per 1,000 requests
   ```bash
   export MOCKFORGE_RAG_API_KEY=sk-...
   mockforge serve --ai-enabled --rag-provider openai
   ```

3. **Anthropic, or OpenAI-compatible APIs**: Similar to OpenAI

### What are AI features used for?

- **Intelligent Mock Generation**: Generate responses from natural language prompts
- **Data Drift Simulation**: Realistic data evolution (order status, stock levels, etc.)
- **AI Event Streams**: Generate WebSocket event sequences from narratives

See [AI_DRIVEN_MOCKING.md](https://github.com/SaaSy-Solutions/mockforge/blob/main/docs/AI_DRIVEN_MOCKING.md).

### How much does AI cost?

- **Ollama**: Free (runs locally)
- **OpenAI GPT-3.5**: ~$0.01 per 1,000 requests
- **OpenAI GPT-4**: ~$0.10 per 1,000 requests
- **Anthropic Claude**: Similar to GPT-4

Use Ollama for development, OpenAI for production if needed.

---

## Plugins

### How do I install plugins?

```bash
# From URL
mockforge plugin install https://example.com/plugin.wasm

# From Git with version
mockforge plugin install https://github.com/user/plugin#v1.0.0

# From local file
mockforge plugin install ./my-plugin.wasm

# List installed
mockforge plugin list
```

### Can I create custom plugins?

Yes! Plugins are written in Rust and compiled to WebAssembly:

1. Use `mockforge-plugin-sdk` crate
2. Implement plugin traits
3. Compile to WASM target
4. Install and use

See the [Plugin Development Guide](../user-guide/plugins.md) and [Add a Custom Plugin Tutorial](../tutorials/add-custom-plugin.md).

### Are plugins sandboxed?

Yes. Plugins run in a **WebAssembly sandbox** with:
- Memory isolation
- CPU/memory limits
- No network access (unless explicitly allowed)
- No file system access (unless explicitly allowed)

See [Plugin Security Model](../../docs/plugins/security/model.md).

---

## Admin UI

### How do I access the Admin UI?

Two modes:

**Standalone** (separate port):
```bash
mockforge serve --admin --admin-port 9080
# Access: http://localhost:9080
```

**Embedded** (under HTTP server):
```bash
mockforge serve --admin-embed --admin-mount-path /admin
# Access: http://localhost:3000/admin
```

### Is authentication available?

**Not yet.** Role-based authentication (Admin/Viewer) is planned for v1.1. The frontend UI components are built, but backend JWT/OAuth integration is pending.

Currently, the Admin UI is accessible without authentication.

### What can I do in the Admin UI?

- View real-time request logs (via Server-Sent Events)
- Monitor performance metrics
- Manage fixtures with drag-and-drop
- Configure latency and fault injection
- Search requests and logs
- View server health and statistics

See [Admin UI Walkthrough](../tutorials/admin-ui-walkthrough.md).

---

## Deployment

### Can I run MockForge in Docker?

Yes:

```bash
# Using Docker Compose
docker-compose up

# Using Docker directly
docker run -p 3000:3000 -p 9080:9080 mockforge
```

See [DOCKER.md](https://github.com/SaaSy-Solutions/mockforge/blob/main/DOCKER.md) for complete documentation.

### How do I deploy to Kubernetes?

Use the Helm chart or create Deployment/Service manifests:

```bash
# Using Helm (if available)
helm install mockforge ./charts/mockforge

# Or use kubectl
kubectl apply -f k8s/deployment.yaml
```

### What ports does MockForge use?

Default ports:
- **3000**: HTTP server
- **3001**: WebSocket server
- **50051**: gRPC server
- **4000**: GraphQL server
- **9080**: Admin UI
- **9090**: Prometheus metrics

All ports are configurable.

---

## Performance & Limits

### How many requests can MockForge handle?

Typical performance (modern hardware):
- **HTTP**: 10,000+ req/s
- **WebSocket**: 1,000+ concurrent connections
- **gRPC**: 5,000+ req/s

Performance depends on:
- Response complexity
- Template expansion
- Validation enabled
- Hardware specs

See our [benchmarks](https://github.com/SaaSy-Solutions/mockforge/tree/main/benches).

### Does MockForge scale horizontally?

Yes. Run multiple instances behind a load balancer:

```bash
# Instance 1
mockforge serve --http-port 3000

# Instance 2
mockforge serve --http-port 3001

# Load balancer distributes traffic
```

For stateless mocking (no shared state), this works great.

### What are the resource requirements?

Minimal:
- **Memory**: ~50MB base + ~10MB per 1,000 concurrent connections
- **CPU**: 1-2 cores sufficient for most workloads
- **Disk**: ~100MB for binary + storage for logs/fixtures

---

## Troubleshooting

### Server won't start - port already in use

```bash
# Find what's using the port
lsof -i :3000

# Use a different port
mockforge serve --http-port 3001
```

### Template variables appear literally in responses

Enable template expansion:
```bash
mockforge serve --response-template-expand
```

### Validation rejecting valid requests

Adjust validation mode:
```bash
mockforge serve --validation warn  # or 'off'
```

### WebSocket connection fails

Check the WebSocket port and replay file:
```bash
# Verify port
netstat -tlnp | grep :3001

# Check replay file exists
ls -la ws-replay.jsonl
```

### Admin UI not loading

Verify the admin UI is enabled and port is correct:
```bash
mockforge serve --admin --admin-port 9080
curl http://localhost:9080
```

For more issues, see the [Troubleshooting Guide](troubleshooting.md).

---

## Development & Contributing

### Can I embed MockForge in my application?

Yes! Use MockForge crates as libraries:

```rust
use mockforge_http::build_router;
use mockforge_core::{ValidationOptions, Config};

let router = build_router(
    Some("api.json".to_string()),
    Some(ValidationOptions::enforce()),
    None,
).await;
```

See the [Rust API Documentation](https://docs.rs/mockforge-core).

### How do I contribute to MockForge?

1. Check [CONTRIBUTING.md](https://github.com/SaaSy-Solutions/mockforge/blob/main/CONTRIBUTING.md)
2. Look for "good first issue" labels
3. Fork, make changes, submit PR
4. Ensure tests pass: `cargo test`
5. Follow code style: `cargo fmt && cargo clippy`

### Where can I report bugs?

[GitHub Issues](https://github.com/SaaSy-Solutions/mockforge/issues)

Please include:
- MockForge version
- Operating system
- Configuration file (if applicable)
- Steps to reproduce
- Expected vs actual behavior
- Error logs

### Is there a community forum?

- **GitHub Discussions**: [Community Forum](https://github.com/SaaSy-Solutions/mockforge/discussions)
- **GitHub Issues**: [Bug Reports & Feature Requests](https://github.com/SaaSy-Solutions/mockforge/issues)
- **Discord**: [Join our community chat](https://discord.gg/2FxXqKpa)

---

## Licensing & Commercial Use

### What license is MockForge under?

Dual-licensed: **MIT OR Apache-2.0**

You can choose either license for your use case.

### Can I use MockForge commercially?

**Yes, absolutely.** Both MIT and Apache-2.0 are permissive licenses that allow commercial use without restrictions.

### Do I need to open-source my configurations?

**No.** Your configuration files, mock data, and custom plugins are yours. Only if you modify MockForge source code and distribute it do licensing terms apply.

### Can I sell MockForge-based services?

Yes. You can offer:
- Hosted MockForge instances
- Custom plugins
- Support services
- Training/consulting

---

## Use Cases

### What use cases does MockForge support?

MockForge supports a wide range of use cases:

1. **Unit Tests** - Embed mock servers directly in test suites across all supported languages
2. **Integration Tests** - Test complex multi-service interactions with stateful mocking
3. **Service Virtualization** - Replace external dependencies with mocks using proxy mode
4. **Development Environments** - Create local development environments without backend dependencies
5. **Isolating from Flaky Dependencies** - Simulate network failures and slow responses
6. **Simulating APIs That Don't Exist Yet** - Generate mocks from API specifications before implementation

See [Ecosystem & Use Cases Guide](../../docs/ECOSYSTEM_AND_USE_CASES.md) for detailed examples and code samples.

### Can I use MockForge for unit testing?

Yes! MockForge SDKs allow you to embed mock servers directly in your unit tests:

**Rust**:
```rust
let mut server = MockServer::new().port(0).start().await?;
server.stub_response("GET", "/api/users/123", json!({"id": 123})).await?;
```

**Python**:
```python
with MockServer(port=0) as server:
    server.stub_response('GET', '/api/users/123', {'id': 123})
```

No separate server process required. See [SDK Documentation](../../sdk/README.md) for examples.

### How do I replace external APIs in my tests?

Use MockForge's proxy mode with record/replay:

```bash
# Record real API interactions
mockforge serve --proxy-enabled \
  --proxy-target https://api.external-service.com \
  --record-responses ./recordings/

# Replay from recordings
mockforge serve --replay-from ./recordings/
```

Or use the SDK to programmatically stub responses. See [Service Virtualization](../../docs/ECOSYSTEM_AND_USE_CASES.md#use-case-3-service-virtualization) for details.

### Can I simulate network failures and slow responses?

Yes! MockForge provides built-in latency and fault injection:

```bash
# Add latency
mockforge serve --latency-mode normal --latency-mean-ms 500

# Inject failures
mockforge serve --failure-rate 0.1 --failure-codes 500,503
```

Or configure in your SDK:
```typescript
const server = await MockServer.start({
  latency: { mode: 'normal', meanMs: 500 },
  failures: { enabled: true, failureRate: 0.1 }
});
```

See [Isolating from Flaky Dependencies](../../docs/ECOSYSTEM_AND_USE_CASES.md#use-case-5-isolating-from-flaky-dependencies) for examples.

### How do I mock an API that doesn't exist yet?

Generate mocks from API specifications:

```bash
# From OpenAPI spec
mockforge serve --spec api-spec.yaml

# From GraphQL schema
mockforge serve --graphql-schema schema.graphql

# From gRPC proto files
mockforge serve --grpc-port 50051 --proto-dir ./proto
```

All endpoints are automatically available with schema-validated responses. See [Simulating APIs That Don't Exist Yet](../../docs/ECOSYSTEM_AND_USE_CASES.md#use-case-6-simulating-apis-that-dont-exist-yet) for details.

## What's Next?

**Ready to start?** Try our **[5-Minute Tutorial](../getting-started/five-minute-api.md)**!

**Need more help?**
- [Full Documentation](https://docs.mockforge.dev/)
- [Ecosystem & Use Cases Guide](../../docs/ECOSYSTEM_AND_USE_CASES.md)
- [GitHub Issues](https://github.com/SaaSy-Solutions/mockforge/issues)
- [Community Discussions](https://github.com/SaaSy-Solutions/mockforge/discussions)
