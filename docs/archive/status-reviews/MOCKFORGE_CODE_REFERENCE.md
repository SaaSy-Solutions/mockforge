# MockForge Code Reference - Quick Lookup Guide

This document provides quick references to key code locations and patterns used in MockForge.

---

## File Locations Quick Reference

### Core Foundation
```
/crates/mockforge-core/
├── src/lib.rs                      # Main exports and module declarations (150+ lines)
├── src/config.rs                   # ServerConfig and all config structs (900+ lines)
├── src/workspace.rs                # Workspace and WorkspaceRegistry (900+ lines)
├── src/routing.rs                  # Route matching and registration
├── src/validation.rs               # Request/response validation (1300+ lines)
├── src/templating.rs               # Template engine with {{}} variable expansion
├── src/openapi*.rs                 # OpenAPI spec parsing and route generation (900+ lines)
├── src/request_chaining.rs         # Multi-step request workflows (1200+ lines)
├── src/latency.rs                  # Latency injection and profiles (600+ lines)
├── src/failure_injection.rs        # Error/chaos injection
└── src/workspace_persistence.rs    # SQLite persistence for workspaces
```

### HTTP Server Implementation
```
/crates/mockforge-http/
├── src/lib.rs                      # Main router builder (700+ lines)
├── src/management.rs               # Admin API endpoints (600+ lines)
├── src/management_ws.rs            # WebSocket management
├── src/ui_builder.rs               # UI-based endpoint builder (900+ lines)
├── src/auth.rs                     # JWT, OAuth2, Basic Auth
├── src/quick_mock.rs               # Quick mock creation helpers (700+ lines)
├── src/spec_import.rs              # OpenAPI spec importing
├── src/rag_ai_generator.rs         # AI-powered response generation
├── src/chain_handlers.rs           # Request chaining support
└── src/coverage.rs                 # Coverage tracking
```

### CLI and Server Control
```
/crates/mockforge-cli/
├── src/main.rs                     # Main entry point (2500+ lines)
│   ├── Line 28:   Cli struct definition with Subcommand enum
│   ├── Line 44:   Serve command definition
│   ├── Line 1282: #[tokio::main] async fn main()
│   ├── Line 2138: async fn handle_serve() - main server startup
│   └── Line 1730: async fn build_server_config_from_cli()
├── src/import_commands.rs          # Import from Postman, Insomnia, etc.
├── src/workspace_commands.rs       # Workspace management commands
├── src/mqtt_commands.rs            # MQTT broker commands
├── src/kafka_commands.rs           # Kafka broker commands
└── src/plugin_commands.rs          # Plugin development commands
```

### Protocol-Specific Implementation
```
/crates/mockforge-ws/src/          # WebSocket implementation
/crates/mockforge-grpc/src/        # gRPC implementation
/crates/mockforge-graphql/src/     # GraphQL implementation
/crates/mockforge-mqtt/src/        # MQTT broker
/crates/mockforge-kafka/src/       # Kafka broker
/crates/mockforge-amqp/src/        # AMQP broker
```

### SDK/Plugin System
```
/crates/mockforge-plugin-core/     # Plugin interfaces (WASM)
/crates/mockforge-plugin-loader/   # Plugin loading and sandboxing
/crates/mockforge-plugin-sdk/      # Helper macros for plugins
/sdk/go/mockforge/plugin.go        # Go SDK for plugins
/sdk/python/mockforge_plugin/sdk.py # Python SDK for plugins
```

### Examples and Configuration
```
/examples/
├── mockforge.config.yaml           # Profile-based config example
├── advanced-config.yaml            # Full-featured config
├── plugins/                        # Example plugin implementations
│   ├── auth-jwt/src/lib.rs
│   ├── auth-basic/src/lib.rs
│   ├── template-crypto/src/lib.rs
│   └── datasource-csv/src/lib.rs
└── *.yaml                          # Various example configs
```

---

## Key Code Patterns

### 1. ServerConfig Loading

**Location**: `/crates/mockforge-cli/src/main.rs:1730`

