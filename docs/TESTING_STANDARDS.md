# MockForge Testing Standards

This document defines the testing standards, coverage requirements, and best practices for MockForge development.

## Coverage Requirements

### Minimum Coverage Thresholds

All crates in the MockForge workspace must maintain the following coverage thresholds:

- **Default Threshold**: 80% line coverage
- **High-Priority Crates**: 85% line coverage (core, http, cli, sdk)
- **Protocol Crates**: 75% line coverage (grpc, ws, graphql, kafka, mqtt, amqp, smtp, ftp, tcp)
- **Infrastructure Crates**: 70-75% line coverage (observability, tracing, analytics)

See `coverage.toml` for per-crate threshold overrides.

### Coverage Measurement

Coverage is measured using `cargo-llvm-cov` and reported per-crate:

```bash
# Generate coverage baseline for all crates
./scripts/coverage-baseline.sh

# Generate coverage for specific crate
cargo llvm-cov --package mockforge-core --all-features --lcov --output-path coverage/lcov.info
```

### Coverage Enforcement

Coverage enforcement follows a gradual rollout:

1. **Phase 1 (Current)**: Reporting only - Generate coverage reports, no enforcement
2. **Phase 2**: Warnings - CI comments on PRs if coverage drops, no blocking
3. **Phase 3**: Enforcement - CI blocks PRs if coverage drops below threshold

Current enforcement mode: `report_only` (see `coverage.toml`)

## Test Organization

### Test File Structure

```
crates/mockforge-{name}/
├── src/
│   ├── lib.rs
│   └── module.rs
└── tests/
    ├── integration.rs          # Integration tests
    ├── unit_tests.rs           # Unit tests (if separate)
    └── fixtures/               # Test fixtures
        └── test_data.json
```

### Test Module Organization

#### Unit Tests

Unit tests should be placed in `#[cfg(test)]` modules within source files:

```rust
// src/module.rs
pub fn process_data(input: &str) -> Result<String, Error> {
    // Implementation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_data_valid_input() {
        // Arrange
        let input = "valid input";

        // Act
        let result = process_data(input);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_data_invalid_input() {
        // Arrange
        let input = "";

        // Act
        let result = process_data(input);

        // Assert
        assert!(result.is_err());
    }
}
```

#### Integration Tests

Integration tests should be placed in `tests/` directory:

```rust
// tests/integration.rs
use mockforge_core::*;

#[tokio::test]
async fn test_complete_workflow() {
    // Test complete workflows
}
```

### Test Naming Conventions

Test names should be descriptive and follow this pattern:

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
```

## Test Patterns

### Arrange-Act-Assert Pattern

All tests should follow the AAA (Arrange-Act-Assert) pattern:

```rust
#[test]
fn test_example() {
    // Arrange: Set up test data and preconditions
    let input = create_test_input();
    let expected = create_expected_output();

    // Act: Execute the function under test
    let result = function_under_test(input);

    // Then: Verify the result matches expectations
    assert_eq!(result, expected);
}
```

### Error Case Testing

Every function that can return an error must have error case tests:

```rust
#[test]
fn test_function_error_conditions() {
    // Test invalid inputs
    assert!(function("").is_err());
    assert!(function("invalid").is_err());

    // Test edge cases
    assert!(function(&"a".repeat(10000)).is_err());
}
```

### Edge Case Testing

Test boundary conditions and edge cases:

```rust
#[test]
fn test_edge_cases() {
    // Empty input
    assert_eq!(process(""), Ok(""));

    // Single item
    assert_eq!(process("a"), Ok("a"));

    // Maximum size
    let large_input = "x".repeat(1000000);
    assert!(process(&large_input).is_ok());

    // Unicode
    assert_eq!(process("🚀"), Ok("🚀"));
}
```

### Property-Based Testing

Use property-based testing for data validation and complex logic:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_template_rendering_with_random_input(
        input in "\\PC*",  // Any printable character
        name in "[a-zA-Z]{1,10}"
    ) {
        let engine = TemplateEngine::new();
        let context = &[("name", &name)];

        // Should not panic regardless of input
        let _result = engine.render(&input, context);
    }
}
```

### Concurrency Testing

Test thread-safety for concurrent operations:

```rust
#[test]
fn test_concurrent_access() {
    use std::sync::Arc;
    use std::thread;

    let registry = Arc::new(RouteRegistry::new());
    let mut handles = vec![];

    // Spawn multiple threads
    for i in 0..10 {
        let registry = Arc::clone(&registry);
        handles.push(thread::spawn(move || {
            registry.add_route(create_test_route(i));
        }));
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify state
    assert_eq!(registry.route_count(), 10);
}
```

## Protocol Crate Testing

### Common Test Patterns for Protocol Crates

All protocol crates (kafka, mqtt, amqp, ftp, tcp, smtp) should include:

1. **Connection Tests**
   - Successful connection establishment
   - Connection failure handling
   - Connection timeout handling

2. **Message Tests**
   - Send/receive messages
   - Message serialization/deserialization
   - Large message handling

3. **Error Handling Tests**
   - Network failures
   - Protocol errors
   - Invalid message formats

4. **Integration Tests**
   - Integration with mockforge-core routing
   - End-to-end message flow

Example:

```rust
// crates/mockforge-kafka/tests/integration.rs
#[tokio::test]
async fn test_kafka_producer_consumer() {
    // Test Kafka producer/consumer integration
}

#[tokio::test]
async fn test_kafka_connection_failure() {
    // Test connection failure handling
}

#[tokio::test]
async fn test_kafka_message_serialization() {
    // Test message serialization
}
```

## Test Utilities and Helpers

### Shared Test Utilities

Create shared test utilities for common patterns:

```rust
// tests/common/mod.rs
pub mod server {
    pub async fn start_test_server() -> TestServer {
        // Start test server
    }
}

pub mod data {
    pub fn sample_openapi_spec() -> &'static str {
        // Return sample OpenAPI spec
    }
}
```

### Mock Servers

Use embedded/test servers for protocol testing:

```rust
#[tokio::test]
async fn test_with_mock_server() {
    let server = MockKafkaServer::new().await;
    let client = KafkaClient::new(server.addr()).await.unwrap();

    // Test with mock server
}
```

## Coverage Gap Analysis

### Identifying Coverage Gaps

1. Run coverage baseline:
   ```bash
   ./scripts/coverage-baseline.sh
   ```

2. Review coverage report:
   ```bash
   cat coverage/summary.txt
   ```

3. Identify untested code paths:
   - Review HTML coverage reports
   - Look for functions with 0% coverage
   - Identify missing error case tests

### Prioritizing Coverage Improvements

1. **High Priority**: User-facing crates with low coverage
2. **Medium Priority**: Protocol crates with minimal tests
3. **Low Priority**: Internal utilities with low user impact

## CI/CD Integration

### Coverage in CI

Coverage is generated in CI for all PRs:

```yaml
# .github/workflows/test.yml
- name: Generate coverage
  run: |
    cargo install cargo-llvm-cov
    cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info
```

### Coverage Reports

Coverage reports are available as:
- JSON summary: `coverage/summary.json`
- CSV summary: `coverage/summary.csv`
- Text summary: `coverage/summary.txt`
- Per-crate HTML: `coverage/crates/{crate_name}/index.html`

## Best Practices

### Test Isolation

- Each test should be independent
- Use temporary directories for file I/O
- Clean up resources in test teardown

### Test Performance

- Keep unit tests fast (< 1ms each)
- Use `#[ignore]` for slow integration tests
- Run slow tests separately in CI

### Test Reliability

- Avoid flaky tests (time-dependent, network-dependent)
- Use deterministic test data
- Mock external dependencies

### Test Documentation

- Document complex test scenarios
- Explain why edge cases are tested
- Note any test limitations

## Maintenance

### Regular Coverage Reviews

- Weekly: Review coverage trends
- Monthly: Analyze coverage gaps
- Quarterly: Update coverage thresholds

### Coverage Monitoring

Track coverage over time:
- Monitor coverage trends
- Identify regressions early
- Set coverage improvement goals

## References

- [Testing Guidelines](../book/src/contributing/testing.md) - Comprehensive testing guide
- [Coverage Configuration](../coverage.toml) - Coverage thresholds and settings
- [Coverage Dashboard](../docs/COVERAGE.md) - Current coverage status
