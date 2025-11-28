# Comprehensive E2E Test Suite - Implementation Guide

Complete guide for creating an end-to-end test suite covering all protocols and SDKs in MockForge.

## Table of Contents

- [Overview](#overview)
- [Test Architecture](#test-architecture)
- [Protocol Coverage](#protocol-coverage)
- [SDK Coverage](#sdk-coverage)
- [Test Infrastructure](#test-infrastructure)
- [Implementation](#implementation)
- [CI/CD Integration](#cicd-integration)

---

## Overview

The E2E test suite validates complete workflows across all MockForge protocols and SDKs, ensuring end-to-end functionality from client to server.

### Test Coverage Goals

- ✅ **All Protocols**: HTTP, gRPC, WebSocket, GraphQL, Kafka, MQTT, AMQP, SMTP, FTP, TCP
- ✅ **All SDKs**: Node.js, Python, Go, Java, .NET, Rust
- ✅ **Cross-Protocol**: Protocol interactions and conversions
- ✅ **Real-World Scenarios**: Common use cases and workflows

---

## Test Architecture

### Test Structure

```
tests/e2e/
├── protocols/          # Protocol-specific E2E tests
│   ├── http/
│   ├── grpc/
│   ├── websocket/
│   ├── graphql/
│   ├── kafka/
│   ├── mqtt/
│   ├── amqp/
│   ├── smtp/
│   ├── ftp/
│   └── tcp/
├── sdks/              # SDK-specific E2E tests
│   ├── nodejs/
│   ├── python/
│   ├── go/
│   ├── java/
│   ├── dotnet/
│   └── rust/
├── cross-protocol/    # Cross-protocol integration tests
├── scenarios/         # Real-world scenario tests
├── fixtures/          # Test data and configurations
└── helpers/           # Shared test utilities
```

### Test Framework

**Rust Integration Tests:**
- Use `tokio::test` for async tests
- Shared test server setup
- Protocol-specific test utilities

**SDK Tests:**
- Language-specific test frameworks
- Shared test scenarios
- Cross-language validation

---

## Protocol Coverage

### HTTP/REST

**Test Scenarios:**

```rust
// tests/e2e/protocols/http/http_e2e_tests.rs

#[tokio::test]
async fn test_http_basic_get() {
    // Start server with HTTP config
    let server = start_test_server(http_config()).await;

    // Make GET request
    let response = reqwest::get(&format!("http://localhost:{}/api/users", server.port))
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body.is_array());
}

#[tokio::test]
async fn test_http_post_with_validation() {
    // Test POST with request validation
    let server = start_test_server(http_config()).await;

    let client = reqwest::Client::new();
    let response = client
        .post(&format!("http://localhost:{}/api/users", server.port))
        .json(&serde_json::json!({
            "name": "Alice",
            "email": "alice@example.com"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 201);
}

#[tokio::test]
async fn test_http_dynamic_stub_creation() {
    // Test Admin API stub creation
    let server = start_test_server(http_config()).await;

    // Create stub via Admin API
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("http://localhost:{}/__mockforge/api/mocks", server.admin_port))
        .json(&serde_json::json!({
            "path": "/api/test",
            "method": "GET",
            "response": {
                "status": 200,
                "body": {"message": "test"}
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 201);

    // Verify stub works
    let test_response = reqwest::get(&format!("http://localhost:{}/api/test", server.port))
        .await
        .unwrap();
    assert_eq!(test_response.status(), 200);
}
```

### gRPC

**Test Scenarios:**

```rust
// tests/e2e/protocols/grpc/grpc_e2e_tests.rs

#[tokio::test]
async fn test_grpc_unary_call() {
    let server = start_test_server(grpc_config()).await;

    let mut client = create_grpc_client(&server).await;

    let request = Request::new(GetUserRequest {
        user_id: "123".to_string(),
    });

    let response = client.get_user(request).await.unwrap();
    assert_eq!(response.into_inner().name, "Test User");
}

#[tokio::test]
async fn test_grpc_streaming() {
    let server = start_test_server(grpc_config()).await;
    let mut client = create_grpc_client(&server).await;

    let request = Request::new(ListUsersRequest {
        page_size: 10,
    });

    let mut stream = client.list_users(request).await.unwrap().into_inner();

    let mut count = 0;
    while let Some(user) = stream.message().await.unwrap() {
        count += 1;
        assert!(!user.name.is_empty());
    }

    assert!(count > 0);
}

#[tokio::test]
async fn test_grpc_http_bridge() {
    // Test gRPC to HTTP bridge conversion
    let server = start_test_server(grpc_with_bridge_config()).await;

    // Call gRPC service via HTTP
    let response = reqwest::get(&format!(
        "http://localhost:{}/api/v1/users/123",
        server.http_port
    ))
    .await
    .unwrap();

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["id"], "123");
}
```

### WebSocket

**Test Scenarios:**

```rust
// tests/e2e/protocols/websocket/websocket_e2e_tests.rs

#[tokio::test]
async fn test_websocket_connection() {
    let server = start_test_server(websocket_config()).await;

    let (mut ws, _) = tokio_tungstenite::connect_async(
        format!("ws://localhost:{}/ws", server.ws_port)
    )
    .await
    .unwrap();

    // Send message
    ws.send(Message::Text(r#"{"type": "ping"}"#.to_string()))
        .await
        .unwrap();

    // Receive response
    let msg = ws.next().await.unwrap().unwrap();
    assert_eq!(msg, Message::Text(r#"{"type": "pong"}"#.to_string()));
}

#[tokio::test]
async fn test_websocket_broadcast() {
    let server = start_test_server(websocket_config()).await;

    // Connect multiple clients
    let mut clients = Vec::new();
    for _ in 0..5 {
        let (ws, _) = tokio_tungstenite::connect_async(
            format!("ws://localhost:{}/ws", server.ws_port)
        )
        .await
        .unwrap();
        clients.push(ws);
    }

    // Send broadcast message
    let admin_client = reqwest::Client::new();
    admin_client
        .post(&format!("http://localhost:{}/__mockforge/api/broadcast", server.admin_port))
        .json(&serde_json::json!({
            "message": {"type": "notification", "text": "Hello"}
        }))
        .send()
        .await
        .unwrap();

    // Verify all clients received message
    for mut client in clients {
        let msg = client.next().await.unwrap().unwrap();
        assert!(msg.to_string().contains("notification"));
    }
}
```

### GraphQL

**Test Scenarios:**

```rust
// tests/e2e/protocols/graphql/graphql_e2e_tests.rs

#[tokio::test]
async fn test_graphql_query() {
    let server = start_test_server(graphql_config()).await;

    let query = r#"
        query {
            users {
                id
                name
                email
            }
        }
    "#;

    let response = reqwest::Client::new()
        .post(&format!("http://localhost:{}/graphql", server.http_port))
        .json(&serde_json::json!({
            "query": query
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["data"]["users"].is_array());
}

#[tokio::test]
async fn test_graphql_mutation() {
    let server = start_test_server(graphql_config()).await;

    let mutation = r#"
        mutation {
            createUser(input: {name: "Alice", email: "alice@example.com"}) {
                id
                name
            }
        }
    "#;

    let response = reqwest::Client::new()
        .post(&format!("http://localhost:{}/graphql", server.http_port))
        .json(&serde_json::json!({
            "query": mutation
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["data"]["createUser"]["id"].is_string());
}
```

### Kafka

**Test Scenarios:**

```rust
// tests/e2e/protocols/kafka/kafka_e2e_tests.rs

#[tokio::test]
async fn test_kafka_produce_consume() {
    let server = start_test_server(kafka_config()).await;

    // Produce message
    let producer = create_kafka_producer(&server).await;
    producer
        .send("test-topic", "key", "value")
        .await
        .unwrap();

    // Consume message
    let consumer = create_kafka_consumer(&server, "test-topic").await;
    let message = consumer.recv().await.unwrap();

    assert_eq!(message.key(), "key");
    assert_eq!(message.value(), "value");
}
```

---

## SDK Coverage

### Node.js SDK

**Test File:** `tests/e2e/sdks/nodejs/nodejs_e2e.test.ts`

```typescript
import { MockServer } from '@mockforge/sdk';

describe('Node.js SDK E2E', () => {
  let server: MockServer;

  beforeEach(async () => {
    server = new MockServer({ port: 0 });
    await server.start();
  });

  afterEach(async () => {
    await server.stop();
  });

  test('should start server and handle requests', async () => {
    await server.stubResponse({
      path: '/api/users',
      method: 'GET',
      response: {
        status: 200,
        body: [{ id: 1, name: 'Alice' }]
      }
    });

    const response = await fetch(`http://localhost:${server.port}/api/users`);
    const data = await response.json();

    expect(response.status).toBe(200);
    expect(data).toHaveLength(1);
    expect(data[0].name).toBe('Alice');
  });

  test('should support dynamic stub updates', async () => {
    await server.stubResponse({
      path: '/api/test',
      method: 'GET',
      response: { status: 200, body: { message: 'initial' } }
    });

    // Update stub
    await server.updateStub({
      path: '/api/test',
      method: 'GET',
      response: { status: 200, body: { message: 'updated' } }
    });

    const response = await fetch(`http://localhost:${server.port}/api/test`);
    const data = await response.json();

    expect(data.message).toBe('updated');
  });

  test('should handle port discovery', async () => {
    const dynamicServer = new MockServer({ port: 0 });
    await dynamicServer.start();

    expect(dynamicServer.port).toBeGreaterThan(0);
    expect(dynamicServer.adminPort).toBeGreaterThan(0);

    await dynamicServer.stop();
  });
});
```

### Python SDK

**Test File:** `tests/e2e/sdks/python/test_python_e2e.py`

```python
import pytest
from mockforge_sdk import MockServer

@pytest.fixture
async def server():
    server = MockServer(port=0)
    await server.start()
    yield server
    await server.stop()

@pytest.mark.asyncio
async def test_basic_request(server):
    await server.stub_response(
        path="/api/users",
        method="GET",
        response={
            "status": 200,
            "body": [{"id": 1, "name": "Alice"}]
        }
    )

    response = await fetch(f"http://localhost:{server.port}/api/users")
    data = await response.json()

    assert response.status == 200
    assert len(data) == 1
    assert data[0]["name"] == "Alice"

@pytest.mark.asyncio
async def test_dynamic_stub_updates(server):
    await server.stub_response(
        path="/api/test",
        method="GET",
        response={"status": 200, "body": {"message": "initial"}}
    )

    await server.update_stub(
        path="/api/test",
        method="GET",
        response={"status": 200, "body": {"message": "updated"}}
    )

    response = await fetch(f"http://localhost:{server.port}/api/test")
    data = await response.json()

    assert data["message"] == "updated"
```

### Go SDK

**Test File:** `tests/e2e/sdks/go/go_e2e_test.go`

```go
package e2e

import (
    "testing"
    "github.com/mockforge/sdk-go"
)

func TestBasicRequest(t *testing.T) {
    server := mockserver.NewMockServer(mockserver.Config{Port: 0})
    defer server.Stop()

    err := server.Start()
    if err != nil {
        t.Fatalf("Failed to start server: %v", err)
    }

    err = server.StubResponse(mockserver.ResponseStub{
        Path:   "/api/users",
        Method: "GET",
        Response: mockserver.Response{
            Status: 200,
            Body:   []map[string]interface{}{{"id": 1, "name": "Alice"}},
        },
    })
    if err != nil {
        t.Fatalf("Failed to create stub: %v", err)
    }

    resp, err := http.Get(fmt.Sprintf("http://localhost:%d/api/users", server.Port()))
    if err != nil {
        t.Fatalf("Request failed: %v", err)
    }
    defer resp.Body.Close()

    if resp.StatusCode != 200 {
        t.Errorf("Expected status 200, got %d", resp.StatusCode)
    }

    var data []map[string]interface{}
    json.NewDecoder(resp.Body).Decode(&data)

    if len(data) != 1 {
        t.Errorf("Expected 1 user, got %d", len(data))
    }
}
```

---

## Test Infrastructure

### Shared Test Server

```rust
// tests/e2e/helpers/test_server.rs

pub struct TestServer {
    pub http_port: u16,
    pub admin_port: u16,
    pub grpc_port: Option<u16>,
    pub ws_port: Option<u16>,
    process: Option<Child>,
}

impl TestServer {
    pub async fn start(config: ServerConfig) -> Result<Self> {
        // Start MockForge server
        let mut cmd = Command::new("mockforge");
        cmd.arg("serve")
            .arg("--http-port")
            .arg("0")
            .arg("--admin-port")
            .arg("0");

        let mut child = cmd.spawn()?;

        // Wait for ports to be assigned
        let (http_port, admin_port) = wait_for_ports(&mut child).await?;

        Ok(Self {
            http_port,
            admin_port,
            grpc_port: config.grpc.map(|g| g.port),
            ws_port: config.websocket.map(|w| w.port),
            process: Some(child),
        })
    }

    pub async fn stop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
            let _ = process.wait().await;
        }
    }
}

async fn wait_for_ports(child: &mut Child) -> Result<(u16, u16)> {
    // Parse ports from stdout
    // Implementation similar to SDK port discovery
}
```

### Test Fixtures

```rust
// tests/e2e/fixtures/configs.rs

pub fn http_config() -> ServerConfig {
    ServerConfig {
        http: HttpConfig {
            port: 0,
            routes: vec![],
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn grpc_config() -> ServerConfig {
    ServerConfig {
        grpc: Some(GrpcConfig {
            port: 0,
            services: vec![],
            ..Default::default()
        }),
        ..Default::default()
    }
}
```

---

## Implementation

### Test Organization

**By Protocol:**
- `tests/e2e/protocols/http/`
- `tests/e2e/protocols/grpc/`
- `tests/e2e/protocols/websocket/`
- etc.

**By SDK:**
- `tests/e2e/sdks/nodejs/`
- `tests/e2e/sdks/python/`
- `tests/e2e/sdks/go/`
- etc.

**Cross-Protocol:**
- `tests/e2e/cross-protocol/grpc_http_bridge.rs`
- `tests/e2e/cross-protocol/websocket_rest.rs`

### Running Tests

```bash
# Run all E2E tests
cargo test --test e2e

# Run protocol-specific tests
cargo test --test http_e2e
cargo test --test grpc_e2e

# Run SDK-specific tests
cd tests/e2e/sdks/nodejs && npm test
cd tests/e2e/sdks/python && pytest
cd tests/e2e/sdks/go && go test
```

---

## CI/CD Integration

### GitHub Actions

```yaml
# .github/workflows/e2e-tests.yml

name: E2E Tests

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  e2e-protocols:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run HTTP E2E tests
        run: cargo test --test http_e2e

      - name: Run gRPC E2E tests
        run: cargo test --test grpc_e2e

      - name: Run WebSocket E2E tests
        run: cargo test --test websocket_e2e

  e2e-sdks:
    strategy:
      matrix:
        sdk: [nodejs, python, go]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup Node.js
        if: matrix.sdk == 'nodejs'
        uses: actions/setup-node@v3
        with:
          node-version: '18'

      - name: Setup Python
        if: matrix.sdk == 'python'
        uses: actions/setup-python@v4
        with:
          python-version: '3.11'

      - name: Setup Go
        if: matrix.sdk == 'go'
        uses: actions/setup-go@v4
        with:
          go-version: '1.21'

      - name: Run SDK E2E tests
        run: |
          cd tests/e2e/sdks/${{ matrix.sdk }}
          # Run SDK-specific test command
```

---

## Summary

The E2E test suite provides:

- ✅ **Protocol Coverage**: All 10+ protocols
- ✅ **SDK Coverage**: All 6 SDKs
- ✅ **Cross-Protocol**: Integration tests
- ✅ **Real-World Scenarios**: Common use cases
- ✅ **CI/CD Integration**: Automated testing

**Status**: Infrastructure exists, comprehensive test coverage needed

---

**Last Updated**: 2024-01-01
**Version**: 1.0
