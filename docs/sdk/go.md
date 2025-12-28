# MockForge Go SDK

Build high-performance MockForge plugins in Go using TinyGo and WebAssembly.

## Installation

```bash
# Prerequisites: Go 1.21+ and TinyGo 0.30+

# Install TinyGo
# macOS
brew install tinygo

# Linux
wget https://github.com/tinygo-org/tinygo/releases/download/v0.30.0/tinygo_0.30.0_amd64.deb
sudo dpkg -i tinygo_0.30.0_amd64.deb

# Get the SDK
go get github.com/mockforge/mockforge/sdk/go/mockforge
```

## Quick Start

Create a new plugin:

```go
// main.go
package main

import "github.com/mockforge/mockforge/sdk/go/mockforge"

type MyAuthPlugin struct{}

func (p *MyAuthPlugin) Authenticate(
    ctx *mockforge.PluginContext,
    creds *mockforge.AuthCredentials,
) (*mockforge.AuthResult, error) {
    if creds.Token == "valid-token" {
        return &mockforge.AuthResult{
            Authenticated: true,
            UserID:        "user123",
            Claims: map[string]interface{}{
                "role": "admin",
            },
        }, nil
    }
    return &mockforge.AuthResult{Authenticated: false}, nil
}

func (p *MyAuthPlugin) GetCapabilities() *mockforge.PluginCapabilities {
    return &mockforge.PluginCapabilities{
        Resources: mockforge.ResourceLimits{
            MaxMemoryBytes: 10 * 1024 * 1024,
            MaxCPUTimeMs:   1000,
        },
    }
}

func main() {
    mockforge.ExportAuthPlugin(&MyAuthPlugin{})
}
```

Build to WebAssembly:

```bash
tinygo build -o plugin.wasm -target=wasi main.go
```

## Plugin Types

### Authentication Plugin

```go
type AuthPlugin interface {
    Authenticate(ctx *PluginContext, creds *AuthCredentials) (*AuthResult, error)
    GetCapabilities() *PluginCapabilities
}
```

Example with HMAC validation:

```go
package main

import (
    "crypto/hmac"
    "crypto/sha256"
    "encoding/hex"
    "github.com/mockforge/mockforge/sdk/go/mockforge"
)

type HMACAuthPlugin struct {
    secretKey []byte
}

func (p *HMACAuthPlugin) Authenticate(
    ctx *mockforge.PluginContext,
    creds *mockforge.AuthCredentials,
) (*mockforge.AuthResult, error) {
    // Get signature from credentials
    providedSig := creds.Token

    // Get timestamp and payload from data
    timestamp := creds.Data["timestamp"]
    payload := creds.Data["payload"]

    // Compute expected signature
    message := timestamp + "." + payload
    mac := hmac.New(sha256.New, p.secretKey)
    mac.Write([]byte(message))
    expectedSig := hex.EncodeToString(mac.Sum(nil))

    // Constant-time comparison
    if hmac.Equal([]byte(providedSig), []byte(expectedSig)) {
        return &mockforge.AuthResult{
            Authenticated: true,
            UserID:        creds.Data["user_id"],
            Claims: map[string]interface{}{
                "timestamp": timestamp,
            },
        }, nil
    }

    return &mockforge.AuthResult{
        Authenticated: false,
        UserID:        "",
    }, nil
}

func (p *HMACAuthPlugin) GetCapabilities() *mockforge.PluginCapabilities {
    return &mockforge.PluginCapabilities{}
}

func main() {
    plugin := &HMACAuthPlugin{
        secretKey: []byte("my-secret-key"),
    }
    mockforge.ExportAuthPlugin(plugin)
}
```

### Template Function Plugin

```go
type TemplatePlugin interface {
    ExecuteFunction(name string, args []interface{}, ctx *ResolutionContext) (interface{}, error)
    GetFunctions() []TemplateFunction
    GetCapabilities() *PluginCapabilities
}
```

Example with string functions:

```go
package main

import (
    "strings"
    "github.com/mockforge/mockforge/sdk/go/mockforge"
)

type StringPlugin struct{}

func (p *StringPlugin) ExecuteFunction(
    name string,
    args []interface{},
    ctx *mockforge.ResolutionContext,
) (interface{}, error) {
    switch name {
    case "uppercase":
        return strings.ToUpper(args[0].(string)), nil

    case "lowercase":
        return strings.ToLower(args[0].(string)), nil

    case "reverse":
        s := args[0].(string)
        runes := []rune(s)
        for i, j := 0, len(runes)-1; i < j; i, j = i+1, j-1 {
            runes[i], runes[j] = runes[j], runes[i]
        }
        return string(runes), nil

    case "truncate":
        s := args[0].(string)
        maxLen := int(args[1].(float64))
        if len(s) > maxLen {
            return s[:maxLen] + "...", nil
        }
        return s, nil

    case "slugify":
        s := strings.ToLower(args[0].(string))
        s = strings.ReplaceAll(s, " ", "-")
        return s, nil
    }

    return nil, fmt.Errorf("unknown function: %s", name)
}

func (p *StringPlugin) GetFunctions() []mockforge.TemplateFunction {
    return []mockforge.TemplateFunction{
        {
            Name:        "uppercase",
            Description: "Convert string to uppercase",
            Parameters: []mockforge.FunctionParameter{
                {Name: "input", Type: "string", Required: true},
            },
            ReturnType: "string",
        },
        {
            Name:        "lowercase",
            Description: "Convert string to lowercase",
            Parameters: []mockforge.FunctionParameter{
                {Name: "input", Type: "string", Required: true},
            },
            ReturnType: "string",
        },
        {
            Name:        "reverse",
            Description: "Reverse a string",
            Parameters: []mockforge.FunctionParameter{
                {Name: "input", Type: "string", Required: true},
            },
            ReturnType: "string",
        },
        {
            Name:        "truncate",
            Description: "Truncate string to max length",
            Parameters: []mockforge.FunctionParameter{
                {Name: "input", Type: "string", Required: true},
                {Name: "maxLen", Type: "integer", Required: true},
            },
            ReturnType: "string",
        },
        {
            Name:        "slugify",
            Description: "Convert string to URL-safe slug",
            Parameters: []mockforge.FunctionParameter{
                {Name: "input", Type: "string", Required: true},
            },
            ReturnType: "string",
        },
    }
}

func (p *StringPlugin) GetCapabilities() *mockforge.PluginCapabilities {
    return &mockforge.PluginCapabilities{}
}

func main() {
    mockforge.ExportTemplatePlugin(&StringPlugin{})
}
```

### Response Generator Plugin

```go
type ResponsePlugin interface {
    GenerateResponse(ctx *PluginContext, req *ResponseRequest) (*ResponseData, error)
    GetCapabilities() *PluginCapabilities
}
```

Example generating JSON responses:

```go
package main

import (
    "encoding/json"
    "fmt"
    "math/rand"
    "time"
    "github.com/mockforge/mockforge/sdk/go/mockforge"
)

type RandomDataPlugin struct{}

func (p *RandomDataPlugin) GenerateResponse(
    ctx *mockforge.PluginContext,
    req *mockforge.ResponseRequest,
) (*mockforge.ResponseData, error) {
    rand.Seed(time.Now().UnixNano())

    // Generate random user data
    user := map[string]interface{}{
        "id":    fmt.Sprintf("user-%d", rand.Intn(10000)),
        "name":  randomName(),
        "email": randomEmail(),
        "age":   rand.Intn(50) + 18,
        "active": rand.Float32() > 0.3,
    }

    body, err := json.Marshal(user)
    if err != nil {
        return nil, err
    }

    return &mockforge.ResponseData{
        StatusCode:  200,
        Headers:     map[string]string{"Content-Type": "application/json"},
        Body:        body,
        ContentType: "application/json",
    }, nil
}

func randomName() string {
    firstNames := []string{"Alice", "Bob", "Charlie", "Diana", "Eve"}
    lastNames := []string{"Smith", "Johnson", "Williams", "Brown", "Jones"}
    return firstNames[rand.Intn(len(firstNames))] + " " + lastNames[rand.Intn(len(lastNames))]
}

func randomEmail() string {
    domains := []string{"example.com", "test.org", "mock.io"}
    return fmt.Sprintf("user%d@%s", rand.Intn(1000), domains[rand.Intn(len(domains))])
}

func (p *RandomDataPlugin) GetCapabilities() *mockforge.PluginCapabilities {
    return &mockforge.PluginCapabilities{}
}

func main() {
    mockforge.ExportResponsePlugin(&RandomDataPlugin{})
}
```

