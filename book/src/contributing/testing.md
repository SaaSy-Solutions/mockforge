# Testing Guidelines

This guide outlines the testing standards and practices for MockForge contributions. Quality testing ensures code reliability, prevents regressions, and maintains system stability.

## Testing Philosophy

### Testing Pyramid

MockForge follows a testing pyramid approach with different types of tests serving different purposes:

```
End-to-End Tests (E2E)
        ↑
Integration Tests
        ↑
Unit Tests
       Base
```

- **Unit Tests**: Test individual functions and modules in isolation
- **Integration Tests**: Test component interactions and data flow
- **End-to-End Tests**: Test complete user workflows and system behavior

### Testing Principles

1. **Test First**: Write tests before implementation when possible
2. **Test Behavior**: Test what the code does, not how it does it
3. **Test Boundaries**: Focus on edge cases and error conditions
4. **Keep Tests Fast**: Tests should run quickly to encourage frequent execution
5. **Make Tests Reliable**: Tests should be deterministic and not flaky

## Unit Testing Requirements

### Test Coverage

All new code must include unit tests with the following minimum coverage:

- **Functions**: Test all public functions with valid inputs
- **Error Cases**: Test all error conditions and edge cases
- **Branches**: Test all conditional branches (if/else, match arms)
- **Loops**: Test loop boundaries (empty, single item, multiple items)

### Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name_description() {
        // Given: Set up test data and preconditions
        let input = create_test_input();
        let expected = create_expected_output();

        // When: Execute the function under test
        let result = function_under_test(input);

        // Then: Verify the result matches expectations
        assert_eq!(result, expected);
    }

    #[test]
    fn test_function_name_error_case() {
        // Given: Set up error condition
        let invalid_input = create_invalid_input();

        // When: Execute the function
        let result = function_under_test(invalid_input);

        // Then: Verify error handling
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, ExpectedError::Variant));
    }
}
```

### Test Naming Conventions

```rust
// Good: Descriptive test names
#[test]
fn test_parse_openapi_spec_validates_required_fields() { ... }
#[test]
fn test_template_engine_handles_missing_variables() { ... }
#[test]
fn test_http_server_rejects_invalid_content_type() { ... }

// Bad: Non-descriptive names
#[test]
fn test_function() { ... }
#[test]
fn test_case_1() { ... }
#[test]
fn test_error() { ... }
```

### Test Data Management

#### Test Fixtures

```rust
// Use shared test fixtures for common data
pub fn sample_openapi_spec() -> &'static str {
    r#"
    openapi: 3.0.3
    info:
      title: Test API
      version: 1.0.0
    paths:
      /users:
        get:
          responses:
            '200':
              description: Success
    "#
}

pub fn sample_user_data() -> User {
    User {
        id: "123".to_string(),
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
    }
}
```

#### Test Utilities

```rust
// Create test utilities for common setup
pub struct TestServer {
    server_handle: Option<JoinHandle<()>>,
    base_url: String,
}

impl TestServer {
    pub async fn new() -> Self {
        // Start test server
        // Return configured instance
    }

    pub fn url(&self) -> &str {
        &self.base_url
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        // Clean up server
    }
}
```

## Integration Testing Standards

### When to Write Integration Tests

Integration tests are required for:

- **API Boundaries**: HTTP endpoints, gRPC services, WebSocket connections
- **Database Operations**: Data persistence and retrieval
- **External Services**: Third-party API integrations
- **File I/O**: Configuration loading, fixture management
- **Component Communication**: Cross-crate interactions

### Integration Test Structure

```rust
#[cfg(test)]
mod integration_tests {
    use mockforge_core::config::MockForgeConfig;

    #[tokio::test]
    async fn test_http_server_startup() {
        // Given: Configure test server
        let config = create_test_config();
        let server = HttpServer::new(config);

        // When: Start the server
        let addr = server.local_addr();
        tokio::spawn(async move {
            server.serve().await.unwrap();
        });

        // Wait for startup
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Then: Verify server is responding
        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{}/health", addr))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
    }
}
```

### Database Testing

```rust
#[cfg(test)]
mod database_tests {
    use sqlx::PgPool;

