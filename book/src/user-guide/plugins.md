# Plugin System

MockForge features a powerful WebAssembly-based plugin system that allows you to extend functionality without modifying the core framework. Plugins run in a secure sandbox with resource limits and provide capabilities for custom response generation, authentication, data sources, and template extensions.

## Overview

The plugin system enables:

- **Custom Response Generators**: Create specialized mock data and responses
- **Authentication Providers**: Implement JWT, OAuth2, and custom authentication schemes
- **Data Source Connectors**: Connect to CSV files, databases, and external APIs
- **Template Extensions**: Add custom template functions and filters
- **Protocol Handlers**: Extend support for custom protocols and formats

## Plugin Architecture

### WebAssembly Runtime

Plugins are compiled to WebAssembly (WASM) and run in an isolated runtime environment:

- **Security Sandbox**: Isolated execution prevents plugins from accessing unauthorized resources
- **Resource Limits**: CPU, memory, and execution time constraints
- **Capability System**: Fine-grained permissions control what plugins can access
- **Cross-platform**: WASM plugins work on any platform MockForge supports

### Plugin Types

MockForge supports several plugin types:

| Type | Description | Interface |
|------|-------------|-----------|
| `response` | Generate custom response data | `ResponseGenerator` |
| `auth` | Handle authentication and authorization | `AuthProvider` |
| `datasource` | Connect to external data sources | `DataSourceConnector` |
| `template` | Add custom template functions | `TemplateExtension` |
| `protocol` | Support custom protocols | `ProtocolHandler` |

## Installing Plugins

### From Plugin Registry

```bash
# Install plugin from registry
mockforge plugin install auth-jwt

# Install specific version
mockforge plugin install auth-jwt@1.2.0

# List available plugins
mockforge plugin search
```

### From Local File

```bash
# Install from local WASM file
mockforge plugin install ./my-plugin.wasm

# Install with manifest
mockforge plugin install ./my-plugin/ --manifest plugin.yaml
```

### From Git Repository

```bash
# Install from Git repository
mockforge plugin install https://github.com/example/mockforge-plugin-custom.git

# Install specific branch/tag
mockforge plugin install https://github.com/example/mockforge-plugin-custom.git#v1.0.0
```

## Plugin Management

### List Installed Plugins

```bash
# List all installed plugins
mockforge plugin list

# Show detailed information
mockforge plugin list --verbose

# Filter by type
mockforge plugin list --type auth
```

### Enable/Disable Plugins

```bash
# Enable plugin
mockforge plugin enable auth-jwt

# Disable plugin
mockforge plugin disable auth-jwt

# Enable plugin for specific workspace
mockforge plugin enable auth-jwt --workspace my-workspace
```

### Update Plugins

```bash
# Update specific plugin
mockforge plugin update auth-jwt

# Update all plugins
mockforge plugin update --all

# Check for updates
mockforge plugin outdated
```

### Remove Plugins

```bash
# Remove plugin
mockforge plugin remove auth-jwt

# Remove plugin and its data
mockforge plugin remove auth-jwt --purge
```

## Plugin Configuration

### Global Configuration

Configure plugins in your MockForge configuration file:

```yaml
plugins:
  enabled: true
  directory: "~/.mockforge/plugins"
  runtime:
    memory_limit_mb: 64
    cpu_limit_percent: 10
    execution_timeout_ms: 5000
  
  # Plugin-specific configuration
  auth-jwt:
    enabled: true
    config:
      secret_key: "${JWT_SECRET}"
      algorithm: "HS256"
      expiration: 3600
  
  datasource-csv:
    enabled: true
    config:
      base_directory: "./data"
      cache_ttl: 300
```

### Environment Variables

