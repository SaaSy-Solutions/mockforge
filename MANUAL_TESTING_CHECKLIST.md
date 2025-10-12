# MockForge Manual Testing Checklist

## 🤖 Automation Status

**Automated Testing Framework Available**: Run `./scripts/run-automated-tests.sh` to execute automated tests covering ~70% of this checklist.

- ✅ **Fully Automated**: Installation & Setup, HTTP/REST Server, CLI Commands, Core Features (Chaos Engineering), Data Generation, Environment Variables, Docker Testing
- 🔄 **Partially Automated**: Other sections have placeholder scripts ready for implementation
- 📋 **Manual Only**: Advanced features requiring human judgment (AI, plugins, security audits, etc.)

**Note**: Automated tests focus on core functionality and CI/CD integration. Manual testing remains essential for comprehensive validation.

---

## 🚀 Installation & Setup

- [x] **Build from Source (Pre-Release)**
  - [x] Clone repository: `git clone https://github.com/SaaSy-Solutions/mockforge.git && cd mockforge`
  - [x] Setup development environment: `make setup`
  - [x] Build project: `make build`
  - [x] Install locally: `make install` (installs to `~/.cargo/bin/mockforge`)
  - [x] Verify installation: `mockforge --version`
  - [x] Alternative: Run directly with `cargo run -p mockforge-cli -- --version`

- [ ] **Docker Installation**
  - [x] Build Docker image: `docker build -t mockforge .`
  - [x] Run with Docker Compose: `make docker-compose-up`
  - [x] Verify all ports are accessible (3000, 3001, 50051, 9080)
  - [x] Test single container: `docker run -p 3000:3000 -p 9080:9080 mockforge`

- [ ] **Configuration**
  - [x] Initialize new project: `mockforge init my-project`
  - [x] Validate configuration: `mockforge config validate`
  - [ ] Test with demo config: `mockforge serve --config examples/advanced-config.yaml`
  - [ ] Test with minimal config: `mockforge serve --admin`

---

## 🎉 Post-Release Testing (After Publishing to crates.io)

> **Note:** Complete this section AFTER running `cargo publish` to verify the published version works correctly.

- [ ] **Install from crates.io**
  - [ ] Install latest version: `cargo install mockforge-cli`
  - [ ] Verify correct version: `mockforge --version`
  - [ ] Check installation location: `which mockforge`
  - [ ] Uninstall and reinstall: `cargo uninstall mockforge-cli && cargo install mockforge-cli`

- [ ] **Install Specific Version**
  - [ ] Install specific version: `cargo install mockforge-cli --version 1.0.0`
  - [ ] Verify version matches: `mockforge --version`

- [ ] **Fresh System Test**
  - [ ] Test on a clean system without prior MockForge installation
  - [ ] Verify no leftover configuration files interfere
  - [ ] Test first-run experience: `mockforge serve --admin`

- [ ] **Update Testing**
  - [ ] Install old version (if available): `cargo install mockforge-cli --version 0.9.0`
  - [ ] Upgrade to latest: `cargo install mockforge-cli --force`
  - [ ] Verify configuration migration (if applicable)

- [ ] **Platform-Specific Installation**
  - [ ] Test on Linux (x86_64)
  - [ ] Test on macOS (Intel)
  - [ ] Test on macOS (Apple Silicon)
  - [ ] Test on Windows (if supported)

- [ ] **Docker Hub Release** (if applicable)
  - [ ] Pull from Docker Hub: `docker pull mockforge/mockforge:latest`
  - [ ] Pull specific version: `docker pull mockforge/mockforge:1.0.0`
  - [ ] Verify image runs: `docker run -p 3000:3000 mockforge/mockforge:latest`

- [ ] **Documentation Verification**
  - [ ] Verify crates.io page displays correctly
  - [ ] Check README renders properly
  - [ ] Verify documentation links work
  - [ ] Check examples in crates.io docs

