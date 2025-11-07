# MockForge Ecosystem & Use Cases

## Overview

MockForge provides a comprehensive multi-language ecosystem and supports diverse use cases for API mocking, service virtualization, and testing. This document covers MockForge's ecosystem capabilities, compares them with WireMock, and provides detailed guidance for common usage scenarios.

---

## Part 1: Multi-Language Ecosystem

### Ecosystem Overview

MockForge offers native SDKs and client libraries across multiple programming languages, enabling developers to embed mock servers directly in their test suites regardless of their technology stack.

#### MockForge vs WireMock: Ecosystem Comparison

| Aspect | MockForge | WireMock |
|--------|-----------|----------|
| **Core Language** | Rust (native) | Java/JVM |
| **SDK Availability** | Native SDKs for 6 languages | Java library + client libraries |
| **Embedded Testing** | ✅ Native embedding in all languages | ✅ Java embedding, clients for others |
| **CLI Requirement** | Optional (Rust SDK embeds directly) | Optional (standalone mode available) |
| **Plugin System** | WASM-based (multi-language) + Remote Protocol | Java extensions |
| **Performance** | ⚡ High (native Rust) | Medium (JVM overhead) |

**Key Difference**: While WireMock is primarily a Java library with client wrappers, MockForge provides native SDKs that can embed the mock server directly or communicate with a standalone server, offering flexibility for different use cases.

### Available SDKs

#### 1. Rust SDK (Native)

**Status**: ✅ Native implementation, zero overhead

The Rust SDK embeds MockForge directly in your Rust projects without requiring the CLI.

```toml
[dev-dependencies]
mockforge-sdk = "0.1"
tokio-test = "0.4"
reqwest = "0.12"
```

**Example**:
```rust
use mockforge_sdk::MockServer;
use serde_json::json;

#[tokio::test]
async fn test_user_api() {
    let mut server = MockServer::new()
        .port(3000)
        .start()
        .await
        .expect("Failed to start server");

    server
        .stub_response("GET", "/api/users/{id}", json!({
            "id": "{{uuid}}",
            "name": "{{faker.name}}",
            "email": "{{faker.email}}"
        }))
        .await
        .expect("Failed to stub response");

    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:3000/api/users/123")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    server.stop().await.unwrap();
}
```

#### 2. Node.js/TypeScript SDK

**Status**: ✅ Full TypeScript support

```bash
npm install @mockforge/sdk
```

**Example**:
```typescript
import { MockServer } from '@mockforge/sdk';

describe('API Tests', () => {
  let server: MockServer;

  beforeEach(async () => {
    server = await MockServer.start({ port: 3000 });
  });

  afterEach(async () => {
    await server.stop();
  });

  it('should mock user API', async () => {
    await server.stubResponse('GET', '/api/users/123', {
      id: 123,
      name: 'John Doe',
      email: 'john@example.com'
    });

    const response = await fetch('http://localhost:3000/api/users/123');
    const data = await response.json();

    expect(data.id).toBe(123);
    expect(data.name).toBe('John Doe');
  });
});
```

#### 3. Python SDK

**Status**: ✅ Context manager support, type hints

```bash
pip install mockforge-sdk
```

**Example**:
```python
from mockforge_sdk import MockServer
import requests

def test_user_api():
    with MockServer(port=3000) as server:
        server.stub_response('GET', '/api/users/123', {
            'id': 123,
            'name': 'John Doe',
            'email': 'john@example.com'
        })

        response = requests.get('http://localhost:3000/api/users/123')
        assert response.status_code == 200

        data = response.json()
        assert data['id'] == 123
        assert data['name'] == 'John Doe'
```

#### 4. Go SDK

**Status**: ✅ Idiomatic Go API

```bash
go get github.com/SaaSy-Solutions/mockforge/sdk/go
```

