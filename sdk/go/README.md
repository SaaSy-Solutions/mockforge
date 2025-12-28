# MockForge Go SDK

Embed MockForge mock servers directly in your Go tests.

## Prerequisites

The Go SDK requires the MockForge CLI to be installed and available in your PATH:

```bash
# Via Cargo
cargo install mockforge-cli

# Or download pre-built binaries from:
# https://github.com/SaaSy-Solutions/mockforge/releases
```

## Installation

```bash
go get github.com/SaaSy-Solutions/mockforge/sdk/go
```

## Usage

### Basic Example

```go
package myapi_test

import (
    "encoding/json"
    "net/http"
    "testing"

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
    assert.Equal(t, "John Doe", data["name"])
}
```

### With OpenAPI Specification

```go
server := mockforge.NewMockServer(mockforge.MockServerConfig{
    Port:        3000,
    OpenAPISpec: "./openapi.yaml",
})
```

### With Custom Configuration

```go
server := mockforge.NewMockServer(mockforge.MockServerConfig{
    Port:       3000,
    Host:       "127.0.0.1",
    ConfigFile: "./mockforge.yaml",
})
```

## API Reference

### `NewMockServer(config MockServerConfig)`

Creates a new mock server.

**MockServerConfig:**
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Port` | `int` | `0` (random) | Port to listen on |
| `Host` | `string` | `127.0.0.1` | Host to bind to |
| `ConfigFile` | `string` | - | Path to MockForge config file |
| `OpenAPISpec` | `string` | - | Path to OpenAPI specification |

### Methods

| Method | Description |
|--------|-------------|
| `Start() error` | Start the server |
| `StubResponse(method, path string, body interface{}) error` | Add a response stub |
| `StubResponseWithOptions(method, path string, body interface{}, opts StubOptions) error` | Add a stub with options |
| `ClearStubs() error` | Remove all stubs |
| `Stop() error` | Stop the server |
| `URL() string` | Get the server URL |
| `Port() int` | Get the server port |
| `IsRunning() bool` | Check if server is running |

### StubOptions

```go
err := server.StubResponseWithOptions("GET", "/api/users", users, mockforge.StubOptions{
    Status:    200,
    Headers:   map[string]string{"X-Custom-Header": "value"},
    LatencyMs: 100,
})
```

## Testing Patterns

### Table-Driven Tests

```go
func TestUserEndpoints(t *testing.T) {
    server := mockforge.NewMockServer(mockforge.MockServerConfig{Port: 3000})
    require.NoError(t, server.Start())
    defer server.Stop()

    tests := []struct {
        name     string
        method   string
        path     string
        body     interface{}
        expected int
    }{
        {"get user", "GET", "/api/users/1", map[string]interface{}{"id": 1}, 200},
        {"create user", "POST", "/api/users", map[string]interface{}{"id": 2}, 201},
    }

    for _, tt := range tests {
        t.Run(tt.name, func(t *testing.T) {
            server.ClearStubs()
            server.StubResponseWithOptions(tt.method, tt.path, tt.body, mockforge.StubOptions{
                Status: tt.expected,
            })
            // ... make request and assert ...
        })
    }
}
```

### Parallel Tests

```go
func TestParallel(t *testing.T) {
    // Each parallel test gets its own server on a random port
    t.Parallel()

    server := mockforge.NewMockServer(mockforge.MockServerConfig{
        Port: 0, // Random port
    })
    require.NoError(t, server.Start())
    defer server.Stop()

    // Use server.URL() for requests
    resp, _ := http.Get(server.URL() + "/api/test")
    // ...
}
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `MOCKFORGE_CLI_PATH` | Custom path to MockForge CLI binary |
| `MOCKFORGE_LOG_LEVEL` | Log level (debug, info, warn, error) |

## Error Handling

```go
server := mockforge.NewMockServer(mockforge.MockServerConfig{Port: 3000})

if err := server.Start(); err != nil {
    if errors.Is(err, mockforge.ErrCLINotFound) {
        t.Skip("MockForge CLI not installed")
    }
    t.Fatalf("Failed to start server: %v", err)
}
```

## License

Apache-2.0 OR MIT