```bash
# Plugin system settings
export MOCKFORGE_PLUGINS_ENABLED=true
export MOCKFORGE_PLUGINS_DIRECTORY=~/.mockforge/plugins

# Runtime limits
export MOCKFORGE_PLUGIN_MEMORY_LIMIT=64
export MOCKFORGE_PLUGIN_CPU_LIMIT=10
export MOCKFORGE_PLUGIN_TIMEOUT=5000

# Plugin-specific settings
export JWT_SECRET=your-secret-key
export CSV_DATA_DIR=./test-data
```

## Developing Plugins

### Plugin Manifest

Every plugin requires a `plugin.yaml` manifest file:

```yaml
# plugin.yaml
name: "auth-jwt"
version: "1.0.0"
description: "JWT authentication provider"
author: "Your Name <email@example.com>"
license: "MIT"
repository: "https://github.com/example/mockforge-plugin-auth-jwt"

# Plugin metadata
type: "auth"
category: "authentication"
tags: ["jwt", "auth", "security"]

# Runtime requirements
runtime:
  wasm_version: "0.1"
  memory_limit_mb: 32
  execution_timeout_ms: 1000

# Capabilities required
capabilities:
  - "network.http.client"
  - "storage.key_value"
  - "template.functions"

# Configuration schema
config_schema:
  type: "object"
  properties:
    secret_key:
      type: "string"
      description: "JWT signing secret"
      required: true
    algorithm:
      type: "string"
      enum: ["HS256", "HS384", "HS512", "RS256"]
      default: "HS256"
    expiration:
      type: "integer"
      description: "Token expiration in seconds"
      default: 3600
      minimum: 60

# Export information
exports:
  auth_provider: "JwtAuthProvider"
  template_functions:
    - "jwt_encode"
    - "jwt_decode"
    - "jwt_verify"
```

### Rust Plugin Development

Create a new Rust project for your plugin:

```bash
cargo new --lib mockforge-plugin-custom
cd mockforge-plugin-custom
```

Add dependencies to `Cargo.toml`:

```toml
[package]
name = "mockforge-plugin-custom"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
mockforge-plugin-core = "0.1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
wasm-bindgen = "0.2"

[dependencies.web-sys]
version = "0.3"
features = [
  "console",
]
```

Implement your plugin in `src/lib.rs`:

```rust
use mockforge_plugin_core::{
    AuthProvider, AuthResult, PluginConfig, PluginError, PluginResult,
    export_auth_provider, export_template_functions
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Deserialize)]
struct JwtConfig {
    secret_key: String,
    algorithm: String,
    expiration: u64,
}

pub struct JwtAuthProvider {
    config: JwtConfig,
}

impl JwtAuthProvider {
    pub fn new(config: PluginConfig) -> PluginResult<Self> {
        let jwt_config: JwtConfig = serde_json::from_value(config.into())?;
        Ok(Self { config: jwt_config })
    }
}

impl AuthProvider for JwtAuthProvider {
    fn authenticate(&self, token: &str) -> PluginResult<AuthResult> {
        // Implement JWT validation logic
        match self.verify_jwt(token) {
            Ok(claims) => Ok(AuthResult::success(claims)),
            Err(e) => Ok(AuthResult::failure(e.to_string())),
        }
    }
    
    fn generate_token(&self, user_id: &str) -> PluginResult<String> {
        // Implement JWT generation logic
        self.create_jwt(user_id)
    }
}

impl JwtAuthProvider {
    fn verify_jwt(&self, token: &str) -> Result<serde_json::Value, PluginError> {
        // JWT verification implementation
        todo!("Implement JWT verification")
    }
    
    fn create_jwt(&self, user_id: &str) -> PluginResult<String> {
        // JWT creation implementation
        todo!("Implement JWT creation")
    }
}

// Template functions
#[wasm_bindgen]
pub fn jwt_encode(payload: &str, secret: &str) -> String {
    // Implement JWT encoding for templates
    todo!("Implement template JWT encoding")
}

#[wasm_bindgen]
pub fn jwt_decode(token: &str) -> String {
    // Implement JWT decoding for templates
    todo!("Implement template JWT decoding")
}

// Export plugin interfaces
export_auth_provider!(JwtAuthProvider);
export_template_functions! {
    "jwt_encode" => jwt_encode,
    "jwt_decode" => jwt_decode,
}
```

