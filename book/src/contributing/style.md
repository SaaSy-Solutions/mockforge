# Code Style Guide

This guide outlines the coding standards and style guidelines for MockForge development. Consistent code style improves readability, maintainability, and collaboration.

## Rust Code Style

MockForge follows the official Rust style guidelines with some project-specific conventions.

### Formatting

Use `rustfmt` for automatic code formatting:

```bash
# Format all code
cargo fmt

# Check formatting without modifying files
cargo fmt --check
```

### Linting

Use `clippy` for additional code quality checks:

```bash
# Run clippy with project settings
cargo clippy

# Run with pedantic mode for stricter checks
cargo clippy -- -W clippy::pedantic
```

### Naming Conventions

#### Functions and Variables

```rust
// Good: snake_case for functions and variables
fn process_user_data(user_id: i32, data: &str) -> Result<User, Error> {
    let processed_data = validate_and_clean(data)?;
    let user_record = create_user_record(user_id, &processed_data)?;
    Ok(user_record)
}

// Bad: camelCase or PascalCase
fn processUserData(userId: i32, data: &str) -> Result<User, Error> {
    let ProcessedData = validate_and_clean(data)?;
    let userRecord = create_user_record(userId, &ProcessedData)?;
    Ok(userRecord)
}
```

#### Types and Traits

```rust
// Good: PascalCase for types
pub struct HttpServer {
    config: ServerConfig,
    router: Router,
}

pub trait RequestHandler {
    fn handle_request(&self, request: Request) -> Response;
}

// Bad: snake_case for types
pub struct http_server {
    config: ServerConfig,
    router: Router,
}
```

#### Constants

```rust
// Good: SCREAMING_SNAKE_CASE for constants
const MAX_CONNECTIONS: usize = 1000;
const DEFAULT_TIMEOUT_SECS: u64 = 30;

// Bad: camelCase or PascalCase
const maxConnections: usize = 1000;
const DefaultTimeoutSecs: u64 = 30;
```

#### Modules and Files

```rust
// Good: snake_case for module names
pub mod request_handler;
pub mod template_engine;

// File: request_handler.rs
// Module: request_handler
```

### Documentation

#### Function Documentation

```rust
/// Processes a user request and returns a response.
///
/// This function handles the complete request processing pipeline:
/// 1. Validates the request data
/// 2. Applies business logic
/// 3. Returns appropriate response
///
/// # Arguments
///
/// * `user_id` - The ID of the user making the request
/// * `request_data` - The request payload as JSON
///
/// # Returns
///
/// Returns a `Result<Response, Error>` where:
/// - `Ok(response)` contains the successful response
/// - `Err(error)` contains details about what went wrong
///
/// # Errors
///
/// This function will return an error if:
/// - The user ID is invalid
/// - The request data is malformed
/// - Database operations fail
///
/// # Examples
///
/// ```rust
/// let user_id = 123;
/// let request_data = r#"{"action": "update_profile"}"#;
/// let response = process_user_request(user_id, request_data)?;
/// assert_eq!(response.status(), 200);
/// ```
pub fn process_user_request(user_id: i32, request_data: &str) -> Result<Response, Error> {
    // Implementation
}
```

#### Module Documentation

```rust
//! # HTTP Server Module
//!
//! This module provides HTTP server functionality for MockForge,
//! including request routing, middleware support, and response handling.
//!
//! ## Architecture
//!
//! The HTTP server uses axum as the underlying web framework and provides:
//! - OpenAPI specification integration
//! - Template-based response generation
//! - Middleware for logging and validation
//!
//! ## Example
//!
//! ```rust
//! use mockforge_http::HttpServer;
//!
//! let server = HttpServer::new(config);
//! server.serve("127.0.0.1:3000").await?;
//! ```
```

### Error Handling

#### Custom Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MockForgeError {
    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("I/O error: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },

    #[error("Template rendering error: {message}")]
    Template { message: String },

    #[error("HTTP error: {status} - {message}")]
    Http { status: u16, message: String },
}
```

#### Result Types

```rust
// Good: Use Result<T, MockForgeError> for fallible operations
pub fn load_config(path: &Path) -> Result<Config, MockForgeError> {
    let content = fs::read_to_string(path)
        .map_err(|e| MockForgeError::Io { source: e })?;

    let config: Config = serde_yaml::from_str(&content)
        .map_err(|e| MockForgeError::Config {
            message: format!("Failed to parse YAML: {}", e),
        })?;

    Ok(config)
}

// Bad: Using Option when you should use Result
pub fn load_config_bad(path: &Path) -> Option<Config> {
    // This loses error information
    None
}
```

### Async Code

#### Async Function Signatures

```rust
// Good: Clear async function signatures
pub async fn process_request(request: Request) -> Result<Response, Error> {
    let data = validate_request(&request).await?;
    let result = process_data(data).await?;
    Ok(create_response(result))
}

// Bad: Unclear async boundaries
pub fn process_request(request: Request) -> impl Future<Output = Result<Response, Error>> {
    async move {
        // Implementation
    }
}
```