- [ ] **Integration with Other Tools**
  - [ ] Test installation alongside other Rust CLI tools
  - [ ] Verify no binary name conflicts
  - [ ] Test shell completion (if provided)

---

## 📡 HTTP/REST Server

### Basic HTTP Operations
- [ ] **Server Startup**
  - [ ] Start HTTP server: `mockforge serve --http-port 3000`
  - [ ] Verify server responds: `curl http://localhost:3000/ping`
  - [ ] Test custom port: `mockforge serve --http-port 8080`
  - [ ] Test host binding: `mockforge serve --host 127.0.0.1`

- [ ] **OpenAPI Integration**
  - [ ] Load OpenAPI spec: `mockforge serve --spec examples/openapi-demo.json`
  - [ ] Verify all paths are registered
  - [ ] Test auto-generated mock responses
  - [ ] Validate request/response against schema

### Request Validation
- [ ] **Validation Modes**
  - [ ] Test `enforce` mode (invalid requests get 400/422)
  - [ ] Test `warn` mode (logs warning, allows request)
  - [ ] Test `off` mode (no validation)
  - [ ] Verify aggregated error messages with `--aggregate-errors`

- [ ] **Validation Overrides**
  - [ ] Configure per-endpoint validation in config
  - [ ] Test validation override for specific routes
  - [ ] Verify `skip_admin_validation: true` works

### Template Expansion
- [ ] **Basic Templates** (set `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true`)
  - [ ] `{{uuid}}` - generates random UUID
  - [ ] `{{now}}` - current RFC3339 timestamp
  - [ ] `{{now+2h}}` - timestamp 2 hours in future
  - [ ] `{{now-30m}}` - timestamp 30 minutes in past
  - [ ] `{{rand.int}}` - random integer
  - [ ] `{{rand.float}}` - random float
  - [ ] `{{randInt 10 99}}` - ranged random integer

- [ ] **Faker Functions**
  - [ ] `{{faker.uuid}}`
  - [ ] `{{faker.email}}`
  - [ ] `{{faker.name}}`
  - [ ] `{{faker.address}}`
  - [ ] `{{faker.phone}}`
  - [ ] `{{faker.company}}`
  - [ ] `{{faker.url}}`
  - [ ] `{{faker.ip}}`
  - [ ] `{{faker.word}}`
  - [ ] `{{faker.sentence}}`
  - [ ] `{{faker.paragraph}}`

### CORS Configuration
- [ ] Enable CORS: `cors_enabled: true`
- [ ] Test cross-origin requests from browser
- [ ] Verify allowed origins configuration
- [ ] Test preflight OPTIONS requests
- [ ] Verify allowed methods and headers

### Custom Routes
- [ ] Define custom routes in config
- [ ] Override OpenAPI-defined routes
- [ ] Test custom response bodies with templates
- [ ] Verify custom headers

---

## 🌐 WebSocket Server

### Basic WebSocket Operations
- [ ] **Server Startup**
  - [ ] Start WebSocket server: `mockforge serve --ws-port 3001`
  - [ ] Set replay file: `MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl`
  - [ ] Verify connection: `websocat ws://localhost:3001/ws`

- [ ] **Scripted Replay**
  - [ ] Test basic message replay from JSONL file
  - [ ] Verify timestamp-based message timing
  - [ ] Test bidirectional messages (in/out)
  - [ ] Verify `waitFor` regex pattern matching

### JSONPath Message Matching
- [ ] **JSONPath Queries**
  - [ ] Test `$.type` - wait for type property
  - [ ] Test `$.user.id` - nested path matching
  - [ ] Test `$.order.status` - order status matching
  - [ ] Test `$.items[0].name` - array element matching

- [ ] **JSON Message Testing**
  - [ ] Send JSON message: `{"type": "login"}`
  - [ ] Verify server responds based on JSONPath match
  - [ ] Test complex nested JSON structures

