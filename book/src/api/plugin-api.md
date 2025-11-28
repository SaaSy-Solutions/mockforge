# Plugin API Reference

This document provides comprehensive API documentation for developing MockForge plugins.

## Overview

MockForge plugins are WebAssembly (WASM) modules that extend the core functionality of MockForge. They run in a secure sandboxed environment and provide various capabilities for authentication, response generation, data sourcing, and template functions.

## Core Types

### PluginContext

The `PluginContext` provides access to plugin runtime information and utilities.

```rust
pub struct PluginContext {
    pub plugin_id: PluginId,
    pub version: PluginVersion,
    pub config: serde_json::Value,
    pub logger: Logger,
    pub metrics: PluginMetrics,
}
```

### PluginResult

Generic result type for plugin operations.

```rust
pub type PluginResult<T> = Result<T, PluginError>;
```

### PluginState

Represents the current state of a plugin.

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum PluginState {
    Unloaded,
    Loading,
    Loaded,
    Error(String),
    Unloading,
}
```

### PluginHealth

Health status of a plugin.

```rust
#[derive(Debug, Clone)]
pub struct PluginHealth {
    pub state: PluginState,
    pub last_error: Option<String>,
    pub metrics: PluginMetrics,
    pub uptime: Duration,
}
```

### PluginMetrics

Performance and usage metrics for plugins.

```rust
#[derive(Debug, Clone, Default)]
pub struct PluginMetrics {
    pub execution_count: u64,
    pub total_execution_time: Duration,
    pub memory_usage: u64,
    pub error_count: u64,
}
```

## Plugin Identification

### PluginId

Unique identifier for a plugin.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginId(String);
```

### PluginVersion

Semantic version for plugins.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}
```

### PluginInfo

Metadata about a plugin.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: PluginId,
    pub name: String,
    pub version: PluginVersion,
    pub description: String,
    pub author: String,
    pub plugin_type: PluginType,
    pub capabilities: Vec<Capability>,
    pub dependencies: Vec<PluginDependency>,
}
```

### PluginMetadata

Extended metadata for plugin management.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub info: PluginInfo,
    pub manifest: PluginManifest,
    pub install_path: PathBuf,
    pub installed_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
}
```

## Plugin Types

### Authentication Plugins

Authentication plugins handle request authentication and authorization.

#### AuthPlugin Trait

```rust
#[async_trait]
pub trait AuthPlugin: Send + Sync {
    /// Get plugin capabilities
    fn capabilities(&self) -> Vec<Capability>;
    
    /// Initialize the plugin
    async fn initialize(&mut self, ctx: PluginContext) -> PluginResult<()>;
    
    /// Authenticate a request
    async fn authenticate(&self, request: AuthRequest) -> PluginResult<AuthResponse>;
    
    /// Validate plugin configuration
    fn validate_config(&self, config: &serde_json::Value) -> PluginResult<()>;
    
    /// Get supported authentication schemes
    fn supported_schemes(&self) -> Vec<String>;
    
    /// Cleanup resources
    async fn cleanup(&mut self) -> PluginResult<()>;
}
```

#### AuthRequest

```rust
#[derive(Debug, Clone)]
pub struct AuthRequest {
    pub headers: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    pub body: Option<String>,
    pub method: String,
    pub path: String,
    pub client_ip: Option<String>,
}
```

#### AuthResponse

```rust
#[derive(Debug, Clone)]
pub struct AuthResponse {
    pub authenticated: bool,
    pub user_id: Option<String>,
    pub roles: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub expires_at: Option<DateTime<Utc>>,
}
```

#### Example Implementation

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
        // Cleanup resources
        Ok(())
    }
}
```

### Response Plugins

Response plugins generate custom responses based on request context.

#### ResponsePlugin Trait

