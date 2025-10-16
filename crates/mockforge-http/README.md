# MockForge HTTP

HTTP/REST protocol support for MockForge with OpenAPI integration, AI-powered responses, and comprehensive management APIs.

This crate provides full-featured HTTP mocking capabilities including automatic OpenAPI spec generation, request validation, AI-powered intelligent responses, real-time monitoring, and extensive management endpoints. Perfect for API development, testing, and microservice simulation.

## Features

- **OpenAPI Integration**: Auto-generate mock endpoints from OpenAPI/Swagger specs
- **Request Validation**: Schema-based validation with configurable enforcement
- **AI-Powered Responses**: Generate contextual responses using LLMs and RAG
- **Management API**: REST and WebSocket APIs for real-time monitoring and control
- **Server-Sent Events**: Stream logs, metrics, and events to clients
- **Request Logging**: Comprehensive HTTP request/response logging
- **Metrics Collection**: Prometheus-compatible performance metrics
- **Authentication**: JWT, OAuth2, and custom auth middleware
- **Rate Limiting**: Configurable request throttling
- **Coverage Tracking**: API endpoint usage and coverage reporting

## Quick Start

### Basic HTTP Server from OpenAPI

```rust,no_run
use mockforge_http::build_router;
use mockforge_core::ValidationOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build router from OpenAPI specification
    let router = build_router(
        Some("./api-spec.json".to_string()),
        Some(ValidationOptions::enforce()),
        None,
    ).await;

    // Start the server
    let addr = "0.0.0.0:3000".parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
```

### Server with Management API

```rust,no_run
use mockforge_http::{build_router, management_router, ManagementState, ServerStats};
use mockforge_core::ValidationOptions;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build main API router
    let api_router = build_router(
        Some("./api.yaml".to_string()),
        Some(ValidationOptions::warn()),
        None,
    ).await;

    // Create management state
    let stats = Arc::new(RwLock::new(ServerStats::default()));
    let mgmt_state = ManagementState::new(stats);

    // Build management router
    let mgmt_router = management_router(mgmt_state);

    // Combine routers
    let app = api_router.nest("/__mockforge", mgmt_router);

    // Start server
    let addr = "0.0.0.0:3000".parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("🚀 MockForge HTTP server running at http://{}", addr);
    println!("📊 Management API at http://{}/__mockforge", addr);
    axum::serve(listener, app).await?;

    Ok(())
}
```

## OpenAPI Integration

### Automatic Endpoint Generation

MockForge HTTP automatically generates mock endpoints from OpenAPI specifications:

```yaml
# api.yaml
openapi: 3.0.0
info:
  title: User API
  version: 1.0.0
paths:
  /users:
    get:
      summary: Get users
      parameters:
        - name: limit
          in: query
          schema:
            type: integer
            default: 10
      responses:
        '200':
          description: Success
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/User'
    post:
      summary: Create user
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CreateUserRequest'
      responses:
        '201':
          description: Created
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/User'

components:
  schemas:
    User:
      type: object
      properties:
        id:
          type: string
        name:
          type: string
        email:
          type: string
    CreateUserRequest:
      type: object
      required:
        - name
        - email
      properties:
        name:
          type: string
        email:
          type: string
```

### Request Validation

Configure validation behavior:

```rust,no_run
use mockforge_core::ValidationOptions;

// Strict validation - reject invalid requests
let strict_validation = ValidationOptions::enforce();

// Lenient validation - log warnings but allow
let lenient_validation = ValidationOptions::warn();

// No validation
let no_validation = ValidationOptions::none();

// Custom validation
let custom_validation = ValidationOptions {
    request_validation: true,
    response_validation: false,
    coerce_types: true,
    ..Default::default()
};
```

## AI-Powered Responses

Generate intelligent, contextually aware responses:

```rust,no_run
use mockforge_http::{AiResponseConfig, process_response_with_ai};
use mockforge_data::RagConfig;

let ai_config = AiResponseConfig {
    enabled: true,
    rag_config: RagConfig {
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        api_key: Some("sk-...".to_string()),
        ..Default::default()
    },
    prompt: "Generate realistic user profile data with appropriate relationships".to_string(),
    schema: Some(user_schema), // Optional JSON schema for response structure
};

// Process request with AI
let response = process_response_with_ai(&ai_config, request_data).await?;
```

### RAG Integration

Use Retrieval-Augmented Generation for enhanced responses:

```rust,no_run
use mockforge_data::{RagEngine, RagConfig};

// Configure RAG engine
let rag_config = RagConfig {
    provider: "openai".to_string(),
    model: "gpt-4".to_string(),
    api_key: Some("sk-...".to_string()),
    semantic_search_enabled: true,
    similarity_threshold: 0.8,
    ..Default::default()
};

let mut rag_engine = RagEngine::new(rag_config);

// Add context documents
rag_engine.add_document("api_docs", "Users have id, name, email, and profile fields...")?;
rag_engine.add_document("business_rules", "User emails must be unique and valid...")?;

// Generate responses with context
let response = rag_engine.generate_with_rag(&schema, &config).await?;
```

## Management API

### REST Endpoints

Access comprehensive server information and control:

```bash
# Health check
curl http://localhost:3000/__mockforge/health

# Server statistics
curl http://localhost:3000/__mockforge/stats

# List fixtures
curl http://localhost:3000/__mockforge/fixtures

# Update configuration
curl -X POST http://localhost:3000/__mockforge/config/validation \
  -H "Content-Type: application/json" \
  -d '{"request_validation": true, "response_validation": false}'
```

### WebSocket Management

Real-time monitoring via WebSocket:

```javascript
// Connect to management WebSocket
const ws = new WebSocket('ws://localhost:3000/__mockforge/ws');

// Listen for events
ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    console.log('Event:', data.type, data.payload);
};

// Events include:
// - request_logged: New request received
// - response_sent: Response sent
// - config_updated: Configuration changed
// - stats_updated: Statistics updated
};
```

### Server-Sent Events

Stream logs and metrics:

```javascript
// Connect to SSE endpoint
const eventSource = new EventSource('http://localhost:3000/__mockforge/logs');

// Listen for log events
eventSource.onmessage = (event) => {
    const log = JSON.parse(event.data);
    console.log('Log:', log.level, log.message);
};
```

## Authentication & Authorization

### JWT Authentication

```rust,no_run
use mockforge_http::auth::{JwtAuth, JwtConfig};

let jwt_config = JwtConfig {
    secret: "your-secret-key".to_string(),
    issuer: Some("mockforge".to_string()),
    audience: Some("api".to_string()),
    ..Default::default()
};

let jwt_auth = JwtAuth::new(jwt_config);

// Use as middleware
let app = Router::new()
    .route("/protected", get(handler))
    .layer(from_fn_with_state(jwt_auth, auth_middleware));
```

### OAuth2 Integration

```rust,no_run
use mockforge_http::auth::oauth2::{OAuth2Config, OAuth2Middleware};

let oauth_config = OAuth2Config {
    client_id: "your-client-id".to_string(),
    client_secret: "your-client-secret".to_string(),
    auth_url: "https://auth.example.com".to_string(),
    token_url: "https://auth.example.com/token".to_string(),
    scopes: vec!["read".to_string(), "write".to_string()],
};

let oauth_middleware = OAuth2Middleware::new(oauth_config);
```

## Request/Response Processing

### Middleware Stack

MockForge HTTP includes a comprehensive middleware stack:

```rust,no_run
use mockforge_http::{
    http_tracing_middleware,
    metrics_middleware,
    op_middleware,
    request_logging,
};

let app = Router::new()
    .route("/api/*path", get(handler))
    // Tracing middleware
    .layer(http_tracing_middleware())
    // Metrics collection
    .layer(metrics_middleware())
    // OpenAPI operation tracking
    .layer(op_middleware())
    // Request logging
    .layer(request_logging());
```

### Latency Simulation

