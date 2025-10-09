# Polyglot Plugin Support Design

## Overview

MockForge's current plugin system is WebAssembly-based with Rust as the primary development language. While this provides excellent performance and security, it creates barriers for developers who prefer other languages. This document outlines a comprehensive strategy to enable polyglot plugin development through two complementary approaches:

1. **WASM SDKs for Multiple Languages** - Enable developers to write WASM plugins in TinyGo, AssemblyScript, and potentially Python (via Pyodide)
2. **Remote Plugin Protocol** - Allow plugins to run as external services, callable via HTTP/gRPC

## Current State

### Strengths
- **Security**: WebAssembly sandbox provides excellent isolation
- **Performance**: Native-speed execution with resource limits
- **Integration**: Direct memory access and tight integration with MockForge core
- **Ecosystem**: Comprehensive Rust SDK with templates and tooling

### Limitations
- **Language Barrier**: Rust knowledge required for complex plugins
- **Development Friction**: Requires Rust toolchain, WASM target, and compile workflow
- **Limited Ecosystem**: Cannot easily use language-specific libraries (e.g., Python's data science stack)

## Approach 1: WASM SDKs for Multiple Languages

Enable developers to write WASM plugins in languages other than Rust while maintaining the security and performance benefits of the WASM runtime.

### Target Languages

#### 1. TinyGo (Go → WASM)
**Rationale**:
- Large Go developer community
- Good WASM support via TinyGo compiler
- Familiar to backend developers
- Strong stdlib for HTTP, JSON, etc.

**Implementation Strategy**:
- Create Go package that mirrors the Rust plugin API
- Use `//go:wasm` directives for WASM exports
- Provide Go-idiomatic interfaces
- Generate WASM with TinyGo compiler

**Example Interface**:
```go
package mockforge

import "encoding/json"

// AuthPlugin interface for authentication plugins
type AuthPlugin interface {
    Authenticate(ctx *PluginContext, credentials *AuthCredentials) (*AuthResult, error)
    GetCapabilities() *PluginCapabilities
}

// Export function to register plugin
func ExportAuthPlugin(plugin AuthPlugin) {
    // TinyGo WASM export magic
}
```

**Challenges**:
- Go's async model differs from Rust's (no direct async/await)
- TinyGo stdlib limitations
- Memory management between Go and WASM host

#### 2. AssemblyScript (TypeScript → WASM)
**Rationale**:
- TypeScript-like syntax familiar to web developers
- Excellent WASM support by design
- Growing ecosystem
- Easy learning curve

**Implementation Strategy**:
- Create AssemblyScript module with TypeScript-like API
- Use AS/Loader for WASM integration
- Provide decorators for plugin exports
- Strong typing with interfaces

**Example Interface**:
```typescript
import { PluginContext, AuthCredentials, AuthResult, PluginCapabilities } from "@mockforge/plugin-sdk";

@plugin("auth")
export class MyAuthPlugin {
    authenticate(ctx: PluginContext, credentials: AuthCredentials): AuthResult {
        // Authentication logic
        return new AuthResult(true, "user123", new Map());
    }

    getCapabilities(): PluginCapabilities {
        return new PluginCapabilities({
            network: { allowHttpOutbound: false },
            filesystem: { allowRead: false },
            resources: { maxMemoryBytes: 10485760 }
        });
    }
}
```

**Challenges**:
- Limited stdlib compared to JavaScript
- Memory management
- Async/await support

#### 3. Python via Pyodide (Experimental)
**Rationale**:
- Huge Python developer community
- Access to data science/ML libraries
- Great for data transformation plugins

**Implementation Strategy**:
- Embed Pyodide runtime
- Bridge Python API to WASM host
- Provide Python package for plugin development
- Consider performance implications

**Example Interface**:
```python
from mockforge_plugin import AuthPlugin, PluginContext, AuthCredentials, AuthResult

class MyAuthPlugin(AuthPlugin):
    def authenticate(self, ctx: PluginContext, credentials: AuthCredentials) -> AuthResult:
        # Authentication logic
        return AuthResult(authenticated=True, user_id="user123", claims={})

    def get_capabilities(self):
        return {
            "network": {"allow_http_outbound": False},
            "filesystem": {"allow_read": False},
            "resources": {"max_memory_bytes": 10485760}
        }

# Register plugin
register_plugin(MyAuthPlugin())
```

**Challenges**:
- Pyodide bundle size (~10-20MB)
- Slow startup time
- Limited Python stdlib access in WASM
- Significant performance overhead

### Technical Implementation

#### 1. Plugin Interface Definition Language (IDL)
Create a language-agnostic interface definition using **WebAssembly Interface Types (WIT)**:

```wit
// plugin.wit
interface plugin {
    // Common types
    record plugin-context {
        method: string,
        uri: string,
        headers: list<tuple<string, string>>,
        body: option<list<u8>>
    }

    record auth-credentials {
        credential-type: string,
        data: list<u8>
    }

    record auth-result {
        authenticated: bool,
        user-id: string,
        claims: list<tuple<string, string>>
    }

    // Auth plugin interface
    resource auth-plugin {
        authenticate: func(ctx: plugin-context, creds: auth-credentials) -> result<auth-result, string>
    }
}
```

#### 2. SDK Generator
Build a tool to generate SDK code for each language from WIT definitions:

```bash
mockforge-plugin-codegen \
    --wit plugin.wit \
    --lang go \
    --output ./sdk/go

mockforge-plugin-codegen \
    --wit plugin.wit \
    --lang assemblyscript \
    --output ./sdk/assemblyscript
```

#### 3. Runtime Adapter
Enhance the plugin loader to detect and adapt different WASM runtimes:

```rust
// In plugin-loader/src/runtime_adapter.rs
pub enum WasmRuntime {
    Rust,           // Standard Rust WASM
    TinyGo,         // TinyGo compiled WASM
    AssemblyScript, // AssemblyScript WASM
    Pyodide,        // Python via Pyodide
}

impl PluginLoader {
    fn detect_runtime(&self, wasm_bytes: &[u8]) -> WasmRuntime {
        // Detect based on WASM custom sections or exports
    }

    fn create_runtime_adapter(&self, runtime: WasmRuntime) -> Box<dyn RuntimeAdapter> {
        match runtime {
            WasmRuntime::Rust => Box::new(RustAdapter::new()),
            WasmRuntime::TinyGo => Box::new(TinyGoAdapter::new()),
            // ...
        }
    }
}
```

## Approach 2: Remote Plugin Protocol

Allow plugins to run as external services, enabling any language/runtime while sacrificing some performance.

### Architecture

#### High-Level Design
```
MockForge Core
    ↓ (HTTP/gRPC)
Remote Plugin Service (any language)
    ↓ (calls)
External Services/Libraries
```

#### Communication Protocol

**Option A: HTTP/JSON (Simpler)**
```http
POST /plugin/authenticate HTTP/1.1
Content-Type: application/json

{
  "context": {
    "method": "GET",
    "uri": "/api/users",
    "headers": {"Authorization": "Bearer token"}
  },
  "credentials": {
    "type": "bearer",
    "token": "eyJhbGc..."
  }
}

Response:
{
  "success": true,
  "result": {
    "authenticated": true,
    "user_id": "user123",
    "claims": {"role": "admin"}
  }
}
```

**Option B: gRPC (Better Performance)**
```protobuf
// plugin.proto
service PluginService {
    rpc Authenticate(AuthRequest) returns (AuthResponse);
    rpc ExecuteTemplateFunction(TemplateFunctionRequest) returns (TemplateFunctionResponse);
    rpc GenerateResponse(ResponseRequest) returns (ResponseResponse);
    rpc QueryDataSource(DataSourceRequest) returns (DataSourceResponse);
}

message AuthRequest {
    PluginContext context = 1;
    AuthCredentials credentials = 2;
}

message AuthResponse {
    bool success = 1;
    string error = 2;
    AuthResult result = 3;
}
```

### Remote Plugin Configuration

Add remote plugin support to `plugin.yaml`:

```yaml
plugin:
  id: "my-remote-plugin"
  version: "0.1.0"
  name: "My Remote Python Plugin"
  description: "A plugin running as an external service"
  types: ["auth"]
  runtime: "remote"  # NEW: Indicate this is a remote plugin

remote:
  protocol: "http"  # or "grpc"
  endpoint: "http://localhost:8080/plugin"
  timeout_ms: 5000
  retry_policy:
    max_retries: 3
    backoff_ms: 1000
  health_check:
    enabled: true
    endpoint: "/health"
    interval_seconds: 30

capabilities:
  # Remote plugins have different security model
  network:
    allow_outbound: true  # The remote service can make network calls
  resources:
    max_concurrent_requests: 10
    timeout_ms: 5000
```

### Security Considerations for Remote Plugins

1. **Authentication**: Remote plugins must authenticate to MockForge
2. **TLS**: Enforce TLS for production deployments
3. **API Keys**: Shared secret or OAuth2 client credentials
4. **Rate Limiting**: Prevent abuse
5. **Network Isolation**: Deploy in same VPC/network when possible

### Remote Plugin SDK (Multiple Languages)

#### Python Example:
```python
# mockforge_remote_plugin/sdk.py
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

class PluginContext(BaseModel):
    method: str
    uri: str
    headers: dict[str, str]

class AuthCredentials(BaseModel):
    credential_type: str
    token: str

class AuthResult(BaseModel):
    authenticated: bool
    user_id: str
    claims: dict[str, any]

class MockForgePlugin:
    def __init__(self):
        self.app = FastAPI()
        self._register_routes()

    def _register_routes(self):
        @self.app.post("/plugin/authenticate")
        async def authenticate(
            context: PluginContext,
            credentials: AuthCredentials
        ):
            result = await self.authenticate(context, credentials)
            return {"success": True, "result": result.dict()}

    async def authenticate(self, ctx: PluginContext, creds: AuthCredentials) -> AuthResult:
        raise NotImplementedError()

# User implements:
class MyAuthPlugin(MockForgePlugin):
    async def authenticate(self, ctx, creds):
        # Use any Python library!
        import jwt
        import requests

        # Validate JWT with external service
        response = requests.post("https://auth.example.com/verify",
                                json={"token": creds.token})

        if response.status_code == 200:
            data = response.json()
            return AuthResult(
                authenticated=True,
                user_id=data["user_id"],
                claims=data["claims"]
            )

        return AuthResult(authenticated=False, user_id="", claims={})

if __name__ == "__main__":
    plugin = MyAuthPlugin()
    plugin.app.run(host="0.0.0.0", port=8080)
```

#### Node.js Example:
```javascript
// mockforge-plugin-sdk
const express = require('express');

class MockForgePlugin {
    constructor() {
        this.app = express();
        this.app.use(express.json());
        this.registerRoutes();
    }

    registerRoutes() {
        this.app.post('/plugin/authenticate', async (req, res) => {
            const { context, credentials } = req.body;
            try {
                const result = await this.authenticate(context, credentials);
                res.json({ success: true, result });
            } catch (error) {
                res.status(500).json({ success: false, error: error.message });
            }
        });
    }

    async authenticate(context, credentials) {
        throw new Error('Not implemented');
    }

    listen(port = 8080) {
        this.app.listen(port, () => {
            console.log(`Plugin listening on port ${port}`);
        });
    }
}

// User implements:
class MyAuthPlugin extends MockForgePlugin {
    async authenticate(context, credentials) {
        // Use any npm package!
        const jwt = require('jsonwebtoken');
        const axios = require('axios');

        // Verify token with external service
        const response = await axios.post('https://auth.example.com/verify', {
            token: credentials.token
        });

        if (response.status === 200) {
            return {
                authenticated: true,
                user_id: response.data.user_id,
                claims: response.data.claims
            };
        }

        return { authenticated: false, user_id: '', claims: {} };
    }
}

const plugin = new MyAuthPlugin();
plugin.listen(8080);
```

### Implementation in MockForge Core

#### 1. Remote Plugin Loader
```rust
// plugin-loader/src/remote_plugin_loader.rs
pub struct RemotePluginLoader {
    client: reqwest::Client,
    config: RemotePluginConfig,
}

impl RemotePluginLoader {
    pub async fn call_authenticate(
        &self,
        context: &PluginContext,
        credentials: &AuthCredentials,
    ) -> Result<AuthResult, PluginError> {
        let request = AuthenticateRequest {
            context: context.clone(),
            credentials: credentials.clone(),
        };

        let response = self.client
            .post(&format!("{}/plugin/authenticate", self.config.endpoint))
            .timeout(Duration::from_millis(self.config.timeout_ms))
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            let result: PluginResponse<AuthResult> = response.json().await?;
            if result.success {
                Ok(result.result)
            } else {
                Err(PluginError::ExecutionFailed(result.error.unwrap_or_default()))
            }
        } else {
            Err(PluginError::RemoteCallFailed(response.status()))
        }
    }
}
```

#### 2. Plugin Runtime Abstraction
```rust
// plugin-core/src/runtime.rs
#[async_trait]
pub trait PluginRuntime: Send + Sync {
    async fn call_auth(
        &self,
        context: &PluginContext,
        credentials: &AuthCredentials,
    ) -> Result<AuthResult, PluginError>;

    // Other plugin type methods...
}

pub enum PluginRuntimeType {
    Wasm(WasmRuntime),
    Remote(RemoteRuntime),
}

impl PluginRuntimeType {
    pub fn runtime(&self) -> Box<dyn PluginRuntime> {
        match self {
            Self::Wasm(wasm) => Box::new(wasm.clone()),
            Self::Remote(remote) => Box::new(remote.clone()),
        }
    }
}
```

## Comparison: WASM SDKs vs Remote Plugins

| Aspect | WASM SDKs | Remote Plugins |
|--------|-----------|----------------|
| **Performance** | Native speed | Network latency (~1-50ms) |
| **Security** | Sandboxed | Requires network trust |
| **Isolation** | Strong (WASM sandbox) | Process-level |
| **Language Support** | Limited (WASM-compatible) | Any language |
| **Deployment** | Single binary | Separate service |
| **Library Access** | Limited | Full ecosystem |
| **Development** | Requires WASM toolchain | Use native tools |
| **Debugging** | Harder (WASM) | Native debugging |
| **Resource Limits** | Strict (memory, CPU) | Harder to enforce |
| **Scalability** | Scales with MockForge | Separate scaling |

## Implementation Roadmap

### Phase 1: Foundation (Weeks 1-2)
- [ ] Design WIT interface definitions
- [ ] Create plugin runtime abstraction layer
- [ ] Implement runtime detection logic
- [ ] Add remote plugin configuration schema

### Phase 2: WASM SDKs (Weeks 3-6)
- [ ] Implement TinyGo SDK with examples
- [ ] Implement AssemblyScript SDK with examples
- [ ] Create plugin templates for each language
- [ ] Build SDK code generator tool
- [ ] Add language-specific documentation

### Phase 3: Remote Plugin Protocol (Weeks 7-10)
- [ ] Implement HTTP protocol support
- [ ] Implement gRPC protocol support
- [ ] Create Python remote plugin SDK
- [ ] Create Node.js remote plugin SDK
- [ ] Add authentication and security features
- [ ] Implement health checking and retry logic

### Phase 4: Tooling and DX (Weeks 11-12)
- [ ] Enhance CLI with multi-language support
- [ ] Add plugin templates to `mockforge plugin new`
- [ ] Update Admin UI to show runtime type
- [ ] Create comprehensive examples
- [ ] Write migration guides

### Phase 5: Documentation and Launch (Week 13-14)
- [ ] Write complete developer documentation
- [ ] Create video tutorials
- [ ] Build example plugins in each language
- [ ] Beta test with community
- [ ] Gather feedback and iterate

## Success Metrics

1. **Adoption**:
   - Number of non-Rust plugins published
   - Languages used for plugin development
   - Community plugin contributions

2. **Performance**:
   - WASM SDK performance parity with Rust
   - Remote plugin latency P50/P95/P99
   - Resource usage comparison

3. **Developer Experience**:
   - Time to build first plugin (by language)
   - Developer satisfaction surveys
   - Documentation clarity ratings

## Community Engagement

### Demand Assessment
1. **Surveys**: Poll users on preferred languages
2. **GitHub Discussions**: Gauge interest in polyglot support
3. **Prototype Release**: Ship TinyGo SDK as experimental feature
4. **Feedback Loop**: Iterate based on early adopter feedback

### Prioritization
Based on community demand:
- **High Demand**: Full implementation with examples
- **Medium Demand**: Basic SDK with minimal examples
- **Low Demand**: Documentation on how to build yourself

## Risks and Mitigations

### Risk 1: Maintenance Burden
**Mitigation**:
- Auto-generate SDKs from WIT definitions
- Community-maintained SDKs for niche languages
- Clear tier system (Tier 1: Rust, Tier 2: Go/AS, Tier 3: Community)

### Risk 2: Performance Degradation
**Mitigation**:
- Benchmark all runtime adapters
- Document performance characteristics
- Recommend WASM for performance-critical plugins

### Risk 3: Security Vulnerabilities
**Mitigation**:
- Comprehensive security review
- Automated security scanning
- Clear security guidelines per runtime
- Plugin signing and verification

### Risk 4: Fragmented Ecosystem
**Mitigation**:
- Consistent API across languages
- Shared examples and patterns
- Central plugin registry
- Quality badges for well-maintained plugins

## Alternatives Considered

### Alternative 1: Rust-Only with Better Docs
**Pros**: Maintains simplicity, better performance
**Cons**: Doesn't address core issue of language barrier

### Alternative 2: JavaScript/TypeScript Only (via QuickJS)
**Pros**: Huge developer base, easy to use
**Cons**: Limited to JS ecosystem, security concerns

### Alternative 3: Plugin Marketplace with Pre-built Plugins
**Pros**: Users don't need to write code
**Cons**: Doesn't solve customization needs

## Conclusion

Polyglot plugin support will significantly lower the barrier to entry for MockForge plugin development. By providing both WASM SDKs and a remote plugin protocol, we offer:

1. **WASM SDKs**: For developers who want performance and tight integration
2. **Remote Plugins**: For developers who want to use existing tools and libraries

This dual approach maximizes flexibility while maintaining the security and performance characteristics that make MockForge plugins powerful.

### Recommended First Steps

1. **TinyGo SDK** (Weeks 1-3): Test the market with Go developers
2. **Remote Plugin Protocol + Python SDK** (Weeks 4-6): Enable rapid prototyping
3. **Gather Feedback** (Week 7): Assess adoption and iterate
4. **Expand Based on Demand** (Weeks 8+): Add AssemblyScript, other languages as needed

This phased approach allows us to validate demand before investing heavily in all languages, while still providing immediate value to the community.