### Connection Management
- [ ] Test max concurrent connections limit
- [ ] Verify connection timeout behavior
- [ ] Test message timeout (30s default)
- [ ] Verify heartbeat interval for long connections
- [ ] Test per-message compression
- [ ] Test max message size limit (16MB default)

---

## 🔌 gRPC Server

### Basic gRPC Operations
- [ ] **Server Startup**
  - [ ] Start gRPC server: `mockforge serve --grpc-port 50051`
  - [ ] Load proto files from directory
  - [ ] Enable reflection: `--enable-reflection`
  - [ ] Test unary RPC calls

- [ ] **gRPC Method Types**
  - [ ] Unary RPC: `grpcurl -plaintext -proto proto/greeter.proto -d '{"name":"Test"}' localhost:50051 Greeter/SayHello`
  - [ ] Client streaming RPC
  - [ ] Server streaming RPC
  - [ ] Bidirectional streaming RPC

### gRPC HTTP Bridge
- [ ] **Bridge Activation**
  - [ ] Enable HTTP bridge in config
  - [ ] Verify bridge starts on correct port
  - [ ] Access bridge health: `GET /api/health`
  - [ ] View available services: `GET /api/services`

- [ ] **REST Endpoint Generation**
  - [ ] Verify auto-generated endpoints: `/api/{service}/{method}`
  - [ ] Test POST request to gRPC method via HTTP
  - [ ] Verify JSON → Protobuf conversion
  - [ ] Verify Protobuf → JSON response conversion

- [ ] **OpenAPI Documentation**
  - [ ] Access OpenAPI docs: `GET /api/docs`
  - [ ] Verify schema generation from protobuf
  - [ ] Test example requests/responses
  - [ ] Verify streaming method documentation

- [ ] **Server-Sent Events (SSE)**
  - [ ] Test server streaming via SSE
  - [ ] Verify event format
  - [ ] Test bidirectional streaming via WebSocket-like behavior

- [ ] **Bridge Statistics**
  - [ ] View stats: `GET /api/stats`
  - [ ] Verify request counts
  - [ ] Check latency metrics
  - [ ] Verify failure rates

---

## 🎨 GraphQL Server (if enabled)

- [ ] Enable GraphQL in config
- [ ] Start GraphQL server on port 4000
- [ ] Load GraphQL schema file
- [ ] Access GraphQL Playground UI
- [ ] Test queries
- [ ] Test mutations
- [ ] Test subscriptions

---

## 📧 SMTP Email Testing

### Basic SMTP Operations
- [ ] **Server Startup**
  - [ ] Start SMTP server: `mockforge serve --smtp --smtp-port 1025`
  - [ ] Verify server accepts connections

- [ ] **RFC 5321 Commands**
  - [ ] Test HELO command
  - [ ] Test EHLO command
  - [ ] Test MAIL FROM
  - [ ] Test RCPT TO
  - [ ] Test DATA
  - [ ] Test QUIT
  - [ ] Test RSET
  - [ ] Test NOOP
  - [ ] Test HELP

### Email Fixtures
- [ ] Configure fixture-based email matching
- [ ] Test regex patterns for recipients
- [ ] Test regex patterns for senders
- [ ] Test regex patterns for subjects
- [ ] Configure auto-reply responses

### Mailbox Features
- [ ] Enable in-memory mailbox
- [ ] Test max mailbox messages limit
- [ ] Export emails to file
- [ ] Verify template expansion in email bodies

---

## 🎯 Data Generation

### Built-in Templates
- [ ] **User Template**
  - [ ] Generate 100 users: `mockforge data template user --rows 100 --output users.json`
  - [ ] Test JSON format
  - [ ] Test CSV format: `--format csv`
  - [ ] Test JSONL format: `--format jsonl`

- [ ] **Product Template**
  - [ ] Generate products: `mockforge data template product --rows 50`
  - [ ] Verify realistic product data

