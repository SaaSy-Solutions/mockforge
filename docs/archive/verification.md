# Request Verification API

MockForge provides a comprehensive request verification API that allows you to programmatically verify that specific requests were made (or not made) during test execution. This is similar to WireMock's verification functionality.

## Overview

The verification API enables you to:

- **Verify request counts**: Check that requests were made exactly N times, at least N times, or never
- **Verify request patterns**: Match requests by method, path, headers, query parameters, and body
- **Verify request sequences**: Ensure requests occurred in a specific order
- **Inspect matched requests**: Get detailed information about all matching requests

## Core Concepts

### VerificationRequest

A `VerificationRequest` defines a pattern to match against logged requests:

```rust
VerificationRequest {
    method: Some("GET".to_string()),           // HTTP method (optional)
    path: Some("/api/users".to_string()),      // URL path (optional, supports wildcards/regex)
    query_params: HashMap::new(),              // Query parameters (optional)
    headers: HashMap::new(),                   // Headers (optional)
    body_pattern: None,                        // Body pattern (optional, supports regex)
}
```

### VerificationCount

A `VerificationCount` specifies the expected number of matching requests:

- `Exactly(n)` - Request must be made exactly N times
- `AtLeast(n)` - Request must be made at least N times
- `AtMost(n)` - Request must be made at most N times
- `Never` - Request must never be made (count must be 0)
- `AtLeastOnce` - Request must be made at least once (count >= 1)

### VerificationResult

A `VerificationResult` contains:

- `matched: bool` - Whether the verification passed
- `count: usize` - Actual count of matching requests
- `expected: VerificationCount` - Expected count assertion
- `matches: Vec<RequestLogEntry>` - All matching request log entries
- `error_message: Option<String>` - Error message if verification failed

## Usage Examples

### Rust SDK

```rust
use mockforge_sdk::{MockServer, Verification};
use mockforge_core::verification::{VerificationRequest, VerificationCount};
use std::collections::HashMap;

#[tokio::test]
async fn test_user_api() {
    // Start a mock server
    let mut server = MockServer::new()
        .port(3000)
        .start()
        .await
        .expect("Failed to start server");

    // Stub a response
    server
        .stub_response("GET", "/api/users", json!({
            "users": []
        }))
        .await
        .expect("Failed to stub response");

    // Make requests to the mock server
    let client = reqwest::Client::new();
    client.get("http://localhost:3000/api/users").send().await.unwrap();
    client.get("http://localhost:3000/api/users").send().await.unwrap();
    client.get("http://localhost:3000/api/users").send().await.unwrap();

    // Verify the request was made exactly 3 times
    let pattern = VerificationRequest {
        method: Some("GET".to_string()),
        path: Some("/api/users".to_string()),
        query_params: HashMap::new(),
        headers: HashMap::new(),
        body_pattern: None,
    };

    let result = server.verify(&pattern, VerificationCount::Exactly(3)).await.unwrap();
    assert!(result.matched, "Expected GET /api/users to be called exactly 3 times");
    assert_eq!(result.count, 3);

    // Verify a request was never made
    let delete_pattern = VerificationRequest {
        method: Some("DELETE".to_string()),
        path: Some("/api/users/1".to_string()),
        query_params: HashMap::new(),
        headers: HashMap::new(),
        body_pattern: None,
    };

    let result = server.verify_never(&delete_pattern).await.unwrap();
    assert!(result.matched, "Expected DELETE /api/users/1 to never be called");

    // Verify at least N requests
    let result = server.verify_at_least(&pattern, 2).await.unwrap();
    assert!(result.matched, "Expected at least 2 requests");

    // Verify request sequence
    let patterns = vec![
        VerificationRequest {
            method: Some("POST".to_string()),
            path: Some("/api/users".to_string()),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body_pattern: None,
        },
        VerificationRequest {
            method: Some("GET".to_string()),
            path: Some("/api/users/1".to_string()),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body_pattern: None,
        },
    ];

    let result = server.verify_sequence(&patterns).await.unwrap();
    assert!(result.matched, "Expected requests to occur in sequence");

    server.stop().await.unwrap();
}
```

### Python SDK