**Example**:
```go
package myapi_test

import (
    "testing"
    "net/http"
    "encoding/json"
    mockforge "github.com/SaaSy-Solutions/mockforge/sdk/go"
    "github.com/stretchr/testify/assert"
)

func TestUserAPI(t *testing.T) {
    server := mockforge.NewMockServer(mockforge.MockServerConfig{
        Port: 3000,
    })

    err := server.Start()
    assert.NoError(t, err)
    defer server.Stop()

    err = server.StubResponse("GET", "/api/users/123", map[string]interface{}{
        "id":    123,
        "name":  "John Doe",
        "email": "john@example.com",
    })
    assert.NoError(t, err)

    resp, err := http.Get("http://localhost:3000/api/users/123")
    assert.NoError(t, err)
    assert.Equal(t, 200, resp.StatusCode)

    var data map[string]interface{}
    json.NewDecoder(resp.Body).Decode(&data)
    assert.Equal(t, float64(123), data["id"])
}
```

#### 5. Java SDK

**Status**: ✅ Maven/Gradle support

**Maven**:
```xml
<dependency>
    <groupId>com.mockforge</groupId>
    <artifactId>mockforge-sdk</artifactId>
    <version>0.1.0</version>
    <scope>test</scope>
</dependency>
```

**Example**:
```java
import com.mockforge.sdk.MockServer;
import com.mockforge.sdk.MockServerConfig;
import com.mockforge.sdk.MockServerException;

public class UserApiTest {
    @Test
    public void testUserApi() throws MockServerException {
        MockServer server = MockServer.start(MockServerConfig.builder()
            .port(3000)
            .build());

        try {
            server.stubResponse("GET", "/api/users/123", Map.of(
                "id", 123,
                "name", "John Doe",
                "email", "john@example.com"
            ));

            // Make requests to http://localhost:3000/api/users/123
        } finally {
            server.stop();
        }
    }
}
```

#### 6. .NET SDK

**Status**: ✅ NuGet package

```bash
dotnet add package MockForge.Sdk
```

**Example**:
```csharp
using MockForge.Sdk;

public class UserApiTests
{
    [Fact]
    public async Task TestUserApi()
    {
        var server = await MockServer.StartAsync(new MockServerConfig
        {
            Port = 3000
        });

        try
        {
            await server.StubResponseAsync("GET", "/api/users/123", new
            {
                id = 123,
                name = "John Doe",
                email = "john@example.com"
            });

            // Make requests to http://localhost:3000/api/users/123
        }
        finally
        {
            server.Dispose();
        }
    }
}
```

### Plugin System: Extending Functionality

MockForge's plugin system enables extending functionality across languages:

1. **WASM Plugins**: Write plugins in Rust, Go (via TinyGo), or AssemblyScript
2. **Remote Plugins**: Run plugins as standalone HTTP/gRPC services in any language

This provides flexibility beyond WireMock's Java-only extension model.

---

## Part 2: Use Cases

### Use Case 1: Unit Tests

**Scenario**: Embed mock servers directly in unit test suites to test components in isolation.

#### MockForge Approach

MockForge SDKs allow embedding mock servers directly in test code, similar to WireMock's `@WireMockTest` annotation but available across all languages.

#### Examples by Language

**Rust**:
```rust
use mockforge_sdk::MockServer;
use serde_json::json;

#[tokio::test]
async fn test_user_service() {
    let mut server = MockServer::new()
        .port(0) // Random port
        .start()
        .await
        .unwrap();

    // Stub external API dependency
    server.stub_response("GET", "/api/external/users/123", json!({
        "id": 123,
        "name": "Test User"
    })).await.unwrap();

    // Test your service that depends on external API
    let user_service = UserService::new(server.url());
    let user = user_service.get_user(123).await.unwrap();

    assert_eq!(user.name, "Test User");

    server.stop().await.unwrap();
}
```

**Node.js/TypeScript**:
```typescript
import { MockServer } from '@mockforge/sdk';

describe('UserService', () => {
  let mockServer: MockServer;

  beforeAll(async () => {
    mockServer = await MockServer.start({ port: 0 });
  });

  afterAll(async () => {
    await mockServer.stop();
  });

  it('should fetch user from external API', async () => {
    await mockServer.stubResponse('GET', '/api/external/users/123', {
      id: 123,
      name: 'Test User'
    });

    const userService = new UserService(mockServer.url());
    const user = await userService.getUser(123);

    expect(user.name).toBe('Test User');
  });
});
```

