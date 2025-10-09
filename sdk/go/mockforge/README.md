# MockForge Go SDK

Build MockForge plugins in Go using TinyGo and WebAssembly!

## 🚀 Quick Start

### Prerequisites

- Go 1.21 or later
- TinyGo 0.30 or later

```bash
# Install TinyGo (if not already installed)
# macOS
brew install tinygo

# Linux
wget https://github.com/tinygo-org/tinygo/releases/download/v0.30.0/tinygo_0.30.0_amd64.deb
sudo dpkg -i tinygo_0.30.0_amd64.deb
```

### Create Your First Plugin

```bash
# Create a new directory for your plugin
mkdir my-auth-plugin
cd my-auth-plugin

# Initialize Go module
go mod init github.com/yourname/my-auth-plugin

# Get the MockForge SDK
go get github.com/mockforge/mockforge/sdk/go/mockforge
```

### Write Your Plugin

Create `main.go`:

```go
package main

import "github.com/mockforge/mockforge/sdk/go/mockforge"

type MyAuthPlugin struct{}

func (p *MyAuthPlugin) Authenticate(
    ctx *mockforge.PluginContext,
    creds *mockforge.AuthCredentials,
) (*mockforge.AuthResult, error) {
    // Validate credentials
    if creds.Token == "secret-token-123" {
        return &mockforge.AuthResult{
            Authenticated: true,
            UserID:        "user123",
            Claims: map[string]interface{}{
                "role":        "admin",
                "permissions": []string{"read", "write"},
            },
        }, nil
    }

    return &mockforge.AuthResult{
        Authenticated: false,
        UserID:        "",
        Claims:        map[string]interface{}{},
    }, nil
}

func (p *MyAuthPlugin) GetCapabilities() *mockforge.PluginCapabilities {
    return &mockforge.PluginCapabilities{
        Network: mockforge.NetworkCapabilities{
            AllowHTTPOutbound: false,
            AllowedHosts:      []string{},
        },
        Filesystem: mockforge.FilesystemCapabilities{
            AllowRead:    false,
            AllowWrite:   false,
            AllowedPaths: []string{},
        },
        Resources: mockforge.ResourceLimits{
            MaxMemoryBytes: 10 * 1024 * 1024, // 10MB
            MaxCPUTimeMs:   1000,              // 1 second
        },
    }
}

func main() {
    plugin := &MyAuthPlugin{}
    mockforge.ExportAuthPlugin(plugin)
}
```

### Build Your Plugin

```bash
# Build to WebAssembly
tinygo build -o plugin.wasm -target=wasi main.go

# The output is plugin.wasm
```

### Create Plugin Manifest

Create `plugin.yaml`:

```yaml
plugin:
  id: "my-auth-plugin"
  version: "0.1.0"
  name: "My Go Auth Plugin"
  description: "Authentication plugin written in Go"
  types: ["auth"]
  author:
    name: "Your Name"
    email: "you@example.com"
  homepage: "https://github.com/yourname/my-auth-plugin"
  repository: "https://github.com/yourname/my-auth-plugin"
  license: "MIT"

capabilities:
  network:
    allow_http_outbound: false
    allowed_hosts: []
  filesystem:
    allow_read: false
    allow_write: false
    allowed_paths: []
  resources:
    max_memory_bytes: 10485760  # 10MB
    max_cpu_time_ms: 1000       # 1 second

dependencies: []
```

### Test Your Plugin

```bash
# Install in MockForge
mockforge plugin install .

# Verify it loaded
mockforge plugin list

# Test it
mockforge plugin test my-auth-plugin
```

## 📖 Plugin Types

### Authentication Plugin

```go
type AuthPlugin interface {
    Authenticate(ctx *PluginContext, creds *AuthCredentials) (*AuthResult, error)
    GetCapabilities() *PluginCapabilities
}
```

### Template Plugin

```go
type TemplatePlugin interface {
    ExecuteFunction(name string, args []interface{}, ctx *ResolutionContext) (interface{}, error)
    GetFunctions() []TemplateFunction
    GetCapabilities() *PluginCapabilities
}
```

### Response Plugin

```go
type ResponsePlugin interface {
    GenerateResponse(ctx *PluginContext, req *ResponseRequest) (*ResponseData, error)
    GetCapabilities() *PluginCapabilities
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

## 🔧 Building and Testing

### Development Workflow

```bash
# Format code
go fmt ./...

# Run tests
go test ./...

