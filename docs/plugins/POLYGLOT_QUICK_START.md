# Polyglot Plugins Quick Start Guide

Welcome to polyglot plugin development for MockForge! This guide will get you up and running with plugins in Go, Python, or AssemblyScript in under 30 minutes.

## 🎯 Choose Your Path

### Path 1: Go Plugin (WASM) ⚡
**Best for**: Performance-critical plugins, Go developers
**Performance**: ~2x Rust speed
**Ecosystem**: Go standard library
**Deployment**: Single WASM file

[Jump to Go Quick Start →](#go-plugin-quick-start)

### Path 2: Python Remote Plugin 🐍
**Best for**: Rapid prototyping, data processing, ML/AI
**Performance**: ~20-50ms latency
**Ecosystem**: Full Python (pandas, numpy, requests, etc.)
**Deployment**: Separate service (Docker/K8s)

[Jump to Python Quick Start →](#python-remote-plugin-quick-start)

### Path 3: AssemblyScript Plugin 🌐
**Best for**: Web developers, simple template functions
**Performance**: Near-native
**Ecosystem**: TypeScript subset
**Deployment**: Single WASM file

[Jump to AssemblyScript Quick Start →](#assemblyscript-plugin-quick-start)

---

## Go Plugin Quick Start

### Step 1: Install Prerequisites

```bash
# Install Go (if not already installed)
# Visit: https://go.dev/dl/

# Install TinyGo
# macOS
brew install tinygo

# Linux
wget https://github.com/tinygo-org/tinygo/releases/download/v0.30.0/tinygo_0.30.0_amd64.deb
sudo dpkg -i tinygo_0.30.0_amd64.deb

# Windows
# Download installer from: https://github.com/tinygo-org/tinygo/releases

# Verify installation
tinygo version
```

### Step 2: Create Your Plugin

```bash
# Create plugin directory
mkdir my-auth-plugin
cd my-auth-plugin

# Initialize Go module
go mod init github.com/yourname/my-auth-plugin

# Get MockForge SDK
go get github.com/mockforge/mockforge/sdk/go/mockforge
```

### Step 3: Write Plugin Code

Create `main.go`:

```go
package main

import "github.com/mockforge/mockforge/sdk/go/mockforge"

type MyAuthPlugin struct{}

func (p *MyAuthPlugin) Authenticate(
    ctx *mockforge.PluginContext,
    creds *mockforge.AuthCredentials,
) (*mockforge.AuthResult, error) {
    // Simple token validation
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
            MaxMemoryBytes: 10 * 1024 * 1024, // 10MB
            MaxCPUTimeMs:   500,               // 500ms
        },
    }
}

func main() {
    mockforge.ExportAuthPlugin(&MyAuthPlugin{})
}
```

### Step 4: Create Plugin Manifest

Create `plugin.yaml`:

```yaml
plugin:
  id: "my-auth-plugin"
  version: "0.1.0"
  name: "My Go Auth Plugin"
  description: "Simple authentication plugin"
  types: ["auth"]
  author:
    name: "Your Name"
    email: "you@example.com"

capabilities:
  resources:
    max_memory_bytes: 10485760
    max_cpu_time_ms: 500
```

### Step 5: Build and Test

```bash
# Build to WASM
tinygo build -o plugin.wasm -target=wasi main.go

# Verify WASM file created
ls -lh plugin.wasm

# Install in MockForge
mockforge plugin install .

# Test the plugin
mockforge plugin test my-auth-plugin
```

### Step 6: Use in MockForge

Add to your `config.yaml`:

```yaml
plugins:
  - id: my-auth-plugin
    enabled: true

endpoints:
  - path: "/api/*"
    auth:
      plugin: my-auth-plugin
      required: true
```

**Done!** 🎉 Your Go plugin is running!

---

## Python Remote Plugin Quick Start

### Step 1: Install Prerequisites

```bash
# Python 3.9+ required
python --version

# Install MockForge Python SDK
pip install mockforge-plugin[fastapi]
```

### Step 2: Create Your Plugin

```bash
# Create plugin directory
mkdir my-python-plugin
cd my-python-plugin

# Create requirements file
cat > requirements.txt << EOF
mockforge-plugin[fastapi]>=0.1.0
requests>=2.31.0
EOF

# Install dependencies
pip install -r requirements.txt
```

### Step 3: Write Plugin Code

Create `plugin.py`:

```python
from mockforge_plugin import RemotePlugin, PluginContext, AuthCredentials, AuthResult

class MyAuthPlugin(RemotePlugin):
    """Simple authentication plugin"""

    def __init__(self):
        super().__init__(name="My Python Auth Plugin", version="0.1.0")

    async def authenticate(
        self,
        ctx: PluginContext,
        creds: AuthCredentials
    ) -> AuthResult:
        # Simple token validation
        if creds.token == "valid-token":
            return AuthResult(
                authenticated=True,
                user_id="user123",
                claims={"role": "admin"}
            )

        return AuthResult(authenticated=False, user_id="")

if __name__ == "__main__":
    plugin = MyAuthPlugin()
    plugin.run(host="0.0.0.0", port=8080)
```

### Step 4: Create Plugin Manifest

Create `plugin.yaml`:

```yaml
plugin:
  id: "my-python-plugin"
  version: "0.1.0"
  name: "My Python Auth Plugin"
  description: "Simple authentication plugin"
  types: ["auth"]
  runtime: "remote"
  author:
    name: "Your Name"
    email: "you@example.com"

remote:
  protocol: "http"
  endpoint: "http://localhost:8080"
  timeout_ms: 5000
```

### Step 5: Run and Test

```bash
# Start the plugin
python plugin.py &

# Test health endpoint
curl http://localhost:8080/health

# Test authentication
curl -X POST http://localhost:8080/plugin/authenticate \
  -H "Content-Type: application/json" \
  -d '{
    "context": {"method": "GET", "uri": "/api/users", "headers": {}},
    "credentials": {"type": "bearer", "token": "valid-token"}
  }'
```

### Step 6: Configure MockForge

Add to your `config.yaml`:

```yaml
plugins:
  - id: my-python-plugin
    enabled: true
    runtime: remote
    endpoint: http://localhost:8080

endpoints:
  - path: "/api/*"
    auth:
      plugin: my-python-plugin
      required: true
```

### Optional: Docker Deployment

Create `Dockerfile`:

```dockerfile
FROM python:3.11-slim

WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY plugin.py plugin.yaml ./

EXPOSE 8080
CMD ["python", "plugin.py"]
```

Build and run:

```bash
docker build -t my-python-plugin .
docker run -d -p 8080:8080 my-python-plugin
```

**Done!** 🎉 Your Python plugin is running!

---

## AssemblyScript Plugin Quick Start

### Step 1: Install Prerequisites

```bash
# Node.js 18+ and npm required
node --version
npm --version

# Install AssemblyScript compiler
npm install -g assemblyscript
```

### Step 2: Create Your Plugin

```bash
# Create plugin directory
mkdir my-as-plugin
cd my-as-plugin

# Initialize npm project
npm init -y

# Install AssemblyScript and MockForge SDK
npm install assemblyscript @mockforge/plugin-sdk-as
```

### Step 3: Write Plugin Code

Create `assembly/index.ts`:

```typescript
import {
  AuthPlugin,
  PluginContext,
  AuthCredentials,
  AuthResult,
  PluginCapabilities
} from "@mockforge/plugin-sdk-as";

@plugin("auth")
export class MyAuthPlugin implements AuthPlugin {
  authenticate(
    ctx: PluginContext,
    creds: AuthCredentials
  ): AuthResult {
    // Simple token validation
    if (creds.token == "valid-token") {
      const claims = new Map<string, string>();
      claims.set("role", "admin");

      return new AuthResult(true, "user123", claims);
    }

    return new AuthResult(false, "", new Map());
  }

  getCapabilities(): PluginCapabilities {
    return new PluginCapabilities({
      maxMemoryBytes: 10 * 1024 * 1024, // 10MB
      maxCPUTimeMs: 500
    });
  }
}
```

### Step 4: Configure AssemblyScript

Create `asconfig.json`:

```json
{
  "targets": {
    "release": {
      "outFile": "build/plugin.wasm",
      "sourceMap": false,
      "optimize": true,
      "runtime": "stub"
    }
  }
}
```

### Step 5: Create Plugin Manifest

Create `plugin.yaml`:

```yaml
plugin:
  id: "my-as-plugin"
  version: "0.1.0"
  name: "My AssemblyScript Plugin"
  description: "Simple authentication plugin"
  types: ["auth"]
  author:
    name: "Your Name"
    email: "you@example.com"

capabilities:
  resources:
    max_memory_bytes: 10485760
    max_cpu_time_ms: 500
```

### Step 6: Build and Test

```bash
# Build to WASM
npm run asbuild:release

# Verify WASM file
ls -lh build/plugin.wasm

# Install in MockForge
mockforge plugin install .

# Test
mockforge plugin test my-as-plugin
```

**Done!** 🎉 Your AssemblyScript plugin is running!

---

## Comparison Table

| Feature | Go (WASM) | Python (Remote) | AssemblyScript (WASM) |
|---------|-----------|-----------------|----------------------|
| Setup Time | 10 min | 5 min | 15 min |
| Performance | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| Latency | ~2-5ms | ~20-50ms | ~1-2ms |
| Library Access | Go stdlib | Full Python | Limited |
| Debugging | Moderate | Easy | Moderate |
| Deployment | Single file | Service | Single file |
| Best For | Backend logic | Data/ML/API calls | Simple functions |

## Next Steps

### 1. Explore Examples

- [Go JWT Auth Plugin](../../examples/plugins/auth-go-jwt/)
- [Python OAuth Plugin](../../examples/plugins/auth-python-oauth/)
- [More Examples →](../../examples/plugins/)

### 2. Read Documentation

- [Full Polyglot Design](./POLYGLOT_PLUGIN_SUPPORT.md)
- [Implementation Roadmap](./POLYGLOT_IMPLEMENTATION_ROADMAP.md)
- [Plugin Development Guide](./development-guide.md)

### 3. Join the Community

- 💬 [GitHub Discussions](https://github.com/mockforge/mockforge/discussions)
- 🐛 [Report Issues](https://github.com/mockforge/mockforge/issues)
- 💬 [Discord Community](https://discord.gg/mockforge)

## Troubleshooting

### Go Plugin Won't Build

```bash
# Check TinyGo version
tinygo version

# Ensure wasi target is installed
tinygo targets | grep wasi

# Clean and rebuild
rm plugin.wasm go.sum
go mod tidy
tinygo build -o plugin.wasm -target=wasi main.go
```

### Python Plugin Won't Start

```bash
# Check Python version
python --version  # Must be 3.9+

# Reinstall dependencies
pip install --force-reinstall -r requirements.txt

# Check port availability
lsof -i :8080
```

### Plugin Not Loading in MockForge

```bash
# Validate plugin manifest
mockforge plugin validate .

# Check MockForge logs
mockforge logs --follow

# Verify plugin loaded
mockforge plugin list
```

## Performance Tips

### Go Plugins
- Minimize allocations
- Use efficient data structures
- Avoid goroutines (limited support)
- Profile with `tinygo build -print-llvm`

### Python Plugins
- Enable request caching
- Use async/await properly
- Deploy multiple instances
- Use connection pooling

### AssemblyScript Plugins
- Use fixed-size arrays when possible
- Avoid boxing/unboxing
- Inline small functions
- Use `--optimize` flag

## Security Best Practices

1. **Validate all inputs** from requests
2. **Use environment variables** for secrets
3. **Limit resource usage** in capabilities
4. **Enable audit logging** for sensitive operations
5. **Test error handling** thoroughly
6. **Keep dependencies updated**

## Getting Help

**Quick Questions**: GitHub Discussions
**Bug Reports**: GitHub Issues
**Real-time Chat**: Discord
**Email**: support@mockforge.dev

---

**Status**: ✅ Ready for Phase 1 Implementation
**Last Updated**: 2025-10-09
**Version**: 1.0