```python
from mockforge_sdk import MockServer
from mockforge_sdk.verification import VerificationRequest, VerificationCount

# Start a mock server
server = MockServer(port=3000).start()

# Stub a response
server.stub_response("GET", "/api/users", {"users": []})

# Make requests
import requests
requests.get("http://localhost:3000/api/users")
requests.get("http://localhost:3000/api/users")
requests.get("http://localhost:3000/api/users")

# Verify the request was made exactly 3 times
pattern = VerificationRequest(
    method="GET",
    path="/api/users",
    query_params={},
    headers={},
    body_pattern=None
)

result = server.verify(pattern, VerificationCount.exactly(3))
assert result.matched, "Expected GET /api/users to be called exactly 3 times"
assert result.count == 3

# Verify a request was never made
delete_pattern = VerificationRequest(
    method="DELETE",
    path="/api/users/1",
    query_params={},
    headers={},
    body_pattern=None
)

result = server.verify_never(delete_pattern)
assert result.matched, "Expected DELETE /api/users/1 to never be called"

# Verify at least N requests
result = server.verify_at_least(pattern, 2)
assert result.matched, "Expected at least 2 requests"

# Verify request sequence
patterns = [
    VerificationRequest(method="POST", path="/api/users", query_params={}, headers={}, body_pattern=None),
    VerificationRequest(method="GET", path="/api/users/1", query_params={}, headers={}, body_pattern=None),
]

result = server.verify_sequence(patterns)
assert result.matched, "Expected requests to occur in sequence"

server.stop()
```

### HTTP API

You can also use the verification API via HTTP endpoints:

#### Verify Requests

```bash
curl -X POST http://localhost:3000/api/verification/verify \
  -H "Content-Type: application/json" \
  -d '{
    "pattern": {
      "method": "GET",
      "path": "/api/users",
      "query_params": {},
      "headers": {},
      "body_pattern": null
    },
    "expected": {
      "type": "exactly",
      "value": 3
    }
  }'
```

Response:
```json
{
  "matched": true,
  "count": 3,
  "expected": {"type": "exactly", "value": 3},
  "matches": [...],
  "error_message": null
}
```

#### Get Count

```bash
curl -X POST http://localhost:3000/api/verification/count \
  -H "Content-Type: application/json" \
  -d '{
    "pattern": {
      "method": "GET",
      "path": "/api/users",
      "query_params": {},
      "headers": {},
      "body_pattern": null
    }
  }'
```

Response:
```json
{
  "count": 3
}
```

#### Verify Never

```bash
curl -X POST http://localhost:3000/api/verification/never \
  -H "Content-Type: application/json" \
  -d '{
    "method": "DELETE",
    "path": "/api/users/1",
    "query_params": {},
    "headers": {},
    "body_pattern": null
  }'
```

#### Verify At Least

```bash
curl -X POST http://localhost:3000/api/verification/at-least \
  -H "Content-Type: application/json" \
  -d '{
    "pattern": {
      "method": "GET",
      "path": "/api/users",
      "query_params": {},
      "headers": {},
      "body_pattern": null
    },
    "min": 2
  }'
```

#### Verify Sequence

```bash
curl -X POST http://localhost:3000/api/verification/sequence \
  -H "Content-Type: application/json" \
  -d '{
    "patterns": [
      {
        "method": "POST",
        "path": "/api/users",
        "query_params": {},
        "headers": {},
        "body_pattern": null
      },
      {
        "method": "GET",
        "path": "/api/users/1",
        "query_params": {},
        "headers": {},
        "body_pattern": null
      }
    ]
  }'
```

## Pattern Matching

### Path Matching

The path field supports multiple matching modes:

1. **Exact match**: `/api/users` matches only `/api/users`
2. **Wildcard (`*`)**: `/api/users/*` matches `/api/users/1`, `/api/users/2`, etc.
3. **Double wildcard (`**`)**: `/api/**` matches `/api/users`, `/api/users/1`, `/api/users/1/posts`, etc.
4. **Regex**: `^/api/users/\d+$` matches `/api/users/123`, `/api/users/456`, etc.

Examples:

```rust
// Exact match
path: Some("/api/users".to_string())

// Single wildcard (matches one segment)
path: Some("/api/users/*".to_string())  // Matches /api/users/1, /api/users/2

// Double wildcard (matches zero or more segments)
path: Some("/api/**".to_string())  // Matches /api/users, /api/users/1, /api/users/1/posts

// Regex
path: Some(r"^/api/users/\d+$".to_string())  // Matches /api/users/123, /api/users/456
```

