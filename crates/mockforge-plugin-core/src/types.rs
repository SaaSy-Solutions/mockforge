//! Common types and interfaces used across all plugin types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use semver;

/// Plugin author information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAuthor {
    pub name: String,
    pub email: Option<String>,
}

impl PluginAuthor {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            email: None,
        }
    }

    pub fn with_email(name: &str, email: &str) -> Self {
        Self {
            name: name.to_string(),
            email: Some(email.to_string()),
        }
    }
}

/// Plugin information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: PluginId,
    pub version: PluginVersion,
    pub name: String,
    pub description: String,
    pub author: PluginAuthor,
}

impl PluginInfo {
    pub fn new(id: PluginId, version: PluginVersion, name: &str, description: &str, author: PluginAuthor) -> Self {
        Self {
            id,
            version,
            name: name.to_string(),
            description: description.to_string(),
            author,
        }
    }
}

/// Plugin manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub info: PluginInfo,
    pub capabilities: Vec<String>,
    pub dependencies: HashMap<PluginId, PluginVersion>,
}

impl PluginManifest {
    pub fn new(info: PluginInfo) -> Self {
        Self {
            info,
            capabilities: Vec::new(),
            dependencies: HashMap::new(),
        }
    }

    pub fn with_capability(mut self, capability: &str) -> Self {
        self.capabilities.push(capability.to_string());
        self
    }

    pub fn with_dependency(mut self, plugin_id: PluginId, version: PluginVersion) -> Self {
        self.dependencies.insert(plugin_id, version);
        self
    }

    /// Get the plugin ID
    pub fn id(&self) -> &PluginId {
        &self.info.id
    }

    /// Load plugin manifest from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> std::result::Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let content = std::fs::read_to_string(path)?;
        let manifest: Self = serde_yaml::from_str(&content)?;
        Ok(manifest)
    }

    /// Validate the manifest
    pub fn validate(&self) -> std::result::Result<(), String> {
        let mut errors = Vec::new();

        if self.info.id.0.is_empty() {
            errors.push("Plugin ID cannot be empty".to_string());
        }

        if self.info.name.is_empty() {
            errors.push("Plugin name cannot be empty".to_string());
        }

        if self.info.version.to_string().is_empty() {
            errors.push("Plugin version cannot be empty".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }
}

/// Plugin metadata container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub capabilities: Vec<String>,
    pub supported_prefixes: Vec<String>,
    pub description: String,
    pub version: String,
}

impl PluginMetadata {
    pub fn new(description: &str) -> Self {
        Self {
            capabilities: Vec::new(),
            supported_prefixes: Vec::new(),
            description: description.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    pub fn with_capability(mut self, capability: &str) -> Self {
        self.capabilities.push(capability.to_string());
        self
    }

    pub fn with_prefix(mut self, prefix: &str) -> Self {
        self.supported_prefixes.push(prefix.to_string());
        self
    }
}

/// Plugin identifier (unique across all plugins)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginId(pub String);

impl PluginId {
    /// Create a new plugin ID
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self(id.into())
    }

    /// Get the plugin ID as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

use std::fmt;

/// Plugin version following semantic versioning
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_release: Option<String>,
    pub build: Option<String>,
}

impl PluginVersion {
    /// Create a new version
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            pre_release: None,
            build: None,
        }
    }

    /// Parse version from string (semver format)
    pub fn parse<S: AsRef<str>>(version: S) -> std::result::Result<Self, String> {
        let version = version.as_ref();

        // Simple semver parsing (can be enhanced with proper semver crate)
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() < 3 {
            return Err(format!("Invalid version format: {}", version));
        }

        let major = parts[0].parse().map_err(|_| "Invalid major version")?;
        let minor = parts[1].parse().map_err(|_| "Invalid minor version")?;
        let patch = parts[2].parse().map_err(|_| "Invalid patch version")?;

        Ok(Self {
            major,
            minor,
            patch,
            pre_release: None,
            build: None,
        })
    }
}

impl fmt::Display for PluginVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.pre_release {
            write!(f, "-{}", pre)?;
        }
        if let Some(build) = &self.build {
            write!(f, "+{}", build)?;
        }
        Ok(())
    }
}