    #[sqlx::test]
    async fn test_user_creation(pool: PgPool) {
        // Given: Clean database state
        sqlx::query!("DELETE FROM users").execute(&pool).await.unwrap();

        // When: Create a user
        let user_id = create_user(&pool, "test@example.com").await.unwrap();

        // Then: Verify user exists
        let user = sqlx::query!("SELECT * FROM users WHERE id = $1", user_id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(user.email, "test@example.com");
    }
}
```

## End-to-End Testing Requirements

### E2E Test Scenarios

E2E tests must cover:

- **Happy Path**: Complete successful user workflows
- **Error Recovery**: System behavior under failure conditions
- **Data Persistence**: State changes across operations
- **Performance**: Response times and resource usage
- **Security**: Authentication and authorization flows

### E2E Test Implementation

```rust
#[cfg(test)]
mod e2e_tests {
    use std::process::Command;
    use std::time::Duration;

    #[test]
    fn test_complete_api_workflow() {
        // Start MockForge server
        let mut server = Command::new("cargo")
            .args(&["run", "--release", "--", "serve", "--spec", "test-api.yaml"])
            .spawn()
            .unwrap();

        // Wait for server startup
        std::thread::sleep(Duration::from_secs(3));

        // Execute complete workflow
        let result = run_workflow_test();
        assert!(result.is_ok());

        // Cleanup
        server.kill().unwrap();
    }
}
```

## Test Quality Standards

### Code Coverage Requirements

- **Minimum Coverage**: 80% overall, 90% for critical paths
- **Branch Coverage**: All conditional branches must be tested
- **Error Path Coverage**: All error conditions must be tested

### Performance Testing

```rust
#[cfg(test)]
mod performance_tests {
    use criterion::Criterion;

    fn benchmark_template_rendering(c: &mut Criterion) {
        let engine = TemplateEngine::new();

        c.bench_function("render_simple_template", |b| {
            b.iter(|| {
                engine.render("Hello {{name}}", &[("name", "World")]);
            })
        });
    }
}
```

### Load Testing

```rust
#[cfg(test)]
mod load_tests {
    use tokio::time::{Duration, Instant};

    #[tokio::test]
    async fn test_concurrent_requests() {
        let client = reqwest::Client::new();
        let start = Instant::now();

        // Spawn 100 concurrent requests
        let handles: Vec<_> = (0..100).map(|_| {
            let client = client.clone();
            tokio::spawn(async move {
                client.get("http://localhost:3000/api/users")
                    .send()
                    .await
                    .unwrap()
            })
        }).collect();

        // Wait for all requests to complete
        for handle in handles {
            let response = handle.await.unwrap();
            assert_eq!(response.status(), 200);
        }

        let duration = start.elapsed();
        assert!(duration < Duration::from_secs(5), "Load test took too long: {:?}", duration);
    }
}
```

## Testing Tools and Frameworks

### Required Testing Dependencies

```toml
[dev-dependencies]
tokio-test = "0.4"
proptest = "1.0"          # Property-based testing
criterion = "0.4"         # Benchmarking
assert_cmd = "2.0"        # CLI testing
predicates = "2.1"        # Value assertions
tempfile = "3.0"          # Temporary files
```

### Mocking and Stubbing

```rust
#[cfg(test)]
mod mock_tests {
    use mockall::mock;

    #[mockall::mock]
    trait Database {
        async fn get_user(&self, id: i32) -> Result<User, Error>;
        async fn save_user(&self, user: User) -> Result<(), Error>;
    }

    #[tokio::test]
    async fn test_service_with_mocks() {
        let mut mock_db = MockDatabase::new();

        mock_db
            .expect_get_user()
            .with(eq(123))
            .returning(|_| Ok(User { id: 123, name: "Test".to_string() }));

        let service = UserService::new(mock_db);
        let user = service.get_user(123).await.unwrap();

        assert_eq!(user.name, "Test");
    }
}
```

### Property-Based Testing

```rust
#[cfg(test)]
mod property_tests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_template_rendering_with_random_input(
            input in "\\PC*",  // Any printable character except control chars
            name in "[a-zA-Z]{1,10}"
        ) {
            let engine = TemplateEngine::new();
            let context = &[("name", &name)];

            // Should not panic regardless of input
            let _result = engine.render(&input, context);
        }
    }
}
```

## Test Organization and Naming

### File Structure

```
src/
├── lib.rs
├── module.rs
└── module/
    ├── mod.rs
    └── submodule.rs

tests/
├── unit/
│   ├── module_tests.rs
│   └── submodule_tests.rs
├── integration/
│   ├── api_tests.rs
│   └── database_tests.rs
└── e2e/
    ├── workflow_tests.rs
    └── performance_tests.rs
```

### Test Module Organization

```rust
// tests/unit/template_tests.rs
#[cfg(test)]
mod template_tests {
    use mockforge_core::templating::TemplateEngine;

    // Unit tests for template functionality
}