```rust
async fn build_server_config_from_cli(serve_args: &ServeArgs) -> ServerConfig {
    // Load config file
    let mut config = if let Some(path) = &serve_args.config_path {
        ServerConfig::from_file(path).await?
    } else {
        ServerConfig::default()
    };

    // Apply CLI overrides
    if let Some(http_port) = serve_args.http_port {
        config.http.port = http_port;
    }

    config
}
```

**Usage Pattern**:
```rust
// Load from YAML
let config = ServerConfig::from_file("mockforge.yaml").await?;

// Or build programmatically
let config = ServerConfig {
    http: HttpConfig {
        port: 3000,
        ..Default::default()
    },
    ..Default::default()
};
```

### 2. HTTP Router Building

**Location**: `/crates/mockforge-http/src/lib.rs:299-310`

```rust
pub async fn build_router(
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    failure_config: Option<FailureConfig>,
) -> Router {
    // Build OpenAPI-based router with validation
    // Automatically generates endpoints from spec
    // Adds middleware for tracing, metrics, auth
}

pub async fn build_router_with_multi_tenant(
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    failure_config: Option<FailureConfig>,
    multi_tenant_config: Option<MultiTenantConfig>,
    route_configs: Option<Vec<RouteConfig>>,
    cors_config: Option<HttpCorsConfig>,
    ai_generator: Option<Arc<dyn AiGenerator + Send + Sync>>,
    smtp_registry: Option<Arc<dyn Any + Send + Sync>>,
) -> Router {
    // More advanced version with multi-tenant support
}
```

**Usage Pattern**:
```rust
let router = build_router(
    Some("api.json".to_string()),
    Some(ValidationOptions {
        request_mode: ValidationMode::Enforce,
        ..Default::default()
    }),
    None,
).await;

let listener = TcpListener::bind("0.0.0.0:3000").await?;
axum::serve(listener, router).await?;
```

### 3. Configuration Structures

**Location**: `/crates/mockforge-core/src/config.rs`

```rust
pub struct ServerConfig {
    pub http: HttpConfig,
    pub websocket: WebSocketConfig,
    pub graphql: GraphQLConfig,
    pub grpc: GrpcConfig,
    pub mqtt: MqttConfig,
    pub smtp: SmtpConfig,
    pub ftp: FtpConfig,
    pub kafka: KafkaConfig,
    pub amqp: AmqpConfig,
    pub core: Config,
    pub admin: AdminConfig,
    pub observability: ObservabilityConfig,
    pub data: DataConfig,
    pub routes: Vec<RouteConfig>,
    pub profiles: HashMap<String, ProfileConfig>,
    pub logging: LoggingConfig,
    pub chaining: ChainingConfig,
}

pub struct HttpConfig {
    pub port: u16,
    pub host: String,
    pub cors: HttpCorsConfig,
    pub tls: Option<TlsConfig>,
    pub validation: HttpValidationConfig,
}

pub struct RouteConfig {
    pub path: String,
    pub method: String,
    pub request: Option<RouteRequestConfig>,
    pub response: RouteResponseConfig,
}

pub struct RouteResponseConfig {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<serde_json::Value>,
}
```

### 4. OpenAPI Route Registration

**Location**: `/crates/mockforge-core/src/openapi_routes.rs`

```rust
pub struct OpenApiRouteRegistry {
    spec: OpenApiSpec,
    routes: Vec<RegisteredRoute>,
    validator: Option<RequestValidator>,
}

impl OpenApiRouteRegistry {
    pub fn new(spec: OpenApiSpec) -> Self {
        // Create registry from OpenAPI spec
        // Automatically discovers all paths and methods
    }

    pub fn new_with_options(
        spec: OpenApiSpec,
        options: ValidationOptions
    ) -> Self {
        // Create with request/response validation
    }

    pub fn get_routes(&self) -> &[RegisteredRoute] {
        // Returns all registered routes
    }

    pub fn validate_request(&self, req: &Request) -> Result<()> {
        // Validate against OpenAPI schema
    }
}
```

### 5. Template Engine

**Location**: `/crates/mockforge-core/src/templating.rs`

