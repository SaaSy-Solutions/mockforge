# MockForge Codebase Exploration Report

## Executive Summary

MockForge is a comprehensive, multi-protocol API mocking framework written in Rust. It's designed as both a standalone CLI application and an embeddable library. The architecture follows a modular, layered approach with clear separation between public SDKs and internal implementations.

**Key Finding**: MockForge is already well-architected for embedding and SDK creation, with clear entry points and configuration structures that can be leveraged for embeddable SDKs.

---

## 1. Current Architecture Overview

### 1.1 Crate Organization

MockForge uses a **Cargo workspace** with 30+ specialized crates organized in layers:

```
Binary Layer (CLI)
    ↓
Protocol/Feature Layer (HTTP, WebSocket, gRPC, GraphQL, MQTT, etc.)
    ↓
Plugin & Extension Layer
    ↓
Foundation Layer (Core, Data, Observability)
```

#### Key Crates:

| Crate | Type | Purpose |
|-------|------|---------|
| `mockforge-cli` | Binary | Main CLI entry point, server orchestration |
| `mockforge-core` | Library | Foundation: routing, validation, templating, request chaining |
| `mockforge-http` | Library | HTTP/REST protocol support with OpenAPI integration |
| `mockforge-ws` | Library | WebSocket protocol support |
| `mockforge-grpc` | Library | gRPC protocol support with protobuf reflection |
| `mockforge-graphql` | Library | GraphQL protocol support |
| `mockforge-data` | Library | Synthetic data generation with faker and RAG capabilities |
| `mockforge-observability` | Library | Prometheus metrics, structured logging |
| `mockforge-plugin-core` | Library | Plugin interfaces and WASM runtime |
| `mockforge-plugin-loader` | Library | Plugin loading and security sandboxing |
| `mockforge-plugin-sdk` | Library | Helper macros and testing utilities |
| `mockforge-ui` | Internal | Web-based admin UI |
| `mockforge-mqtt`, `mockforge-kafka`, `mockforge-amqp` | Libraries | Async protocol support |

### 1.2 Workspace Structure

```
/home/rclanan/dev/projects/work/mockforge/
├── crates/                 # All Rust crates
│   ├── mockforge-cli/      # Binary with CLI commands
│   ├── mockforge-core/     # Foundation library
│   ├── mockforge-http/     # HTTP server implementation
│   ├── mockforge-ws/       # WebSocket support
│   ├── mockforge-grpc/     # gRPC support
│   ├── ... other protocols
│   └── mockforge-plugin-*/  # Plugin system
├── sdk/                    # Language SDKs (Go, Python)
│   ├── go/mockforge/       # Go SDK (basic plugin interface)
│   └── python/             # Python SDK (basic plugin interface)
├── examples/               # Example configs and usage
├── docs/                   # Documentation
└── deploy/                 # Deployment configurations
```

---

## 2. How Mocks Are Created and Managed

### 2.1 Mock Configuration Hierarchy

Mocks are configured through **YAML configuration files** with the following structure:

```yaml
http:
  port: 3000
  routes:
    - path: "/api/users/{id}"
      method: GET
      response:
        status: 200
        body: { "id": "{{uuid}}", "name": "{{faker.name}}" }

websocket:
  port: 3001
  routes: [...]

grpc:
  port: 50051
  services: [...]

routes:                    # Custom routes
  - path: "/health"
    method: GET
    response:
      status: 200
      body: { "status": "ok" }
```

### 2.2 Mock Creation Methods

#### Method 1: OpenAPI-Driven (HTTP)

**File**: `/crates/mockforge-http/src/lib.rs` (line 299)

```rust
pub async fn build_router(
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    failure_config: Option<FailureConfig>,
) -> Router
```

This automatically:
- Parses OpenAPI/Swagger specs
- Generates mock endpoints
- Validates requests against schemas
- Enables request/response validation modes:
  - `Enforce`: Strict validation
  - `Warn`: Log violations but allow
  - `None`: No validation

#### Method 2: YAML Configuration-Driven

**File**: `/crates/mockforge-core/src/config.rs` (line 184)

```rust
pub struct ServerConfig {
    pub http: HttpConfig,
    pub websocket: WebSocketConfig,
    pub graphql: GraphQLConfig,
    pub grpc: GrpcConfig,
    pub routes: Vec<RouteConfig>,
    pub profiles: HashMap<String, ProfileConfig>,
    // ... more protocol configs
}
```

Routes can include:
- **Request validation**: JSON schema validation
- **Response templating**: `{{uuid}}`, `{{faker.name}}`, `{{now}}`
- **Authentication**: JWT, OAuth2, Basic Auth, API Keys
- **Latency injection**: Fixed or normal distribution
- **Failure injection**: HTTP error codes with probability
- **Request chaining**: Multi-step workflows