**Python**:
```python
from mockforge_sdk import MockServer
import pytest

@pytest.fixture
def mock_server():
    with MockServer(port=0) as server:
        yield server

def test_user_service(mock_server):
    mock_server.stub_response('GET', '/api/external/users/123', {
        'id': 123,
        'name': 'Test User'
    })

    user_service = UserService(mock_server.url())
    user = user_service.get_user(123)

    assert user.name == 'Test User'
```

**Go**:
```go
func TestUserService(t *testing.T) {
    server := mockforge.NewMockServer(mockforge.MockServerConfig{
        Port: 0, // Random port
    })
    defer server.Stop()

    server.Start()

    server.StubResponse("GET", "/api/external/users/123", map[string]interface{}{
        "id":   123,
        "name": "Test User",
    })

    userService := NewUserService(server.URL())
    user, err := userService.GetUser(123)
    assert.NoError(t, err)
    assert.Equal(t, "Test User", user.Name)
}
```

**Java**:
```java
@Test
public void testUserService() throws MockServerException {
    MockServer server = MockServer.start(MockServerConfig.builder()
        .port(0) // Random port
        .build());

    try {
        server.stubResponse("GET", "/api/external/users/123", Map.of(
            "id", 123,
            "name", "Test User"
        ));

        UserService userService = new UserService(server.getUrl());
        User user = userService.getUser(123);

        assertEquals("Test User", user.getName());
    } finally {
        server.stop();
    }
}
```

**.NET**:
```csharp
[Fact]
public async Task TestUserService()
{
    var server = await MockServer.StartAsync(new MockServerConfig
    {
        Port = 0 // Random port
    });

    try
    {
        await server.StubResponseAsync("GET", "/api/external/users/123", new
        {
            id = 123,
            name = "Test User"
        });

        var userService = new UserService(server.GetUrl());
        var user = await userService.GetUser(123);

        Assert.Equal("Test User", user.Name);
    }
    finally
    {
        server.Dispose();
    }
}
```

#### Comparison with WireMock

| Feature | MockForge | WireMock |
|---------|-----------|----------|
| **Language Support** | 6 languages natively | Java natively, clients for others |
| **Embedded Mode** | ✅ All languages | ✅ Java only |
| **Test Framework Integration** | ✅ All major frameworks | ✅ JUnit, TestNG |
| **Template Support** | ✅ Advanced (faker, UUIDs, time) | ⚠️ Basic |

---

### Use Case 2: Integration Tests

**Scenario**: Test complex multi-service interactions with stateful mocking and protocol-specific features.

#### MockForge Approach

MockForge supports multi-protocol mocking (HTTP, gRPC, WebSocket) in a single server, enabling comprehensive integration testing.

#### Examples

**Multi-Protocol Integration Test (Rust)**:
```rust
use mockforge_sdk::MockServer;

#[tokio::test]
async fn test_payment_flow() {
    let mut server = MockServer::new()
        .port(3000)
        .grpc_port(50051)
        .start()
        .await
        .unwrap();

    // HTTP endpoint for order creation
    server.stub_response("POST", "/api/orders", json!({
        "order_id": "{{uuid}}",
        "status": "pending"
    })).await.unwrap();

    // gRPC service for payment processing
    server.stub_grpc_response(
        "payment.PaymentService",
        "ProcessPayment",
        json!({
            "success": true,
            "transaction_id": "{{uuid}}"
        })
    ).await.unwrap();

    // Test the full flow
    let order = create_order(&server.url()).await;
    let payment = process_payment(&server.grpc_url(), order.id).await;

    assert!(payment.success);

    server.stop().await.unwrap();
}
```

**Stateful Integration Test (Node.js)**:
```typescript
describe('Order Processing Flow', () => {
  let server: MockServer;

  beforeAll(async () => {
    server = await MockServer.start({ port: 3000 });
  });

  it('should handle order lifecycle', async () => {
    // Initial state: order created
    await server.stubResponse('POST', '/api/orders', {
      order_id: '{{uuid}}',
      status: 'pending'
    });

    // State transition: order confirmed
    await server.stubResponse('PUT', '/api/orders/{id}/confirm', {
      order_id: '{{uuid}}',
      status: 'confirmed'
    }, {
      status: 200,
      headers: { 'X-Order-State': 'confirmed' }
    });

    const order = await createOrder(server.url());
    const confirmed = await confirmOrder(server.url(), order.id);

    expect(confirmed.status).toBe('confirmed');
  });
});
```