- [ ] **Order Template**
  - [ ] Generate orders: `mockforge data template order --rows 200`
  - [ ] Verify referential integrity with users/products

### Custom Schema Generation
- [ ] Generate from JSON schema: `mockforge data schema schema.json --rows 200`
- [ ] Test complex nested schemas
- [ ] Verify relationship support

### RAG-Powered Generation
- [ ] **With Ollama (free)**
  - [ ] Install Ollama: `ollama pull llama2`
  - [ ] Generate with RAG: `mockforge data template user --rows 100 --rag --rag-provider ollama`

- [ ] **With OpenAI**
  - [ ] Set API key: `export MOCKFORGE_RAG_API_KEY=sk-...`
  - [ ] Generate: `mockforge data template user --rag --rag-provider openai`

---

## 🧠 AI-Powered Features

### Intelligent Mock Generation
- [ ] **Setup**
  - [ ] Configure AI provider (Ollama/OpenAI/Anthropic)
  - [ ] Test with free Ollama: `--rag-provider ollama --rag-model llama2`
  - [ ] Test with OpenAI: `--rag-provider openai --rag-model gpt-3.5-turbo`

- [ ] **Intelligent Responses**
  - [ ] Test natural language prompt: "Generate realistic customer data for SaaS platform"
  - [ ] Verify schema-aware generation
  - [ ] Test response caching

### Data Drift Simulation
- [ ] **Order Status Progression**
  - [ ] Configure drift in config
  - [ ] Verify status transitions: pending → processing → shipped → delivered
  - [ ] Test state machine transitions

- [ ] **Stock Depletion**
  - [ ] Configure stock drift rules
  - [ ] Verify quantities decrease with purchases

- [ ] **Price Fluctuation**
  - [ ] Configure price drift
  - [ ] Verify realistic price changes over time

- [ ] **Test Drift**
  - [ ] Run drift test: `mockforge test-ai drift --initial-data examples/order.json --iterations 10`

### AI Event Streams (WebSocket)
- [ ] **Narrative-Driven Events**
  - [ ] Test event stream generation: `mockforge test-ai event-stream --narrative "5 minutes of stock market data" --event-count 20`
  - [ ] Configure in WebSocket response config
  - [ ] Verify time-based strategies
  - [ ] Test count-based strategies
  - [ ] Test conditional event triggers

---

## 🔌 Plugin System

### Plugin Management
- [ ] **Installation**
  - [ ] Install from URL: `mockforge plugin install https://github.com/user/plugin#v1.0.0`
  - [ ] Install from Git repo
  - [ ] Install from local file
  - [ ] Install from ZIP archive
  - [ ] Verify checksum validation

- [ ] **Plugin Types**
  - [ ] Test custom response generator plugin
  - [ ] Test authentication provider (JWT/OAuth2)
  - [ ] Test data source connector (CSV/database)
  - [ ] Test template extension plugin

### Plugin Security
- [ ] Verify WASM sandbox isolation
- [ ] Test memory limits (50MB default)
- [ ] Test execution timeout (1000ms default)
- [ ] Verify network access controls
- [ ] Verify file access controls

### Example Plugins
- [ ] Test `auth-jwt` plugin
- [ ] Test `auth-basic` plugin
- [ ] Test `template-crypto` plugin
- [ ] Test `datasource-csv` plugin
- [ ] Test `response-graphql` plugin

---

## 🔒 Security & Encryption

### End-to-End Encryption
- [ ] **Enable Encryption**
  - [ ] Configure algorithm: AES-256-GCM or ChaCha20-Poly1305
  - [ ] Generate encryption key
  - [ ] Enable auto-encryption for sensitive fields

- [ ] **Template Functions**
  - [ ] Test encryption in templates
  - [ ] Test decryption in templates
  - [ ] Verify secure key storage

