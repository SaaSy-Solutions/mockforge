# Plugin Development Guide

This guide provides comprehensive instructions for developing MockForge plugins, from initial setup to publishing.

## Table of Contents

- [Getting Started](#getting-started)
- [Plugin Types](#plugin-types)
- [Development Environment](#development-environment)
- [Building and Testing](#building-and-testing)
- [Advanced Patterns](#advanced-patterns)
- [Performance Optimization](#performance-optimization)
- [Security Considerations](#security-considerations)
- [Publishing](#publishing)
- [Troubleshooting](#troubleshooting)

## Getting Started

### Prerequisites

- Rust 1.70+ with WebAssembly target
- MockForge Plugin SDK
- Basic understanding of Rust and WebAssembly

### Installation

```bash
# Install WebAssembly target
rustup target add wasm32-unknown-unknown

# Install MockForge Plugin CLI
cargo install mockforge-plugin-cli
```

### Create Your First Plugin

```bash
# Create a new plugin project
mockforge-plugin-cli new my-auth-plugin --type auth

cd my-auth-plugin
```

This creates a basic plugin structure:

```
my-plugin/
├── Cargo.toml
├── plugin.yaml
├── src/
│   ├── lib.rs
│   ├── auth.rs
│   ├── response.rs
│   └── utils.rs
├── tests/
│   ├── integration_tests.rs
│   └── unit_tests.rs
├── examples/
│   └── usage.rs
└── README.md
```

## Plugin Types

### Authentication Plugins

Authentication plugins handle request authentication and authorization.

**Use Cases:**

- JWT token validation
- OAuth2 flows
- API key authentication
- Custom authentication schemes

**Example Implementation:**

```rust
use mockforge_plugin_core::{AuthPlugin, AuthRequest, AuthResponse, PluginContext, PluginResult, Result};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde_json::Value;

pub struct JwtAuthPlugin {
    secret: String,
    algorithm: Algorithm,
}

#[async_trait]
impl AuthPlugin for JwtAuthPlugin {
    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Authentication]
    }

    async fn initialize(&mut self, ctx: PluginContext) -> PluginResult<()> {
        self.secret = ctx.config["secret"]
            .as_str()
            .ok_or("Missing secret in config")?
            .to_string();
        Ok(())
    }

    async fn authenticate(&self, request: AuthRequest) -> PluginResult<AuthResponse> {
        let token = request.headers
            .get("authorization")
            .and_then(|h| h.strip_prefix("Bearer "))
            .ok_or("Missing or invalid authorization header")?;

        let token_data = decode::<Value>(
            token,
            &DecodingKey::from_secret(self.secret.as_ref()),
            &Validation::new(self.algorithm),
        ).map_err(|e| format!("Invalid token: {}", e))?;

        Ok(AuthResponse {
            authenticated: true,
            user_id: token_data.claims["sub"].as_str().map(|s| s.to_string()),
            roles: token_data.claims["roles"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
                .unwrap_or_default(),
            metadata: HashMap::new(),
            expires_at: token_data.claims["exp"]
                .as_i64()
                .map(|exp| DateTime::from_timestamp(exp, 0).unwrap_or_default()),
        })
    }

    fn validate_config(&self, config: &Value) -> PluginResult<()> {
        if !config["secret"].is_string() {
            return Err("Secret must be a string".into());
        }
        Ok(())
    }

    fn supported_schemes(&self) -> Vec<String> {
        vec!["Bearer".to_string()]
    }

    async fn cleanup(&mut self) -> PluginResult<()> {
        Ok(())
    }
}
```

### Response Plugins

Response plugins generate custom responses based on request context.

**Use Cases:**

- GraphQL response generation
- Custom data formatting
- Dynamic content generation
- Protocol-specific responses

**Example Implementation:**

```rust
use mockforge_plugin_core::{ResponsePlugin, ResponseRequest, ResponseData, PluginContext, PluginResult, Result};
use serde_json::Value;

pub struct GraphQLResponsePlugin {
    schema: String,
}

#[async_trait]
impl ResponsePlugin for GraphQLResponsePlugin {
    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::ResponseGeneration]
    }

    async fn initialize(&mut self, ctx: PluginContext) -> PluginResult<()> {
        self.schema = ctx.config["schema"]
            .as_str()
            .ok_or("Missing schema in config")?
            .to_string();
        Ok(())
    }

    async fn can_handle(&self, request: ResponseRequest) -> PluginResult<bool> {
        Ok(request.headers
            .get("content-type")
            .map(|ct| ct.contains("application/graphql"))
            .unwrap_or(false))
    }

    async fn generate_response(&self, request: ResponseRequest) -> PluginResult<ResponseData> {
        let query = request.body.ok_or("Missing GraphQL query")?;
        
        // Simple mock response based on query
        let response_body = match query.contains("user") {
            true => json!({
                "data": {
                    "user": {
                        "id": "123",
                        "name": "John Doe",
                        "email": "john@example.com"
                    }
                }
            }),
            false => json!({
                "data": null,
                "errors": [{"message": "Unknown query"}]
            })
        };

        Ok(ResponseData {
            status: 200,
            headers: HashMap::from([
                ("content-type".to_string(), "application/json".to_string()),
            ]),
            body: response_body.to_string(),
            content_type: "application/json".to_string(),
        })
    }

    fn priority(&self) -> u32 {
        100
    }

    fn validate_config(&self, config: &Value) -> PluginResult<()> {
        if !config["schema"].is_string() {
            return Err("Schema must be a string".into());
        }
        Ok(())
    }

    fn supported_content_types(&self) -> Vec<String> {
        vec!["application/graphql".to_string(), "application/json".to_string()]
    }

    async fn cleanup(&mut self) -> PluginResult<()> {
        Ok(())
    }
}
```

### Data Source Plugins

Data source plugins connect to external data sources.

**Use Cases:**

- Database connections
- CSV file readers
- REST API clients
- File system access

**Example Implementation:**

```rust
use mockforge_plugin_core::{DataSourcePlugin, DataConnection, DataQuery, DataResult, PluginContext, PluginResult, Result};
use csv::ReaderBuilder;
use std::io::Cursor;

pub struct CsvDataSourcePlugin {
    file_path: String,
}

#[async_trait]
impl DataSourcePlugin for CsvDataSourcePlugin {
    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::DataSource]
    }

    async fn initialize(&mut self, ctx: PluginContext) -> PluginResult<()> {
        self.file_path = ctx.config["file_path"]
            .as_str()
            .ok_or("Missing file_path in config")?
            .to_string();
        Ok(())
    }

    async fn connect(&self) -> PluginResult<DataConnection> {
        Ok(DataConnection {
            connection_id: "csv_connection".to_string(),
            connection_type: "csv".to_string(),
            metadata: HashMap::from([
                ("file_path".to_string(), self.file_path.clone()),
            ]),
        })
    }

    async fn query(&self, _connection: &DataConnection, query: DataQuery) -> PluginResult<DataResult> {
        let file_content = std::fs::read_to_string(&self.file_path)
            .map_err(|e| format!("Failed to read CSV file: {}", e))?;

        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(Cursor::new(file_content));

        let headers = reader.headers()
            .map_err(|e| format!("Failed to read headers: {}", e))?
            .iter()
            .map(|h| h.to_string())
            .collect::<Vec<_>>();

        let mut rows = Vec::new();
        for result in reader.records() {
            let record = result.map_err(|e| format!("Failed to read record: {}", e))?;
            let mut row = HashMap::new();
            for (i, field) in record.iter().enumerate() {
                if i < headers.len() {
                    row.insert(headers[i].clone(), field.to_string());
                }
            }
            rows.push(row);
        }

        Ok(DataResult {
            rows,
            columns: headers,
            row_count: rows.len(),
            execution_time: Duration::from_millis(1),
        })
    }

    fn validate_config(&self, config: &Value) -> PluginResult<()> {
        if !config["file_path"].is_string() {
            return Err("file_path must be a string".into());
        }
        Ok(())
    }

    fn supported_query_types(&self) -> Vec<String> {
        vec!["select".to_string()]
    }

    async fn cleanup(&mut self) -> PluginResult<()> {
        Ok(())
    }
}
```

### Template Plugins

Template plugins extend the templating system with custom functions.

**Use Cases:**

- Custom data generators
- Cryptographic functions
- Business logic helpers
- External service integrations

**Example Implementation:**

```rust
use mockforge_plugin_core::{TemplatePlugin, TemplateFunction, PluginContext, PluginResult, Result};
use sha2::{Sha256, Digest};
use base64::encode;

pub struct CryptoPlugin {
    secret_key: String,
}

#[async_trait]
impl TemplatePlugin for CryptoPlugin {
    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::TemplateFunction]
    }

    async fn initialize(&mut self, ctx: PluginContext) -> PluginResult<()> {
        self.secret_key = ctx.config["secret_key"]
            .as_str()
            .ok_or("Missing secret_key in config")?
            .to_string();
        Ok(())
    }

    async fn register_functions(&self) -> PluginResult<Vec<TemplateFunction>> {
        Ok(vec![
            TemplateFunction {
                name: "hash".to_string(),
                description: "Generate SHA256 hash".to_string(),
                parameters: vec![
                    FunctionParameter {
                        name: "input".to_string(),
                        parameter_type: "string".to_string(),
                        required: true,
                        default_value: None,
                    }
                ],
                return_type: "string".to_string(),
            },
            TemplateFunction {
                name: "encrypt".to_string(),
                description: "Encrypt string with secret key".to_string(),
                parameters: vec![
                    FunctionParameter {
                        name: "data".to_string(),
                        parameter_type: "string".to_string(),
                        required: true,
                        default_value: None,
                    }
                ],
                return_type: "string".to_string(),
            },
        ])
    }

    async fn execute_function(&self, name: &str, args: Vec<String>) -> PluginResult<String> {
        match name {
            "hash" => {
                let input = args.get(0).ok_or("Missing input argument")?;
                let mut hasher = Sha256::new();
                hasher.update(input.as_bytes());
                let result = hasher.finalize();
                Ok(format!("{:x}", result))
            },
            "encrypt" => {
                let data = args.get(0).ok_or("Missing data argument")?;
                // Simple XOR encryption for demo
                let encrypted: String = data.chars()
                    .zip(self.secret_key.chars().cycle())
                    .map(|(c, k)| (c as u8 ^ k as u8) as char)
                    .collect();
                Ok(encode(encrypted))
            },
            _ => Err(format!("Unknown function: {}", name).into()),
        }
    }

    fn validate_config(&self, config: &Value) -> PluginResult<()> {
        if !config["secret_key"].is_string() {
            return Err("secret_key must be a string".into());
        }
        Ok(())
    }

    fn supported_functions(&self) -> Vec<String> {
        vec!["hash".to_string(), "encrypt".to_string()]
    }

    async fn cleanup(&mut self) -> PluginResult<()> {
        Ok(())
    }
}
```

## Development Environment

### Project Structure

```text
my-plugin/
├── Cargo.toml
├── plugin.yaml
├── src/
│   ├── lib.rs
│   ├── auth.rs
│   ├── response.rs
│   └── utils.rs
├── tests/
│   ├── integration_tests.rs
│   └── unit_tests.rs
├── examples/
│   └── usage.rs
└── README.md
```

### Cargo.toml Configuration

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
mockforge-plugin-core = "0.1.6"
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }

# Plugin-specific dependencies
jsonwebtoken = "9.0"  # For JWT auth plugin
csv = "1.0"           # For CSV data source plugin
sha2 = "0.10"         # For crypto template plugin
base64 = "0.21"       # For crypto template plugin

[dev-dependencies]
mockforge-plugin-core = { version = "0.1.6", features = ["test-utils"] }
tokio-test = "0.4"
```

### Plugin Manifest (plugin.yaml)

```yaml
name: "my-plugin"
version: "0.1.0"
description: "A sample MockForge plugin"
author: "Plugin Author"
type: "auth"  # auth, response, datasource, template
capabilities:
  - "authentication"
dependencies:
  - name: "mockforge-plugin-core"
    version: ">=0.1.0"
runtime:
  wasm_version: "1.0"
  memory_limit: "64MB"
  execution_timeout: "5s"
permissions:
  network_access: false
  file_read: true
  file_write: false
config_schema:
  type: "object"
  properties:
    secret_key:
      type: "string"
      description: "Secret key for authentication"
    algorithm:
      type: "string"
      enum: ["HS256", "HS384", "HS512"]
      default: "HS256"
  required: ["secret_key"]
```

## Building and Testing

### Build the Plugin

```bash
cargo build --release --target wasm32-unknown-unknown
```

The compiled plugin will be at:

```text
target/wasm32-unknown-unknown/release/my_plugin.wasm
```

### Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_plugin_core::test_utils::*;

    #[tokio::test]
    async fn test_plugin_initialization() {
        let mut plugin = MyPlugin::new();
        let ctx = create_test_context();
        
        let result = plugin.initialize(ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_plugin_functionality() {
        let mut plugin = MyPlugin::new();
        let ctx = create_test_context();
        plugin.initialize(ctx).await.unwrap();
        
        // Test specific functionality
        let result = plugin.some_method().await;
        assert!(result.is_ok());
    }
}
```

### Integration Testing

```rust
#[tokio::test]
async fn test_plugin_integration() {
    let loader = PluginLoader::new();
    let plugin_path = "target/wasm32-unknown-unknown/release/my_plugin.wasm";
    
    // Load plugin
    let plugin_id = loader.load_plugin(plugin_path).await.unwrap();
    
    // Test functionality
    let health = loader.get_plugin_health(&plugin_id).await.unwrap();
    assert_eq!(health.state, PluginState::Loaded);
    
    // Unload plugin
    loader.unload_plugin(&plugin_id).await.unwrap();
}
```

### Test Plugin Locally

```bash
# Test plugin functionality
mockforge-plugin-cli test my-plugin.wasm --action authenticate --input '{"token": "test"}'

# Benchmark plugin performance
mockforge-plugin-cli bench my-plugin.wasm --iterations 1000

# Validate plugin manifest
mockforge-plugin-cli validate my-plugin.wasm
```

## Advanced Patterns

### Plugin Composition

Combine multiple plugins for complex functionality:

```rust
pub struct CompositeAuthPlugin {
    jwt_plugin: JwtAuthPlugin,
    api_key_plugin: ApiKeyAuthPlugin,
}

#[async_trait]
impl AuthPlugin for CompositeAuthPlugin {
    async fn authenticate(&self, request: AuthRequest) -> PluginResult<AuthResponse> {
        // Try JWT first
        if let Ok(response) = self.jwt_plugin.authenticate(request.clone()).await {
            if response.authenticated {
                return Ok(response);
            }
        }
        
        // Fall back to API key
        self.api_key_plugin.authenticate(request).await
    }
}
```

### Caching Strategies

Implement caching for better performance:

```rust
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct CachedDataSourcePlugin {
    cache: HashMap<String, (DataResult, Instant)>,
    cache_ttl: Duration,
}

impl CachedDataSourcePlugin {
    async fn get_cached_result(&self, query: &str) -> Option<DataResult> {
        if let Some((result, timestamp)) = self.cache.get(query) {
            if timestamp.elapsed() < self.cache_ttl {
                return Some(result.clone());
            }
        }
        None
    }
    
    fn cache_result(&mut self, query: String, result: DataResult) {
        self.cache.insert(query, (result, Instant::now()));
    }
}
```

### Resource Management

Implement proper resource cleanup:

```rust
pub struct DatabasePlugin {
    connection_pool: Option<ConnectionPool>,
}

impl Drop for DatabasePlugin {
    fn drop(&mut self) {
        if let Some(pool) = self.connection_pool.take() {
            // Cleanup connection pool
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    pool.close().await;
                });
            });
        }
    }
}
```

### Error Handling Patterns

Implement comprehensive error handling:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Authentication failed: {0}")]
    AuthError(String),
    
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

impl From<PluginError> for mockforge_plugin_core::PluginError {
    fn from(err: PluginError) -> Self {
        mockforge_plugin_core::PluginError::Custom(err.to_string())
    }
}
```

## Performance Optimization

### Memory Management

Optimize memory usage in WebAssembly:

```rust
// Use Box to move large data to heap
let large_data = Box::new(vec![0u8; 1024 * 1024]);

// Use String::with_capacity for known sizes
let mut buffer = String::with_capacity(1024);

// Use Vec::with_capacity for collections
let mut items = Vec::with_capacity(100);
```

### Async Optimization

Use async patterns efficiently:

```rust
// Use join! for parallel operations
use futures::future::join;

let (result1, result2) = join!(
    async_operation_1(),
    async_operation_2()
).await;

// Use select! for timeout handling
use tokio::select;
use tokio::time::{timeout, Duration};

let result = select! {
    result = async_operation() => result,
    _ = timeout(Duration::from_secs(5), async {}) => Err("Timeout".into()),
};
```

### CPU Optimization

Optimize CPU-intensive operations:

```rust
// Use rayon for parallel processing
use rayon::prelude::*;

let results: Vec<_> = data
    .par_iter()
    .map(|item| expensive_operation(item))
    .collect();

// Cache expensive computations
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static! {
    static ref COMPUTATION_CACHE: Mutex<HashMap<String, f64>> = Mutex::new(HashMap::new());
}
```

## Security Considerations

### Input Validation

Always validate inputs:

```rust
fn validate_input(input: &str) -> PluginResult<()> {
    if input.is_empty() {
        return Err("Input cannot be empty".into());
    }
    
    if input.len() > 1000 {
        return Err("Input too long".into());
    }
    
    // Check for dangerous patterns
    if input.contains("..") || input.contains("//") {
        return Err("Invalid input pattern".into());
    }
    
    Ok(())
}
```

### Secure Random Generation

Use secure random number generators:

```rust
use rand::rngs::OsRng;
use rand::RngCore;

fn generate_secure_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}
```

### Capability Restrictions

Respect capability restrictions:

```rust
fn check_network_permission(&self) -> PluginResult<()> {
    if !self.permissions.network_access {
        return Err("Network access not permitted".into());
    }
    Ok(())
}

fn check_file_permission(&self, path: &str) -> PluginResult<()> {
    if !self.permissions.file_read {
        return Err("File read not permitted".into());
    }
    
    // Additional path validation
    if path.contains("..") {
        return Err("Path traversal not allowed".into());
    }
    
    Ok(())
}
```

## Publishing

### Package Plugin

```bash
# Package plugin for distribution
mockforge-plugin-cli package my-plugin.wasm --output my-plugin-1.0.0.zip

# Sign plugin (if you have a certificate)
mockforge-plugin-cli sign my-plugin.wasm --certificate my-cert.pem --private-key my-key.pem
```

### Publish to Registry

```bash
# Publish to MockForge plugin registry
mockforge-plugin-cli publish my-plugin-1.0.0.zip --registry https://plugins.mockforge.dev

# Update existing plugin
mockforge-plugin-cli publish my-plugin-1.1.0.zip --update
```

### Version Management

Follow semantic versioning:

- **MAJOR**: Breaking changes to API
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes, backward compatible

Update version in both `Cargo.toml` and `plugin.yaml`:

```toml
# Cargo.toml
[package]
version = "1.1.0"
```

```yaml
# plugin.yaml
version: "1.1.0"
```

## Troubleshooting

### Common Issues

**Plugin fails to load:**
- Check WebAssembly target is installed
- Verify plugin manifest syntax
- Ensure all dependencies are available

**Performance issues:**
- Profile memory usage
- Check for memory leaks
- Optimize hot paths
- Use async operations for I/O

**Authentication failures:**
- Validate token format
- Check secret key configuration
- Verify algorithm compatibility
- Test with known good tokens

### Debugging

Enable debug logging:

```rust
use log::{debug, error, info, warn};

async fn debug_operation(&self) -> PluginResult<()> {
    debug!("Starting operation");
    
    match self.perform_operation().await {
        Ok(result) => {
            info!("Operation completed successfully");
            Ok(result)
        },
        Err(e) => {
            error!("Operation failed: {}", e);
            Err(e)
        }
    }
}
```

### Testing in Isolation

Test plugins independently:

```bash
# Test with mock data
mockforge-plugin-cli test my-plugin.wasm --mock-data test-data.json

# Test with different configurations
mockforge-plugin-cli test my-plugin.wasm --config test-config.yaml

# Test error conditions
mockforge-plugin-cli test my-plugin.wasm --error-scenarios
```

### Performance Profiling

Profile plugin performance:

```bash
# Benchmark with different loads
mockforge-plugin-cli bench my-plugin.wasm --load-levels 1,10,100,1000

# Memory profiling
mockforge-plugin-cli profile my-plugin.wasm --memory

# CPU profiling
mockforge-plugin-cli profile my-plugin.wasm --cpu
```

## Examples

### Complete Auth Plugin

[Link to examples directory]

## Contributing

[Contribution guidelines]

## License

MIT

This comprehensive development guide provides everything needed to create, test, optimize, and publish MockForge plugins effectively.