**WebSocket Integration Test (Python)**:
```python
import asyncio
from mockforge_sdk import MockServer

async def test_realtime_notifications():
    with MockServer(port=3000, ws_port=3001) as server:
        # HTTP endpoint to trigger notification
        server.stub_response('POST', '/api/notify', {
            'message_id': '{{uuid}}',
            'status': 'sent'
        })

        # WebSocket connection for real-time updates
        async with websockets.connect(server.ws_url()) as ws:
            # Trigger notification
            await trigger_notification(server.url())

            # Receive WebSocket message
            message = await ws.recv()
            data = json.loads(message)

            assert data['type'] == 'notification'
            assert 'message_id' in data
```

#### Comparison with WireMock

| Feature | MockForge | WireMock |
|---------|-----------|----------|
| **Multi-Protocol** | ✅ HTTP, gRPC, WebSocket, GraphQL | ⚠️ HTTP only |
| **Stateful Mocking** | ✅ Full support | ✅ Full support |
| **Protocol-Specific** | ✅ Native gRPC, WebSocket | ❌ HTTP only |
| **Scenario Switching** | ✅ Header-based | ✅ State machine |

---

### Use Case 3: Service Virtualization

**Scenario**: Replace external dependencies (third-party APIs, microservices) with mocks for development and testing.

#### MockForge Approach

MockForge provides proxy mode with record/replay capabilities, allowing you to capture real API behavior and replay it later.

#### Examples

**Record/Replay Workflow**:
```bash
# Step 1: Record real API interactions
mockforge serve --proxy-enabled \
  --proxy-target https://api.external-service.com \
  --record-responses ./recordings/

# Step 2: Replay from recordings
mockforge serve --replay-from ./recordings/
```

**Programmatic Service Virtualization (Rust)**:
```rust
use mockforge_sdk::MockServer;

#[tokio::test]
async fn test_with_virtualized_service() {
    let mut server = MockServer::new()
        .port(3000)
        .proxy_config(ProxyConfig {
            enabled: true,
            target_url: Some("https://api.external-service.com".to_string()),
            record_responses: true,
            replay_from: Some("./recordings/".to_string()),
            ..Default::default()
        })
        .start()
        .await
        .unwrap();

    // First call: proxies to real API and records
    let response1 = reqwest::get("http://localhost:3000/api/data")
        .await
        .unwrap();

    // Subsequent calls: replay from recording
    let response2 = reqwest::get("http://localhost:3000/api/data")
        .await
        .unwrap();

    // Both responses are identical (from recording)
    assert_eq!(response1.text().await.unwrap(), response2.text().await.unwrap());

    server.stop().await.unwrap();
}
```

**OpenAPI-Driven Virtualization (Node.js)**:
```typescript
import { MockServer } from '@mockforge/sdk';

// Virtualize an external API from its OpenAPI spec
const server = await MockServer.start({
  port: 3000,
  openApiSpec: './external-api-openapi.json'
});

// All endpoints from OpenAPI spec are automatically mocked
const response = await fetch('http://localhost:3000/api/v1/users');
const users = await response.json();

// Responses are generated from schema
expect(users).toBeArray();
expect(users[0]).toHaveProperty('id');
expect(users[0]).toHaveProperty('name');
```

**Proxy with Conditional Routing (Python)**:
```python
from mockforge_sdk import MockServer, ProxyConfig

with MockServer(
    port=3000,
    proxy_config=ProxyConfig(
        enabled=True,
        target_url="https://api.external-service.com",
        rules=[
            {
                "pattern": "/api/v1/*",
                "upstream_url": "https://api.external-service.com/v1",
                "enabled": True
            },
            {
                "pattern": "/api/v2/*",
                "upstream_url": "https://api.external-service.com/v2",
                "enabled": True
            }
        ]
    )
) as server:
    # Requests to /api/v1/* proxy to v1
    # Requests to /api/v2/* proxy to v2
    # Other requests use mocks
    pass
```

#### Comparison with WireMock