### Workspace Sync Encryption
- [ ] Test encrypt-on-push to Git
- [ ] Test decrypt-on-pull from Git
- [ ] Verify encrypted files in repository

---

## 📁 Workspace Synchronization

### File System Watching
- [ ] **Start Sync Daemon**
  - [ ] Start sync: `mockforge sync start --directory ./workspace-sync`
  - [ ] Create/modify file in directory
  - [ ] Verify real-time sync
  - [ ] Test debounce delay (1000ms default)

### Git Integration
- [ ] Enable Git sync in config
- [ ] Test auto-commit on file changes
- [ ] Test auto-push to remote (if enabled)
- [ ] Test branch configuration
- [ ] Verify conflict resolution

---

## 🎛️ Admin UI

### Standalone Mode
- [ ] **Start Admin UI**
  - [ ] Start standalone: `mockforge serve --admin --admin-port 9080`
  - [ ] Access: `http://localhost:9080/`
  - [ ] Verify dashboard loads

### Embedded Mode
- [ ] **Embed Under HTTP Server**
  - [ ] Configure `mount_path: "/admin"` in config
  - [ ] Start server: `mockforge serve`
  - [ ] Access: `http://localhost:3000/admin/`
  - [ ] Verify static assets load correctly

### Admin Features
- [ ] **Dashboard**
  - [ ] View real-time server status
  - [ ] View live logs (via SSE)
  - [ ] Monitor active connections
  - [ ] View request metrics

- [ ] **Configuration Management**
  - [ ] Adjust latency settings via UI
  - [ ] Configure fault injection rates
  - [ ] Update proxy settings
  - [ ] Test real-time config updates

- [ ] **Request Logging**
  - [ ] View logged requests
  - [ ] Filter by method/path/status
  - [ ] Real-time log streaming
  - [ ] Export logs

- [ ] **Metrics Visualization**
  - [ ] View request rates
  - [ ] View latency percentiles (p50, p95, p99)
  - [ ] View error rates
  - [ ] View performance graphs

- [ ] **Fixture Management**
  - [ ] Upload fixtures via drag-and-drop
  - [ ] Organize fixtures in tree view
  - [ ] Edit fixture content
  - [ ] Delete fixtures

### Admin API Endpoints
- [ ] Test `GET /__mockforge/dashboard`
- [ ] Test `GET /__mockforge/health`
- [ ] Test `GET /__mockforge/logs`
- [ ] Test `GET /__mockforge/metrics`
- [ ] Test `GET /__mockforge/fixtures`
- [ ] Test `POST /__mockforge/config/*` endpoints

---

## ⚙️ Core Features (Chaos Engineering)

### Latency Simulation
- [ ] **Enable Latency**
  - [ ] Set `latency_enabled: true`
  - [ ] Configure base latency (50ms)
  - [ ] Configure jitter (20ms)

- [ ] **Latency Distributions**
  - [ ] Test `fixed` distribution
  - [ ] Test `normal` distribution (with std_dev)
  - [ ] Test `pareto` distribution (with shape parameter)

- [ ] **Latency Profiles**
  - [ ] Configure per-tag latency overrides
  - [ ] Test auth operations (100ms)
  - [ ] Test payment operations (200ms)
  - [ ] Test search operations (50ms)

### Failure Injection
- [ ] **Enable Failures**
  - [ ] Set `failures_enabled: true`
  - [ ] Configure global error rate (5%)
  - [ ] Verify random failures occur

- [ ] **Per-Tag Failures**
  - [ ] Configure auth failures (10% rate, 401/403 status)
  - [ ] Configure payment failures (2% rate, 402/503 status)
  - [ ] Verify tag filtering (exclude health checks)

### Proxy Mode
- [ ] **Hybrid Mode**
  - [ ] Enable proxy to upstream API
  - [ ] Configure fallback mode: `forward_unknown`
  - [ ] Test known endpoints (mocked)
  - [ ] Test unknown endpoints (proxied to upstream)
  - [ ] Verify response caching (if enabled)

