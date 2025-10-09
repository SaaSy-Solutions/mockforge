# Chaos Engine Integration Guide

This guide explains how to integrate the random chaos mode into MockForge server implementations (HTTP, WebSocket, gRPC, GraphQL).

## Overview

The random chaos mode is now fully integrated into MockForge's configuration system. When users specify `--chaos-random` flags, the configuration is:

1. Parsed from CLI arguments
2. Stored in `ServerConfig.core.chaos_random`
3. Available for server implementations to use

## Configuration Flow

```
CLI flags (--chaos-random, etc.)
    ↓
handle_serve() creates ChaosConfig
    ↓
Stored in config.core.chaos_random
    ↓
Server implementations create ChaosEngine
    ↓
Apply to requests via middleware
```

## For Server Implementers

If you're implementing or modifying a MockForge server (HTTP, WebSocket, gRPC, etc.), here's how to integrate chaos mode:

### Step 1: Access the Configuration

```rust
use mockforge_core::{ServerConfig, ChaosEngine};

fn setup_server(config: &ServerConfig) {
    // Check if chaos mode is enabled
    if config.core.is_chaos_random_enabled() {
        // Create chaos engine
        if let Some(engine) = config.core.create_chaos_engine() {
            // Use the engine in your middleware
            setup_chaos_middleware(engine);
        }
    }
}
```

### Step 2: Create Shared State

```rust
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    // ... other state fields
    chaos_engine: Option<Arc<ChaosEngine>>,
}

impl AppState {
    fn new(config: &ServerConfig) -> Self {
        Self {
            chaos_engine: config.core.create_chaos_engine().map(Arc::new),
            // ... initialize other fields
        }
    }
}
```

### Step 3: Implement Middleware

#### HTTP/REST Example (Axum)

```rust
use axum::{
    middleware::{self, Next},
    extract::State,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};
use mockforge_core::ChaosResult;
use tokio::time::{sleep, Duration};

async fn chaos_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if let Some(engine) = &state.chaos_engine {
        // Extract tags from request if needed
        let tags = extract_tags_from_request(&request);

        match engine.process_request(&tags).await {
            ChaosResult::Success => {
                // Continue normally
                next.run(request).await
            }
            ChaosResult::Error { status_code, message } => {
                // Return error response
                let status = StatusCode::from_u16(status_code)
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                (status, message).into_response()
            }
            ChaosResult::Delay { delay_ms } => {
                // Inject delay then continue
                sleep(Duration::from_millis(delay_ms)).await;
                next.run(request).await
            }
            ChaosResult::Timeout { timeout_ms: _ } => {
                // Simulate timeout
                (StatusCode::GATEWAY_TIMEOUT, "Request timeout (chaos)").into_response()
            }
        }
    } else {
        // No chaos engine, continue normally
        next.run(request).await
    }
}

// Apply to router
fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/*path", get(handler))
        .layer(middleware::from_fn_with_state(state.clone(), chaos_middleware))
        .with_state(state)
}
```

#### WebSocket Example

```rust
use tokio_tungstenite::tungstenite::Message;

async fn handle_websocket_message(
    message: Message,
    chaos_engine: &Option<Arc<ChaosEngine>>,
) -> Result<Message, Error> {
    if let Some(engine) = chaos_engine {
        match engine.process_request(&[]).await {
            ChaosResult::Success => {
                // Process message normally
                process_message(message).await
            }
            ChaosResult::Error { .. } => {
                // Close connection or send error message
                Err(Error::ChaosInjected)
            }
            ChaosResult::Delay { delay_ms } => {
                // Inject delay
                sleep(Duration::from_millis(delay_ms)).await;
                process_message(message).await
            }
            ChaosResult::Timeout { .. } => {
                // Drop message (simulate packet loss)
                Ok(Message::Close(None))
            }
        }
    } else {
        process_message(message).await
    }
}
```

#### gRPC Example

```rust
use tonic::{Request, Response, Status};

async fn handle_grpc_request<T>(
    request: Request<T>,
    chaos_engine: &Option<Arc<ChaosEngine>>,
) -> Result<Response<T>, Status> {
    if let Some(engine) = chaos_engine {
        match engine.process_request(&[]).await {
            ChaosResult::Success => {
                // Process normally
                process_grpc_request(request).await
            }
            ChaosResult::Error { status_code, message } => {
                // Map HTTP status to gRPC status
                let grpc_status = match status_code {
                    500 => Status::internal(message),
                    502 => Status::unavailable(message),
                    503 => Status::unavailable(message),
                    504 => Status::deadline_exceeded(message),
                    _ => Status::unknown(message),
                };
                Err(grpc_status)
            }
            ChaosResult::Delay { delay_ms } => {
                // Inject delay
                sleep(Duration::from_millis(delay_ms)).await;
                process_grpc_request(request).await
            }
            ChaosResult::Timeout { .. } => {
                Err(Status::deadline_exceeded("Request timeout (chaos)"))
            }
        }
    } else {
        process_grpc_request(request).await
    }
}
```