```rust
#[async_trait]
pub trait ResponsePlugin: Send + Sync {
    /// Get plugin capabilities
    fn capabilities(&self) -> Vec<Capability>;
    
    /// Initialize the plugin
    async fn initialize(&mut self, ctx: PluginContext) -> PluginResult<()>;
    
    /// Check if plugin can handle the request
    async fn can_handle(&self, request: ResponseRequest) -> PluginResult<bool>;
    
    /// Generate response
    async fn generate_response(&self, request: ResponseRequest) -> PluginResult<ResponseData>;
    
    /// Get plugin priority (higher = more important)
    fn priority(&self) -> u32;
    
    /// Validate plugin configuration
    fn validate_config(&self, config: &serde_json::Value) -> PluginResult<()>;
    
    /// Get supported content types
    fn supported_content_types(&self) -> Vec<String>;
    
    /// Cleanup resources
    async fn cleanup(&mut self) -> PluginResult<()>;
}
```

#### ResponseRequest

```rust
#[derive(Debug, Clone)]
pub struct ResponseRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    pub body: Option<String>,
    pub context: HashMap<String, String>,
}
```

#### ResponseData

```rust
#[derive(Debug, Clone)]
pub struct ResponseData {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub content_type: String,
}
```

#### Example Implementation

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

#### DataSourcePlugin Trait

```rust
#[async_trait]
pub trait DataSourcePlugin: Send + Sync {
    /// Get plugin capabilities
    fn capabilities(&self) -> Vec<Capability>;
    
    /// Initialize the plugin
    async fn initialize(&mut self, ctx: PluginContext) -> PluginResult<()>;
    
    /// Connect to data source
    async fn connect(&self) -> PluginResult<DataConnection>;
    
    /// Query data source
    async fn query(&self, connection: &DataConnection, query: DataQuery) -> PluginResult<DataResult>;
    
    /// Validate plugin configuration
    fn validate_config(&self, config: &serde_json::Value) -> PluginResult<()>;
    
    /// Get supported query types
    fn supported_query_types(&self) -> Vec<String>;
    
    /// Cleanup resources
    async fn cleanup(&mut self) -> PluginResult<()>;
}
```

#### DataConnection

```rust
#[derive(Debug, Clone)]
pub struct DataConnection {
    pub connection_id: String,
    pub connection_type: String,
    pub metadata: HashMap<String, String>,
}
```

#### DataQuery

```rust
#[derive(Debug, Clone)]
pub struct DataQuery {
    pub query_type: String,
    pub query_string: String,
    pub parameters: HashMap<String, String>,
    pub timeout: Option<Duration>,
}
```

#### DataResult

```rust
#[derive(Debug, Clone)]
pub struct DataResult {
    pub rows: Vec<HashMap<String, String>>,
    pub columns: Vec<String>,
    pub row_count: usize,
    pub execution_time: Duration,
}
```

#### Example Implementation

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

#### TemplatePlugin Trait

```rust
#[async_trait]
pub trait TemplatePlugin: Send + Sync {
    /// Get plugin capabilities
    fn capabilities(&self) -> Vec<Capability>;
    
    /// Initialize the plugin
    async fn initialize(&mut self, ctx: PluginContext) -> PluginResult<()>;
    
    /// Register template functions
    async fn register_functions(&self) -> PluginResult<Vec<TemplateFunction>>;
    
    /// Execute a template function
    async fn execute_function(&self, name: &str, args: Vec<String>) -> PluginResult<String>;
    
    /// Validate plugin configuration
    fn validate_config(&self, config: &serde_json::Value) -> PluginResult<()>;
    
    /// Get supported function names
    fn supported_functions(&self) -> Vec<String>;
    
    /// Cleanup resources
    async fn cleanup(&mut self) -> PluginResult<()>;
}
```

#### TemplateFunction

```rust
#[derive(Debug, Clone)]
pub struct TemplateFunction {
    pub name: String,
    pub description: String,
    pub parameters: Vec<FunctionParameter>,
    pub return_type: String,
}
```

#### FunctionParameter

```rust
#[derive(Debug, Clone)]
pub struct FunctionParameter {
    pub name: String,
    pub parameter_type: String,
    pub required: bool,
    pub default_value: Option<String>,
}
```

#### Example Implementation