### Traffic Shaping
- [ ] Enable traffic shaping
- [ ] Test bandwidth limits (1 Mbps)
- [ ] Test packet loss percentage
- [ ] Test max connections limit

---

## 📊 Observability

### Prometheus Metrics
- [ ] **Enable Prometheus**
  - [ ] Set `prometheus.enabled: true`
  - [ ] Start metrics server on port 9090
  - [ ] Access: `http://localhost:9090/metrics`

- [ ] **Verify Metrics**
  - [ ] HTTP request counts (by method, path, status)
  - [ ] Request duration histograms
  - [ ] Active connections gauge
  - [ ] Error rates

### OpenTelemetry Tracing
- [ ] Configure OTLP exporter endpoint
- [ ] Set service name
- [ ] Configure trace sampling ratio
- [ ] Verify traces exported
- [ ] View in Jaeger/Zipkin

### Request Recording
- [ ] Enable request recorder
- [ ] Configure database URL (SQLite/Postgres)
- [ ] Set max body size
- [ ] Verify requests saved to database
- [ ] Query recorded requests

---

## 🔗 Advanced Features

### Request Chaining
- [ ] **Create Chain**
  - [ ] Define multi-step request chain in config
  - [ ] Test variable extraction with JSONPath
  - [ ] Test variable substitution in subsequent requests
  - [ ] Verify validation of final response

- [ ] **Test Scenarios**
  - [ ] User creation flow (POST /users → POST /users/{id}/profile → GET /users/{id})
  - [ ] Order flow (create order → update status → get order)

### Fixtures & Scenarios
- [ ] **Predefined Fixtures**
  - [ ] Load fixture from directory
  - [ ] Test happy path scenarios
  - [ ] Test validation error scenarios
  - [ ] Test edge cases

### Cross-Endpoint Validation
- [ ] Enable referential integrity checks
- [ ] Test foreign key validation
- [ ] Test relationship consistency
- [ ] Verify error messages for violations

### Stateful Mocking
- [ ] **Enable State Management**
  - [ ] Configure storage backend (memory/Redis/file)
  - [ ] Test state persistence across requests
  - [ ] Verify state reset functionality

---

## 🧪 Import/Export Features

### Import from Tools
- [ ] **Postman Import**
  - [ ] Import Postman collection: `mockforge import postman collection.json`
  - [ ] Import Postman environment variables
  - [ ] Verify converted config

- [ ] **Insomnia Import**
  - [ ] Import Insomnia workspace
  - [ ] Verify request conversion

- [ ] **cURL Import**
  - [ ] Convert cURL command to config
  - [ ] Verify headers and body

- [ ] **HAR Import**
  - [ ] Import HAR file (browser network recording)
  - [ ] Verify requests extracted

- [ ] **OpenAPI Import**
  - [ ] Import OpenAPI 3.0 spec
  - [ ] Verify routes generated
  - [ ] Test auto-generated responses

---

## 📚 CLI Commands

### Server Commands
- [ ] `mockforge serve` - start all servers
- [ ] `mockforge serve --config config.yaml` - with config
- [ ] `mockforge admin --port 9080` - standalone admin

### Data Commands
- [ ] `mockforge data template user --rows 100`
- [ ] `mockforge data template product --format csv`
- [ ] `mockforge data schema schema.json`

### Sync Commands
- [ ] `mockforge sync start --directory ./workspace`
- [ ] `mockforge sync stop`
- [ ] `mockforge sync status`

### Plugin Commands
- [ ] `mockforge plugin install <url>`
- [ ] `mockforge plugin list`
- [ ] `mockforge plugin remove <name>`

### Config Commands
- [ ] `mockforge config validate`
- [ ] `mockforge config diff`

### AI Test Commands
- [ ] `mockforge test-ai intelligent-mock --prompt "..."`
- [ ] `mockforge test-ai drift --initial-data file.json --iterations 10`
- [ ] `mockforge test-ai event-stream --narrative "..." --event-count 20`