### Header Matching

Headers are matched case-insensitively by name, and exactly by value:

```rust
let pattern = VerificationRequest {
    method: Some("GET".to_string()),
    path: Some("/api/users".to_string()),
    query_params: HashMap::new(),
    headers: HashMap::from([
        ("Authorization".to_string(), "Bearer token123".to_string()),
        ("Content-Type".to_string(), "application/json".to_string()),
    ]),
    body_pattern: None,
};
```

### Body Matching

Body patterns support exact match or regex:

```rust
let pattern = VerificationRequest {
    method: Some("POST".to_string()),
    path: Some("/api/users".to_string()),
    query_params: HashMap::new(),
    headers: HashMap::new(),
    body_pattern: Some(r#"{"name":".*"}"#.to_string()),  // Regex pattern
};
```

## Best Practices

1. **Use specific patterns**: The more specific your pattern, the more reliable your verification
2. **Verify after requests**: Make sure to verify after all requests have been made
3. **Use appropriate count assertions**: Use `Exactly` when you know the exact count, `AtLeast` when you only care about minimum
4. **Inspect matches**: Use the `matches` field in `VerificationResult` to debug failed verifications
5. **Clear logs between tests**: Consider clearing request logs between test cases for isolation

## Integration with Test Frameworks

### Rust (tokio-test)

```rust
#[tokio::test]
async fn test_user_creation_flow() {
    let mut server = MockServer::new().port(3000).start().await.unwrap();

    // Setup stubs...

    // Execute test scenario...

    // Verify requests
    let pattern = VerificationRequest {
        method: Some("POST".to_string()),
        path: Some("/api/users".to_string()),
        query_params: HashMap::new(),
        headers: HashMap::new(),
        body_pattern: None,
    };

    let result = server.verify(&pattern, VerificationCount::Exactly(1)).await.unwrap();
    assert!(result.matched, "User creation request should be made exactly once");
}
```

### Python (pytest)

```python
import pytest
from mockforge_sdk import MockServer
from mockforge_sdk.verification import VerificationRequest, VerificationCount

@pytest.mark.asyncio
async def test_user_creation_flow():
    server = MockServer(port=3000).start()

    # Setup stubs...

    # Execute test scenario...

    # Verify requests
    pattern = VerificationRequest(
        method="POST",
        path="/api/users",
        query_params={},
        headers={},
        body_pattern=None
    )

    result = server.verify(pattern, VerificationCount.exactly(1))
    assert result.matched, "User creation request should be made exactly once"
```

## API Reference

### Rust SDK

- `MockServer::verify(pattern, expected) -> Result<VerificationResult>`
- `MockServer::verify_never(pattern) -> Result<VerificationResult>`
- `MockServer::verify_at_least(pattern, min) -> Result<VerificationResult>`
- `MockServer::verify_sequence(patterns) -> Result<VerificationResult>`

### HTTP API Endpoints

- `POST /api/verification/verify` - Verify requests with count assertion
- `POST /api/verification/count` - Get count of matching requests
- `POST /api/verification/sequence` - Verify request sequence
- `POST /api/verification/never` - Verify request was never made
- `POST /api/verification/at-least` - Verify at least N requests

### Admin API Endpoints

- `POST /__mockforge/verification/verify` - Verify requests (Admin UI)
- `POST /__mockforge/verification/count` - Get count (Admin UI)
- `POST /__mockforge/verification/sequence` - Verify sequence (Admin UI)
- `POST /__mockforge/verification/never` - Verify never (Admin UI)
- `POST /__mockforge/verification/at-least` - Verify at least (Admin UI)

## Limitations

1. **Query parameters**: Currently, query parameter matching is limited because `RequestLogEntry` doesn't store query parameters separately. This will be enhanced in a future release.

2. **Request body**: Body matching requires the request body to be stored in metadata. Not all request types may have body information available.

3. **Performance**: Verification queries scan all logged requests. For high-volume scenarios, consider using the count endpoint which is more efficient.

## See Also

- [Request Logging](../crates/mockforge-core/src/request_logger.rs) - How requests are logged
- [SDK Documentation](../sdk/README.md) - SDK usage guide
- [Admin UI](../docs/ADMIN_UI_V2.md) - Admin UI documentation