| Feature | MockForge | WireMock |
|---------|-----------|----------|
| **Proxy Mode** | ✅ Full support | ✅ Full support |
| **Record/Replay** | ✅ Built-in | ✅ Built-in |
| **OpenAPI Integration** | ✅ Auto-generate from spec | ⚠️ Manual mapping |
| **Multi-Protocol Proxy** | ✅ HTTP, gRPC, WebSocket | ⚠️ HTTP only |

---

### Use Case 4: Development/Stub Environments

**Scenario**: Create local development environments without backend dependencies, enabling parallel team development.

#### MockForge Approach

MockForge can run as a standalone server with configuration files, making it easy to share mock environments across teams.

#### Examples

**Standalone Development Server**:
```bash
# Start with OpenAPI spec
mockforge serve --spec api-spec.yaml --admin

# Or with configuration file
mockforge serve --config dev-config.yaml
```

**Configuration File (dev-config.yaml)**:
```yaml
http:
  port: 3000
  openapi_spec: ./api-spec.yaml

grpc:
  port: 50051
  proto_dir: ./proto

websocket:
  port: 3001
  replay_file: ./ws-scripts.jsonl

# Enable Admin UI for team collaboration
admin:
  enabled: true
  port: 9080

# Workspace sync for team collaboration
workspace:
  enabled: true
  directory: ./mocks
  git_sync: true
```

**Team Collaboration with Workspace Sync**:
```bash
# Developer 1: Start server with workspace
mockforge serve --workspace ./shared-mocks --admin

# Developer 2: Connect to same workspace
mockforge serve --workspace ./shared-mocks --admin

# Changes sync automatically via file watching
```

**CI/CD Integration**:
```yaml
# .github/workflows/test.yml
- name: Start MockForge
  run: |
    mockforge serve --spec api-spec.yaml --port 3000 &
    sleep 2

- name: Run Tests
  run: npm test

- name: Stop MockForge
  run: pkill mockforge
```

**Docker Compose for Team Environments**:
```yaml
version: '3.8'
services:
  mockforge:
    image: ghcr.io/saasy-solutions/mockforge:latest
    ports:
      - "3000:3000"
      - "9080:9080"
    volumes:
      - ./mocks:/app/mocks
      - ./api-spec.yaml:/app/api-spec.yaml
    command: serve --spec /app/api-spec.yaml --admin
```

#### Comparison with WireMock

| Feature | MockForge | WireMock |
|---------|-----------|----------|
| **Standalone Mode** | ✅ Full support | ✅ Full support |
| **Configuration Files** | ✅ YAML/JSON | ✅ JSON |
| **Workspace Sync** | ✅ Git integration | ❌ No |
| **Admin UI** | ✅ Modern React UI | ⚠️ Basic |
| **Docker Support** | ✅ Official images | ✅ Community images |

---

### Use Case 5: Isolating from Flaky Dependencies

**Scenario**: Test application behavior under various failure conditions (network failures, timeouts, slow responses) without depending on flaky external services.

#### MockForge Approach

MockForge provides built-in latency injection, fault injection, and failure simulation capabilities.

#### Examples

**Latency Injection (Rust)**:
```rust
use mockforge_sdk::{MockServer, LatencyProfile};

#[tokio::test]
async fn test_with_latency() {
    let mut server = MockServer::new()
        .port(3000)
        .latency(LatencyProfile::Normal {
            mean_ms: 500,
            std_dev_ms: 100,
        })
        .start()
        .await
        .unwrap();

    server.stub_response("GET", "/api/data", json!({
        "data": "test"
    })).await.unwrap();

    let start = std::time::Instant::now();
    let _response = reqwest::get("http://localhost:3000/api/data")
        .await
        .unwrap();
    let duration = start.elapsed();

    // Response should take ~500ms (with variance)
    assert!(duration.as_millis() >= 400);
    assert!(duration.as_millis() <= 600);
}
```

**Failure Injection (Node.js)**:
```typescript
import { MockServer, FailureConfig } from '@mockforge/sdk';

const server = await MockServer.start({
  port: 3000,
  failures: {
    enabled: true,
    failure_rate: 0.1, // 10% failure rate
    error_codes: [500, 503, 504],
    timeout_rate: 0.05 // 5% timeout rate
  }
});

await server.stubResponse('GET', '/api/data', {
  data: 'test'
});

// Some requests will fail (10% chance)
for (let i = 0; i < 100; i++) {
  const response = await fetch('http://localhost:3000/api/data');
  // Handle both success and failure cases
  if (!response.ok) {
    console.log(`Request ${i} failed with ${response.status}`);
  }
}
```

