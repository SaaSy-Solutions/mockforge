# Protocol Abstraction Layer

This document describes the protocol abstraction layer in MockForge, which provides unified interfaces for working with multiple protocols (HTTP, GraphQL, gRPC, WebSocket).

## Overview

The protocol abstraction layer solves the problem of code duplication and inconsistency across different protocol implementations. It provides:

1. **Unified Spec-Driven Mocking**: A common interface for loading specs (OpenAPI, GraphQL schemas, Proto files) and generating mock responses
2. **Protocol-Agnostic Middleware**: Middleware that works across all protocols (logging, metrics, latency injection)
3. **Consistent Request/Response Models**: Normalized request and response types that work across protocols

## Architecture

### Core Types

#### `Protocol` Enum

Represents the protocol type:

```rust
pub enum Protocol {
    Http,      // HTTP/REST
    GraphQL,   // GraphQL
    Grpc,      // gRPC
    WebSocket, // WebSocket
}
```

#### `ProtocolRequest`

A unified request representation:

```rust
pub struct ProtocolRequest {
    pub protocol: Protocol,
    pub operation: String,              // "GET", "Query.users", "greeter.SayHello"
    pub path: String,                   // "/users", "/graphql", "/"
    pub metadata: HashMap<String, String>, // Headers, metadata, etc.
    pub body: Option<Vec<u8>>,          // Request payload
    pub client_ip: Option<String>,
}
```

#### `ProtocolResponse`

A unified response representation:

```rust
pub struct ProtocolResponse {
    pub status: ResponseStatus,
    pub metadata: HashMap<String, String>,
    pub body: Vec<u8>,
    pub content_type: String,
}
```

#### `ResponseStatus`

Protocol-agnostic status representation:

```rust
pub enum ResponseStatus {
    HttpStatus(u16),      // HTTP status codes (200, 404, etc.)
    GrpcStatus(i32),      // gRPC status codes (0 = OK)
    GraphQLStatus(bool),  // Success/failure
    WebSocketStatus(bool),
}
```

### Traits

#### `SpecRegistry` Trait

Interface for spec-driven mocking registries:

```rust
pub trait SpecRegistry: Send + Sync {
    fn protocol(&self) -> Protocol;
    fn operations(&self) -> Vec<SpecOperation>;
    fn find_operation(&self, operation: &str, path: &str) -> Option<SpecOperation>;
    fn validate_request(&self, request: &ProtocolRequest) -> Result<ValidationResult>;
    fn generate_mock_response(&self, request: &ProtocolRequest) -> Result<ProtocolResponse>;
}
```

**Future Implementations:**
- `OpenApiSpecRegistry` - Already exists as `OpenApiRouteRegistry` (can be adapted)
- `GraphQLSchemaRegistry` - Generate mocks from GraphQL schema files
- `GrpcProtoRegistry` - Generate mocks from .proto files

#### `ProtocolMiddleware` Trait

Interface for protocol-agnostic middleware:

```rust
#[async_trait::async_trait]
pub trait ProtocolMiddleware: Send + Sync {
    fn name(&self) -> &str;

    async fn process_request(&self, request: &mut ProtocolRequest) -> Result<()>;

    async fn process_response(
        &self,
        request: &ProtocolRequest,
        response: &mut ProtocolResponse,
    ) -> Result<()>;

    fn supports_protocol(&self, protocol: Protocol) -> bool;
}
```

## Built-in Middleware

### LoggingMiddleware

Logs requests and responses across all protocols:

```rust
use mockforge_core::protocol_abstraction::{LoggingMiddleware, ProtocolMiddleware};

let logging = LoggingMiddleware::new(true); // log_bodies = true

// Use in middleware chain
let mut request = ProtocolRequest { /* ... */ };
logging.process_request(&mut request).await?;
```

Features:
- Logs to centralized request logger
- Protocol-specific log entry creation (HTTP, gRPC, GraphQL, WebSocket)
- Tracks request duration
- Includes request/response sizes

### MetricsMiddleware

Collects metrics for all protocols:

```rust
use mockforge_core::protocol_abstraction::MetricsMiddleware;

let metrics = MetricsMiddleware::new();

// Automatically logs:
// - Request duration
// - Status codes
// - Response sizes
// - Success/failure rates
```

### LatencyMiddleware

Injects latency across all protocols:

```rust
use mockforge_core::protocol_abstraction::LatencyMiddleware;
use mockforge_core::{LatencyProfile, latency::LatencyInjector};

let profile = LatencyProfile::new(100, 25); // 100ms base, 25ms jitter
let injector = LatencyInjector::new(profile, Default::default());
let latency = LatencyMiddleware::new(injector);
```

## Middleware Chain

Compose multiple middleware:

```rust
use mockforge_core::protocol_abstraction::{
    MiddlewareChain, LoggingMiddleware, MetricsMiddleware, LatencyMiddleware,
};
use std::sync::Arc;

let chain = MiddlewareChain::new()
    .with_middleware(Arc::new(LoggingMiddleware::new(false)))
    .with_middleware(Arc::new(MetricsMiddleware::new()))
    .with_middleware(Arc::new(LatencyMiddleware::new(injector)));

// Process request through chain
chain.process_request(&mut request).await?;

// ... handle request ...

// Process response through chain (in reverse order)
chain.process_response(&request, &mut response).await?;
```

## Benefits

### 1. Code Reuse

Before (duplicated logging in each protocol):

```rust
// In mockforge-http
async fn log_http_request(...) { /* HTTP-specific logging */ }

// In mockforge-grpc
async fn log_grpc_request(...) { /* gRPC-specific logging */ }

// In mockforge-graphql
async fn log_graphql_request(...) { /* GraphQL-specific logging */ }
```

