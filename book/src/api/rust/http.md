# HTTP Module

The `mockforge_http` crate provides comprehensive HTTP/REST API mocking capabilities with OpenAPI integration, AI-powered responses, and advanced management features.

## Modules

### Core Functions

#### `build_router`

```rust
pub async fn build_router(
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    failure_config: Option<FailureConfig>,
) -> Router
```

Creates a basic HTTP router with optional OpenAPI specification support.

**Parameters:**
- `spec_path`: Optional path to OpenAPI specification file
- `options`: Optional validation options for request/response validation
- `failure_config`: Optional failure injection configuration

**Returns:** Axum `Router` configured for HTTP mocking

**Example:**
```rust
use mockforge_http::build_router;
use mockforge_core::ValidationOptions;

let router = build_router(
    Some("./api.yaml".to_string()),
    Some(ValidationOptions::enforce()),
    None,
).await;
```

#### `build_router_with_auth`

```rust
pub async fn build_router_with_auth(
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    auth_config: Option<AuthConfig>,
) -> Router
```

Creates an HTTP router with authentication support.

**Parameters:**
- `spec_path`: Optional path to OpenAPI specification file
- `options`: Optional validation options
- `auth_config`: Authentication configuration (OAuth2, JWT, API keys)

**Returns:** Axum `Router` with authentication middleware

**Example:**
```rust
use mockforge_http::build_router_with_auth;
use mockforge_core::config::AuthConfig;

let auth_config = AuthConfig {
    oauth2: Some(OAuth2Config {
        client_id: "client123".to_string(),
        client_secret: "secret".to_string(),
        ..Default::default()
    }),
    ..Default::default()
};

let router = build_router_with_auth(
    Some("./api.yaml".to_string()),
    None,
    Some(auth_config),
).await;
```

#### `build_router_with_chains`

```rust
pub async fn build_router_with_chains(
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    chain_config: Option<RequestChainingConfig>,
) -> Router
```

Creates an HTTP router with request chaining support for multi-step workflows.

**Parameters:**
- `spec_path`: Optional path to OpenAPI specification file
- `options`: Optional validation options
- `chain_config`: Request chaining configuration

**Returns:** Axum `Router` with chaining capabilities

#### `build_router_with_multi_tenant`

```rust
pub async fn build_router_with_multi_tenant(
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    failure_config: Option<FailureConfig>,
    multi_tenant_config: Option<MultiTenantConfig>,
    route_configs: Option<Vec<RouteConfig>>,
    cors_config: Option<HttpCorsConfig>,
) -> Router
```

Creates an HTTP router with multi-tenant workspace support.

**Parameters:**
- `spec_path`: Optional path to OpenAPI specification file
- `options`: Optional validation options
- `failure_config`: Optional failure injection configuration
- `multi_tenant_config`: Multi-tenant workspace configuration
- `route_configs`: Custom route configurations
- `cors_config`: CORS configuration

**Returns:** Axum `Router` with multi-tenant support

#### `build_router_with_traffic_shaping`

```rust
pub async fn build_router_with_traffic_shaping(
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    traffic_shaper: Option<TrafficShaper>,
    traffic_shaping_enabled: bool,
) -> Router
```

Creates an HTTP router with traffic shaping capabilities.

**Parameters:**
- `spec_path`: Optional path to OpenAPI specification file
- `options`: Optional validation options
- `traffic_shaper`: Traffic shaping configuration
- `traffic_shaping_enabled`: Whether traffic shaping is active

**Returns:** Axum `Router` with traffic shaping middleware

### Server Functions

#### `serve_router`

```rust
pub async fn serve_router(
    port: u16,
    app: Router,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
```

Starts the HTTP server on the specified port.

**Parameters:**
- `port`: Port number to bind to
- `app`: Axum router to serve

**Returns:** `Result<(), Error>` indicating server startup success

**Errors:**
- Port binding failures
- Server startup errors

#### `start`

```rust
pub async fn start(
    port: u16,
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
```

Convenience function to build and start an HTTP server.

**Parameters:**
- `port`: Port number to bind to
- `spec_path`: Optional path to OpenAPI specification file
- `options`: Optional validation options

#### `start_with_auth_and_latency`

```rust
pub async fn start_with_auth_and_latency(
    port: u16,
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    auth_config: Option<AuthConfig>,
    latency_profile: Option<LatencyProfile>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
```

Starts HTTP server with authentication and latency simulation.

**Parameters:**
- `port`: Port number to bind to
- `spec_path`: Optional path to OpenAPI specification file
- `options`: Optional validation options
- `auth_config`: Authentication configuration
- `latency_profile`: Latency injection profile

### Management API

#### `management_router`

```rust
pub fn management_router(state: ManagementState) -> Router
```

Creates a management API router for server control and monitoring.

**Parameters:**
- `state`: Management state containing server statistics and configuration

**Returns:** Axum `Router` with management endpoints

**Endpoints:**
- `GET /health` - Health check
- `GET /stats` - Server statistics
- `GET /routes` - Route information
- `GET /coverage` - API coverage metrics
- `GET/POST/PUT/DELETE /mocks` - Mock management

