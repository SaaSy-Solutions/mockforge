# Testing Guide

This guide covers MockForge's comprehensive testing strategy, including unit tests, integration tests, end-to-end tests, and testing best practices.

## Testing Overview

MockForge employs a multi-layered testing approach to ensure code quality and prevent regressions:

- **Unit Tests**: Individual functions and modules
- **Integration Tests**: Component interactions
- **End-to-End Tests**: Full system workflows
- **Performance Tests**: Load and performance validation
- **Security Tests**: Vulnerability and access control testing

## Unit Testing

### Running Unit Tests

```bash
# Run all unit tests
cargo test --lib

# Run tests for specific crate
cargo test -p mockforge-core

# Run specific test function
cargo test test_template_rendering

# Run tests matching pattern
cargo test template

# Run tests with output
cargo test -- --nocapture
```

### Writing Unit Tests

#### Basic Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        // Arrange
        let input = "test input";
        let expected = "expected output";

        // Act
        let result = process_input(input);

        // Assert
        assert_eq!(result, expected);
    }

    #[test]
    fn test_error_conditions() {
        // Test error cases
        let result = process_input("");
        assert!(result.is_err());
    }
}
```

#### Async Tests

```rust
#[cfg(test)]
mod async_tests {
    use tokio::test;

    #[tokio::test]
    async fn test_async_operation() {
        let result = async_operation().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let (result1, result2) = tokio::join(
            async_operation(),
            another_async_operation()
        );

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }
}
```

## Integration Testing

### Component Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use mockforge_core::config::MockForgeConfig;
    use mockforge_http::HttpServer;

    #[tokio::test]
    async fn test_http_server_integration() {
        // Start test server
        let config = test_config();
        let server = HttpServer::new(config);
        let addr = server.local_addr();

        tokio::spawn(async move {
            server.serve().await.unwrap();
        });

        // Wait for server to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Test HTTP request
        let client = reqwest::Client::new();
        let response = client
            .get(&format!("http://{}/health", addr))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
    }
}
```

## End-to-End Testing

### Full System Tests

```rust
#[cfg(test)]
mod e2e_tests {
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_full_openapi_workflow() {
        // Start MockForge server
        let mut server = Command::new("cargo")
            .args(&["run", "--bin", "mockforge-cli", "serve",
                   "--spec", "examples/openapi-demo.json",
                   "--http-port", "3000"])
            .spawn()
            .unwrap();

        // Wait for server to start
        thread::sleep(Duration::from_secs(2));

        // Test API endpoints
        test_user_endpoints();
        test_product_endpoints();

        // Stop server
        server.kill().unwrap();
    }
}
```

## Performance Testing

### Load Testing

```bash
# Using hey for HTTP load testing
hey -n 1000 -c 10 http://localhost:3000/users

# Using wrk for more detailed benchmarking
wrk -t 4 -c 100 -d 30s http://localhost:3000/users
```

### Benchmarking

```rust
// In benches/benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_template_rendering(c: &mut Criterion) {
    let engine = TemplateEngine::new();

    c.bench_function("template_render_simple", |b| {
        b.iter(|| {
            engine.render("Hello {{name}}", &Context::from_value("name", "World"))
        })
    });
}

criterion_group!(benches, benchmark_template_rendering);
criterion_main!(benches);
```

Run benchmarks:

```bash
cargo bench
```

## Security Testing

### Input Validation Tests

```rust
#[cfg(test)]
mod security_tests {
    #[test]
    fn test_sql_injection_prevention() {
        let input = "'; DROP TABLE users; --";
        let result = sanitize_input(input);

        // Ensure dangerous characters are escaped
        assert!(!result.contains("DROP"));
    }

    #[test]
    fn test_template_injection() {
        let engine = TemplateEngine::new();
        let malicious = "{{#exec}}rm -rf /{{/exec}}";

        // Should not execute dangerous commands
        let result = engine.render(malicious, &Context::new());
        assert!(!result.contains("exec"));
    }
}
```

## Continuous Integration

### GitHub Actions Testing

```yaml
# .github/workflows/test.yml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v2
    - uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        override: true

    - name: Cache dependencies
      uses: actions/cache@v2
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

    - name: Run tests
      run: cargo test --verbose

    - name: Run clippy
      run: cargo clippy -- -D warnings

    - name: Check formatting
      run: cargo fmt --check

    - name: Run security audit
      run: cargo audit
```

This comprehensive testing guide ensures MockForge maintains high code quality and prevents regressions across all components and integration points.
