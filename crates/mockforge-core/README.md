# MockForge Core

Core functionality and shared logic for the MockForge mocking framework.

This crate provides the foundational building blocks used across all MockForge protocols
(HTTP, WebSocket, gRPC, GraphQL). It can be used as a library to programmatically create
and manage mock servers, or to build custom mocking solutions.

## Overview

MockForge Core includes:

- **Routing & Validation**: OpenAPI-based route registration and request validation
- **Request/Response Processing**: Template expansion, data generation, and transformation
- **Chaos Engineering**: Latency injection, failure simulation, and traffic shaping
- **Proxy & Hybrid Mode**: Forward requests to real backends with intelligent fallback
- **Request Chaining**: Multi-step request workflows with context passing
- **Workspace Management**: Organize and persist mock configurations
- **Observability**: Request logging, metrics collection, and tracing

## Quick Start: Embedding MockForge

### Creating a Simple HTTP Mock Server

```rust,no_run
use mockforge_core::{
    OpenApiSpec, OpenApiRouteRegistry, ValidationOptions,
    LatencyProfile, Config,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load OpenAPI specification
    let spec = OpenApiSpec::from_file("api.json").await?;

    // Create route registry with validation
    let registry = OpenApiRouteRegistry::new(spec, ValidationOptions::default());

    // Configure core features
    let config = Config {
        latency_enabled: true,
        failures_enabled: false,
        default_latency: LatencyProfile::normal(),
        ..Default::default()
    };

    // Build your HTTP server with the registry
    // (See mockforge-http crate for router building)

    Ok(())
}
```

### Request Chaining

Chain multiple requests together with shared context:

```rust,no_run
use mockforge_core::{ChainDefinition, ChainRequest, RequestChainRegistry};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut registry = RequestChainRegistry::new();

// Define a chain: create user → add to group → verify membership
let chain = ChainDefinition {
    name: "user_onboarding".to_string(),
    steps: vec![
        ChainRequest {
            method: "POST".to_string(),
            path: "/users".to_string(),
            body: Some(r#"{"name": "{{faker.name}}"}"#.to_string()),
            extract: vec![("user_id".to_string(), "$.id".to_string())],
            ..Default::default()
        },
        ChainRequest {
            method: "POST".to_string(),
            path: "/groups/{{user_id}}/members".to_string(),
            ..Default::default()
        },
    ],
};

registry.register_chain("user_onboarding", chain)?;
# Ok(())
# }
```

### Latency & Failure Injection

Simulate realistic network conditions and errors:

```rust,no_run
use mockforge_core::{LatencyProfile, FailureConfig, create_failure_injector};

// Configure latency simulation
let latency = LatencyProfile::slow(); // 300-800ms

// Configure failure injection
let failure_config = FailureConfig {
    global_error_rate: 0.05, // 5% of requests fail
    default_status_codes: vec![500, 502, 503],
    ..Default::default()
};

let injector = create_failure_injector(Some(failure_config));
```

## Key Modules

### OpenAPI Support
- [`openapi`]: Parse and work with OpenAPI specifications
- [`openapi_routes`]: Register routes from OpenAPI specs with validation
- [`validation`]: Request/response validation against schemas

### Request Processing
- [`routing`]: Route matching and registration
- [`templating`]: Template variable expansion ({{uuid}}, {{now}}, etc.)
- [`request_chaining`]: Multi-step request workflows
- [`overrides`]: Dynamic request/response modifications

### Chaos Engineering
- [`latency`]: Latency injection with configurable profiles
- [`failure_injection`]: Simulate service failures and errors
- [`traffic_shaping`]: Bandwidth limiting and packet loss

### Proxy & Hybrid
- [`proxy`]: Forward requests to upstream services
- [`ws_proxy`]: WebSocket proxy with message transformation

### Persistence & Import
- [`workspace`]: Workspace management for organizing mocks
- [`workspace_import`]: Import from Postman, Insomnia, cURL, HAR
- [`record_replay`]: Record real requests and replay as fixtures

### Observability
- [`request_logger`]: Centralized request logging
- [`performance`]: Performance metrics and profiling

## Feature Flags

This crate supports several optional features:

- `openapi`: OpenAPI specification support (enabled by default)
- `validation`: Request/response validation (enabled by default)
- `templating`: Template expansion (enabled by default)
- `chaos`: Chaos engineering features (enabled by default)
- `proxy`: Proxy and hybrid mode (enabled by default)
- `workspace`: Workspace management (enabled by default)

## Examples

See the [examples directory](https://github.com/SaaSy-Solutions/mockforge/tree/main/examples)
for complete working examples.

## Related Crates

- [`mockforge-http`](https://docs.rs/mockforge-http): HTTP/REST mock server
- [`mockforge-grpc`](https://docs.rs/mockforge-grpc): gRPC mock server
- [`mockforge-ws`](https://docs.rs/mockforge-ws): WebSocket mock server
- [`mockforge-graphql`](https://docs.rs/mockforge-graphql): GraphQL mock server
- [`mockforge-plugin-core`](https://docs.rs/mockforge-plugin-core): Plugin development
- [`mockforge-data`](https://docs.rs/mockforge-data): Synthetic data generation

## Documentation

- [MockForge Book](https://docs.mockforge.dev/)
- [API Reference](https://docs.rs/mockforge-core)
- [GitHub Repository](https://github.com/SaaSy-Solutions/mockforge)