---

## 🌍 Environment Variables

### Test All Major Env Vars
- [ ] `MOCKFORGE_HTTP_PORT=8080`
- [ ] `MOCKFORGE_WS_PORT=8081`
- [ ] `MOCKFORGE_GRPC_PORT=9090`
- [ ] `MOCKFORGE_ADMIN_PORT=9091`
- [ ] `MOCKFORGE_ADMIN_ENABLED=true`
- [ ] `MOCKFORGE_LATENCY_ENABLED=true`
- [ ] `MOCKFORGE_LOG_LEVEL=debug`
- [ ] `MOCKFORGE_REQUEST_VALIDATION=warn`
- [ ] `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true`
- [ ] `MOCKFORGE_FAKE_TOKENS=false`
- [ ] `MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl`
- [ ] `MOCKFORGE_RAG_API_KEY=sk-...`
- [ ] `MOCKFORGE_GRPC_HTTP_BRIDGE_ENABLED=true`

---

## 🐳 Docker Testing

### Docker Deployment
- [ ] Build image: `docker build -t mockforge .`
- [ ] Run container with port mappings
- [ ] Mount examples directory as volume
- [ ] Test environment variable overrides
- [ ] Verify all services accessible
- [ ] Test Docker Compose multi-service setup

### Docker Persistence
- [ ] Mount config file as volume
- [ ] Mount fixtures directory
- [ ] Mount state directory (if using file storage)
- [ ] Verify data persists across container restarts

---

## 🔥 Performance & Load Testing

### Baseline Performance
- [ ] Test throughput: 1000 requests/sec
- [ ] Test concurrent connections: 100+
- [ ] Measure latency under load
- [ ] Test memory usage over time
- [ ] Test large request/response bodies (10MB+)

### Stress Testing
- [ ] Test max concurrent WebSocket connections
- [ ] Test gRPC streaming with many concurrent streams
- [ ] Test Admin UI responsiveness under load
- [ ] Test plugin execution performance

---

## 🔍 Edge Cases & Error Handling

### Error Scenarios
- [ ] Test malformed JSON requests
- [ ] Test missing required fields
- [ ] Test invalid content types
- [ ] Test oversized requests
- [ ] Test connection timeouts
- [ ] Test invalid OpenAPI specs
- [ ] Test missing proto files
- [ ] Test corrupted replay files

### Recovery Testing
- [ ] Kill and restart server
- [ ] Test state recovery (if using persistent storage)
- [ ] Test graceful shutdown
- [ ] Test rapid start/stop cycles

---

## 📖 Documentation Verification

- [ ] Test all examples in `examples/` directory
- [ ] Verify all README examples work
- [ ] Test 5-minute tutorial
- [ ] Verify troubleshooting guide solutions
- [ ] Test all CLI commands in docs

---

## ✅ Final Checks

- [ ] Test on Linux
- [ ] Test on macOS
- [ ] Test on Windows (if supported)
- [ ] Verify all logs are informative
- [ ] Check for memory leaks (long-running test)
- [ ] Verify clean shutdown on SIGTERM
- [ ] Test with real-world OpenAPI specs (GitHub, Stripe, etc.)
- [ ] End-to-end integration test (all protocols simultaneously)

---

## 📝 Release Readiness

- [ ] All core features working
- [ ] No critical bugs
- [ ] Performance meets expectations
- [ ] Documentation accurate
- [ ] Examples all functional
- [ ] Security audit passed
- [ ] Docker images working
- [ ] CLI UX is intuitive

---

## 📋 Testing Notes

Use this section to track issues found during testing:

### Issues Found
1.
2.
3.

### Performance Observations
-
-

### Recommendations
-
-

---

**Last Updated:** 2025-10-09
**Tested By:**
**Version:**
**Platform:**