#### Method 3: Workspace-Based Management

**File**: `/crates/mockforge-core/src/workspace.rs` (line 38)

```rust
pub struct Workspace {
    pub name: String,
    pub collections: HashMap<EntityId, MockCollection>,
    pub folders: HashMap<EntityId, Folder>,
    pub environments: HashMap<String, Environment>,
    // ...
}
```

Workspaces provide:
- **Collections**: Organized groups of mocks
- **Folders**: Hierarchical organization
- **Environments**: Different configurations (dev, staging, prod)
- **Sync**: Bidirectional file synchronization
- **Persistence**: SQLite-backed storage

### 2.3 Mock Response Features

#### Templating Engine
- **Dynamic values**: `{{uuid}}`, `{{timestamp}}`, `{{now}}`
- **Faker integration**: `{{faker.name}}`, `{{faker.email}}`, `{{faker.address}}`
- **Request context**: `{{request.headers.x-user-id}}`
- **Custom functions**: Via plugin system

#### Response Transformation
- **JSON Path matching**: Conditional responses based on request content
- **Chain operations**: Multi-step request flows
- **Conditional logic**: Branching based on request properties

#### AI-Powered Generation
**File**: `/crates/mockforge-data/rag/`
- Generate realistic responses using LLMs
- Support for OpenAI, Anthropic, Ollama
- Schema-aware generation

---

## 3. Existing Server Implementation

### 3.1 Server Entry Point

**File**: `/crates/mockforge-cli/src/main.rs`