#### `management_ws_router`

```rust
pub fn ws_management_router(state: WsManagementState) -> Router
```

Creates a WebSocket management router for real-time monitoring.

**Parameters:**
- `state`: WebSocket management state

**Returns:** Axum `Router` with WebSocket management endpoints

### AI Integration

#### `process_response_with_ai`

```rust
pub async fn process_response_with_ai(
    response_body: Option<Value>,
    intelligent_config: Option<Value>,
    drift_config: Option<Value>,
) -> Result<Value>
```

Processes a response body using AI features if configured.

**Parameters:**
- `response_body`: Base response body as JSON Value
- `intelligent_config`: Intelligent mock generation configuration
- `drift_config`: Data drift simulation configuration

**Returns:** `Result<Value, Error>` with processed response

**Example:**
```rust
use mockforge_http::process_response_with_ai;
use serde_json::json;

let config = json!({
    "enabled": true,
    "prompt": "Generate realistic user data"
});

let response = process_response_with_ai(
    Some(json!({"name": "John"})),
    Some(config),
    None,
).await?;
```

### Data Structures

#### `HttpServerState`

```rust
pub struct HttpServerState {
    pub routes: Vec<RouteInfo>,
    pub rate_limiter: Option<Arc<GlobalRateLimiter>>,
}
```

Shared state for HTTP server route information and rate limiting.

**Fields:**
- `routes`: Vector of route information
- `rate_limiter`: Optional global rate limiter

**Methods:**
```rust
impl HttpServerState {
    pub fn new() -> Self
    pub fn with_routes(routes: Vec<RouteInfo>) -> Self
    pub fn with_rate_limiter(rate_limiter: Arc<GlobalRateLimiter>) -> Self
}
```

#### `RouteInfo`

```rust
pub struct RouteInfo {
    pub method: String,
    pub path: String,
    pub operation_id: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub parameters: Vec<String>,
}
```

Information about an HTTP route.

**Fields:**
- `method`: HTTP method (GET, POST, etc.)
- `path`: Route path pattern
- `operation_id`: Optional OpenAPI operation ID
- `summary`: Optional route summary
- `description`: Optional route description
- `parameters`: List of parameter names

#### `ManagementState`

```rust
pub struct ManagementState {
    pub mocks: Arc<RwLock<Vec<MockConfig>>>,
    pub spec: Option<Arc<OpenApiSpec>>,
    pub spec_path: Option<String>,
    pub port: u16,
    pub start_time: Instant,
    pub request_counter: Arc<RwLock<u64>>,
}
```

State for the management API.

**Fields:**
- `mocks`: Thread-safe vector of mock configurations
- `spec`: Optional OpenAPI specification
- `spec_path`: Optional path to spec file
- `port`: Server port
- `start_time`: Server startup timestamp
- `request_counter`: Request counter for statistics

**Methods:**
```rust
impl ManagementState {
    pub fn new(
        spec: Option<Arc<OpenApiSpec>>,
        spec_path: Option<String>,
        port: u16,
    ) -> Self
}
```

#### `MockConfig`

```rust
pub struct MockConfig {
    pub id: String,
    pub name: String,
    pub method: String,
    pub path: String,
    pub response: MockResponse,
    pub enabled: bool,
    pub latency_ms: Option<u64>,
    pub status_code: Option<u16>,
}
```

Configuration for a mock endpoint.

**Fields:**
- `id`: Unique mock identifier
- `name`: Human-readable name
- `method`: HTTP method
- `path`: Route path
- `response`: Mock response configuration
- `enabled`: Whether mock is active
- `latency_ms`: Optional latency injection
- `status_code`: Optional status code override

#### `MockResponse`

```rust
pub struct MockResponse {
    pub body: Value,
    pub headers: Option<HashMap<String, String>>,
}
```

Mock response configuration.

**Fields:**
- `body`: JSON response body
- `headers`: Optional HTTP headers

#### `ServerStats`

```rust
pub struct ServerStats {
    pub uptime_seconds: u64,
    pub total_requests: u64,
    pub active_mocks: usize,
    pub enabled_mocks: usize,
    pub registered_routes: usize,
}
```

Server statistics.

**Fields:**
- `uptime_seconds`: Server uptime in seconds
- `total_requests`: Total requests processed
- `active_mocks`: Number of configured mocks
- `enabled_mocks`: Number of enabled mocks
- `registered_routes`: Number of registered routes

#### `ServerConfig`

```rust
pub struct ServerConfig {
    pub version: String,
    pub port: u16,
    pub has_openapi_spec: bool,
    pub spec_path: Option<String>,
}
```

Server configuration information.

**Fields:**
- `version`: MockForge version
- `port`: Server port
- `has_openapi_spec`: Whether OpenAPI spec is loaded
- `spec_path`: Optional path to spec file

### AI Types

#### `AiResponseConfig`

```rust
pub struct AiResponseConfig {
    pub enabled: bool,
    pub rag_config: RagConfig,
    pub prompt: String,
    pub schema: Option<Value>,
}
```

Configuration for AI-powered response generation.

