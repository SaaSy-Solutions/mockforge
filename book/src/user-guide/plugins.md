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

Plugin system settings (registry URL, runtime limits, directory)
are YAML-only — configure under `plugins.*` in `mockforge.yaml`.
The token for publishing to the registry is the only env var:

```bash
export MOCKFORGE_REGISTRY_TOKEN=mfreg_...

# Plugin-specific settings (per-plugin auth/data, NOT MockForge core)
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

## Hook Points Reference

MockForge plugins implement one or more of seven traits, each hooked into a
specific point in the request / response lifecycle. A single plugin can
implement several traits to extend multiple stages.

| Trait | When it runs | Use cases |
|---|---|---|
| **`TemplatePlugin`** | During response templating | Custom `{{my.fn arg}}` template functions, inline crypto / encoding helpers |
| **`AuthPlugin`** | Before route matching | Custom JWT/OAuth validators, header-based auth schemes, request signing verification |
| **`ResponsePlugin`** | After route matching, before write | Generate the response body from scratch (e.g. AI-driven, computed) |
| **`ResponseModifierPlugin`** | After response generation | Post-process bodies (compression, redaction, watermarking, schema masking) |
| **`DataSourcePlugin`** | When a route requests external data | CSV / database / external-API connectors backing fixtures |
| **`BackendGeneratorPlugin`** | At spec import time | Generate full mock backends from non-OpenAPI specs (gRPC reflect, custom DSLs) |
| **`ClientGeneratorPlugin`** | At spec import time | Emit client SDKs from a spec for testing |

Each trait gets a `PluginContext` with:
- The current request (method, path, headers, body)
- A handle to the in-memory storage (`storage.key_value` capability)
- A logger (`tracing` macros, scoped to the plugin name)
- Random / time / template helpers from the host

See [`crates/mockforge-plugin-core/src/`](https://github.com/SaaSy-Solutions/mockforge/tree/main/crates/mockforge-plugin-core/src)
for the canonical trait definitions.

## Publishing to the Registry

The plugin registry at [registry.mockforge.dev](https://registry.mockforge.dev)
hosts community plugins, signed and checksummed, with semver-pinnable
versions. Publishing is a four-step flow.

### 1. Generate publisher keys

```bash
# One-time setup: create a publisher key pair stored in your config dir
mockforge plugin keygen --name "your-name"

# Inspect: prints the public key + key ID
mockforge plugin keygen --show
```

Your private key signs every release; the public key gets registered with
the registry on first publish. Keep the private key file readable only by
your user account.

### 2. Authenticate to the registry

```bash
# Create an account at https://registry.mockforge.dev and copy your token
export MOCKFORGE_REGISTRY_TOKEN=mfreg_...
```

You can also store the token in `~/.config/mockforge/registry-token` so
it's not in your shell history.

### 3. Prepare the manifest

```yaml
# plugin.yaml in your plugin's repo root
name: "my-cool-plugin"
version: "1.0.0"          # semver, monotonically increasing per release
description: "A short, scannable summary"
author: "your-name"
license: "MIT OR Apache-2.0"
homepage: "https://github.com/you/my-cool-plugin"
repository: "https://github.com/you/my-cool-plugin"

capabilities:
  - "network.http.client"
  - "storage.key_value"

runtime:
  memory_limit_mb: 64
  cpu_limit_percent: 5
  execution_timeout_ms: 1000

# Versions of MockForge this plugin targets
mockforge:
  min_version: "0.3.125"
  max_version: "0.4.0"

# Files to include in the published bundle (relative to repo root)
include:
  - "target/wasm32-unknown-unknown/release/my_cool_plugin.wasm"
  - "plugin.yaml"
  - "README.md"
  - "LICENSE-*"
```

Validate before pushing:

```bash
mockforge plugin validate ./plugin.yaml
```

### 4. Publish

```bash
# Build for WASM target, then publish
cargo build --target wasm32-unknown-unknown --release
mockforge plugin publish ./plugin.yaml
```

Under the hood:

1. Bundle is zipped from the `include:` list
2. SHA-256 checksum is computed
3. Bundle + checksum are signed with your publisher key
4. Signed package is uploaded with the bearer
   token from `MOCKFORGE_REGISTRY_TOKEN`
5. Registry validates the signature against your registered public key

### Yanking a release

If a release has a critical bug, yank it (existing pins keep working but
the version disappears from search):

```bash
mockforge plugin yank my-cool-plugin@1.0.0 --reason "panics on empty input"
```

Yanks aren't deletions — they're advisory. To fully remove a release for
GDPR / legal reasons, contact registry operators.

## Verification & Trust

When you `mockforge plugin install <name>@<version>`, MockForge:

1. Resolves the version constraint against the registry
2. Downloads the signed bundle
3. Verifies the SHA-256 checksum matches the registry's stored value
4. Verifies the package signature against the publisher's registered
   public key
5. Caches the bundle locally for reproducibility

If any verification step fails, install aborts with a clear error and
nothing gets unpacked.

### Lockfile

A `mockforge-plugins.lock` file at your project root pins exact versions
and checksums:

```toml
# Generated by `mockforge plugin install` — commit this file.
[[plugin]]
name = "auth-jwt"
version = "1.2.0"
source = "registry"
checksum = "sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
publisher = "did:key:z6Mki..."
```

Every subsequent `mockforge plugin install` (no version specified) resolves
to the lockfile, so CI and dev get byte-identical bundles. To intentionally
upgrade, run `mockforge plugin update <name>` which rewrites the lockfile.

### Air-gapped installs

Mirror the registry to your own filesystem / object store:

```bash
# Pull the plugins your project uses into a local cache
mockforge plugin mirror --output ./plugin-mirror

# On the air-gapped box, point installs at the mirror
export MOCKFORGE_PLUGIN_REGISTRY_URL=file:///path/to/plugin-mirror
mockforge plugin install my-cool-plugin@1.0.0
```

The verification flow is identical against a mirror — checksums + signatures
travel with the bundle, so trust doesn't depend on the registry being online.

### Verifying a third-party plugin manually

```bash
# Inspect a bundle without installing it
mockforge plugin inspect ./suspicious-plugin.tar.gz

# Output includes:
#   - Manifest contents
#   - Declared capabilities (with red flags highlighted)
#   - SHA-256 checksum
#   - Signer public key + first-seen date
#   - List of WASM imports (host functions called)
```

Always run `inspect` on plugins from unfamiliar publishers before installing.

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