# Build for testing
tinygo build -o plugin.wasm -target=wasi main.go

# Build optimized for production
tinygo build -o plugin.wasm -target=wasi -opt=2 main.go
```

### Testing

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
        Headers: map[string]string{
            "Content-Type": "application/json",
        },
    }

    creds := &mockforge.AuthCredentials{
        Type:  "bearer",
        Token: "secret-token-123",
    }

    result, err := plugin.Authenticate(ctx, creds)
    if err != nil {
        t.Fatalf("Expected no error, got: %v", err)
    }

    if !result.Authenticated {
        t.Error("Expected authentication to succeed")
    }

    if result.UserID != "user123" {
        t.Errorf("Expected UserID 'user123', got '%s'", result.UserID)
    }
}
```

## 🎯 Examples

See the [examples directory](./examples) for complete working examples:

- [auth-jwt](./examples/auth-jwt) - JWT authentication
- [template-hash](./examples/template-hash) - Hashing functions
- [response-json](./examples/response-json) - JSON response generation
- [datasource-sqlite](./examples/datasource-sqlite) - SQLite data source

## 📚 API Reference

### Core Types

#### PluginContext
```go
type PluginContext struct {
    Method  string            // HTTP method
    URI     string            // Request URI
    Headers map[string]string // Request headers
    Body    []byte            // Request body
}
```

#### AuthCredentials
```go
type AuthCredentials struct {
    Type  string            // Credential type (bearer, basic, etc.)
    Token string            // Token value
    Data  map[string]string // Additional data
}
```

#### AuthResult
```go
type AuthResult struct {
    Authenticated bool                   // Whether authentication succeeded
    UserID        string                 // User identifier
    Claims        map[string]interface{} // Additional claims
}
```

#### PluginCapabilities
```go
type PluginCapabilities struct {
    Network    NetworkCapabilities
    Filesystem FilesystemCapabilities
    Resources  ResourceLimits
}
```

## 🐛 Debugging

### Enable Logging

TinyGo doesn't support full Go stdlib logging, but you can use print:

```go
import "fmt"

func (p *MyAuthPlugin) Authenticate(...) (*AuthResult, error) {
    fmt.Println("Authenticating user")
    // Your code
}
```

### Common Issues

#### 1. Import Errors

If you get import errors, ensure you're using TinyGo-compatible packages:

```bash
# Check TinyGo compatibility
tinygo build -target=wasi main.go
```

#### 2. Memory Issues

If you hit memory limits, increase in `plugin.yaml`:

```yaml
resources:
  max_memory_bytes: 20971520  # 20MB
```

#### 3. Build Errors

Clean and rebuild:

```bash
rm plugin.wasm
tinygo build -o plugin.wasm -target=wasi main.go
```

## 🚀 Performance Tips

1. **Minimize allocations**: Reuse objects when possible
2. **Use small data structures**: WASM has limited memory
3. **Avoid goroutines**: Limited scheduler support in WASM
4. **Profile your code**: Use TinyGo's profiling tools

## 📦 Packaging and Distribution

### Create Release Package

```bash
# Create tarball
tar -czf my-plugin-0.1.0.tar.gz \
    plugin.yaml \
    plugin.wasm \
    README.md

# Publish to GitHub releases
gh release create v0.1.0 my-plugin-0.1.0.tar.gz
```

### Installation by Users

```bash
# From local file
mockforge plugin install ./my-plugin-0.1.0.tar.gz

# From GitHub release
mockforge plugin install https://github.com/user/plugin/releases/download/v0.1.0/my-plugin-0.1.0.tar.gz

# From plugin registry (coming soon)
mockforge plugin install my-auth-plugin
```

## 🤝 Contributing

We welcome contributions! Please see:
- [CONTRIBUTING.md](../../../CONTRIBUTING.md)
- [Plugin Development Guide](../../../docs/plugins/development-guide.md)

## 📄 License

MIT OR Apache-2.0

## 🔗 Resources

- [MockForge Documentation](https://docs.mockforge.dev)
- [TinyGo Documentation](https://tinygo.org/docs/)
- [WebAssembly Guide](https://webassembly.org/)
- [Plugin Examples](./examples)

## 💬 Support

- GitHub Issues: [Report bugs](https://github.com/mockforge/mockforge/issues)
- GitHub Discussions: [Ask questions](https://github.com/mockforge/mockforge/discussions)
- Discord: [Join community](https://discord.gg/mockforge)