```rust,no_run
use mockforge_http::latency_profiles::LatencyProfile;

// Fixed delay
let fixed_latency = LatencyProfile::with_fixed_delay(500); // 500ms

// Variable delay
let variable_latency = LatencyProfile::with_range(100, 2000); // 100-2000ms

// Realistic network simulation
let network_latency = LatencyProfile::network(); // Simulates real network conditions
```

### Request Chaining

Create multi-step request workflows:

```rust,no_run
use mockforge_http::chain_handlers::{ChainHandler, ChainRequest};

let chain = ChainHandler::new(vec![
    ChainRequest {
        method: "POST".to_string(),
        path: "/users".to_string(),
        body: Some(r#"{"name": "John"}"#.to_string()),
        extract: vec![("user_id".to_string(), "$.id".to_string())],
    },
    ChainRequest {
        method: "POST".to_string(),
        path: "/users/{{user_id}}/posts".to_string(),
        body: Some(r#"{"title": "Hello", "content": "World"}"#.to_string()),
    },
]);

// Execute chain
let result = chain.execute().await?;
```

## Metrics & Monitoring

### Prometheus Metrics

```rust,no_run
use mockforge_http::metrics_middleware;

// Metrics are automatically exposed at /metrics
// Available metrics:
// - http_requests_total
// - http_request_duration_seconds
// - http_response_size_bytes
// - mockforge_active_connections
// - mockforge_memory_usage
```

### Coverage Reporting

Track API usage and test coverage:

```rust,no_run
use mockforge_http::coverage::{calculate_coverage, CoverageReport};

// Calculate coverage from request logs
let report = calculate_coverage(request_logs).await?;

// Coverage includes:
// - Route coverage (endpoints hit)
// - Method coverage (HTTP methods used)
// - Parameter coverage (query/path params used)
// - Response code coverage
println!("API Coverage: {:.1}%", report.overall_coverage * 100.0);
```

## Advanced Features

### Template-Based Responses

Use Handlebars templates for dynamic responses:

```rust,no_run
use mockforge_http::templating::TemplateEngine;

// Template with variables
let template = r#"
{
  "id": "{{uuid}}",
  "name": "{{faker.name}}",
  "email": "{{faker.email}}",
  "timestamp": "{{now}}"
}
"#;

let engine = TemplateEngine::new();
let response = engine.render(template, &context)?;
```

### Failure Injection

Simulate various failure scenarios:

```rust,no_run
use mockforge_core::failure_injection::{FailureConfig, FailureInjector};

let failure_config = FailureConfig {
    global_error_rate: 0.1, // 10% failure rate
    default_status_codes: vec![500, 502, 503],
    timeout_rate: 0.05,    // 5% timeouts
    ..Default::default()
};

let injector = FailureInjector::new(failure_config);
// Automatically injects failures into responses
```

### Rate Limiting

Control request throughput:

```rust,no_run
use mockforge_http::middleware::rate_limit::{RateLimitConfig, RateLimiter};

let rate_limit = RateLimitConfig {
    requests_per_second: 100,
    burst_size: 20,
    per_ip: true,
    per_endpoint: false,
};

let limiter = RateLimiter::new(rate_limit);
// Applied as middleware to routes
```

## Testing with MockForge HTTP

### Integration Testing

```rust,no_run
use reqwest::Client;
use mockforge_http::build_router;

#[tokio::test]
async fn test_user_api() {
    // Start test server
    let router = build_router(Some("test-api.yaml".to_string()), None, None).await;
    let server = TestServer::new(router).unwrap();

    // Test client
    let client = Client::new();

    // Create user
    let response = client
        .post(server.url("/users"))
        .json(&serde_json::json!({
            "name": "John Doe",
            "email": "john@example.com"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 201);

    let user: serde_json::Value = response.json().await.unwrap();
    assert!(user.get("id").is_some());

    // Get user
    let user_id = user["id"].as_str().unwrap();
    let response = client
        .get(server.url(&format!("/users/{}", user_id)))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}
```