impl PluginVersion {
    /// Convert to semver::Version
    pub fn to_semver(&self) -> Result<semver::Version> {
        let mut version_str = format!("{}.{}.{}", self.major, self.minor, self.patch);
        if let Some(pre) = &self.pre_release {
            version_str.push_str(&format!("-{}", pre));
        }
        if let Some(build) = &self.build {
            version_str.push_str(&format!("+{}", build));
        }
        semver::Version::parse(&version_str).map_err(|e| PluginError::InternalError { message: format!("Invalid version: {}", e) })
    }

    /// Create from semver::Version
    pub fn from_semver(version: &semver::Version) -> Self {
        Self {
            major: version.major as u32,
            minor: version.minor as u32,
            patch: version.patch as u32,
            pre_release: if version.pre.is_empty() { None } else { Some(version.pre.to_string()) },
            build: if version.build.is_empty() { None } else { Some(version.build.to_string()) },
        }
    }
}

/// Plugin capabilities (permissions and features)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginCapabilities {
    /// Network access permissions
    pub network: NetworkPermissions,
    /// File system access permissions
    pub filesystem: FilesystemPermissions,
    /// Resource limits
    pub resources: ResourceLimits,
    /// Custom capabilities
    pub custom: HashMap<String, serde_json::Value>,
}



impl PluginCapabilities {
    /// Create PluginCapabilities from a list of capability strings
    pub fn from_strings(capabilities: &[String]) -> Self {
        let mut result = Self::default();

        for cap in capabilities {
            match cap.as_str() {
                "network:http" => result.network.allow_http = true,
                "filesystem:read" => result.filesystem.read_paths.push("*".to_string()),
                "filesystem:write" => result.filesystem.write_paths.push("*".to_string()),
                _ => {
                    // For custom capabilities, store them in the custom map
                    result.custom.insert(cap.clone(), serde_json::Value::Bool(true));
                }
            }
        }

        result
    }

    /// Check if the plugin has a specific capability
    pub fn has_capability(&self, capability: &str) -> bool {
        match capability {
            "network:http" => self.network.allow_http,
            "filesystem:read" => !self.filesystem.read_paths.is_empty(),
            "filesystem:write" => !self.filesystem.write_paths.is_empty(),
            _ => self.custom.contains_key(capability),
        }
    }
}

/// Network access permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPermissions {
    /// Allow outbound HTTP/HTTPS requests
    pub allow_http: bool,
    /// Allowed host patterns (glob patterns)
    pub allowed_hosts: Vec<String>,
    /// Maximum concurrent connections
    pub max_connections: u32,
}

impl Default for NetworkPermissions {
    fn default() -> Self {
        Self {
            allow_http: false,
            allowed_hosts: Vec::new(),
            max_connections: 10,
        }
    }
}

/// File system access permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemPermissions {
    /// Allow read access to paths
    pub read_paths: Vec<String>,
    /// Allow write access to paths
    pub write_paths: Vec<String>,
    /// Allow temporary file creation
    pub allow_temp_files: bool,
}

impl Default for FilesystemPermissions {
    fn default() -> Self {
        Self {
            read_paths: Vec::new(),
            write_paths: Vec::new(),
            allow_temp_files: true,
        }
    }
}

/// Resource limits for plugin execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory usage in bytes
    pub max_memory_bytes: usize,
    /// Maximum CPU usage (0.0-1.0, where 1.0 = 100% of one core)
    pub max_cpu_percent: f64,
    /// Maximum execution time in milliseconds
    pub max_execution_time_ms: u64,
    /// Maximum number of concurrent executions
    pub max_concurrent_executions: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 10 * 1024 * 1024, // 10MB
            max_cpu_percent: 0.5,               // 50% of one core
            max_execution_time_ms: 5000,        // 5 seconds
            max_concurrent_executions: 5,
        }
    }
}

/// Plugin execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginContext {
    /// Plugin ID
    pub plugin_id: PluginId,
    /// Plugin version
    pub version: PluginVersion,
    /// Execution timeout
    pub timeout_ms: u64,
    /// Request ID for tracing
    pub request_id: String,
    /// Environment variables available to plugin
    pub environment: HashMap<String, String>,
    /// Custom context data
    pub custom: HashMap<String, serde_json::Value>,
}