### Building Plugins

Build your plugin to WebAssembly:

```bash
# Install wasm-pack if not already installed
cargo install wasm-pack

# Build the plugin
wasm-pack build --target web --out-dir pkg

# The WASM file will be in pkg/mockforge_plugin_custom.wasm
```

### Testing Plugins

MockForge provides a testing framework for plugins:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_plugin_core::test_utils::*;

    #[test]
    fn test_jwt_authentication() {
        let config = test_config! {
            "secret_key": "test-secret",
            "algorithm": "HS256",
            "expiration": 3600
        };
        
        let provider = JwtAuthProvider::new(config).unwrap();
        
        // Test valid token
        let token = provider.generate_token("user123").unwrap();
        let result = provider.authenticate(&token).unwrap();
        assert!(result.is_success());
        
        // Test invalid token
        let invalid_result = provider.authenticate("invalid.token.here").unwrap();
        assert!(invalid_result.is_failure());
    }
}
```

## Plugin Examples

MockForge includes several example plugins to demonstrate different capabilities:

### Authentication Plugins

#### Basic Authentication (`auth-basic`)

```yaml
# examples/plugins/auth-basic/plugin.yaml
name: "auth-basic"
type: "auth"
description: "HTTP Basic Authentication provider"

config_schema:
  type: "object"
  properties:
    users:
      type: "object"
      description: "Username to password mapping"
    realm:
      type: "string"
      default: "MockForge"
```

Usage in MockForge configuration:

```yaml
plugins:
  auth-basic:
    enabled: true
    config:
      realm: "API Access"
      users:
        admin: "password123"
        user: "userpass"
```

#### JWT Authentication (`auth-jwt`)

Advanced JWT authentication with support for multiple algorithms:

```yaml
# examples/plugins/auth-jwt/plugin.yaml
name: "auth-jwt"
type: "auth"
description: "JWT authentication provider with multiple algorithm support"

capabilities:
  - "storage.key_value"
  - "template.functions"

config_schema:
  type: "object"
  properties:
    secret_key:
      type: "string"
      required: true
    algorithm:
      type: "string"
      enum: ["HS256", "HS384", "HS512", "RS256", "RS384", "RS512"]
      default: "HS256"
    issuer:
      type: "string"
      description: "JWT issuer claim"
    audience:
      type: "string"
      description: "JWT audience claim"
```

### Data Source Plugins

#### CSV Data Source (`datasource-csv`)

Connect to CSV files as data sources:

```yaml
# examples/plugins/datasource-csv/plugin.yaml
name: "datasource-csv"
type: "datasource"
description: "CSV file data source connector"

config_schema:
  type: "object"
  properties:
    base_directory:
      type: "string"
      description: "Base directory for CSV files"
      required: true
    cache_ttl:
      type: "integer"
      description: "Cache TTL in seconds"
      default: 300
    delimiter:
      type: "string"
      description: "CSV delimiter"
      default: ","
```

Usage in templates:

```yaml
response:
  status: 200
  body:
    users: "{{datasource.csv('users.csv').random(5)}}"
    products: "{{datasource.csv('products.csv').filter('category', 'electronics')}}"
```

### Template Plugins

#### Crypto Functions (`template-crypto`)

Add cryptographic template functions:

```yaml
# examples/plugins/template-crypto/plugin.yaml
name: "template-crypto"
type: "template"
description: "Cryptographic template functions"

exports:
  template_functions:
    - "crypto_hash"
    - "crypto_hmac"
    - "crypto_encrypt"
    - "crypto_decrypt"
    - "crypto_random"