```rust
pub struct TemplateEngine {
    functions: HashMap<String, Box<dyn Fn(&[Value]) -> Value>>,
}

impl TemplateEngine {
    pub fn expand(&self, template: &str, context: &Context) -> Result<String> {
        // Replace {{variable}} with context values
        // Supports:
        // - {{uuid}}, {{now}}, {{timestamp}}
        // - {{faker.name}}, {{faker.email}}, etc.
        // - {{request.headers.X-User-Id}}
        // - {{custom_function(arg1, arg2)}}
    }
}
```

### 6. Workspace Management

**Location**: `/crates/mockforge-core/src/workspace.rs:38`

```rust
pub struct Workspace {
    pub name: String,
    pub collections: HashMap<EntityId, MockCollection>,
    pub folders: HashMap<EntityId, Folder>,
    pub environments: HashMap<String, Environment>,
    pub active_environment: Option<String>,
    pub sync_config: SyncConfig,
}

impl Workspace {
    pub fn new(name: String) -> Self { }

    pub fn create_environment(&mut self, name: String) -> Result<EntityId> { }

    pub fn get_active_environment(&self) -> &Environment { }

    pub fn get_variable(&self, key: &str) -> Option<&String> { }

    pub fn to_filtered_for_sync(&self) -> Workspace { }
}

pub struct WorkspaceRegistry {
    workspaces: HashMap<String, Workspace>,
}
```

### 7. Request Chaining

**Location**: `/crates/mockforge-core/src/request_chaining.rs`

```rust
pub struct ChainDefinition {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub links: Vec<ChainLink>,  // Steps in the chain
    pub variables: HashMap<String, String>,
}

pub struct ChainLink {
    pub request: ChainRequest,  // HTTP request to make
    pub extract: HashMap<String, String>,  // Extract values from response
    pub store_as: Option<String>,  // Store response as variable
}

pub struct RequestChainRegistry {
    chains: HashMap<String, ChainDefinition>,
}

impl RequestChainRegistry {
    pub async fn execute_chain(&self, chain_id: &str) -> Result<ChainResult> {
        // Execute multi-step workflow
        // Pass context between steps
    }
}
```

### 8. Latency Injection

**Location**: `/crates/mockforge-core/src/latency.rs`

```rust
pub enum LatencyProfile {
    Fixed { delay_ms: u64 },
    Normal {
        mean_ms: u64,
        std_dev: f64,
        min_ms: u64,
        max_ms: u64,
    },
    Exponential { lambda: f64, max_ms: u64 },
    Bimodal { ... },
}

pub struct LatencyInjector {
    profile: LatencyProfile,
}

impl LatencyInjector {
    pub async fn apply(&self) {
        // Sleep for the configured latency
    }
}
```

### 9. Management API (Admin Endpoints)

**Location**: `/crates/mockforge-http/src/management.rs`

```rust
pub struct ManagementState {
    http_port: u16,
    spec_path: Option<String>,
}

pub fn management_router(state: ManagementState) -> Router {
    // Provides these endpoints:
    // GET  /__mockforge/health
    // GET  /__mockforge/stats
    // GET  /__mockforge/logs (SSE)
    // GET  /__mockforge/metrics
    // GET  /__mockforge/fixtures
    // GET  /__mockforge/routes
    // POST /__mockforge/config/...
}
```

### 10. Server Startup (Main Flow)

**Location**: `/crates/mockforge-cli/src/main.rs:2138`

```rust
async fn handle_serve(
    config_path: Option<PathBuf>,
    http_port: Option<u16>,
    ws_port: Option<u16>,
    grpc_port: Option<u16>,
    admin: bool,
    admin_port: Option<u16>,
    metrics: bool,
    metrics_port: Option<u16>,
    // ... many more options
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 1. Load and validate configuration
    let mut config = build_server_config_from_cli(&serve_args).await;

    // 2. Apply CLI overrides
    if let Some(port) = http_port {
        config.http.port = port;
    }

    // 3. Validate ports are available
    ensure_ports_available(&final_ports)?;

    // 4. Initialize observability
    mockforge_observability::init_logging(logging_config)?;

    // 5. If tracing enabled, initialize OpenTelemetry
    if tracing {
        initialize_opentelemetry_tracing(&otel_config, &logging_config)?;
    }

    // 6. Build servers
    #[cfg(feature = "http")]
    {
        let http_router = build_http_router(&config).await;
        let listener = TcpListener::bind(format!("{}:{}", config.http.host, config.http.port))
            .await?;
        tokio::spawn(axum::serve(listener, http_router));
    }

    #[cfg(feature = "ws")]
    {
        let ws_router = build_ws_router(&config).await;
        // ... bind and spawn
    }

    // ... repeat for gRPC, GraphQL, MQTT, Kafka, AMQP

    // 7. If admin UI enabled, start it
    if config.admin.enabled {
        // Start admin UI on separate port
    }

    // 8. If metrics enabled, start Prometheus endpoint
    if config.observability.prometheus.enabled {
        // Start metrics server
    }

    // 9. Wait for signals
    tokio::signal::ctrl_c().await?;

    println!("Shutting down gracefully...");
}
```