The CLI provides a `serve` command that orchestrates all servers:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize logging
    mockforge_observability::init_logging(config);

    // Handle serve command
    handle_serve(
        config_path, http_port, ws_port, grpc_port,
        admin, metrics, tracing, recorder, chaos, // ... more args
    ).await
}
```

### 3.2 Server Startup Flow

**Function**: `handle_serve()` (line 2138 in main.rs)

1. **Configuration Loading**
   - Load from YAML file or use defaults
   - Apply environment overrides
   - Merge profile settings

2. **Port Validation**
   - Check all required ports are available
   - Support dry-run mode for validation

3. **Feature Initialization**
   - Initialize logging (file/JSON/console)
   - Setup OpenTelemetry tracing if enabled
   - Initialize Prometheus metrics if enabled
   - Setup API Flight Recorder (SQLite-based request recording)
   - Configure chaos engineering if enabled

4. **Server Startup**
   - Launch HTTP server (Axum-based)
   - Launch WebSocket server
   - Launch gRPC server
   - Launch GraphQL server
   - Launch Admin UI (if enabled)
   - Launch Metrics endpoint (if enabled)

### 3.3 HTTP Server Builder

**File**: `/crates/mockforge-http/src/lib.rs` (line 310)

```rust
pub async fn build_router_with_multi_tenant(
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    failure_config: Option<FailureConfig>,
    multi_tenant_config: Option<MultiTenantConfig>,
    route_configs: Option<Vec<RouteConfig>>,
    cors_config: Option<HttpCorsConfig>,
    ai_generator: Option<Arc<dyn AiGenerator + Send + Sync>>,
    smtp_registry: Option<Arc<dyn Any + Send + Sync>>,
) -> Router
```

The router includes:
- **OpenAPI route registration**: Auto-generated from specs
- **Rate limiting middleware**: Per-IP or global
- **CORS middleware**: Configurable origins and methods
- **Tracing middleware**: Distributed tracing integration
- **Metrics middleware**: Request/response metrics collection
- **Authentication middleware**: Multiple auth strategies
- **Management API**: REST endpoints for runtime control

### 3.4 Supported Protocols

All run simultaneously on different ports:

| Protocol | Port | Implementation Crate | Features |
|----------|------|---------------------|----------|
| HTTP/REST | 3000 | `mockforge-http` | OpenAPI, validation, templating |
| WebSocket | 3001 | `mockforge-ws` | Message validation, event streaming |
| gRPC | 50051 | `mockforge-grpc` | Protobuf reflection, service mocking |
| GraphQL | 4000 | `mockforge-graphql` | Schema-based mocking |
| MQTT | 1883 | `mockforge-mqtt` | Topic-based pub/sub |
| Kafka | 9092 | `mockforge-kafka` | Topic/partition simulation |
| AMQP | 5672 | `mockforge-amqp` | Exchange/queue simulation |
| SMTP | 1025 | `mockforge-smtp` | Email capture and retrieval |
| FTP | 2121 | `mockforge-ftp` | Virtual filesystem |

---

## 4. CLI Interface and Server Control

### 4.1 Main CLI Commands

**File**: `/crates/mockforge-cli/src/main.rs` (line 41-500)

```
mockforge serve [OPTIONS]           # Start mock servers
mockforge admin [OPTIONS]           # Start admin UI only
mockforge sync [OPTIONS]            # Sync workspace directory
mockforge data [COMMAND]            # Generate synthetic data
mockforge [http|ws|grpc] [COMMAND]  # Protocol-specific management
mockforge mqtt [COMMAND]            # MQTT-specific operations
mockforge kafka [COMMAND]           # Kafka-specific operations
mockforge amqp [COMMAND]            # AMQP-specific operations
mockforge plugin [COMMAND]          # Plugin development tools
mockforge import [COMMAND]          # Import from Postman, Insomnia, curl
```

### 4.2 Serve Command Options

Key startup options (from lines 44-383):

```
--config <PATH>                     # YAML config file
--profile <NAME>                    # Use named profile (dev, ci, prod)
--http-port <PORT>                  # HTTP port (default 3000)
--ws-port <PORT>                    # WebSocket port (default 3001)
--grpc-port <PORT>                  # gRPC port (default 50051)
--admin                             # Enable admin UI
--admin-port <PORT>                 # Admin UI port (default 9080)
--metrics                           # Enable Prometheus metrics
--metrics-port <PORT>               # Metrics port (default 9090)
--tracing                           # Enable OpenTelemetry tracing
--recorder                          # Enable API Flight Recorder
--dry-run                           # Validate config without starting
--chaos                             # Enable chaos engineering
--traffic-shaping                   # Enable bandwidth throttling
--network-profile <PROFILE>         # Apply network condition profile
--ai-enabled                        # Enable AI features
--rag-provider <PROVIDER>           # AI provider (openai, anthropic, ollama)
--rag-model <MODEL>                 # AI model name
```

### 4.3 Signal Handling

The CLI gracefully handles:
- `SIGTERM`/`SIGINT` (Ctrl+C): Graceful shutdown
- Server state cleanup
- Database connection closure
- Log file flushing

---

## 5. SDK and Library Code

### 5.1 Existing SDK Implementations

#### Go SDK
**Location**: `/sdk/go/mockforge/plugin.go`

- Provides plugin interfaces for Go developers
- Uses TinyGo + WebAssembly compilation
- Interfaces:
  - `AuthPlugin`: Authentication logic
  - `TemplatePlugin`: Custom template functions
  - `ResponsePlugin`: Response generation
  - `DataSourcePlugin`: Database integration

#### Python SDK
**Location**: `/sdk/python/mockforge_plugin/sdk.py`

- Remote plugin system (runs as external HTTP service)
- Uses FastAPI for HTTP endpoints
- Same plugin interfaces as Go
- Example: Token validation, template functions, data generation

### 5.2 Core Library Exports

**mockforge-core** exports these key types:

```rust
// Configuration
pub use config::ServerConfig;
pub use openapi::OpenApiSpec;
pub use workspace::Workspace;

// Routing and validation
pub use openapi_routes::OpenApiRouteRegistry;
pub use openapi_routes::ValidationOptions;
pub use routing::Route;

// Request processing
pub use templating::TemplateEngine;
pub use request_chaining::RequestChainRegistry;

