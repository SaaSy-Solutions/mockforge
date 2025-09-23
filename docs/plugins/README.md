# MockForge Plugin System

MockForge's plugin system enables developers to extend and customize API mocking capabilities through secure, sandboxed WebAssembly modules. This system allows for custom authentication methods, template functions, response generators, and data source integrations.

## 🚀 Quick Start

### 1. Create Your First Plugin

```bash
# Create a new plugin project
cargo new --lib my-auth-plugin
cd my-auth-plugin
```

### 2. Add Dependencies

```toml
[package]
name = "my-auth-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]  # Important: Compile to WebAssembly

[dependencies]
mockforge-plugin-core = { path = "../../../crates/mockforge-plugin-core" }
wit-bindgen = "0.34"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### 3. Implement Your Plugin

```rust
use mockforge_plugin_core::*;
use std::collections::HashMap;

// For AuthPlugin
#[derive(Debug)]
pub struct MyAuthPlugin;

#[async_trait::async_trait]
impl AuthPlugin for MyAuthPlugin {
    async fn authenticate(
        &self,
        context: &PluginContext,
        credentials: &AuthCredentials,
    ) -> PluginResult<AuthResult> {
        // Your authentication logic here
        PluginResult::success(AuthResult::Authenticated {
            user_id: "user123".to_string(),
            claims: HashMap::new(),
        })
    }

    fn get_capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            network: NetworkCapabilities {
                allow_http_outbound: false,
                allowed_hosts: vec![],
            },
            filesystem: FilesystemCapabilities {
                allow_read: false,
                allow_write: false,
                allowed_paths: vec![],
            },
            resources: PluginResources {
                max_memory_bytes: 10 * 1024 * 1024, // 10MB
                max_cpu_time_ms: 1000, // 1 second
            },
        }
    }
}

mockforge_plugin_core::export_plugin!(MyAuthPlugin);
```

### 4. Create Plugin Manifest

Create `plugin.yaml`:

```yaml
plugin:
  id: "my-auth-plugin"
  version: "0.1.0"
  name: "My Custom Auth Plugin"
  description: "Custom authentication for my API"
  types: ["auth"]
  author:
    name: "Your Name"
    email: "your.email@example.com"
  homepage: "https://github.com/your/plugin"
  repository: "https://github.com/your/plugin"

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

### 5. Build and Install

```bash
# Build WebAssembly module
cargo build --target wasm32-wasi --release

# Install plugin
mockforge plugin install ./target/wasm32-wasi/release/my_auth_plugin.wasm
```

## 📋 Plugin Types

MockForge supports four main plugin types:

### 🔐 Authentication Plugins (`auth`)
Handle custom authentication methods like OAuth, SAML, LDAP, or proprietary auth schemes.

**Interface:**
```rust
#[async_trait::async_trait]
pub trait AuthPlugin: Send + Sync {
    async fn authenticate(
        &self,
        context: &PluginContext,
        credentials: &AuthCredentials,
    ) -> PluginResult<AuthResult>;
}
```

### 🏷️ Template Plugins (`template`)
Add custom template functions for dynamic content generation.

**Interface:**
```rust
#[async_trait::async_trait]
pub trait TemplatePlugin: Send + Sync {
    async fn execute_function(
        &self,
        function_name: &str,
        args: &[Value],
        context: &PluginContext,
    ) -> PluginResult<Value>;
}
```

### 📤 Response Plugins (`response`)
Implement custom response generation logic for complex scenarios.

**Interface:**
```rust
#[async_trait::async_trait]
pub trait ResponsePlugin: Send + Sync {
    async fn generate_response(
        &self,
        context: &PluginContext,
        config: &ResponsePluginConfig,
    ) -> PluginResult<Value>;
}
```

### 🗄️ Data Source Plugins (`datasource`)
Connect to external data sources like databases, APIs, or files.

**Interface:**
```rust
#[async_trait::async_trait]
pub trait DataSourcePlugin: Send + Sync {
    async fn query(
        &self,
        query: &str,
        parameters: &HashMap<String, Value>,
        context: &PluginContext,
    ) -> PluginResult<DataSet>;
}
```

## 🛡️ Security Model

### Sandboxing
- **WebAssembly Execution**: All plugins run in secure WebAssembly sandbox
- **Resource Limits**: Configurable memory and CPU time limits
- **Capability-Based Access**: Explicit permissions for network and filesystem access

### Validation
- **Manifest Validation**: Plugin manifests are validated on load
- **WASM Module Validation**: WebAssembly modules are verified for safety
- **Dependency Resolution**: Plugin dependencies are checked and resolved

### Permissions
Plugins must explicitly declare capabilities:

```yaml
capabilities:
  network:
    allow_http_outbound: true
    allowed_hosts: ["api.example.com", "*.trusted.com"]
  filesystem:
    allow_read: true
    allow_write: false
    allowed_paths: ["/data/input"]
  resources:
    max_memory_bytes: 67108864  # 64MB
    max_cpu_time_ms: 5000       # 5 seconds
```

## 🛠️ Development Tools

### CLI Commands
```bash
# List installed plugins
mockforge plugin list

# Install a plugin
mockforge plugin install /path/to/plugin.wasm

# Remove a plugin
mockforge plugin remove plugin-id

# Validate a plugin
mockforge plugin validate /path/to/plugin.wasm

# Reload all plugins
mockforge plugin reload
```

### Admin UI
The admin interface provides:
- **Plugin Dashboard**: View installed plugins and their status
- **Installation Interface**: Install plugins via file upload or URL
- **Health Monitoring**: Real-time plugin health and performance metrics
- **Capability Management**: Review and manage plugin permissions

## 📚 Examples

### Authentication Plugin
[See examples/plugins/auth-oauth2/](examples/plugins/auth-oauth2/)

### Template Plugin
[See examples/plugins/template-crypto/](examples/plugins/template-crypto/)

### Response Plugin
[See examples/plugins/response-graphql/](examples/plugins/response-graphql/)

### Data Source Plugin
[See examples/plugins/datasource-postgres/](examples/plugins/datasource-postgres/)

## 🔧 API Reference

- [Plugin Core API](api-reference/core.md) - Core types and traits
- [Security Model](security/model.md) - Security and sandboxing details
- [Manifest Format](api-reference/manifest.md) - Plugin manifest specification
- [CLI Reference](api-reference/cli.md) - Command-line interface

## 🤝 Contributing

We welcome plugin contributions! Please see our [Plugin Development Guide](development-guide.md) for detailed instructions on creating and submitting plugins.

## 📄 License

MockForge plugins are licensed under the same terms as MockForge itself (MIT OR Apache-2.0).