impl PluginContext {
    /// Create a new plugin context
    pub fn new(plugin_id: PluginId, version: PluginVersion) -> Self {
        Self {
            plugin_id,
            version,
            timeout_ms: 5000,
            request_id: uuid::Uuid::new_v4().to_string(),
            environment: HashMap::new(),
            custom: HashMap::new(),
        }
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Add environment variable
    pub fn with_env<S: Into<String>>(mut self, key: S, value: S) -> Self {
        self.environment.insert(key.into(), value.into());
        self
    }

    /// Add custom context data
    pub fn with_custom<S: Into<String>>(mut self, key: S, value: serde_json::Value) -> Self {
        self.custom.insert(key.into(), value);
        self
    }
}

/// Plugin execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResult<T> {
    /// Success flag
    pub success: bool,
    /// Result data (if successful)
    pub data: Option<T>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Execution metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

impl<T> PluginResult<T> {
    /// Create a successful result
    pub fn success(data: T, execution_time_ms: u64) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            metadata: HashMap::new(),
            execution_time_ms,
        }
    }

    /// Create a failure result
    pub fn failure<S: Into<String>>(error: S, execution_time_ms: u64) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.into()),
            metadata: HashMap::new(),
            execution_time_ms,
        }
    }

    /// Check if result is successful
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// Get result data (panics if not successful)
    pub fn unwrap(self) -> T {
        self.data.expect("Called unwrap on failed plugin result")
    }

    /// Get result data with error handling
    pub fn data(self) -> Option<T> {
        self.data
    }

    /// Get error message
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

/// Plugin lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginState {
    /// Plugin is unloaded
    Unloaded,
    /// Plugin is loading
    Loading,
    /// Plugin is loaded but not initialized
    Loaded,
    /// Plugin is initializing
    Initializing,
    /// Plugin is ready for use
    Ready,
    /// Plugin is executing
    Executing,
    /// Plugin encountered an error
    Error,
    /// Plugin is being unloaded
    Unloading,
}

impl fmt::Display for PluginState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginState::Unloaded => write!(f, "unloaded"),
            PluginState::Loading => write!(f, "loading"),
            PluginState::Loaded => write!(f, "loaded"),
            PluginState::Initializing => write!(f, "initializing"),
            PluginState::Ready => write!(f, "ready"),
            PluginState::Executing => write!(f, "executing"),
            PluginState::Error => write!(f, "error"),
            PluginState::Unloading => write!(f, "unloading"),
        }
    }
}

impl PluginState {
    /// Check if plugin is ready for use
    pub fn is_ready(&self) -> bool {
        matches!(self, PluginState::Ready)
    }
}

/// Plugin health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHealth {
    /// Current state
    pub state: PluginState,
    /// Last health check timestamp
    pub last_check: chrono::DateTime<chrono::Utc>,
    /// Health check result
    pub healthy: bool,
    /// Health check message
    pub message: String,
    /// Performance metrics
    pub metrics: PluginMetrics,
}

impl Default for PluginHealth {
    fn default() -> Self {
        Self {
            state: PluginState::Unloaded,
            last_check: chrono::Utc::now(),
            healthy: true,
            message: "Plugin initialized".to_string(),
            metrics: PluginMetrics::default(),
        }
    }
}

impl PluginHealth {
    /// Create a healthy status
    pub fn healthy(message: String, metrics: PluginMetrics) -> Self {
        Self {
            state: PluginState::Ready,
            last_check: chrono::Utc::now(),
            healthy: true,
            message,
            metrics,
        }
    }

    /// Create an unhealthy status
    pub fn unhealthy(state: PluginState, message: String, metrics: PluginMetrics) -> Self {
        Self {
            state,
            last_check: chrono::Utc::now(),
            healthy: false,
            message,
            metrics,
        }
    }
}

/// Plugin performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetrics {
    /// Total executions
    pub total_executions: u64,
    /// Successful executions
    pub successful_executions: u64,
    /// Failed executions
    pub failed_executions: u64,
    /// Average execution time in milliseconds
    pub avg_execution_time_ms: f64,
    /// Maximum execution time in milliseconds
    pub max_execution_time_ms: u64,
    /// Current memory usage in bytes
    pub memory_usage_bytes: usize,
    /// Peak memory usage in bytes
    pub peak_memory_usage_bytes: usize,
}

impl Default for PluginMetrics {
    fn default() -> Self {
        Self {
            total_executions: 0,
            successful_executions: 0,
            failed_executions: 0,
            avg_execution_time_ms: 0.0,
            max_execution_time_ms: 0,
            memory_usage_bytes: 0,
            peak_memory_usage_bytes: 0,
        }
    }
}