// tests/integration/http_tests.rs
#[cfg(test)]
mod http_integration_tests {
    use mockforge_http::HttpServer;

    // Integration tests for HTTP server
}

// tests/e2e/api_workflow_tests.rs
#[cfg(test)]
mod e2e_tests {
    // End-to-end workflow tests
}
```

## CI/CD Integration

### GitHub Actions Testing

```yaml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v3
    - uses: dtolnay/rust-toolchain@stable

    - name: Cache dependencies
      uses: Swatinem/rust-cache@v2

    - name: Check formatting
      run: cargo fmt --check

    - name: Run clippy
      run: cargo clippy -- -D warnings

    - name: Run tests
      run: cargo test --verbose

    - name: Run integration tests
      run: cargo test --test integration

    - name: Generate coverage
      run: |
        cargo install cargo-tarpaulin
        cargo tarpaulin --out Xml --output-dir coverage

    - name: Upload coverage
      uses: codecov/codecov-action@v3
      with:
        file: coverage/cobertura.xml
```

### Test Result Reporting

```yaml
- name: Run tests with JUnit output
  run: |
    cargo install cargo2junit
    cargo test -- -Z unstable-options --format json | cargo2junit > test-results.xml

- name: Publish test results
  uses: EnricoMi/publish-unit-test-result-action@v2
  with:
    files: test-results.xml
```

## Best Practices

### Test Isolation

```rust
#[cfg(test)]
mod isolated_tests {
    use tempfile::TempDir;

    #[test]
    fn test_file_operations() {
        // Use temporary directory for isolation
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Test file operations
        write_test_file(&file_path);
        assert!(file_path.exists());

        // Cleanup happens automatically
    }
}
```

### Test Data Management

```rust
#[cfg(test)]
mod test_data {
    use once_cell::sync::Lazy;

    static TEST_USERS: Lazy<Vec<User>> = Lazy::new(|| {
        vec![
            User { id: 1, name: "Alice".to_string() },
            User { id: 2, name: "Bob".to_string() },
        ]
    });

    #[test]
    fn test_user_operations() {
        let users = TEST_USERS.clone();
        // Use shared test data
    }
}
```

### Asynchronous Testing

```rust
#[cfg(test)]
mod async_tests {
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_async_operation_with_timeout() {
        let result = timeout(Duration::from_secs(5), async_operation()).await;

        match result {
            Ok(Ok(data)) => assert!(data.is_valid()),
            Ok(Err(e)) => panic!("Operation failed: {}", e),
            Err(_) => panic!("Operation timed out"),
        }
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let (result1, result2) = tokio::join(
            operation1(),
            operation2()
        );

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }
}
```

### Test Flakiness Prevention

```rust
#[cfg(test)]
mod reliable_tests {
    #[test]
    fn test_with_retries() {
        let mut attempts = 0;
        let max_attempts = 3;

        loop {
            attempts += 1;

            match potentially_flaky_operation() {
                Ok(result) => {
                    assert!(result.is_valid());
                    break;
                }
                Err(e) if attempts < max_attempts => {
                    eprintln!("Attempt {} failed: {}, retrying...", attempts, e);
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
                Err(e) => panic!("Operation failed after {} attempts: {}", max_attempts, e),
            }
        }
    }
}
```

## Security Testing

### Input Validation Testing

```rust
#[cfg(test)]
mod security_tests {
    #[test]
    fn test_sql_injection_prevention() {
        let malicious_input = "'; DROP TABLE users; --";
        let result = sanitize_sql_input(malicious_input);

        assert!(!result.contains("DROP"));
        assert!(!result.contains(";"));
    }

    #[test]
    fn test_xss_prevention() {
        let malicious_input = "<script>alert('xss')</script>";
        let result = sanitize_html_input(malicious_input);

        assert!(!result.contains("<script>"));
        assert!(result.contains("&lt;script&gt;"));
    }

    #[test]
    fn test_path_traversal_prevention() {
        let malicious_input = "../../../etc/passwd";
        let result = validate_file_path(malicious_input);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ValidationError::PathTraversal));
    }
}
```

### Authentication Testing

```rust
#[cfg(test)]
mod auth_tests {
    #[tokio::test]
    async fn test_unauthorized_access() {
        let client = create_test_client();

        let response = client
            .get("/admin/users")
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 401);
    }

    #[tokio::test]
    async fn test_authorized_access() {
        let client = create_authenticated_client();

        let response = client
            .get("/admin/users")
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
    }
}
```

This comprehensive testing guide ensures MockForge maintains high quality and reliability through thorough automated testing at all levels.