#### Tokio Usage

```rust
use tokio::sync::{Mutex, RwLock};

// Good: Use appropriate synchronization primitives
pub struct SharedState {
    data: RwLock<HashMap<String, String>>,
    counter: Mutex<i64>,
}

impl SharedState {
    pub async fn get_data(&self, key: &str) -> Option<String> {
        let data = self.data.read().await;
        data.get(key).cloned()
    }

    pub async fn increment_counter(&self) -> i64 {
        let mut counter = self.counter.lock().await;
        *counter += 1;
        *counter
    }
}
```

### Testing

#### Unit Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_basic_case() {
        // Given
        let input = "test input";
        let expected = "expected output";

        // When
        let result = process_input(input);

        // Then
        assert_eq!(result, expected);
    }

    #[test]
    fn test_function_error_case() {
        // Given
        let input = "";

        // When
        let result = process_input(input);

        // Then
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_async_function() {
        // Given
        let client = create_test_client().await;

        // When
        let response = client.make_request().await.unwrap();

        // Then
        assert_eq!(response.status(), 200);
    }
}
```

#### Test Organization

```rust
// tests/integration_tests.rs
#[cfg(test)]
mod integration_tests {
    use mockforge_core::config::MockForgeConfig;

    #[tokio::test]
    async fn test_full_http_flow() {
        // Test complete request/response cycle
        let server = TestServer::new().await;
        let client = TestClient::new(server.url());

        let response = client.get("/api/users").await;
        assert_eq!(response.status(), 200);
    }
}
```

### Performance Considerations

#### Memory Management

```rust
// Good: Use references when possible
pub fn process_data(data: &str) -> Result<String, Error> {
    // Avoid cloning unless necessary
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }
    Ok(data.to_uppercase())
}

// Good: Use Cow for flexible ownership
use std::borrow::Cow;

pub fn normalize_string<'a>(input: &'a str) -> Cow<'a, str> {
    if input.chars().all(|c| c.is_lowercase()) {
        Cow::Borrowed(input)
    } else {
        Cow::Owned(input.to_lowercase())
    }
}
```

#### Zero-Cost Abstractions

```rust
// Good: Use iterators for memory efficiency
pub fn find_active_users(users: &[User]) -> impl Iterator<Item = &User> {
    users.iter().filter(|user| user.is_active)
}

// Bad: Collect into Vec unnecessarily
pub fn find_active_users_bad(users: &[User]) -> Vec<&User> {
    users.iter().filter(|user| user.is_active).collect()
}
```

## Project-Specific Conventions

### Configuration Handling

```rust
// Good: Use builder pattern for complex configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub tls: Option<TlsConfig>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            tls: None,
        }
    }
}

impl ServerConfig {
    pub fn builder() -> ServerConfigBuilder {
        ServerConfigBuilder::default()
    }
}
```

### Logging

```rust
use tracing::{info, warn, error, debug, instrument};

// Good: Use structured logging
#[instrument(skip(config))]
pub async fn start_server(config: &ServerConfig) -> Result<(), Error> {
    info!("Starting server", host = %config.host, port = config.port);

    if let Err(e) = setup_server(config).await {
        error!("Failed to start server", error = %e);
        return Err(e);
    }

    info!("Server started successfully");
    Ok(())
}
```

### Feature Flags

```rust
// Good: Use feature flags for optional functionality
#[cfg(feature = "grpc")]
pub mod grpc {
    // gRPC-specific code
}

#[cfg(feature = "websocket")]
pub mod websocket {
    // WebSocket-specific code
}
```

## Code Review Checklist

Before submitting code for review, ensure:

- [ ] Code is formatted with `cargo fmt`
- [ ] No clippy warnings remain
- [ ] All tests pass
- [ ] Documentation is updated
- [ ] No TODO comments left in production code
- [ ] Error messages are user-friendly
- [ ] Performance considerations are addressed
- [ ] Security implications are reviewed

## Tools and Automation

### Pre-commit Hooks

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Format code
cargo fmt --check
if [ $? -ne 0 ]; then
    echo "Code is not formatted. Run 'cargo fmt' to fix."
    exit 1
fi

# Run clippy
cargo clippy -- -D warnings
if [ $? -ne 0 ]; then
    echo "Clippy found issues. Fix them before committing."
    exit 1
fi

# Run tests
cargo test
if [ $? -ne 0 ]; then
    echo "Tests are failing. Fix them before committing."
    exit 1
fi
```

### CI Configuration

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v2
    - uses: actions-rs/toolchain@v1
      with:
        toolchain: stable

    - name: Check formatting
      run: cargo fmt --check

    - name: Run clippy
      run: cargo clippy -- -D warnings

    - name: Run tests
      run: cargo test --verbose

    - name: Run security audit
      run: cargo audit
```

This style guide ensures MockForge maintains high code quality and consistency across the entire codebase. Following these guidelines makes the code more readable, maintainable, and collaborative.