### Data Source Plugin

```go
type DataSourcePlugin interface {
    Query(query *DataQuery, ctx *PluginContext) (*DataResult, error)
    GetSchema() (map[string]interface{}, error)
    GetCapabilities() *PluginCapabilities
}
```

Example in-memory data source:

```go
package main

import (
    "strings"
    "github.com/mockforge/mockforge/sdk/go/mockforge"
)

type InMemoryDB struct {
    users []map[string]interface{}
}

func (p *InMemoryDB) Query(
    query *mockforge.DataQuery,
    ctx *mockforge.PluginContext,
) (*mockforge.DataResult, error) {
    // Simple query parser (SELECT * FROM users WHERE name LIKE '%alice%')
    q := strings.ToLower(query.Query)

    var results []map[string]interface{}

    if strings.Contains(q, "from users") {
        results = p.users

        // Apply WHERE filter if present
        if strings.Contains(q, "where") {
            if nameFilter, ok := query.Parameters["name"]; ok {
                var filtered []map[string]interface{}
                for _, user := range results {
                    if strings.Contains(
                        strings.ToLower(user["name"].(string)),
                        strings.ToLower(nameFilter.(string)),
                    ) {
                        filtered = append(filtered, user)
                    }
                }
                results = filtered
            }
        }
    }

    columns := []mockforge.ColumnInfo{
        {Name: "id", DataType: "string"},
        {Name: "name", DataType: "string"},
        {Name: "email", DataType: "string"},
    }

    return &mockforge.DataResult{
        Columns: columns,
        Rows:    results,
    }, nil
}

func (p *InMemoryDB) GetSchema() (map[string]interface{}, error) {
    return map[string]interface{}{
        "tables": []map[string]interface{}{
            {
                "name": "users",
                "columns": []map[string]string{
                    {"name": "id", "type": "string"},
                    {"name": "name", "type": "string"},
                    {"name": "email", "type": "string"},
                },
            },
        },
    }, nil
}

func (p *InMemoryDB) GetCapabilities() *mockforge.PluginCapabilities {
    return &mockforge.PluginCapabilities{}
}

func main() {
    db := &InMemoryDB{
        users: []map[string]interface{}{
            {"id": "1", "name": "Alice", "email": "alice@example.com"},
            {"id": "2", "name": "Bob", "email": "bob@example.com"},
            {"id": "3", "name": "Charlie", "email": "charlie@example.com"},
        },
    }
    mockforge.ExportDataSourcePlugin(db)
}
```

## Core Types

### PluginContext

```go
type PluginContext struct {
    Method  string            // HTTP method (GET, POST, etc.)
    URI     string            // Request URI
    Headers map[string]string // Request headers
    Body    []byte            // Request body
}
```

### AuthCredentials

```go
type AuthCredentials struct {
    Type  string            // Credential type (bearer, basic, api-key)
    Token string            // Token value
    Data  map[string]string // Additional credential data
}
```

### AuthResult

```go
type AuthResult struct {
    Authenticated bool                   // Authentication success
    UserID        string                 // User identifier
    Claims        map[string]interface{} // JWT-like claims
}
```

### PluginCapabilities

```go
type PluginCapabilities struct {
    Network    NetworkCapabilities
    Filesystem FilesystemCapabilities
    Resources  ResourceLimits
}

type NetworkCapabilities struct {
    AllowHTTPOutbound bool
    AllowedHosts      []string
}

type FilesystemCapabilities struct {
    AllowRead    bool
    AllowWrite   bool
    AllowedPaths []string
}

type ResourceLimits struct {
    MaxMemoryBytes int64
    MaxCPUTimeMs   int64
}
```

## Building and Packaging

### Development Build

```bash
tinygo build -o plugin.wasm -target=wasi main.go
```

### Optimized Build

```bash
tinygo build -o plugin.wasm -target=wasi -opt=2 -no-debug main.go
```

### Plugin Manifest

Create `plugin.yaml`:

```yaml
plugin:
  id: "my-go-plugin"
  version: "1.0.0"
  name: "My Go Plugin"
  description: "A plugin written in Go"
  types: ["auth"]  # or ["template", "response", "datasource"]
  author:
    name: "Your Name"
    email: "you@example.com"
  license: "MIT"

capabilities:
  network:
    allow_http_outbound: false
  filesystem:
    allow_read: false
    allow_write: false
  resources:
    max_memory_bytes: 10485760
    max_cpu_time_ms: 1000

wasm:
  file: "plugin.wasm"
```

### Package for Distribution

```bash
# Create tarball
tar -czf my-plugin-1.0.0.tar.gz \
    plugin.yaml \
    plugin.wasm \
    README.md

# Install locally
mockforge plugin install ./my-plugin-1.0.0.tar.gz

# Or publish to registry
mockforge plugin publish ./my-plugin-1.0.0.tar.gz
```

## Testing

```go
package main

import (
    "testing"
    "github.com/mockforge/mockforge/sdk/go/mockforge"
)

func TestAuthenticate(t *testing.T) {
    plugin := &MyAuthPlugin{}

    ctx := &mockforge.PluginContext{
        Method: "POST",
        URI:    "/login",
    }

    // Test valid token
    creds := &mockforge.AuthCredentials{
        Type:  "bearer",
        Token: "valid-token",
    }

    result, err := plugin.Authenticate(ctx, creds)
    if err != nil {
        t.Fatalf("Unexpected error: %v", err)
    }

    if !result.Authenticated {
        t.Error("Expected authentication to succeed")
    }

    if result.UserID != "user123" {
        t.Errorf("Expected UserID 'user123', got '%s'", result.UserID)
    }

    // Test invalid token
    creds.Token = "invalid"
    result, err = plugin.Authenticate(ctx, creds)
    if err != nil {
        t.Fatalf("Unexpected error: %v", err)
    }

    if result.Authenticated {
        t.Error("Expected authentication to fail")
    }
}

func TestTemplateFunction(t *testing.T) {
    plugin := &StringPlugin{}

    tests := []struct {
        name     string
        args     []interface{}
        expected string
    }{
        {"uppercase", []interface{}{"hello"}, "HELLO"},
        {"lowercase", []interface{}{"WORLD"}, "world"},
        {"reverse", []interface{}{"abc"}, "cba"},
    }

    for _, tt := range tests {
        result, err := plugin.ExecuteFunction(tt.name, tt.args, nil)
        if err != nil {
            t.Errorf("%s: unexpected error: %v", tt.name, err)
        }
        if result != tt.expected {
            t.Errorf("%s: expected %q, got %q", tt.name, tt.expected, result)
        }
    }
}
```

Run tests:

```bash
go test ./...
```

## TinyGo Limitations

When using TinyGo for WASM, be aware of these limitations:

1. **No goroutines** - Limited scheduler support
2. **No reflection** - Use code generation instead
3. **Limited stdlib** - Some packages unavailable
4. **No cgo** - Pure Go only

### Workarounds

```go
// Instead of reflect, use type assertions
func handleValue(v interface{}) string {
    switch val := v.(type) {
    case string:
        return val
    case int:
        return strconv.Itoa(val)
    case float64:
        return strconv.FormatFloat(val, 'f', -1, 64)
    default:
        return fmt.Sprintf("%v", val)
    }
}

// Instead of json.Unmarshal with reflect
// Use manual parsing or code generation
```

## Debugging

TinyGo has limited debugging support, but you can use print statements:

```go
import "fmt"

func (p *MyPlugin) Authenticate(ctx *PluginContext, creds *AuthCredentials) (*AuthResult, error) {
    fmt.Printf("Authenticating token: %s\n", creds.Token[:8]+"...")
    // ...
}
```

View logs when running MockForge:

```bash
mockforge serve --log-level debug
```

## Performance Tips

1. **Minimize allocations** - Reuse buffers and slices
2. **Use small data structures** - WASM has limited memory
3. **Avoid string concatenation** - Use strings.Builder
4. **Pre-allocate slices** - `make([]T, 0, expectedSize)`

```go
// Good: Pre-allocate
result := make([]string, 0, len(input))
for _, item := range input {
    result = append(result, process(item))
}

// Bad: Reallocate on each append
var result []string
for _, item := range input {
    result = append(result, process(item))
}
```

## See Also

- [SDK Overview](./README.md)
- [Node.js SDK](./nodejs.md)
- [Python SDK](./python.md)
- [Plugin Development Guide](../plugins/development-guide.md)