// Features
pub use latency::LatencyInjector;
pub use failure_injection::FailureInjector;
pub use traffic_shaping::TrafficShaper;
```

### 5.3 mockforge-http Library Exports

**File**: `/crates/mockforge-http/src/lib.rs` (lines 186-195)

Public API:
- `build_router()`: Build HTTP router from spec
- `management_router()`: Admin/management API
- `management_router_with_ui_builder()`: With UI builder
- `ws_management_router()`: WebSocket management API
- `process_response_with_ai()`: AI-powered response generation
- `collect_http_metrics()`: Metrics collection
- `ManagementState`: Shared state for management APIs

---

## 6. Configuration Structures for Mocks

### 6.1 Complete Configuration Hierarchy

```rust
ServerConfig {
    // Protocol configurations
    http: HttpConfig {
        port: u16,
        host: String,
        cors: HttpCorsConfig { ... },
        tls: Option<TlsConfig> { ... },
        validation: HttpValidationConfig { ... },
    },
    websocket: WebSocketConfig { ... },
    graphql: GraphQLConfig { ... },
    grpc: GrpcConfig { ... },
    mqtt: MqttConfig { ... },
    smtp: SmtpConfig { ... },
    ftp: FtpConfig { ... },
    kafka: KafkaConfig { ... },
    amqp: AmqpConfig { ... },

    // Core features
    core: Config {
        latency_enabled: bool,
        default_latency: LatencyProfile { ... },
        failures_enabled: bool,
        failure_config: FailureConfig { ... },
        traffic_shaping_enabled: bool,
        traffic_shaping: TrafficShapingConfig { ... },
        chaos_random: Option<ChaosConfig>,
    },

    // Admin & UI
    admin: AdminConfig {
        enabled: bool,
        port: u16,
        host: String,
        auth_required: bool,
        mount_path: String,
    },

    // Observability
    observability: ObservabilityConfig {
        prometheus: PrometheusConfig { ... },
        opentelemetry: Option<OpenTelemetryConfig> { ... },
        recorder: Option<RecorderConfig> { ... },
        chaos: Option<ChaosConfig> { ... },
    },

    // Data & AI
    data: DataConfig {
        rag: RagConfig {
            enabled: bool,
            provider: String,
            model: Option<String>,
            api_key: Option<String>,
        },
    },

    // Routes
    routes: Vec<RouteConfig> {
        path: String,
        method: String,
        request: Option<RouteRequestConfig>,
        response: RouteResponseConfig {
            status: u16,
            headers: HashMap<String, String>,
            body: Option<Value>,
        },
    },

    // Profiles (named configurations)
    profiles: HashMap<String, ProfileConfig> { ... },

    // Logging
    logging: LoggingConfig {
        level: String,
        json_format: bool,
        file_path: Option<PathBuf>,
    },
}
```

### 6.2 Route Configuration Example

```yaml
routes:
  - path: "/api/users/{id}"
    method: GET
    request:
      validation:
        schema:
          type: object
          properties:
            id:
              type: string
              pattern: "^[0-9]+$"
    response:
      status: 200
      headers:
        content-type: "application/json"
      body:
        id: "{{request.path_params.id}}"
        name: "{{faker.name}}"
        email: "{{faker.email}}"
        created_at: "{{now}}"

  - path: "/api/users"
    method: POST
    request:
      validation:
        schema:
          type: object
          required: [name, email]
          properties:
            name:
              type: string
            email:
              type: string
              format: email
    response:
      status: 201
      body:
        id: "{{uuid}}"
        name: "{{request.body.name}}"
        email: "{{request.body.email}}"
```

### 6.3 Latency Profiles

```rust
pub enum LatencyProfile {
    Fixed { delay_ms: u64 },
    Normal { mean_ms: u64, std_dev: f64, min_ms: u64, max_ms: u64 },
    Exponential { lambda: f64, max_ms: u64 },
    Bimodal { ... },
}
```

Configuration in YAML:

```yaml
core:
  latency_enabled: true
  latency:
    mode: normal
    mean_ms: 200
    std_dev: 50.0
    min_ms: 50
    max_ms: 1000
```

---

## 7. Key Files for SDK Development

### Core Entry Points

| File | Purpose | Key Exports |
|------|---------|-------------|
| `/crates/mockforge-core/src/lib.rs` | Foundation | `ServerConfig`, `OpenApiSpec`, `OpenApiRouteRegistry` |
| `/crates/mockforge-http/src/lib.rs` | HTTP server | `build_router()`, `management_router()` |
| `/crates/mockforge-cli/src/main.rs` | CLI | Command definitions, `handle_serve()` |
| `/crates/mockforge-core/src/config.rs` | Configuration | All config structs |
| `/crates/mockforge-core/src/workspace.rs` | Mock management | `Workspace`, `WorkspaceRegistry` |

### Protocol Implementations

- **HTTP**: `/crates/mockforge-http/src/` - 332KB of route handling, management, AI
- **WebSocket**: `/crates/mockforge-ws/` - Event streaming, message validation
- **gRPC**: `/crates/mockforge-grpc/` - Service mocking, protobuf reflection
- **GraphQL**: `/crates/mockforge-graphql/` - Schema mocking, query validation
- **Async**: Kafka, MQTT, AMQP with topic/queue simulation

---

## 8. Opportunities for Embeddable SDKs

### 8.1 Library-First Architecture

MockForge is already designed as a library:

1. **mockforge-core** is completely standalone
   - No CLI dependencies
   - No binary bloat
   - Can be used in any Rust application

2. **Protocol libraries** are independent
   - `mockforge-http` can be used without other protocols
   - Each has clear API boundaries
   - Minimal dependencies

3. **Configuration system** is data-driven
   - YAML-based or programmatic
   - Easy to embed in SDKs
   - No CLI required

### 8.2 Potential SDK Entry Points

#### Rust SDK (In-Process)
```rust
// Start an in-process mock server
let config = ServerConfig::from_yaml("config.yaml").await?;
let mut http_server = HttpServerBuilder::new(config.http).build().await?;
http_server.start().await?;