/// Token resolution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionContext {
    /// Request metadata
    pub metadata: HashMap<String, String>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Request context (HTTP method, path, etc.)
    pub request_context: Option<RequestMetadata>,
    /// Timestamp when resolution started
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ResolutionContext {
    pub fn new() -> Self {
        Self {
            metadata: HashMap::new(),
            environment: std::env::vars().collect(),
            request_context: None,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn with_request(mut self, request: RequestMetadata) -> Self {
        self.request_context = Some(request);
        self
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

impl Default for ResolutionContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Request metadata for token resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMetadata {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
}

impl RequestMetadata {
    pub fn new(method: &str, path: &str) -> Self {
        Self {
            method: method.to_string(),
            path: path.to_string(),
            headers: HashMap::new(),
            query_params: HashMap::new(),
        }
    }

    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_query_param(mut self, key: &str, value: &str) -> Self {
        self.query_params.insert(key.to_string(), value.to_string());
        self
    }
}

/// Core plugin error types
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Token resolution failed: {message}")]
    ResolutionFailed { message: String },

    #[error("Invalid token format: {token}")]
    InvalidToken { token: String },

    #[error("Plugin configuration error: {message}")]
    ConfigurationError { message: String },

    #[error("Plugin execution timeout")]
    Timeout,

    #[error("Plugin permission denied: {action}")]
    PermissionDenied { action: String },

    #[error("Plugin dependency missing: {dependency}")]
    DependencyMissing { dependency: String },

    #[error("Plugin internal error: {message}")]
    InternalError { message: String },

    #[error("Plugin execution error: {message}")]
    ExecutionError { message: String },

    #[error("Security violation: {violation}")]
    SecurityViolation { violation: String },

    #[error("WASM module error: {message}")]
    WasmError { message: String },

    #[error("WASM runtime error: {0}")]
    WasmRuntimeError(#[from] wasmtime::Error),
}

impl PluginError {
    pub fn resolution_failed(message: &str) -> Self {
        Self::ResolutionFailed {
            message: message.to_string(),
        }
    }

    pub fn invalid_token(token: &str) -> Self {
        Self::InvalidToken {
            token: token.to_string(),
        }
    }

    pub fn config_error(message: &str) -> Self {
        Self::ConfigurationError {
            message: message.to_string(),
        }
    }

    pub fn execution<S: Into<String>>(message: S) -> Self {
        Self::ExecutionError {
            message: message.into(),
        }
    }

    pub fn security<S: Into<String>>(violation: S) -> Self {
        Self::SecurityViolation {
            violation: violation.into(),
        }
    }

    pub fn wasm<S: Into<String>>(message: S) -> Self {
        Self::WasmError {
            message: message.into(),
        }
    }
}

/// Base plugin instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInstance {
    pub id: PluginId,
    pub manifest: PluginManifest,
    pub state: PluginState,
    pub health: PluginHealth,
}

impl PluginInstance {
    pub fn new(id: PluginId, manifest: PluginManifest) -> Self {
        Self {
            id,
            manifest,
            state: PluginState::Unloaded,
            health: PluginHealth::default(),
        }
    }

    pub fn set_state(&mut self, state: PluginState) {
        let is_error = matches!(state, PluginState::Error);
        self.state = state;
        if is_error {
            self.health.healthy = false;
            self.health.last_check = chrono::Utc::now();
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.health.healthy
    }
}

/// Result type alias for plugin operations
pub type Result<T> = std::result::Result<T, PluginError>;

impl From<std::io::Error> for PluginError {
    fn from(error: std::io::Error) -> Self {
        Self::InternalError {
            message: error.to_string(),
        }
    }
}

impl From<serde_json::Error> for PluginError {
    fn from(error: serde_json::Error) -> Self {
        Self::InternalError {
            message: error.to_string(),
        }
    }
}

impl From<&str> for PluginError {
    fn from(message: &str) -> Self {
        Self::InternalError {
            message: message.to_string(),
        }
    }
}

impl From<String> for PluginError {
    fn from(message: String) -> Self {
        Self::InternalError { message }
    }
}

impl From<Vec<String>> for PluginError {
    fn from(errors: Vec<String>) -> Self {
        Self::InternalError {
            message: format!("Multiple errors: {}", errors.join(", ")),
        }
    }
}
