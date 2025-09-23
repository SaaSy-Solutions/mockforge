# Plugin Core API Reference

This document provides detailed API reference for the MockForge Plugin Core crate.

## 📦 Crate Overview

The `mockforge-plugin-core` crate provides the fundamental types, traits, and utilities for building MockForge plugins.

```toml
[dependencies]
mockforge-plugin-core = "0.1.0"
```

## 🔧 Core Types

### PluginContext

Context information passed to plugin functions.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginContext {
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Request URI
    pub uri: String,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Request body (if any)
    pub body: Option<Value>,
    /// Custom data passed from host
    pub custom: HashMap<String, Value>,
}

impl PluginContext {
    pub fn new(
        method: String,
        uri: String,
        headers: HashMap<String, String>,
        body: Option<Value>
    ) -> Self { ... }

    pub fn with_custom<S: Into<String>>(mut self, key: S, value: Value) -> Self { ... }
    pub fn get_header(&self, name: &str) -> Option<&String> { ... }
    pub fn get_custom_value(&self, key: &str) -> Option<&Value> { ... }
}
```

### PluginCapabilities

Defines the capabilities and permissions requested by a plugin.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginCapabilities {
    pub network: NetworkCapabilities,
    pub filesystem: FilesystemCapabilities,
    pub resources: PluginResources,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkCapabilities {
    pub allow_http_outbound: bool,
    pub allowed_hosts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilesystemCapabilities {
    pub allow_read: bool,
    pub allow_write: bool,
    pub allowed_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginResources {
    pub max_memory_bytes: u64,
    pub max_cpu_time_ms: u64,
}
```

### PluginResult

Standardized result type for plugin operations.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResult<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub metadata: HashMap<String, Value>,
}

impl<T> PluginResult<T> {
    pub fn success(data: T, execution_time_ms: u64) -> Self { ... }
    pub fn failure(error: String, execution_time_ms: u64) -> Self { ... }
    pub fn with_metadata<S: Into<String>>(mut self, key: S, value: Value) -> Self { ... }
}
```

## 🔐 Authentication Plugin API

### AuthPlugin Trait

```rust
#[async_trait::async_trait]
pub trait AuthPlugin: Send + Sync {
    async fn authenticate(
        &self,
        context: &PluginContext,
        credentials: &AuthCredentials,
    ) -> PluginResult<AuthResult>;

    fn get_capabilities(&self) -> PluginCapabilities;
}
```

### AuthCredentials

Authentication credentials passed to plugins.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthCredentials {
    Basic { username: String, password: String },
    Bearer(String),
    ApiKey { key: String, header_name: Option<String> },
    Custom(HashMap<String, Value>),
}
```

### AuthResult

Result of authentication operations.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthResult {
    Authenticated {
        user_id: String,
        claims: HashMap<String, Value>,
    },
    Denied {
        reason: String,
    },
}
```

## 🏷️ Template Plugin API

### TemplatePlugin Trait

```rust
#[async_trait::async_trait]
pub trait TemplatePlugin: Send + Sync {
    async fn execute_function(
        &self,
        function_name: &str,
        args: &[Value],
        context: &PluginContext,
    ) -> PluginResult<Value>;

    fn get_functions(&self) -> Vec<TemplateFunction>;
}
```

### TemplateFunction

Metadata for template functions.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateFunction {
    pub name: String,
    pub description: String,
    pub parameters: Vec<FunctionParameter>,
    pub return_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionParameter {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub description: String,
}
```

## 📤 Response Plugin API

### ResponsePlugin Trait

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

### ResponsePluginConfig