### Load Testing

```rust,no_run
use mockforge_http::build_router;
use tokio::task;

#[tokio::test]
async fn load_test_api() {
    // Start server
    let router = build_router(Some("api.yaml".to_string()), None, None).await;
    let server = TestServer::new(router).unwrap();

    // Spawn multiple concurrent requests
    let mut handles = vec![];
    for i in 0..100 {
        let url = server.url("/api/test");
        let handle = task::spawn(async move {
            let client = Client::new();
            let response = client.get(&url).send().await.unwrap();
            assert_eq!(response.status(), 200);
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        handle.await.unwrap();
    }
}
```

## Configuration

### Server Configuration

```yaml
# mockforge.yaml
http:
  port: 3000
  openapi_spec: "api.yaml"
  validation:
    request: true
    response: false
    coerce_types: true

management:
  enabled: true
  port: 9080

metrics:
  enabled: true
  prometheus:
    port: 9090

ai:
  enabled: true
  rag:
    provider: "openai"
    model: "gpt-4"
    api_key: "sk-..."

auth:
  jwt:
    enabled: true
    secret: "your-secret"
    issuer: "mockforge"
```

### Environment Variables

```bash
# Server
export MOCKFORGE_HTTP_PORT=3000
export MOCKFORGE_OPENAPI_SPEC=api.yaml

# AI
export MOCKFORGE_AI_ENABLED=true
export MOCKFORGE_RAG_PROVIDER=openai
export MOCKFORGE_RAG_API_KEY=sk-...

# Auth
export MOCKFORGE_JWT_SECRET=your-secret-key
```

## Performance Considerations

- **Memory Usage**: Large OpenAPI specs may increase memory usage
- **Validation Overhead**: Request/response validation adds processing time
- **AI Processing**: RAG and LLM calls introduce latency
- **Metrics Collection**: Enable only required metrics for production
- **Connection Pooling**: Configure appropriate connection limits

## Examples

### Complete Server Setup

```rust,no_run
use axum::{routing::get, Router};
use mockforge_http::{
    build_router, management_router, ManagementState, ServerStats,
    http_tracing_middleware, metrics_middleware,
};
use mockforge_core::ValidationOptions;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build main API router from OpenAPI spec
    let api_router = build_router(
        Some("./api/openapi.yaml".to_string()),
        Some(ValidationOptions::warn()),
        None,
    ).await;

    // Create management state
    let stats = Arc::new(RwLock::new(ServerStats::default()));
    let mgmt_state = ManagementState::new(stats.clone());
    let mgmt_router = management_router(mgmt_state);

    // Combine routers with middleware
    let app = Router::new()
        .merge(api_router)
        .nest("/__mockforge", mgmt_router)
        .layer(CorsLayer::permissive())
        .layer(http_tracing_middleware())
        .layer(metrics_middleware());

    // Start server
    let addr = "0.0.0.0:3000".parse()?;
    println!("🚀 MockForge HTTP server running at http://{}", addr);
    println!("📖 OpenAPI docs at http://{}/docs", addr);
    println!("📊 Management API at http://{}/__mockforge", addr);
    println!("📈 Metrics at http://{}/metrics", addr);

    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;

    Ok(())
}
```

## Troubleshooting

### Common Issues

**OpenAPI validation errors:**
- Check schema syntax and references
- Ensure all required fields are defined
- Validate against OpenAPI specification

**AI response generation failures:**
- Verify API keys and network connectivity
- Check model availability and rate limits
- Review prompt and schema configurations

**Performance issues:**
- Profile with metrics endpoint
- Check validation overhead
- Optimize AI configuration for production

## Related Crates

- [`mockforge-core`](https://docs.rs/mockforge-core): Core mocking functionality
- [`mockforge-data`](https://docs.rs/mockforge-data): Synthetic data generation
- [`mockforge-observability`](https://docs.rs/mockforge-observability): Metrics and monitoring

## License

Licensed under MIT OR Apache-2.0
