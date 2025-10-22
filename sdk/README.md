# MockForge Developer SDKs

Embed MockForge mock servers directly in your unit and integration tests.

## Prerequisites

**Important:** The Node.js, Python, and Go SDKs require the MockForge CLI to be installed and available in your PATH.

### Install MockForge CLI

```bash
# Via Cargo
cargo install mockforge-cli

# Or download pre-built binaries from:
# https://github.com/SaaSy-Solutions/mockforge/releases
```

Verify installation:
```bash
mockforge --version
```

**Note:** The Rust SDK embeds MockForge directly and does not require the CLI.

## Available SDKs

- **[Rust SDK](#rust-sdk)** - Native Rust library
- **[Node.js/TypeScript SDK](#nodejs-sdk)** - JavaScript/TypeScript support
- **[Python SDK](#python-sdk)** - Python support
- **[Go SDK](#go-sdk)** - Go support

## Features

- ✅ **`startMock()`** - Start embedded mock servers
- ✅ **`stopMock()`** - Stop and cleanup servers
- ✅ **`stubResponse()`** - Define mock responses programmatically
- ✅ **Offline Mode** - Works without network dependencies (local mode)
- ✅ **Multi-Language** - Tested in Rust, Node.js, Python, and Go

---

## Rust SDK

### Installation

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
mockforge-sdk = "0.1"
tokio-test = "0.4"
reqwest = "0.12"
serde_json = "1.0"
```

### Usage

```rust
use mockforge_sdk::MockServer;
use serde_json::json;

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
        .stub_response("GET", "/api/users/{id}", json!({
            "id": "{{uuid}}",
            "name": "{{faker.name}}",
            "email": "{{faker.email}}"
        }))
        .await
        .expect("Failed to stub response");

    // Make requests to the mock
    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:3000/api/users/123")
        .send()
        .await
        .expect("Failed to make request");

    assert_eq!(response.status(), 200);

    // Stop the server
    server.stop().await.expect("Failed to stop server");
}
```

### API Reference

#### `MockServer::new()`
Creates a new mock server builder.

**Methods:**
- `.port(port: u16)` - Set the HTTP port
- `.host(host: &str)` - Set the host address
- `.config_file(path: &Path)` - Load configuration from YAML file
- `.openapi_spec(path: &Path)` - Load routes from OpenAPI spec
- `.latency(profile: LatencyProfile)` - Enable latency simulation
- `.failures(config: FailureConfig)` - Enable failure injection
- `.start()` - Start the server (returns `Result<MockServer>`)

#### `MockServer` Methods
- `stub_response(method, path, body)` - Add a response stub
- `add_stub(stub: ResponseStub)` - Add a pre-built stub
- `clear_stubs()` - Remove all stubs
- `stop()` - Stop the server
- `url()` - Get the server URL
- `port()` - Get the server port
- `is_running()` - Check if server is running

---

## Node.js SDK

### Installation

```bash
npm install @mockforge/sdk
```

### Usage

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

### API Reference

#### `MockServer.start(config)`
Starts a mock server.

**Config Options:**
- `port?: number` - Port to listen on (default: random)
- `host?: string` - Host to bind to (default: 127.0.0.1)
- `configFile?: string` - Path to MockForge config file
- `openApiSpec?: string` - Path to OpenAPI specification

#### Methods
- `stubResponse(method, path, body, options?)` - Add a response stub
- `clearStubs()` - Remove all stubs
- `stop()` - Stop the server
- `url()` - Get the server URL
- `getPort()` - Get the server port
- `isRunning()` - Check if server is running

---

## Python SDK

### Installation

```bash
pip install mockforge-sdk
```

### Usage

```python
from mockforge_sdk import MockServer
import requests

def test_user_api():
    # Context manager automatically starts/stops server
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

### API Reference

#### `MockServer(port=0, host='127.0.0.1', config_file=None, openapi_spec=None)`
Creates a mock server instance.

#### Methods
- `start()` - Start the server
- `stub_response(method, path, body, status=200, headers=None, latency_ms=None)` - Add a stub
- `clear_stubs()` - Remove all stubs
- `stop()` - Stop the server
- `url()` - Get the server URL
- `get_port()` - Get the server port
- `is_running()` - Check if server is running

**Context Manager:**
```python
with MockServer(port=3000) as server:
    # Server automatically starts
    server.stub_response('GET', '/api/test', {'status': 'ok'})
    # ... make requests ...
# Server automatically stops
```

---

## Go SDK

### Installation

```bash
go get github.com/SaaSy-Solutions/mockforge/sdk/go
```

### Usage

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

    // Stub a response
    err = server.StubResponse("GET", "/api/users/123", map[string]interface{}{
        "id":    123,
        "name":  "John Doe",
        "email": "john@example.com",
    })
    assert.NoError(t, err)

    // Make request
    resp, err := http.Get("http://localhost:3000/api/users/123")
    assert.NoError(t, err)
    assert.Equal(t, 200, resp.StatusCode)

    var data map[string]interface{}
    json.NewDecoder(resp.Body).Decode(&data)
    assert.Equal(t, float64(123), data["id"])
}
```

### API Reference

#### `NewMockServer(config MockServerConfig)`
Creates a new mock server.

**Config:**
- `Port int` - Port to listen on
- `Host string` - Host to bind to (default: 127.0.0.1)
- `ConfigFile string` - Path to MockForge config file
- `OpenAPISpec string` - Path to OpenAPI specification

#### Methods
- `Start()` - Start the server
- `StubResponse(method, path string, body interface{})` - Add a stub
- `StubResponseWithOptions(method, path string, body interface{}, status int, headers map[string]string, latencyMs *int)` - Add stub with options
- `ClearStubs()` - Remove all stubs
- `Stop()` - Stop the server
- `URL()` - Get the server URL
- `Port()` - Get the server port
- `IsRunning()` - Check if server is running

---

## Advanced Features

### Template Support

All SDKs support MockForge's template syntax for dynamic responses:

```rust
// Rust
server.stub_response("GET", "/api/users/{id}", json!({
    "id": "{{uuid}}",
    "name": "{{faker.name}}",
    "email": "{{faker.email}}",
    "created_at": "{{now}}",
    "random_status": "{{random(['active', 'pending', 'inactive'])}}"
})).await?;
```

```typescript
// TypeScript
await server.stubResponse('GET', '/api/users/{id}', {
  id: '{{uuid}}',
  name: '{{faker.name}}',
  email: '{{faker.email}}',
  created_at: '{{now}}'
});
```

```python
# Python
server.stub_response('GET', '/api/users/{id}', {
    'id': '{{uuid}}',
    'name': '{{faker.name}}',
    'email': '{{faker.email}}',
    'created_at': '{{now}}'
})
```

```go
// Go
server.StubResponse("GET", "/api/users/{id}", map[string]interface{}{
    "id":         "{{uuid}}",
    "name":       "{{faker.name}}",
    "email":      "{{faker.email}}",
    "created_at": "{{now}}",
})
```

### Response Options

Configure response behavior:

```rust
// Rust - custom status and headers
server.add_stub(
    ResponseStub::new("POST", "/api/users", json!({"status": "created"}))
        .status(201)
        .header("X-Request-ID", "{{uuid}}")
        .latency(500) // 500ms delay
).await?;
```

```typescript
// TypeScript
await server.stubResponse('POST', '/api/users',
  { status: 'created' },
  {
    status: 201,
    headers: { 'X-Request-ID': '{{uuid}}' },
    latencyMs: 500
  }
);
```

```python
# Python
server.stub_response(
    'POST', '/api/users',
    {'status': 'created'},
    status=201,
    headers={'X-Request-ID': '{{uuid}}'},
    latency_ms=500
)
```

```go
// Go
latency := 500
server.StubResponseWithOptions(
    "POST", "/api/users",
    map[string]interface{}{"status": "created"},
    201,
    map[string]string{"X-Request-ID": "{{uuid}}"},
    &latency,
)
```

---

## Examples

See the `examples/` directory for complete working examples:

- [Rust example](../examples/sdk-rust/)
- [Node.js example](../examples/sdk-nodejs/)
- [Python example](../examples/sdk-python/)
- [Go example](../examples/sdk-go/)

---

## Requirements

All SDKs require MockForge to be installed and available in your PATH:

```bash
# Install MockForge CLI
cargo install mockforge-cli
```

Or use the pre-built binaries from the [releases page](https://github.com/SaaSy-Solutions/mockforge/releases).

---

## License

MIT License - see [LICENSE](../LICENSE-MIT) for details.