// Or use builder pattern
let mock = MockServerBuilder::new()
    .with_port(3000)
    .with_spec("api.json")
    .build()
    .await?;
mock.start().await?;
```

#### Other Languages (Out-of-Process)
- **Go SDK**: Embed MockForge binary + control via HTTP API
- **Python SDK**: Subprocess wrapper + REST client
- **Node.js SDK**: Similar approach with CLI subprocess
- **Java SDK**: Via process management

### 8.3 Key APIs to Expose

For in-process SDKs:

1. **Server Control**
   - `start()` / `stop()` / `restart()`
   - Port management
   - Graceful shutdown

2. **Mock Management**
   - `add_mock()` / `remove_mock()`
   - `update_response()`
   - `add_route()`
   - `import_spec()`

3. **Query & Inspection**
   - `get_routes()` / `get_request_logs()`
   - `get_metrics()`
   - `verify_call()`

4. **Configuration**
   - `set_latency()` / `set_chaos()`
   - `enable_auth()` / `disable_auth()`
   - `set_traffic_shaping()`

---

## 9. Technology Stack

### Frameworks & Libraries

| Component | Library | Version |
|-----------|---------|---------|
| Async Runtime | `tokio` | 1.0 |
| Web Framework | `axum` | 0.8 |
| Serialization | `serde` + `serde_json` | 1.0 |
| OpenAPI | `openapiv3` | 2.2 |
| gRPC | `tonic` | 0.10+ |
| GraphQL | `async-graphql` | 0.10+ |
| MQTT | `rumqttc` | 0.24 |
| Kafka | `rdkafka` | 0.38 |
| AMQP | `lapin` | 2.3 |
| Metrics | `prometheus` | (custom) |
| Tracing | `tracing` + `opentelemetry` | 0.1/0.21 |
| Authentication | `jsonwebtoken` | 9.0 |
| Data Generation | Custom faker | In-house |
| WASM | `wasmtime` | (runtime) |

---

## 10. Deployment Options

### 1. Standalone Binary
```bash
mockforge serve --config mockforge.yaml
```

### 2. Docker
```dockerfile
FROM rust:latest
COPY mockforge /app
CMD ["/app/mockforge", "serve", "--config", "/config/mockforge.yaml"]
```

### 3. Kubernetes
- Helm charts available
- K8s operator available
- Native resource management

### 4. Embedded in Application
```rust
// In your Rust application
use mockforge_http::build_router;

let router = build_router(
    Some("api.json".to_string()),
    None,
    None,
).await;

let listener = TcpListener::bind("0.0.0.0:3000").await?;
axum::serve(listener, router).await?;
```

---

## 11. Recommendations for SDK Development

### For Creating Embeddable SDKs:

1. **Expose High-Level APIs**
   - Builder pattern for configuration
   - Async/await support
   - Type-safe configuration

2. **Provide Multiple Levels**
   - Beginner: Simple `start_server()` function
   - Intermediate: Builder pattern
   - Advanced: Direct library usage

3. **Handle Lifecycle**
   - Clear server startup/shutdown
   - Resource cleanup
   - Error handling and recovery

4. **Support Configuration**
   - YAML files
   - Programmatic API
   - Environment variables
   - Environment variable overrides

5. **Expose Management APIs**
   - HTTP endpoints for management
   - WebSocket for real-time updates
   - Request inspection/logging
   - Metrics collection

6. **Keep Dependencies Light**
   - Users shouldn't need full CLI
   - Optional feature flags
   - Minimal transitive dependencies

---

## 12. Summary

MockForge is an **exceptionally well-architected** framework for creating embeddable SDKs:

✅ **Strengths**:
- Clean library/CLI separation
- Independent protocol implementations
- Configuration is data-driven (YAML + structs)
- Already has HTTP/WebSocket/gRPC servers
- Plugin system for extensibility
- Comprehensive observability built-in
- Workspace management system

⚠️ **Considerations**:
- Some crates marked `publish = false` (UI, recorder, etc.)
- CLI heavily depends on multiple protocols
- Configuration merging logic is complex
- Database (SQLite) for recordings

🎯 **Next Steps**:
1. Design SDK API surface
2. Create builder pattern wrappers
3. Expose management HTTP APIs
4. Add language-specific bindings (Go, Python, Node)
5. Create comprehensive examples
6. Document server lifecycle management