**Fields:**
- `enabled`: Whether AI responses are enabled
- `rag_config`: RAG (Retrieval-Augmented Generation) configuration
- `prompt`: AI generation prompt
- `schema`: Optional response schema

#### `AiResponseHandler`

```rust
pub struct AiResponseHandler { /* fields omitted */ }
```

Handler for AI-powered response generation.

**Methods:**
```rust
impl AiResponseHandler {
    pub fn new(
        intelligent_config: Option<IntelligentMockConfig>,
        drift_config: Option<DataDriftConfig>,
    ) -> Result<Self>

    pub fn is_enabled(&self) -> bool

    pub async fn generate_response(&mut self, base_response: Option<Value>) -> Result<Value>

    pub async fn reset_drift(&self)
}
```

### Coverage Types

#### `CoverageReport`

```rust
pub struct CoverageReport {
    pub routes: HashMap<String, RouteCoverage>,
    pub total_routes: usize,
    pub covered_routes: usize,
    pub coverage_percentage: f64,
}
```

API coverage report.

**Fields:**
- `routes`: Coverage data per route
- `total_routes`: Total number of routes
- `covered_routes`: Number of covered routes
- `coverage_percentage`: Coverage percentage (0.0-100.0)

#### `RouteCoverage`

```rust
pub struct RouteCoverage {
    pub method: String,
    pub path: String,
    pub methods: HashMap<String, MethodCoverage>,
    pub total_requests: u64,
    pub covered_methods: usize,
}
```

Coverage information for a specific route.

**Fields:**
- `method`: HTTP method
- `path`: Route path
- `methods`: Coverage per HTTP method
- `total_requests`: Total requests to this route
- `covered_methods`: Number of methods with coverage

#### `MethodCoverage`

```rust
pub struct MethodCoverage {
    pub request_count: u64,
    pub response_codes: HashMap<u16, u64>,
    pub last_request: Option<DateTime<Utc>>,
}
```

Coverage information for a specific HTTP method.

**Fields:**
- `request_count`: Number of requests
- `response_codes`: Response code distribution
- `last_request`: Timestamp of last request

### Coverage Functions

#### `calculate_coverage`

```rust
pub fn calculate_coverage(
    routes: &[RouteInfo],
    request_logs: &[RequestLogEntry],
) -> CoverageReport
```

Calculates API coverage from route information and request logs.

**Parameters:**
- `routes`: Available routes
- `request_logs`: Historical request logs

**Returns:** `CoverageReport` with coverage statistics

#### `get_coverage_handler`

```rust
pub async fn get_coverage_handler(State(state): State<HttpServerState>) -> Json<Value>
```

Axum handler for coverage endpoint.

**Returns:** JSON response with coverage data

### Middleware Functions

#### `collect_http_metrics`

```rust
pub fn collect_http_metrics(request: &Request, response: &Response, duration: Duration)
```

Collects HTTP metrics for observability.

**Parameters:**
- `request`: HTTP request
- `response`: HTTP response
- `duration`: Request processing duration

#### `http_tracing_middleware`

```rust
pub fn http_tracing_middleware(
    request: Request,
    next: Next,
) -> impl Future<Output = Response>
```

Middleware for HTTP request tracing.

**Parameters:**
- `request`: Incoming HTTP request
- `next`: Next middleware in chain

**Returns:** Future resolving to HTTP response

### Error Types

All functions return `Result<T, Box<dyn std::error::Error + Send + Sync>>` for error handling. Common errors include:

- File I/O errors (spec file reading)
- JSON parsing errors
- Server binding errors
- Validation errors
- AI service errors

### Constants

- `DEFAULT_RATE_LIMIT_RPM`: Default requests per minute (1000)
- `DEFAULT_RATE_LIMIT_BURST`: Default burst size (2000)

### Feature Flags

- `data-faker`: Enables rich data generation features

## Examples

### Basic HTTP Server

```rust
use mockforge_http::build_router;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = build_router(
        Some("./api.yaml".to_string()),
        None,
        None,
    ).await;

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, router).await?;
    Ok(())
}
```

### Server with Management API

```rust
use mockforge_http::{build_router, management_router, ManagementState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build main router
    let app = build_router(None, None, None).await;

    // Add management API
    let mgmt_state = ManagementState::new(None, None, 3000);
    let mgmt_router = management_router(mgmt_state);

    let app = app.nest("/__mockforge", mgmt_router);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

### AI-Powered Responses

```rust
use mockforge_http::{AiResponseConfig, process_response_with_ai};
use mockforge_data::RagConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ai_config = AiResponseConfig {
        enabled: true,
        rag_config: RagConfig {
            provider: "openai".to_string(),
            model: "gpt-3.5-turbo".to_string(),
            api_key: Some("sk-...".to_string()),
            ..Default::default()
        },
        prompt: "Generate realistic user data".to_string(),
        schema: None,
    };

    let response = process_response_with_ai(
        Some(serde_json::json!({"id": 1})),
        Some(serde_json::to_value(ai_config)?),
        None,
    ).await?;

    println!("AI response: {}", response);
    Ok(())
}
```