```

Template usage:

```yaml
response:
  body:
    user_id: "{{uuid}}"
    password_hash: "{{crypto_hash(faker.password, 'sha256')}}"
    api_key: "{{crypto_random(32, 'hex')}}"
    signature: "{{crypto_hmac(request.body, env.API_SECRET, 'sha256')}}"
```

### Response Plugins

#### GraphQL Response Generator (`response-graphql`)

Generate GraphQL responses from schema:

```yaml
# examples/plugins/response-graphql/plugin.yaml
name: "response-graphql"
type: "response"
description: "GraphQL response generator"

config_schema:
  type: "object"
  properties:
    schema_file:
      type: "string"
      description: "Path to GraphQL schema file"
      required: true
    resolvers:
      type: "object"
      description: "Custom resolver configuration"
```

## Security Considerations

### Capability System

Plugins must declare required capabilities:

```yaml
# plugin.yaml
capabilities:
  - "network.http.client"     # Make HTTP requests
  - "network.http.server"     # Handle HTTP requests
  - "storage.key_value"       # Access key-value storage
  - "storage.file.read"       # Read files
  - "storage.file.write"      # Write files
  - "template.functions"      # Register template functions
  - "crypto.random"           # Access random number generation
  - "crypto.hash"             # Access hashing functions
```

### Resource Limits

Configure resource limits per plugin:

```yaml
plugins:
  my-plugin:
    runtime:
      memory_limit_mb: 64        # Maximum memory usage
      cpu_limit_percent: 5       # Maximum CPU usage
      execution_timeout_ms: 2000 # Maximum execution time
      network_timeout_ms: 1000   # Network request timeout
```

### Sandboxing

Plugins run in a secure sandbox that:

- Prevents access to the host file system outside permitted directories
- Limits network access to declared endpoints
- Restricts system calls and resource usage
- Isolates plugin memory from the host process

## Best Practices

### Plugin Development

1. **Keep plugins focused**: Each plugin should have a single, clear purpose
2. **Minimize resource usage**: Use efficient algorithms and limit memory allocation
3. **Handle errors gracefully**: Return meaningful error messages
4. **Document configuration**: Provide clear schema and examples
5. **Test thoroughly**: Include comprehensive tests for all functionality

### Plugin Usage

1. **Review plugin capabilities**: Understand what permissions plugins require
2. **Monitor resource usage**: Check plugin performance and resource consumption
3. **Keep plugins updated**: Regularly update to get security fixes and improvements
4. **Use official plugins**: Prefer plugins from trusted sources
5. **Test in development**: Thoroughly test plugins before production use

### Security

1. **Audit plugin code**: Review plugin source code when possible
2. **Limit capabilities**: Only grant necessary permissions
3. **Monitor logs**: Watch for suspicious plugin behavior
4. **Use resource limits**: Prevent plugins from consuming excessive resources
5. **Isolate environments**: Use separate plugin configurations for development and production

## Troubleshooting

### Common Issues

#### Plugin Won't Load

```bash
# Check plugin status
mockforge plugin status my-plugin

# Validate plugin manifest
mockforge plugin validate ./my-plugin/plugin.yaml

# Check logs for errors
mockforge logs --filter "plugin"
```

#### Runtime Errors

```bash
# Enable debug logging
RUST_LOG=mockforge_plugin_loader=debug mockforge serve

# Check resource limits
mockforge plugin stats my-plugin

# Validate configuration
mockforge plugin config validate my-plugin
```

#### Performance Issues

```bash
# Monitor plugin performance
mockforge plugin stats --watch

# Check memory usage
mockforge plugin stats --memory

# Profile plugin execution
mockforge plugin profile my-plugin
```

### Debug Mode

Enable debug mode for plugin development:

```yaml
plugins:
  debug_mode: true
  verbose_logging: true
  enable_profiling: true
```

This comprehensive plugin system enables powerful extensibility while maintaining security and performance. Plugins can significantly extend MockForge's capabilities for specialized use cases and integrations.