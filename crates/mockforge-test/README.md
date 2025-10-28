# mockforge-test

Test utilities for [MockForge](https://mockforge.dev) - easy integration with test frameworks like Playwright, Vitest, and any Rust test framework.

## Features

- **🚀 Easy Server Spawning**: Start and stop MockForge servers programmatically
- **✅ Health Checks**: Wait for server readiness with configurable timeouts
- **🔄 Scenario Management**: Switch scenarios/workspaces per-test
- **🧹 Automatic Cleanup**: Processes are automatically cleaned up when dropped
- **⚙️ Profile Support**: Run with different MockForge profiles
- **🔌 Protocol Support**: Configure HTTP, WebSocket, gRPC, and admin endpoints

## Installation

Add this to your `Cargo.toml`:

```toml
[dev-dependencies]
mockforge-test = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Quick Start

### Basic Usage

```rust
use mockforge_test::MockForgeServer;

#[tokio::test]
async fn test_with_mockforge() {
    // Start MockForge server (auto-assigns port)
    let server = MockForgeServer::builder()
        .build()
        .await
        .expect("Failed to start server");

    // Server is ready - run your tests
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/users", server.base_url()))
        .send()
        .await
        .expect("Failed to get users");

    assert!(response.status().is_success());

    // Server automatically stops when dropped
}
```

### With Specific Port

```rust
use mockforge_test::MockForgeServer;

#[tokio::test]
async fn test_with_custom_port() {
    let server = MockForgeServer::builder()
        .http_port(3000)
        .build()
        .await
        .expect("Failed to start server");

    assert_eq!(server.http_port(), 3000);
    assert_eq!(server.base_url(), "http://localhost:3000");
}
```

### With OpenAPI Spec

```rust
use mockforge_test::MockForgeServer;

#[tokio::test]
async fn test_with_openapi_spec() {
    let server = MockForgeServer::builder()
        .spec_file("tests/fixtures/petstore.yaml")
        .build()
        .await
        .expect("Failed to start server");

    // Test your OpenAPI endpoints
    let response = reqwest::get(format!("{}/pets", server.base_url()))
        .await
        .expect("Failed to get pets");

    assert!(response.status().is_success());
}
```

## Scenario Management

Switch between different test scenarios on the fly:

```rust
use mockforge_test::MockForgeServer;

#[tokio::test]
async fn test_scenario_switching() {
    let server = MockForgeServer::builder()
        .build()
        .await
        .expect("Failed to start server");

    // Test default scenario
    // ...

    // Switch to authenticated user scenario
    server
        .scenario("user-authenticated")
        .await
        .expect("Failed to switch scenario");

    // Test authenticated endpoints
    // ...

    // Switch to error scenario
    server
        .scenario("server-errors")
        .await
        .expect("Failed to switch scenario");

    // Test error handling
    // ...
}
```

### Loading Workspace from File

```rust
use mockforge_test::MockForgeServer;

#[tokio::test]
async fn test_with_workspace() {
    let server = MockForgeServer::builder()
        .build()
        .await
        .expect("Failed to start server");

    // Load a workspace configuration
    server
        .load_workspace("tests/fixtures/test-workspace.json")
        .await
        .expect("Failed to load workspace");

    // Test with the workspace configuration
    // ...
}
```

### Dynamic Mock Updates

```rust
use mockforge_test::MockForgeServer;
use serde_json::json;

#[tokio::test]
async fn test_dynamic_mocks() {
    let server = MockForgeServer::builder()
        .build()
        .await
        .expect("Failed to start server");

    // Update mock for a specific endpoint
    server
        .update_mock(
            "/users/123",
            json!({
                "id": 123,
                "name": "Alice",
                "email": "alice@example.com"
            }),
        )
        .await
        .expect("Failed to update mock");

    // Test with the updated mock
    let response = reqwest::get(format!("{}/users/123", server.base_url()))
        .await
        .expect("Failed to get user");

    let user: serde_json::Value = response.json().await.unwrap();
    assert_eq!(user["name"], "Alice");
}
```

## Advanced Configuration

### Multiple Protocols

```rust
use mockforge_test::MockForgeServer;

#[tokio::test]
async fn test_multi_protocol() {
    let server = MockForgeServer::builder()
        .http_port(3000)
        .ws_port(3001)
        .grpc_port(3002)
        .build()
        .await
        .expect("Failed to start server");

    // Test HTTP endpoint
    let http_response = reqwest::get(format!("http://localhost:3000/health"))
        .await
        .expect("Failed to get health");
    assert!(http_response.status().is_success());

    // Test WebSocket, gRPC, etc.
    // ...
}
```

### With Admin UI and Metrics

```rust
use mockforge_test::MockForgeServer;

#[tokio::test]
async fn test_with_observability() {
    let server = MockForgeServer::builder()
        .enable_admin(true)
        .admin_port(3100)
        .enable_metrics(true)
        .metrics_port(9090)
        .build()
        .await
        .expect("Failed to start server");

    // Access admin UI at http://localhost:3100
    // Access metrics at http://localhost:9090/metrics
}
```

### With Profile

```rust
use mockforge_test::MockForgeServer;

#[tokio::test]
async fn test_with_profile() {
    let server = MockForgeServer::builder()
        .profile("testing")
        .build()
        .await
        .expect("Failed to start server");

    // Server uses the "testing" profile configuration
}
```

### Custom Binary Path

```rust
use mockforge_test::MockForgeServer;

#[tokio::test]
async fn test_with_custom_binary() {
    let server = MockForgeServer::builder()
        .binary_path("/path/to/mockforge")
        .build()
        .await
        .expect("Failed to start server");
}
```

## Server Management

### Health Checks

```rust
use mockforge_test::MockForgeServer;

#[tokio::test]
async fn test_health_check() {
    let server = MockForgeServer::builder()
        .build()
        .await
        .expect("Failed to start server");

    // Check if server is ready
    assert!(server.is_ready().await);

    // Get detailed health status
    let health = server.health_check().await.expect("Health check failed");
    assert_eq!(health.status, "healthy");
    println!("Server uptime: {}s", health.uptime_seconds);
    println!("Server version: {}", health.version);
}
```

### Server Statistics

```rust
use mockforge_test::MockForgeServer;

#[tokio::test]
async fn test_server_stats() {
    let server = MockForgeServer::builder()
        .build()
        .await
        .expect("Failed to start server");

    // Get server statistics
    let stats = server.get_stats().await.expect("Failed to get stats");
    println!("Server stats: {:?}", stats);
}
```

### Reset Mocks

```rust
use mockforge_test::MockForgeServer;

#[tokio::test]
async fn test_reset_mocks() {
    let server = MockForgeServer::builder()
        .build()
        .await
        .expect("Failed to start server");

    // Make some changes
    server.update_mock("/test", serde_json::json!({"test": true}))
        .await
        .expect("Failed to update mock");

    // Reset all mocks to initial state
    server.reset().await.expect("Failed to reset mocks");
}
```

## Integration Examples

### With Playwright (Node.js/TypeScript)

While this is a Rust crate, you can use it to spawn MockForge servers for JavaScript/TypeScript tests:

1. Create a Rust helper binary that uses `mockforge-test`
2. Call it from your Playwright global setup

Example Rust helper:

```rust
// bin/test-server.rs
use mockforge_test::MockForgeServer;

#[tokio::main]
async fn main() {
    let server = MockForgeServer::builder()
        .http_port(3000)
        .build()
        .await
        .expect("Failed to start server");

    println!("MockForge server started on port {}", server.http_port());

    // Keep running until interrupted
    tokio::signal::ctrl_c().await.ok();
}
```

Then in your `playwright.config.ts`:

```typescript
import { defineConfig } from '@playwright/test';
import { exec } from 'child_process';

export default defineConfig({
  globalSetup: async () => {
    // Start MockForge
    const server = exec('cargo run --bin test-server');
    // Wait for server to be ready
    await new Promise(resolve => setTimeout(resolve, 2000));
    return () => server.kill();
  },
  // ...
});
```

### With Vitest

Similar to Playwright, create a Rust helper and call it from Vitest's `globalSetup`:

```typescript
// vitest.config.ts
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globalSetup: './test/setup.ts',
  },
});
```

```typescript
// test/setup.ts
import { exec } from 'child_process';