```rust
use mockforge_plugin_core::{TemplatePlugin, TemplateFunction, FunctionParameter, PluginContext, PluginResult, Result};
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

## Capabilities

Plugins declare their capabilities to help MockForge understand what they can do.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    Authentication,
    ResponseGeneration,
    DataSource,
    TemplateFunction,
    Custom(String),
}
```

## Security Model

### Sandboxing

Plugins run in a WebAssembly sandbox with the following restrictions:

- **Memory Isolation**: Plugins cannot access host memory directly
- **Resource Limits**: CPU and memory usage are capped
- **No Network Access**: Plugins cannot make external requests (unless explicitly allowed)
- **File System Restrictions**: Limited file access based on capabilities

### Capability-Based Permissions

```rust
#[derive(Debug, Clone)]
pub struct PluginPermissions {
    pub network_access: bool,
    pub file_read: bool,
    pub file_write: bool,
    pub max_memory_mb: u64,
    pub max_execution_ms: u64,
}
```

### Security Validation

All plugins undergo security validation:

1. **Code Signing**: Plugins must be signed by trusted publishers
2. **Capability Verification**: Declared capabilities are validated
3. **Resource Limits**: Memory and execution time limits are enforced
4. **Sandbox Isolation**: WebAssembly sandbox prevents host access

## Testing

### Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_plugin_core::test_utils::*;

    #[tokio::test]
    async fn test_jwt_auth_plugin() {
        let mut plugin = JwtAuthPlugin::new();
        let ctx = create_test_context();
        
        plugin.initialize(ctx).await.unwrap();
        
        let request = AuthRequest {
            headers: HashMap::from([
                ("authorization".to_string(), "Bearer valid_token".to_string()),
            ]),
            query_params: HashMap::new(),
            body: None,
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            client_ip: None,
        };
        
        let response = plugin.authenticate(request).await.unwrap();
        assert!(response.authenticated);
    }
}
```

### Integration Testing

```rust
#[tokio::test]
async fn test_plugin_lifecycle() {
    let loader = PluginLoader::new();
    let plugin_path = "test_plugin.wasm";
    
    // Load plugin
    let plugin_id = loader.load_plugin(plugin_path).await.unwrap();
    
    // Test functionality
    let health = loader.get_plugin_health(&plugin_id).await.unwrap();
    assert_eq!(health.state, PluginState::Loaded);
    
    // Unload plugin
    loader.unload_plugin(&plugin_id).await.unwrap();
}
```

### Performance Testing

```rust
#[tokio::test]
async fn test_plugin_performance() {
    let mut plugin = JwtAuthPlugin::new();
    let ctx = create_test_context();
    plugin.initialize(ctx).await.unwrap();
    
    let start = Instant::now();
    for _ in 0..1000 {
        let request = create_test_auth_request();
        plugin.authenticate(request).await.unwrap();
    }
    let duration = start.elapsed();
    
    assert!(duration.as_millis() < 1000); // Should complete in under 1 second
}
```

## Best Practices

### Error Handling

- Use descriptive error messages
- Implement proper error propagation
- Log errors with appropriate levels
- Handle edge cases gracefully

### Resource Management

- Clean up resources in the `cleanup` method
- Use RAII patterns where possible
- Monitor memory usage
- Implement timeouts for long operations

### Configuration

- Validate all configuration parameters
- Provide sensible defaults
- Document configuration options
- Use type-safe configuration structures

### Performance

- Cache frequently used data
- Use async operations for I/O
- Implement connection pooling for data sources
- Profile and optimize hot paths

### Security

- Validate all inputs
- Use secure random number generators
- Implement proper authentication flows
- Follow principle of least privilege

## Plugin Manifest

Plugins must include a `plugin.yaml` manifest file:

```yaml
name: "my-plugin"
version: "1.0.0"
description: "A sample MockForge plugin"
author: "Plugin Author"
type: "auth"
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
      description: "Secret key for encryption"
  required: ["secret_key"]
```

This comprehensive API reference provides everything needed to develop MockForge plugins effectively and securely.