## Advanced Usage

### Tag-Based Chaos

The chaos engine supports tag-based targeting (though tags are optional):

```rust
// Extract tags from request path, headers, etc.
let tags = vec![
    request.uri().path().split('/').nth(1).unwrap_or("").to_string(),
    // Add more tags as needed
];

let result = engine.process_request(&tags).await;
```

### Dynamic Configuration

The chaos engine can be updated at runtime:

```rust
use mockforge_core::ChaosConfig;

// Update chaos configuration
let new_config = ChaosConfig::new(0.2, 0.5)
    .with_delay_range(200, 1000);

engine.update_config(new_config).await;
```

### Statistics and Monitoring

Get current chaos statistics:

```rust
let stats = engine.get_statistics().await;
println!("Chaos enabled: {}", stats.enabled);
println!("Error rate: {:.1}%", stats.error_rate * 100.0);
println!("Delay rate: {:.1}%", stats.delay_rate * 100.0);
```

## Integration Checklist

When integrating chaos mode into a server implementation:

- [ ] Check if `config.core.chaos_random` is set
- [ ] Create `ChaosEngine` using `config.core.create_chaos_engine()`
- [ ] Store engine in shared application state (wrapped in `Arc`)
- [ ] Implement middleware/interceptor to apply chaos to requests
- [ ] Handle all `ChaosResult` variants appropriately
- [ ] Test with different chaos configurations
- [ ] Document chaos behavior in server-specific docs

## Configuration Reference

### From Code

```rust
use mockforge_core::{Config, ChaosConfig};

let mut config = Config::default();
config.chaos_random = Some(
    ChaosConfig::new(0.15, 0.40)  // 15% errors, 40% delays
        .with_delay_range(100, 2000)
        .with_status_codes(vec![500, 502, 503])
        .with_timeouts(5000)
);
```

### From CLI

```bash
mockforge serve \
  --chaos-random \
  --chaos-random-error-rate 0.15 \
  --chaos-random-delay-rate 0.40 \
  --chaos-random-min-delay 100 \
  --chaos-random-max-delay 2000 \
  --spec api.yaml
```

### From Config File (YAML)

```yaml
core:
  chaos_random:
    enabled: true
    error_rate: 0.15
    delay_rate: 0.40
    min_delay_ms: 100
    max_delay_ms: 2000
    status_codes: [500, 502, 503]
    inject_timeouts: false
```

## Testing Your Integration

### Unit Tests

```rust
#[tokio::test]
async fn test_chaos_integration() {
    let mut config = Config::default();
    config.chaos_random = Some(ChaosConfig::new(1.0, 0.0)); // 100% errors

    let engine = config.create_chaos_engine().unwrap();

    // Test that errors are injected
    let result = engine.process_request(&[]).await;
    assert!(matches!(result, ChaosResult::Error { .. }));
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_chaos_with_real_requests() {
    // Build server with chaos enabled
    let config = create_test_config_with_chaos();
    let app = build_router(config);

    // Make request
    let response = app
        .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Some requests should fail with 5xx status
    // (May need multiple attempts due to randomness)
}
```

## Best Practices

1. **Always check if chaos is enabled** before creating the engine
2. **Wrap the engine in Arc** for efficient sharing across threads
3. **Handle all ChaosResult variants** - don't ignore Timeout or Delay
4. **Log chaos events** for debugging and monitoring
5. **Make chaos optional** - don't break normal operation if disabled
6. **Consider protocol differences** - HTTP status codes don't map 1:1 to gRPC/WebSocket
7. **Test with various rates** - ensure behavior is reasonable at 0%, 50%, and 100%

## Examples

Complete examples are available in:

- `examples/chaos_engine_integration.rs` - Full HTTP middleware example
- See server implementations in `mockforge-http`, `mockforge-ws`, `mockforge-grpc`

## Questions?

If you have questions about integrating chaos mode into a server implementation:

1. Check the example in `examples/chaos_engine_integration.rs`
2. Look at existing implementations in other server crates
3. Review the `ChaosEngine` API documentation in `mockforge-core`
4. File an issue on GitHub if you need help

## Summary

The chaos engine is now fully integrated into MockForge's configuration system. Server implementations just need to:

1. Check `config.core.chaos_random`
2. Create `ChaosEngine` from config
3. Apply it in middleware
4. Handle `ChaosResult` variants

The hard work of random injection, rate limiting, and configuration is all handled by the `ChaosEngine` - implementations just need to wire it up!