---

## Configuration Examples

### Minimal Configuration (YAML)

**File**: `/examples/mockforge.config.yaml`

```yaml
http:
  port: 3000
  host: "0.0.0.0"
  cors:
    enabled: true
    allowed_origins: ["*"]

websocket:
  port: 3001

grpc:
  port: 50051

logging:
  level: "info"
```

### Full-Featured Configuration

**File**: `/examples/advanced-config.yaml`

```yaml
http:
  port: 3000
  host: "0.0.0.0"
  cors:
    enabled: true
    allowed_origins: ["*"]
    allowed_methods: [GET, POST, PUT, DELETE]
  validation:
    request_mode: "enforce"
    response_mode: "warn"

routes:
  - path: "/api/users/{id}"
    method: GET
    response:
      status: 200
      headers:
        content-type: "application/json"
      body:
        id: "{{request.path_params.id}}"
        name: "{{faker.name}}"
        email: "{{faker.email}}"
        created_at: "{{now}}"

core:
  latency_enabled: true
  latency:
    mode: normal
    mean_ms: 200
    std_dev: 50.0
    min_ms: 50
    max_ms: 1000
  failures_enabled: false

admin:
  enabled: true
  port: 9080

observability:
  prometheus:
    enabled: true
    port: 9090
  recorder:
    enabled: true
    database_path: "./recordings.db"
    max_requests: 10000

profiles:
  dev:
    logging:
      level: "debug"
    admin:
      enabled: true
  prod:
    logging:
      level: "warn"
      json_format: true
    core:
      latency_enabled: false
```

---

## Testing Patterns

### Integration Tests

**Location**: `/crates/mockforge-http/tests/`

```rust
#[tokio::test]
async fn test_openapi_route_generation() {
    let spec_path = "openapi.json";
    let router = build_router(
        Some(spec_path.to_string()),
        None,
        None,
    ).await;

    let client = TestClient::new(router);
    let response = client.get("/api/users/123").send().await;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await;
    assert!(body["id"].is_string());
}
```

---

## Performance Notes

### Memory Usage
- Typical MockForge instance: 50-100MB
- Per additional protocol: +10-20MB
- With large OpenAPI spec: +5-10MB

### Latency Overhead
- Request processing: < 1ms (typical)
- Templating expansion: < 5ms (for most templates)
- Validation: < 10ms (depends on schema complexity)

### Concurrency
- Handles 10K+ concurrent connections (with sufficient memory)
- Each protocol runs on separate thread/task
- Rate limiting per-IP available

---

## Debugging Tips

### Enable Detailed Logging
```bash
mockforge serve --config mockforge.yaml -v debug
```

### Check Port Availability
```bash
# Dry-run mode validates without starting
mockforge serve --config mockforge.yaml --dry-run
```

### View Workspace Contents
```bash
mockforge workspace list
mockforge workspace show <workspace-id>
```

### Check HTTP Routes
```bash
# Via API Flight Recorder
curl http://localhost:9090/metrics | grep routes
```

---

## Dependencies

### Core Dependencies
- `tokio`: Async runtime
- `axum`: Web framework
- `serde` + `serde_json`: Serialization
- `openapiv3`: OpenAPI parsing
- `tonic`: gRPC framework
- `tracing`: Structured logging

### Version Compatibility
- Rust Edition 2021
- MSRV (Minimum Supported Rust Version): 1.70+
- Works on Linux, macOS, Windows