export async function setup() {
  const server = exec('cargo run --bin test-server');
  await new Promise(resolve => setTimeout(resolve, 2000));
  return () => server.kill();
}
```

### With Rust Test Frameworks

For native Rust testing, you can use the helper function:

```rust
use mockforge_test::with_mockforge;

#[tokio::test]
async fn test_with_helper() {
    with_mockforge(|server| async move {
        // Your test code here
        let response = reqwest::get(format!("{}/health", server.base_url()))
            .await?;

        assert!(response.status().is_success());
        Ok(())
    })
    .await
    .expect("Test failed");
}
```

## Configuration Reference

### ServerConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `http_port` | `u16` | `0` (auto) | HTTP server port |
| `ws_port` | `Option<u16>` | `None` | WebSocket server port |
| `grpc_port` | `Option<u16>` | `None` | gRPC server port |
| `admin_port` | `Option<u16>` | `None` | Admin UI port |
| `metrics_port` | `Option<u16>` | `None` | Metrics/Prometheus port |
| `spec_file` | `Option<PathBuf>` | `None` | OpenAPI spec file path |
| `workspace_dir` | `Option<PathBuf>` | `None` | Workspace directory |
| `profile` | `Option<String>` | `None` | Configuration profile |
| `enable_admin` | `bool` | `false` | Enable admin UI |
| `enable_metrics` | `bool` | `false` | Enable metrics endpoint |
| `extra_args` | `Vec<String>` | `[]` | Additional CLI arguments |
| `health_timeout` | `Duration` | `30s` | Health check timeout |
| `health_interval` | `Duration` | `100ms` | Health check interval |
| `working_dir` | `Option<PathBuf>` | `None` | Working directory |
| `env_vars` | `Vec<(String, String)>` | `[]` | Environment variables |
| `binary_path` | `Option<PathBuf>` | `None` | Path to mockforge binary |

## Requirements

- MockForge CLI must be installed and available in PATH, or specify `binary_path`
- Install via: `cargo install mockforge-cli`

## Error Handling

All operations return `Result<T, Error>` where `Error` provides detailed information:

```rust
use mockforge_test::{MockForgeServer, Error};

#[tokio::test]
async fn test_error_handling() {
    match MockForgeServer::builder().build().await {
        Ok(server) => {
            // Server started successfully
        }
        Err(Error::BinaryNotFound) => {
            eprintln!("MockForge binary not found. Please install it first.");
        }
        Err(Error::HealthCheckTimeout(secs)) => {
            eprintln!("Server didn't become healthy within {}s", secs);
        }
        Err(e) => {
            eprintln!("Failed to start server: {}", e);
        }
    }
}
```

## Logging

Enable logging to see detailed information:

```rust
// In your test setup
tracing_subscriber::fmt()
    .with_env_filter("mockforge_test=debug")
    .init();
```

Or set the `RUST_LOG` environment variable:

```bash
RUST_LOG=mockforge_test=debug cargo test
```

## Examples

See the [examples](../../examples) directory for complete working examples.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](../../CONTRIBUTING.md) for details.

## License

This project is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Links

- [MockForge Documentation](https://docs.mockforge.dev)
- [MockForge GitHub](https://github.com/SaaSy-Solutions/mockforge)
- [API Documentation](https://docs.rs/mockforge-test)