**Network Condition Simulation (Python)**:
```python
from mockforge_sdk import MockServer, LatencyProfile

# Simulate 3G network conditions
with MockServer(
    port=3000,
    latency=LatencyProfile.fixed(duration_ms=1000),  # 1 second delay
    failures={
        'enabled': True,
        'failure_rate': 0.05,  # 5% packet loss simulation
        'error_codes': [503, 504]
    }
) as server:
    server.stub_response('GET', '/api/data', {
        'data': 'test'
    })

    # Test retry logic
    response = requests.get('http://localhost:3000/api/data', timeout=2)
    # May fail or succeed depending on failure injection
```

**Chaos Engineering Patterns (Go)**:
```go
server := mockforge.NewMockServer(mockforge.MockServerConfig{
    Port: 3000,
    Latency: mockforge.LatencyProfile{
        Mode: "exponential",
        MeanMs: 500,
    },
    Failures: mockforge.FailureConfig{
        Enabled: true,
        FailureRate: 0.2, // 20% failure rate
        ErrorCodes: []int{500, 503},
        TimeoutRate: 0.1, // 10% timeout
    },
})

server.Start()
defer server.Stop()

// Test application resilience
for i := 0; i < 100; i++ {
    resp, err := http.Get("http://localhost:3000/api/data")
    // Application should handle failures gracefully
}
```

#### Comparison with WireMock

| Feature | MockForge | WireMock |
|---------|-----------|----------|
| **Latency Injection** | ✅ Fixed, Normal, Exponential | ✅ Fixed, LogNormal |
| **Failure Injection** | ✅ Configurable rates | ✅ Scenarios |
| **Timeout Simulation** | ✅ Built-in | ✅ Built-in |
| **Network Profiles** | ✅ 3G, 4G, 5G presets | ⚠️ Manual configuration |
| **Chaos Patterns** | ✅ Advanced | ⚠️ Basic |

---

### Use Case 6: Simulating APIs That Don't Exist Yet

**Scenario**: Generate mocks from API specifications (OpenAPI, GraphQL, gRPC) before the actual API is implemented, enabling parallel development.

#### MockForge Approach

MockForge can generate realistic mocks directly from API specifications, with optional AI-powered enhancement for more realistic data.

#### Examples

**OpenAPI-Driven Mock Generation**:
```bash
# Generate mocks from OpenAPI spec
mockforge serve --spec api-spec.yaml

# All endpoints are automatically available
curl http://localhost:3000/api/users
curl http://localhost:3000/api/users/123
curl -X POST http://localhost:3000/api/users -d '{"name":"John"}'
```

**Programmatic Generation (Rust)**:
```rust
use mockforge_sdk::MockServer;

#[tokio::test]
async fn test_with_openapi_spec() {
    let mut server = MockServer::new()
        .port(3000)
        .openapi_spec("./api-spec.yaml")
        .start()
        .await
        .unwrap();

    // All endpoints from OpenAPI spec are automatically mocked
    let response = reqwest::get("http://localhost:3000/api/users")
        .await
        .unwrap();

    let users: Vec<serde_json::Value> = response.json().await.unwrap();

    // Data is generated from schema
    assert!(!users.is_empty());
    assert!(users[0].get("id").is_some());
    assert!(users[0].get("name").is_some());
}
```

**AI-Powered Mock Generation**:
```bash
# Enable AI-powered generation for more realistic data
mockforge serve --spec api-spec.yaml \
  --ai-enabled \
  --rag-provider ollama \
  --rag-model llama2
```

**GraphQL Schema Mocking**:
```bash
# Generate mocks from GraphQL schema
mockforge serve --graphql-schema schema.graphql --graphql-port 4000
```

**gRPC Service Mocking**:
```bash
# Generate mocks from .proto files
mockforge serve --grpc-port 50051 --proto-dir ./proto
```