After (unified middleware):

```rust
// Single implementation works for all protocols
let logging = LoggingMiddleware::new(true);
```

### 2. Consistency

All protocols now have:
- Standardized logging format
- Consistent metrics collection
- Uniform latency injection
- Same validation patterns

### 3. Easier Testing

Test middleware once, works everywhere:

```rust
#[tokio::test]
async fn test_logging_middleware() {
    let middleware = LoggingMiddleware::new(false);

    // Test with HTTP
    let mut http_request = ProtocolRequest {
        protocol: Protocol::Http,
        operation: "GET".to_string(),
        // ...
    };
    assert!(middleware.process_request(&mut http_request).await.is_ok());

    // Same middleware works with gRPC
    let mut grpc_request = ProtocolRequest {
        protocol: Protocol::Grpc,
        operation: "greeter.SayHello".to_string(),
        // ...
    };
    assert!(middleware.process_request(&mut grpc_request).await.is_ok());
}
```

### 4. Extensibility

Easy to add new middleware that works across all protocols:

```rust
pub struct AuthMiddleware {
    // ...
}

#[async_trait::async_trait]
impl ProtocolMiddleware for AuthMiddleware {
    fn name(&self) -> &str { "AuthMiddleware" }

    async fn process_request(&self, request: &mut ProtocolRequest) -> Result<()> {
        // Extract auth token from metadata
        let token = request.metadata.get("authorization");
        // Validate token
        // Works for HTTP headers, gRPC metadata, GraphQL headers, WS headers
        Ok(())
    }

    // ... other methods ...
}
```

## Migration Guide

### For HTTP (OpenAPI)

Current code:
```rust
let registry = OpenApiRouteRegistry::new(spec);
let router = registry.build_router();
```

Future (with abstraction):
```rust
let registry: Arc<dyn SpecRegistry> = Arc::new(OpenApiSpecRegistry::new(spec));
let middleware_chain = MiddlewareChain::new()
    .with_middleware(Arc::new(LoggingMiddleware::new(false)))
    .with_middleware(Arc::new(MetricsMiddleware::new()));
```

### For GraphQL

Current code (hardcoded schema):
```rust
let schema = GraphQLSchema::new();
```

Future (spec-driven):
```rust
// Load schema from file
let registry: Arc<dyn SpecRegistry> =
    Arc::new(GraphQLSchemaRegistry::from_file("schema.graphql")?);

// Generate resolvers from schema
let schema = registry.generate_schema()?;
```

### For gRPC

Current code:
```rust
let service = DynamicGrpcService::new(proto_service, latency_injector);
```

Future (unified):
```rust
let registry: Arc<dyn SpecRegistry> =
    Arc::new(GrpcProtoRegistry::from_file("service.proto")?);

let middleware = MiddlewareChain::new()
    .with_middleware(Arc::new(LatencyMiddleware::new(injector)))
    .with_middleware(Arc::new(LoggingMiddleware::new(true)));
```

## Future Enhancements

1. **Unified Authentication Middleware**
   - JWT validation across all protocols
   - API key validation
   - OAuth2 token introspection

2. **Spec-Driven GraphQL Registry**
   - Load GraphQL schemas from files
   - Generate dynamic resolvers
   - Validate queries against schema

3. **Spec-Driven gRPC Registry**
   - Leverage existing `ProtoParser`
   - Create `SpecRegistry` implementation
   - Generate type-safe mocks

4. **Request Matching Abstraction**
   - Unified request fingerprinting
   - Cross-protocol caching
   - Consistent replay/record

5. **Response Transformation Middleware**
   - Protocol conversion (HTTP → gRPC, etc.)
   - Response templating
   - Data masking/redaction

## Example: Creating Custom Middleware

```rust
use mockforge_core::protocol_abstraction::{
    Protocol, ProtocolMiddleware, ProtocolRequest, ProtocolResponse,
};
use mockforge_core::Result;

pub struct CacheMiddleware {
    cache: Arc<RwLock<HashMap<String, ProtocolResponse>>>,
}

impl CacheMiddleware {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn cache_key(&self, request: &ProtocolRequest) -> String {
        format!("{}:{}:{}", request.protocol, request.operation, request.path)
    }
}

#[async_trait::async_trait]
impl ProtocolMiddleware for CacheMiddleware {
    fn name(&self) -> &str {
        "CacheMiddleware"
    }

    async fn process_request(&self, request: &mut ProtocolRequest) -> Result<()> {
        // Check if response is cached
        let key = self.cache_key(request);
        if let Some(_cached) = self.cache.read().await.get(&key) {
            request.metadata.insert("x-cache".to_string(), "HIT".to_string());
        } else {
            request.metadata.insert("x-cache".to_string(), "MISS".to_string());
        }
        Ok(())
    }

    async fn process_response(
        &self,
        request: &ProtocolRequest,
        response: &mut ProtocolResponse,
    ) -> Result<()> {
        // Cache successful responses
        if response.status.is_success() {
            let key = self.cache_key(request);
            self.cache.write().await.insert(key, response.clone());
        }
        Ok(())
    }

    fn supports_protocol(&self, _protocol: Protocol) -> bool {
        true // Support all protocols
    }
}
```

## Testing

Run tests for the protocol abstraction layer:

```bash
cargo test -p mockforge-core protocol_abstraction
```

## See Also

- [Architecture Documentation](./ARCHITECTURE.md)
- [Middleware Guide](./MIDDLEWARE.md)
- [OpenAPI Integration](./OPENAPI.md)
- [gRPC Support](./GRPC.md)
- [GraphQL Support](./GRAPHQL.md)