Configuration for response plugins.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsePluginConfig {
    pub rules: Vec<ResponseRule>,
    pub default_response: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseRule {
    pub condition: String,
    pub response: Value,
}
```

## 🗄️ Data Source Plugin API

### DataSourcePlugin Trait

```rust
#[async_trait::async_trait]
pub trait DataSourcePlugin: Send + Sync {
    async fn query(
        &self,
        query: &str,
        parameters: &HashMap<String, Value>,
        context: &PluginContext,
    ) -> PluginResult<DataSet>;

    fn get_schema(&self) -> PluginResult<DataSourceSchema>;
}
```

### Data Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSet {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<DataRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
}

#[derive(Debug, Clone)]
pub struct DataRow {
    values: Vec<Value>,
}

impl DataRow {
    pub fn new(values: Vec<Value>) -> Self { ... }
    pub fn from(values: Vec<Value>) -> Self { ... }
    pub fn get(&self, index: usize) -> Option<&Value> { ... }
    pub fn len(&self) -> usize { ... }
    pub fn to_json(&self) -> Value { ... }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceSchema {
    pub tables: Vec<TableInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
}
```

## 📋 Plugin Manifest API

### PluginManifest

Plugin metadata and configuration.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub plugin: PluginInfo,
    pub capabilities: PluginCapabilities,
    pub dependencies: Vec<PluginDependency>,
    pub configuration: Option<PluginConfiguration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub version: String,
    pub name: String,
    pub description: String,
    pub types: Vec<String>,
    pub author: PluginAuthor,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAuthor {
    pub name: String,
    pub email: Option<String>,
    pub homepage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub id: String,
    pub version: String,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfiguration {
    pub schema: Value,
}
```

## 🔌 Plugin Runtime API

### PluginInstance

Represents a loaded plugin instance.

```rust
#[derive(Debug)]
pub struct PluginInstance {
    plugin_id: PluginId,
    manifest: PluginManifest,
    module: Module,
    store: Store<()>,
    state: PluginState,
    metrics: PluginMetrics,
    config: RuntimeConfig,
    created_at: DateTime<Utc>,
}
```

### PluginRuntime

Manages multiple plugin instances.

```rust
#[derive(Debug)]
pub struct PluginRuntime {
    plugins: RwLock<HashMap<PluginId, Arc<RwLock<PluginInstance>>>>,
    config: RuntimeConfig,
}

impl PluginRuntime {
    pub fn new(config: RuntimeConfig) -> Self { ... }

    pub async fn load_plugin(
        &self,
        plugin_id: PluginId,
        manifest: PluginManifest,
        wasm_bytes: &[u8],
    ) -> Result<()> { ... }

    pub async fn unload_plugin(&self, plugin_id: &PluginId) -> Result<()> { ... }

    pub async fn execute_plugin_function<T>(
        &self,
        plugin_id: &PluginId,
        function_name: &str,
        context: &PluginContext,
        input: &[u8],
    ) -> Result<PluginResult<T>>
    where
        T: DeserializeOwned { ... }
}
```

## 🛡️ Error Handling

### PluginError

Comprehensive error types for the plugin system.

```rust
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("WASM error: {0}")]
    Wasm(String),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Execution error: {0}")]
    Execution(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl PluginError {
    pub fn wasm(msg: &str) -> Self { ... }
    pub fn config(msg: &str) -> Self { ... }
    pub fn execution(msg: &str) -> Self { ... }
    pub fn invalid_input(msg: &str) -> Self { ... }
    pub fn not_found(msg: &str) -> Self { ... }
    pub fn permission_denied(msg: &str) -> Self { ... }
    pub fn internal(msg: &str) -> Self { ... }
}
```

## 🔧 Utility Functions

### Plugin ID Type

```rust
pub type PluginId = String;
```

### Export Macro

```rust
#[macro_export]
macro_rules! export_plugin {
    ($plugin_type:ty) => {
        // Implementation details for exporting plugin
    };
}
```

## 📊 Metrics and Health

### PluginMetrics

Performance and usage metrics.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginMetrics {
    pub total_executions: u64,
    pub avg_execution_time_ms: f64,
    pub last_execution_time_ms: u64,
    pub error_count: u64,
    pub last_error: Option<String>,
}
```

### PluginHealth

Health status information.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHealth {
    pub is_healthy: bool,
    pub message: String,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl PluginHealth {
    pub fn healthy(message: String) -> Self { ... }
    pub fn unhealthy(message: String, error: Option<String>) -> Self { ... }
}
```

## 🔄 Plugin States

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginState {
    Loaded,
    Executing,
    Unloaded,
    Failed(String),
    Loading,
    Initializing,
    Ready,
    Error,
    Unloading,
}

impl PluginState {
    pub fn is_ready(&self) -> bool { ... }
}
```

## 📝 Usage Examples

### Basic Plugin Implementation

```rust
use mockforge_plugin_core::*;

#[derive(Debug)]
pub struct MyPlugin;

#[async_trait::async_trait]
impl TemplatePlugin for MyPlugin {
    async fn execute_function(
        &self,
        function_name: &str,
        args: &[Value],
        context: &PluginContext,
    ) -> PluginResult<Value> {
        match function_name {
            "hello" => {
                let name = args.get(0)
                    .and_then(|v| v.as_str())
                    .unwrap_or("World");

                PluginResult::success(
                    serde_json::json!({ "message": format!("Hello, {}!", name) })
                )
            }
            _ => PluginResult::failure("Unknown function".to_string(), 0),
        }
    }

    fn get_functions(&self) -> Vec<TemplateFunction> {
        vec![TemplateFunction {
            name: "hello".to_string(),
            description: "Returns a greeting message".to_string(),
            parameters: vec![FunctionParameter {
                name: "name".to_string(),
                param_type: "string".to_string(),
                required: false,
                description: "Name to greet".to_string(),
            }],
            return_type: "object".to_string(),
        }]
    }
}

mockforge_plugin_core::export_plugin!(MyPlugin);
```