**Schema-Based Data Generation (Node.js)**:
```typescript
import { MockServer } from '@mockforge/sdk';

// Start server with OpenAPI spec
const server = await MockServer.start({
  port: 3000,
  openApiSpec: './api-spec.yaml',
  // Enable template expansion for dynamic data
  templateExpansion: true
});

// All endpoints are automatically available
const response = await fetch('http://localhost:3000/api/users');
const users = await response.json();

// Data is generated from schema with realistic values
console.log(users); // [{ id: "uuid", name: "John Doe", email: "john@example.com" }, ...]
```

**Custom Schema Generation (Python)**:
```python
from mockforge_sdk import MockServer

# Generate mocks from JSON Schema
with MockServer(
    port=3000,
    schema_file='./custom-schema.json'
) as server:
    # Endpoints generated from schema
    response = requests.get('http://localhost:3000/api/data')
    data = response.json()

    # Data conforms to schema
    assert 'required_field' in data
```

#### Comparison with WireMock

| Feature | MockForge | WireMock |
|---------|-----------|----------|
| **OpenAPI Support** | ✅ Full auto-generation | ⚠️ Manual mapping |
| **GraphQL Support** | ✅ Native schema mocking | ❌ No |
| **gRPC Support** | ✅ Native proto mocking | ❌ No |
| **AI Enhancement** | ✅ LLM-powered generation | ❌ No |
| **Schema Validation** | ✅ Built-in | ⚠️ Manual |

---

## Part 3: Comparison Summary

### Language Support Matrix

| Language | MockForge | WireMock |
|----------|-----------|----------|
| **Rust** | ✅ Native SDK | ⚠️ Client library |
| **Java** | ✅ Native SDK | ✅ Native library |
| **Node.js/TypeScript** | ✅ Native SDK | ⚠️ Client library |
| **Python** | ✅ Native SDK | ⚠️ Client library |
| **Go** | ✅ Native SDK | ⚠️ Client library |
| **.NET/C#** | ✅ Native SDK | ⚠️ Client library |
| **Ruby** | ⚠️ HTTP client | ⚠️ Client library |
| **PHP** | ⚠️ HTTP client | ⚠️ Client library |

### Use Case Coverage Matrix

| Use Case | MockForge | WireMock |
|----------|-----------|----------|
| **Unit Tests** | ✅ All languages | ✅ Java, clients for others |
| **Integration Tests** | ✅ Multi-protocol | ✅ HTTP only |
| **Service Virtualization** | ✅ Full proxy + record/replay | ✅ Full proxy + record/replay |
| **Development Environments** | ✅ Workspace sync, Admin UI | ✅ Standalone mode |
| **Flaky Dependency Isolation** | ✅ Advanced chaos patterns | ✅ Basic failure injection |
| **Non-Existent API Simulation** | ✅ OpenAPI/GraphQL/gRPC | ⚠️ Manual setup |

### Feature Parity Analysis

**MockForge Advantages**:
- Multi-protocol support (HTTP, gRPC, WebSocket, GraphQL)
- Native SDKs for 6 languages
- AI-powered mock generation
- Advanced data generation (RAG-powered)
- Workspace synchronization
- Modern Admin UI

**WireMock Advantages**:
- Mature ecosystem (longer history)
- Extensive Java community
- More third-party integrations
- Wider adoption in enterprise

### Migration Guide from WireMock

**For Java Projects**:
1. Replace `wiremock-jre8` dependency with `mockforge-sdk`
2. Update imports from `com.github.tomakehurst.wiremock` to `com.mockforge.sdk`
3. API is similar, but MockForge uses builder pattern more extensively
4. Migrate stubs to MockForge's `stubResponse()` API

**For Other Languages**:
1. Install MockForge SDK for your language
2. Replace WireMock client calls with MockForge SDK calls
3. MockForge provides native embedding (no separate server required for most use cases)

---

## Conclusion

MockForge provides a comprehensive multi-language ecosystem that matches and extends WireMock's capabilities, with native SDK support across 6 languages and advanced features like multi-protocol mocking, AI-powered generation, and workspace synchronization. Whether you're writing unit tests, integration tests, or setting up development environments, MockForge offers native support for your language of choice.

For detailed SDK documentation, see [SDK README](../sdk/README.md).

For specific use case examples, see the [examples](../examples/) directory.
