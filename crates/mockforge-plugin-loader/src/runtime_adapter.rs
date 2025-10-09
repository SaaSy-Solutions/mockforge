//! Runtime adapter for supporting multiple plugin runtimes
//!
//! This module provides an abstraction layer that allows MockForge to load and execute
//! plugins written in different languages and compiled with different WASM runtimes.
//!
//! Supported runtimes:
//! - Rust (native, via wasmtime)
//! - TinyGo (Go compiled to WASM)
//! - AssemblyScript (TypeScript-like, compiled to WASM)
//! - Remote (HTTP/gRPC-based plugins in any language)

use crate::*;
use async_trait::async_trait;
use mockforge_plugin_core::{
    AuthCredentials, AuthResult, DataQuery, DataResult, PluginContext, PluginError, PluginId,
    ResponseData, ResponseRequest, ResolutionContext,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Enum representing different plugin runtime types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeType {
    /// Native Rust plugin compiled to WASM
    Rust,
    /// TinyGo compiled plugin
    TinyGo,
    /// AssemblyScript compiled plugin
    AssemblyScript,
    /// Remote plugin accessed via HTTP/gRPC
    Remote(RemoteRuntimeConfig),
}

/// Configuration for remote plugin runtime
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteRuntimeConfig {
    /// Protocol to use (http or grpc)
    pub protocol: RemoteProtocol,
    /// Endpoint URL
    pub endpoint: String,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    /// Maximum number of retries
    pub max_retries: u32,
    /// Authentication configuration
    pub auth: Option<RemoteAuthConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteProtocol {
    Http,
    Grpc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteAuthConfig {
    /// Authentication type (bearer, api_key, etc.)
    pub auth_type: String,
    /// Authentication value
    pub value: String,
}

/// Trait that all runtime adapters must implement
///
/// This allows different WASM runtimes and remote plugins to be used interchangeably
#[async_trait]
pub trait RuntimeAdapter: Send + Sync {
    /// Get the runtime type
    fn runtime_type(&self) -> RuntimeType;

    /// Initialize the runtime (called once during plugin load)
    async fn initialize(&mut self) -> Result<(), PluginError>;

    /// Call authentication plugin
    async fn call_auth(
        &self,
        context: &PluginContext,
        credentials: &AuthCredentials,
    ) -> Result<AuthResult, PluginError>;

    /// Call template function plugin
    async fn call_template_function(
        &self,
        function_name: &str,
        args: &[serde_json::Value],
        context: &ResolutionContext,
    ) -> Result<serde_json::Value, PluginError>;

    /// Call response generator plugin
    async fn call_response_generator(
        &self,
        context: &PluginContext,
        request: &ResponseRequest,
    ) -> Result<ResponseData, PluginError>;

    /// Call data source query plugin
    async fn call_datasource_query(
        &self,
        query: &DataQuery,
        context: &PluginContext,
    ) -> Result<DataResult, PluginError>;

    /// Health check - returns true if the plugin is healthy
    async fn health_check(&self) -> Result<bool, PluginError>;

    /// Cleanup resources (called during plugin unload)
    async fn cleanup(&mut self) -> Result<(), PluginError>;

    /// Get runtime-specific metrics
    fn get_metrics(&self) -> HashMap<String, serde_json::Value> {
        HashMap::new()
    }
}

/// Detect runtime type from WASM binary
pub fn detect_runtime_type(wasm_bytes: &[u8]) -> Result<RuntimeType, PluginError> {
    // Parse WASM module to detect runtime type
    // This is a simplified version - real implementation would parse WASM sections

    // Check for TinyGo signatures
    if has_tinygo_signature(wasm_bytes) {
        return Ok(RuntimeType::TinyGo);
    }

    // Check for AssemblyScript signatures
    if has_assemblyscript_signature(wasm_bytes) {
        return Ok(RuntimeType::AssemblyScript);
    }

    // Default to Rust
    Ok(RuntimeType::Rust)
}

/// Check if WASM binary has TinyGo signature
fn has_tinygo_signature(wasm_bytes: &[u8]) -> bool {
    // Look for TinyGo-specific exports or custom sections
    // TinyGo typically exports functions like "resume" and "getsp"
    // This is a placeholder - real implementation would use wasmparser

    // Simple heuristic: check for "tinygo" in custom sections
    String::from_utf8_lossy(wasm_bytes).contains("tinygo")
}

/// Check if WASM binary has AssemblyScript signature
fn has_assemblyscript_signature(wasm_bytes: &[u8]) -> bool {
    // Look for AssemblyScript-specific exports or custom sections
    // AS typically has "__new", "__pin", "__unpin" exports

    // Simple heuristic: check for "assemblyscript" in custom sections
    String::from_utf8_lossy(wasm_bytes).contains("assemblyscript")
}

/// Factory for creating runtime adapters
pub struct RuntimeAdapterFactory;

impl RuntimeAdapterFactory {
    /// Create a runtime adapter for the given runtime type
    pub fn create(
        runtime_type: RuntimeType,
        plugin_id: PluginId,
        wasm_bytes: Vec<u8>,
    ) -> Result<Box<dyn RuntimeAdapter>, PluginError> {
        match runtime_type {
            RuntimeType::Rust => {
                Ok(Box::new(RustAdapter::new(plugin_id, wasm_bytes)?))
            }
            RuntimeType::TinyGo => {
                Ok(Box::new(TinyGoAdapter::new(plugin_id, wasm_bytes)?))
            }
            RuntimeType::AssemblyScript => {
                Ok(Box::new(AssemblyScriptAdapter::new(plugin_id, wasm_bytes)?))
            }
            RuntimeType::Remote(config) => {
                Ok(Box::new(RemoteAdapter::new(plugin_id, config)?))
            }
        }
    }
}

// ============================================================================
// Rust Runtime Adapter (Existing Implementation)
// ============================================================================

pub struct RustAdapter {
    plugin_id: PluginId,
    _wasm_bytes: Vec<u8>,
    // TODO: Add wasmtime instance
}

impl RustAdapter {
    pub fn new(plugin_id: PluginId, wasm_bytes: Vec<u8>) -> Result<Self, PluginError> {
        Ok(Self {
            plugin_id,
            _wasm_bytes: wasm_bytes,
        })
    }
}

#[async_trait]
impl RuntimeAdapter for RustAdapter {
    fn runtime_type(&self) -> RuntimeType {
        RuntimeType::Rust
    }

    async fn initialize(&mut self) -> Result<(), PluginError> {
        // Initialize wasmtime instance with the Rust WASM module
        // This is the existing implementation
        Ok(())
    }

    async fn call_auth(
        &self,
        _context: &PluginContext,
        _credentials: &AuthCredentials,
    ) -> Result<AuthResult, PluginError> {
        // Call Rust WASM plugin's auth function
        // This is the existing implementation
        todo!("Integrate with existing Rust plugin runtime")
    }

    async fn call_template_function(
        &self,
        _function_name: &str,
        _args: &[serde_json::Value],
        _context: &ResolutionContext,
    ) -> Result<serde_json::Value, PluginError> {
        todo!("Integrate with existing Rust plugin runtime")
    }

    async fn call_response_generator(
        &self,
        _context: &PluginContext,
        _request: &ResponseRequest,
    ) -> Result<ResponseData, PluginError> {
        todo!("Integrate with existing Rust plugin runtime")
    }

    async fn call_datasource_query(
        &self,
        _query: &DataQuery,
        _context: &PluginContext,
    ) -> Result<DataResult, PluginError> {
        todo!("Integrate with existing Rust plugin runtime")
    }

    async fn health_check(&self) -> Result<bool, PluginError> {
        Ok(true)
    }

    async fn cleanup(&mut self) -> Result<(), PluginError> {
        Ok(())
    }
}

// ============================================================================
// TinyGo Runtime Adapter
// ============================================================================

pub struct TinyGoAdapter {
    plugin_id: PluginId,
    _wasm_bytes: Vec<u8>,
    // TODO: Add wasmtime instance configured for TinyGo
}

impl TinyGoAdapter {
    pub fn new(plugin_id: PluginId, wasm_bytes: Vec<u8>) -> Result<Self, PluginError> {
        Ok(Self {
            plugin_id,
            _wasm_bytes: wasm_bytes,
        })
    }
}

#[async_trait]
impl RuntimeAdapter for TinyGoAdapter {
    fn runtime_type(&self) -> RuntimeType {
        RuntimeType::TinyGo
    }

    async fn initialize(&mut self) -> Result<(), PluginError> {
        // Initialize wasmtime instance with TinyGo-specific configuration
        // TinyGo requires special memory management and import handling
        tracing::info!("Initializing TinyGo plugin: {}", self.plugin_id);
        Ok(())
    }

    async fn call_auth(
        &self,
        _context: &PluginContext,
        _credentials: &AuthCredentials,
    ) -> Result<AuthResult, PluginError> {
        // Call TinyGo WASM plugin's auth function
        // Needs to handle Go's calling conventions and memory layout
        todo!("Implement TinyGo auth calling")
    }

    async fn call_template_function(
        &self,
        _function_name: &str,
        _args: &[serde_json::Value],
        _context: &ResolutionContext,
    ) -> Result<serde_json::Value, PluginError> {
        todo!("Implement TinyGo template function calling")
    }

    async fn call_response_generator(
        &self,
        _context: &PluginContext,
        _request: &ResponseRequest,
    ) -> Result<ResponseData, PluginError> {
        todo!("Implement TinyGo response generator calling")
    }

    async fn call_datasource_query(
        &self,
        _query: &DataQuery,
        _context: &PluginContext,
    ) -> Result<DataResult, PluginError> {
        todo!("Implement TinyGo datasource query calling")
    }

    async fn health_check(&self) -> Result<bool, PluginError> {
        Ok(true)
    }

    async fn cleanup(&mut self) -> Result<(), PluginError> {
        Ok(())
    }
}

// ============================================================================
// AssemblyScript Runtime Adapter
// ============================================================================

pub struct AssemblyScriptAdapter {
    plugin_id: PluginId,
    _wasm_bytes: Vec<u8>,
}

impl AssemblyScriptAdapter {
    pub fn new(plugin_id: PluginId, wasm_bytes: Vec<u8>) -> Result<Self, PluginError> {
        Ok(Self {
            plugin_id,
            _wasm_bytes: wasm_bytes,
        })
    }
}

#[async_trait]
impl RuntimeAdapter for AssemblyScriptAdapter {
    fn runtime_type(&self) -> RuntimeType {
        RuntimeType::AssemblyScript
    }

    async fn initialize(&mut self) -> Result<(), PluginError> {
        tracing::info!("Initializing AssemblyScript plugin: {}", self.plugin_id);
        Ok(())
    }

    async fn call_auth(
        &self,
        _context: &PluginContext,
        _credentials: &AuthCredentials,
    ) -> Result<AuthResult, PluginError> {
        todo!("Implement AssemblyScript auth calling")
    }

    async fn call_template_function(
        &self,
        _function_name: &str,
        _args: &[serde_json::Value],
        _context: &ResolutionContext,
    ) -> Result<serde_json::Value, PluginError> {
        todo!("Implement AssemblyScript template function calling")
    }

    async fn call_response_generator(
        &self,
        _context: &PluginContext,
        _request: &ResponseRequest,
    ) -> Result<ResponseData, PluginError> {
        todo!("Implement AssemblyScript response generator calling")
    }

    async fn call_datasource_query(
        &self,
        _query: &DataQuery,
        _context: &PluginContext,
    ) -> Result<DataResult, PluginError> {
        todo!("Implement AssemblyScript datasource query calling")
    }

    async fn health_check(&self) -> Result<bool, PluginError> {
        Ok(true)
    }

    async fn cleanup(&mut self) -> Result<(), PluginError> {
        Ok(())
    }
}

// ============================================================================
// Remote Runtime Adapter (HTTP/gRPC)
// ============================================================================

pub struct RemoteAdapter {
    plugin_id: PluginId,
    config: RemoteRuntimeConfig,
    client: reqwest::Client,
}

impl RemoteAdapter {
    pub fn new(plugin_id: PluginId, config: RemoteRuntimeConfig) -> Result<Self, PluginError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|e| PluginError::Runtime(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            plugin_id,
            config,
            client,
        })
    }

    async fn call_remote_plugin(
        &self,
        endpoint: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, PluginError> {
        let url = format!("{}{}", self.config.endpoint, endpoint);

        let mut request = self.client.post(&url).json(&body);

        // Add authentication if configured
        if let Some(auth) = &self.config.auth {
            request = match auth.auth_type.as_str() {
                "bearer" => request.bearer_auth(&auth.value),
                "api_key" => request.header("X-API-Key", &auth.value),
                _ => request,
            };
        }

        let response = request
            .send()
            .await
            .map_err(|e| PluginError::Runtime(format!("Remote plugin call failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(PluginError::Runtime(format!(
                "Remote plugin returned error status: {}",
                response.status()
            )));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| PluginError::Runtime(format!("Failed to parse response: {}", e)))?;

        Ok(result)
    }
}

#[async_trait]
impl RuntimeAdapter for RemoteAdapter {
    fn runtime_type(&self) -> RuntimeType {
        RuntimeType::Remote(self.config.clone())
    }

    async fn initialize(&mut self) -> Result<(), PluginError> {
        tracing::info!("Initializing remote plugin: {}", self.plugin_id);

        // Perform health check during initialization
        self.health_check().await?;

        Ok(())
    }

    async fn call_auth(
        &self,
        context: &PluginContext,
        credentials: &AuthCredentials,
    ) -> Result<AuthResult, PluginError> {
        let body = serde_json::json!({
            "context": context,
            "credentials": credentials,
        });

        let result = self.call_remote_plugin("/plugin/authenticate", body).await?;

        // Parse the AuthResult from the response
        serde_json::from_value(result)
            .map_err(|e| PluginError::Runtime(format!("Failed to parse AuthResult: {}", e)))
    }

    async fn call_template_function(
        &self,
        function_name: &str,
        args: &[serde_json::Value],
        context: &ResolutionContext,
    ) -> Result<serde_json::Value, PluginError> {
        let body = serde_json::json!({
            "function_name": function_name,
            "args": args,
            "context": context,
        });

        self.call_remote_plugin("/plugin/template/execute", body).await
    }

    async fn call_response_generator(
        &self,
        context: &PluginContext,
        request: &ResponseRequest,
    ) -> Result<ResponseData, PluginError> {
        let body = serde_json::json!({
            "context": context,
            "request": request,
        });

        let result = self.call_remote_plugin("/plugin/response/generate", body).await?;

        serde_json::from_value(result)
            .map_err(|e| PluginError::Runtime(format!("Failed to parse ResponseData: {}", e)))
    }

    async fn call_datasource_query(
        &self,
        query: &DataQuery,
        context: &PluginContext,
    ) -> Result<DataResult, PluginError> {
        let body = serde_json::json!({
            "query": query,
            "context": context,
        });

        let result = self.call_remote_plugin("/plugin/datasource/query", body).await?;

        serde_json::from_value(result)
            .map_err(|e| PluginError::Runtime(format!("Failed to parse DataResult: {}", e)))
    }

    async fn health_check(&self) -> Result<bool, PluginError> {
        // Try to call health endpoint
        let url = format!("{}/health", self.config.endpoint);

        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    async fn cleanup(&mut self) -> Result<(), PluginError> {
        tracing::info!("Cleaning up remote plugin: {}", self.plugin_id);
        Ok(())
    }

    fn get_metrics(&self) -> HashMap<String, serde_json::Value> {
        let mut metrics = HashMap::new();
        metrics.insert("plugin_id".to_string(), serde_json::json!(self.plugin_id.as_str()));
        metrics.insert("endpoint".to_string(), serde_json::json!(self.config.endpoint));
        metrics.insert("protocol".to_string(), serde_json::json!(format!("{:?}", self.config.protocol)));
        metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_type_detection() {
        // Test with empty bytes (should default to Rust)
        let empty_bytes = vec![];
        let runtime = detect_runtime_type(&empty_bytes).unwrap();
        assert_eq!(runtime, RuntimeType::Rust);
    }

    #[test]
    fn test_remote_runtime_config() {
        let config = RemoteRuntimeConfig {
            protocol: RemoteProtocol::Http,
            endpoint: "http://localhost:8080".to_string(),
            timeout_ms: 5000,
            max_retries: 3,
            auth: Some(RemoteAuthConfig {
                auth_type: "bearer".to_string(),
                value: "secret-token".to_string(),
            }),
        };

        assert_eq!(config.endpoint, "http://localhost:8080");
        assert_eq!(config.timeout_ms, 5000);
    }
}
