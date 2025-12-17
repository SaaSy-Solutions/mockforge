//! Common types and interfaces used across all plugin types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use semver;

/// Plugin author information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAuthor {
    /// Author's name
    pub name: String,
    /// Author's email address (optional)
    pub email: Option<String>,
}

impl PluginAuthor {
    /// Creates a new plugin author with just a name
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            email: None,
        }
    }

    /// Creates a new plugin author with name and email
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
    /// Unique identifier for the plugin
    pub id: PluginId,
    /// Semantic version of the plugin
    pub version: PluginVersion,
    /// Human-readable name of the plugin
    pub name: String,
    /// Brief description of the plugin's functionality
    pub description: String,
    /// Plugin author information
    pub author: PluginAuthor,
}

impl PluginInfo {
    /// Creates new plugin information
    pub fn new(
        id: PluginId,
        version: PluginVersion,
        name: &str,
        description: &str,
        author: PluginAuthor,
    ) -> Self {
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
    /// Plugin metadata and information
    pub info: PluginInfo,
    /// List of capabilities provided by this plugin
    pub capabilities: Vec<String>,
    /// Map of plugin dependencies with their required versions
    pub dependencies: HashMap<PluginId, PluginVersion>,
}

impl PluginManifest {
    /// Creates a new plugin manifest with the given info
    pub fn new(info: PluginInfo) -> Self {
        Self {
            info,
            capabilities: Vec::new(),
            dependencies: HashMap::new(),
        }
    }

    /// Adds a capability to this plugin (builder pattern)
    pub fn with_capability(mut self, capability: &str) -> Self {
        self.capabilities.push(capability.to_string());
        self
    }

    /// Adds a dependency to this plugin (builder pattern)
    pub fn with_dependency(mut self, plugin_id: PluginId, version: PluginVersion) -> Self {
        self.dependencies.insert(plugin_id, version);
        self
    }

    /// Get the plugin ID
    pub fn id(&self) -> &PluginId {
        &self.info.id
    }

    /// Load plugin manifest from file
    pub fn from_file<P: AsRef<Path>>(
        path: P,
    ) -> std::result::Result<Self, Box<dyn std::error::Error + Send + Sync>> {
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
    /// List of capabilities provided by this plugin
    pub capabilities: Vec<String>,
    /// List of URL/path prefixes this plugin supports
    pub supported_prefixes: Vec<String>,
    /// Human-readable description of the plugin
    pub description: String,
    /// Plugin version string
    pub version: String,
}

impl PluginMetadata {
    /// Creates new plugin metadata with a description
    pub fn new(description: &str) -> Self {
        Self {
            capabilities: Vec::new(),
            supported_prefixes: Vec::new(),
            description: description.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Adds a capability to this plugin metadata (builder pattern)
    pub fn with_capability(mut self, capability: &str) -> Self {
        self.capabilities.push(capability.to_string());
        self
    }

    /// Adds a supported prefix to this plugin metadata (builder pattern)
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
    /// Major version number (breaking changes)
    pub major: u32,
    /// Minor version number (backwards-compatible additions)
    pub minor: u32,
    /// Patch version number (backwards-compatible bug fixes)
    pub patch: u32,
    /// Pre-release identifier (e.g., "alpha", "beta.1")
    pub pre_release: Option<String>,
    /// Build metadata
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
        semver::Version::parse(&version_str).map_err(|e| PluginError::InternalError {
            message: format!("Invalid version: {}", e),
        })
    }

    /// Create from semver::Version
    pub fn from_semver(version: &semver::Version) -> Self {
        Self {
            major: version.major as u32,
            minor: version.minor as u32,
            patch: version.patch as u32,
            pre_release: if version.pre.is_empty() {
                None
            } else {
                Some(version.pre.to_string())
            },
            build: if version.build.is_empty() {
                None
            } else {
                Some(version.build.to_string())
            },
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
    /// Creates a new PluginMetrics with all values initialized to zero
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
    /// Creates a new resolution context with current environment variables
    pub fn new() -> Self {
        Self {
            metadata: HashMap::new(),
            environment: std::env::vars().collect(),
            request_context: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Adds request metadata to the context (builder pattern)
    pub fn with_request(mut self, request: RequestMetadata) -> Self {
        self.request_context = Some(request);
        self
    }

    /// Adds a metadata key-value pair to the context (builder pattern)
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
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Request path
    pub path: String,
    /// HTTP headers
    pub headers: HashMap<String, String>,
    /// URL query parameters
    pub query_params: HashMap<String, String>,
}

impl RequestMetadata {
    /// Creates new request metadata with method and path
    pub fn new(method: &str, path: &str) -> Self {
        Self {
            method: method.to_string(),
            path: path.to_string(),
            headers: HashMap::new(),
            query_params: HashMap::new(),
        }
    }

    /// Adds a header to the request metadata (builder pattern)
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// Adds a query parameter to the request metadata (builder pattern)
    pub fn with_query_param(mut self, key: &str, value: &str) -> Self {
        self.query_params.insert(key.to_string(), value.to_string());
        self
    }
}

/// Core plugin error types
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    /// Token resolution failed with a specific error message
    #[error("Token resolution failed: {message}")]
    ResolutionFailed {
        /// Detailed error message
        message: String,
    },

    /// Invalid token format was encountered
    #[error("Invalid token format: {token}")]
    InvalidToken {
        /// The invalid token string
        token: String,
    },

    /// Plugin configuration is invalid or missing
    #[error("Plugin configuration error: {message}")]
    ConfigurationError {
        /// Configuration error details
        message: String,
    },

    /// Plugin execution exceeded the timeout limit
    #[error("Plugin execution timeout")]
    Timeout,

    /// Plugin attempted an action it doesn't have permission for
    #[error("Plugin permission denied: {action}")]
    PermissionDenied {
        /// The action that was denied
        action: String,
    },

    /// A required plugin dependency is missing
    #[error("Plugin dependency missing: {dependency}")]
    DependencyMissing {
        /// The missing dependency identifier
        dependency: String,
    },

    /// Internal plugin error
    #[error("Plugin internal error: {message}")]
    InternalError {
        /// Internal error details
        message: String,
    },

    /// Plugin execution failed
    #[error("Plugin execution error: {message}")]
    ExecutionError {
        /// Execution error details
        message: String,
    },

    /// Security policy violation
    #[error("Security violation: {violation}")]
    SecurityViolation {
        /// Details of the security violation
        violation: String,
    },

    /// WASM module loading or initialization error
    #[error("WASM module error: {message}")]
    WasmError {
        /// WASM error details
        message: String,
    },

    /// WASM runtime error
    #[error("WASM runtime error: {0}")]
    WasmRuntimeError(#[from] wasmtime::Error),
}

impl PluginError {
    /// Creates a new resolution failed error
    pub fn resolution_failed(message: &str) -> Self {
        Self::ResolutionFailed {
            message: message.to_string(),
        }
    }

    /// Creates a new invalid token error
    pub fn invalid_token(token: &str) -> Self {
        Self::InvalidToken {
            token: token.to_string(),
        }
    }

    /// Creates a new configuration error
    pub fn config_error(message: &str) -> Self {
        Self::ConfigurationError {
            message: message.to_string(),
        }
    }

    /// Creates a new execution error
    pub fn execution<S: Into<String>>(message: S) -> Self {
        Self::ExecutionError {
            message: message.into(),
        }
    }

    /// Creates a new security violation error
    pub fn security<S: Into<String>>(violation: S) -> Self {
        Self::SecurityViolation {
            violation: violation.into(),
        }
    }

    /// Creates a new WASM error
    pub fn wasm<S: Into<String>>(message: S) -> Self {
        Self::WasmError {
            message: message.into(),
        }
    }
}

/// Base plugin instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInstance {
    /// Unique identifier for this plugin instance
    pub id: PluginId,
    /// Plugin manifest with metadata
    pub manifest: PluginManifest,
    /// Current state of the plugin
    pub state: PluginState,
    /// Health status of the plugin
    pub health: PluginHealth,
}

impl PluginInstance {
    /// Creates a new plugin instance
    pub fn new(id: PluginId, manifest: PluginManifest) -> Self {
        Self {
            id,
            manifest,
            state: PluginState::Unloaded,
            health: PluginHealth::default(),
        }
    }

    /// Updates the plugin state and health status
    pub fn set_state(&mut self, state: PluginState) {
        let is_error = matches!(state, PluginState::Error);
        self.state = state;
        if is_error {
            self.health.healthy = false;
            self.health.last_check = chrono::Utc::now();
        }
    }

    /// Checks if the plugin is currently healthy
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_compiles() {
        // Basic compilation test
    }

    // PluginAuthor tests
    #[test]
    fn test_plugin_author_new() {
        let author = PluginAuthor::new("John Doe");
        assert_eq!(author.name, "John Doe");
        assert!(author.email.is_none());
    }

    #[test]
    fn test_plugin_author_with_email() {
        let author = PluginAuthor::with_email("John Doe", "john@example.com");
        assert_eq!(author.name, "John Doe");
        assert_eq!(author.email, Some("john@example.com".to_string()));
    }

    #[test]
    fn test_plugin_author_clone() {
        let author = PluginAuthor::with_email("Test", "test@test.com");
        let cloned = author.clone();
        assert_eq!(author.name, cloned.name);
        assert_eq!(author.email, cloned.email);
    }

    #[test]
    fn test_plugin_author_debug() {
        let author = PluginAuthor::new("Test");
        let debug = format!("{:?}", author);
        assert!(debug.contains("PluginAuthor"));
        assert!(debug.contains("Test"));
    }

    #[test]
    fn test_plugin_author_serialize() {
        let author = PluginAuthor::with_email("Test", "test@example.com");
        let json = serde_json::to_string(&author).unwrap();
        assert!(json.contains("Test"));
        assert!(json.contains("test@example.com"));
    }

    #[test]
    fn test_plugin_author_deserialize() {
        let json = r#"{"name":"Test","email":"test@example.com"}"#;
        let author: PluginAuthor = serde_json::from_str(json).unwrap();
        assert_eq!(author.name, "Test");
        assert_eq!(author.email, Some("test@example.com".to_string()));
    }

    // PluginInfo tests
    #[test]
    fn test_plugin_info_new() {
        let author = PluginAuthor::new("Author");
        let info = PluginInfo::new(
            PluginId::new("test-plugin"),
            PluginVersion::new(1, 0, 0),
            "Test Plugin",
            "A test plugin",
            author,
        );
        assert_eq!(info.name, "Test Plugin");
        assert_eq!(info.description, "A test plugin");
        assert_eq!(info.id.as_str(), "test-plugin");
    }

    #[test]
    fn test_plugin_info_clone() {
        let author = PluginAuthor::new("Author");
        let info = PluginInfo::new(
            PluginId::new("test-plugin"),
            PluginVersion::new(1, 0, 0),
            "Test Plugin",
            "A test plugin",
            author,
        );
        let cloned = info.clone();
        assert_eq!(info.name, cloned.name);
        assert_eq!(info.id, cloned.id);
    }

    // PluginManifest tests
    #[test]
    fn test_plugin_manifest_new() {
        let author = PluginAuthor::new("Author");
        let info = PluginInfo::new(
            PluginId::new("test-plugin"),
            PluginVersion::new(1, 0, 0),
            "Test",
            "Test",
            author,
        );
        let manifest = PluginManifest::new(info);
        assert!(manifest.capabilities.is_empty());
        assert!(manifest.dependencies.is_empty());
    }

    #[test]
    fn test_plugin_manifest_with_capability() {
        let author = PluginAuthor::new("Author");
        let info = PluginInfo::new(
            PluginId::new("test"),
            PluginVersion::new(1, 0, 0),
            "Test",
            "Test",
            author,
        );
        let manifest = PluginManifest::new(info)
            .with_capability("network:http")
            .with_capability("filesystem:read");
        assert_eq!(manifest.capabilities.len(), 2);
        assert!(manifest.capabilities.contains(&"network:http".to_string()));
    }

    #[test]
    fn test_plugin_manifest_with_dependency() {
        let author = PluginAuthor::new("Author");
        let info = PluginInfo::new(
            PluginId::new("test"),
            PluginVersion::new(1, 0, 0),
            "Test",
            "Test",
            author,
        );
        let manifest = PluginManifest::new(info)
            .with_dependency(PluginId::new("dep-plugin"), PluginVersion::new(2, 0, 0));
        assert_eq!(manifest.dependencies.len(), 1);
    }

    #[test]
    fn test_plugin_manifest_id() {
        let author = PluginAuthor::new("Author");
        let info = PluginInfo::new(
            PluginId::new("my-plugin"),
            PluginVersion::new(1, 0, 0),
            "Test",
            "Test",
            author,
        );
        let manifest = PluginManifest::new(info);
        assert_eq!(manifest.id().as_str(), "my-plugin");
    }

    #[test]
    fn test_plugin_manifest_validate_success() {
        let author = PluginAuthor::new("Author");
        let info = PluginInfo::new(
            PluginId::new("valid-plugin"),
            PluginVersion::new(1, 0, 0),
            "Valid Plugin",
            "A valid plugin",
            author,
        );
        let manifest = PluginManifest::new(info);
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_plugin_manifest_validate_empty_id() {
        let author = PluginAuthor::new("Author");
        let info =
            PluginInfo::new(PluginId::new(""), PluginVersion::new(1, 0, 0), "Test", "Test", author);
        let manifest = PluginManifest::new(info);
        let result = manifest.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ID cannot be empty"));
    }

    #[test]
    fn test_plugin_manifest_validate_empty_name() {
        let author = PluginAuthor::new("Author");
        let info =
            PluginInfo::new(PluginId::new("test"), PluginVersion::new(1, 0, 0), "", "Test", author);
        let manifest = PluginManifest::new(info);
        let result = manifest.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("name cannot be empty"));
    }

    // PluginMetadata tests
    #[test]
    fn test_plugin_metadata_new() {
        let metadata = PluginMetadata::new("A test plugin");
        assert_eq!(metadata.description, "A test plugin");
        assert!(metadata.capabilities.is_empty());
        assert!(metadata.supported_prefixes.is_empty());
    }

    #[test]
    fn test_plugin_metadata_with_capability() {
        let metadata =
            PluginMetadata::new("Test").with_capability("mock").with_capability("validate");
        assert_eq!(metadata.capabilities.len(), 2);
        assert!(metadata.capabilities.contains(&"mock".to_string()));
    }

    #[test]
    fn test_plugin_metadata_with_prefix() {
        let metadata = PluginMetadata::new("Test").with_prefix("/api/v1").with_prefix("/api/v2");
        assert_eq!(metadata.supported_prefixes.len(), 2);
        assert!(metadata.supported_prefixes.contains(&"/api/v1".to_string()));
    }

    #[test]
    fn test_plugin_metadata_clone() {
        let metadata = PluginMetadata::new("Test").with_capability("mock");
        let cloned = metadata.clone();
        assert_eq!(metadata.description, cloned.description);
        assert_eq!(metadata.capabilities, cloned.capabilities);
    }

    // PluginId tests
    #[test]
    fn test_plugin_id_new() {
        let id = PluginId::new("test-plugin");
        assert_eq!(id.as_str(), "test-plugin");
    }

    #[test]
    fn test_plugin_id_new_from_string() {
        let id = PluginId::new(String::from("dynamic-id"));
        assert_eq!(id.as_str(), "dynamic-id");
    }

    #[test]
    fn test_plugin_id_display() {
        let id = PluginId::new("display-test");
        assert_eq!(format!("{}", id), "display-test");
    }

    #[test]
    fn test_plugin_id_equality() {
        let id1 = PluginId::new("test");
        let id2 = PluginId::new("test");
        let id3 = PluginId::new("different");
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_plugin_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PluginId::new("plugin-1"));
        set.insert(PluginId::new("plugin-2"));
        set.insert(PluginId::new("plugin-1")); // Duplicate
        assert_eq!(set.len(), 2);
    }

    // PluginVersion tests
    #[test]
    fn test_plugin_version_new() {
        let version = PluginVersion::new(1, 2, 3);
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert!(version.pre_release.is_none());
        assert!(version.build.is_none());
    }

    #[test]
    fn test_plugin_version_parse_valid() {
        let version = PluginVersion::parse("1.2.3").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
    }

    #[test]
    fn test_plugin_version_parse_invalid_format() {
        let result = PluginVersion::parse("1.2");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid version format"));
    }

    #[test]
    fn test_plugin_version_parse_invalid_major() {
        let result = PluginVersion::parse("a.2.3");
        assert!(result.is_err());
    }

    #[test]
    fn test_plugin_version_display() {
        let version = PluginVersion::new(1, 2, 3);
        assert_eq!(format!("{}", version), "1.2.3");
    }

    #[test]
    fn test_plugin_version_display_with_prerelease() {
        let mut version = PluginVersion::new(1, 0, 0);
        version.pre_release = Some("alpha".to_string());
        assert_eq!(format!("{}", version), "1.0.0-alpha");
    }

    #[test]
    fn test_plugin_version_display_with_build() {
        let mut version = PluginVersion::new(1, 0, 0);
        version.build = Some("build123".to_string());
        assert_eq!(format!("{}", version), "1.0.0+build123");
    }

    #[test]
    fn test_plugin_version_display_full() {
        let mut version = PluginVersion::new(2, 1, 0);
        version.pre_release = Some("beta.1".to_string());
        version.build = Some("20231201".to_string());
        assert_eq!(format!("{}", version), "2.1.0-beta.1+20231201");
    }

    #[test]
    fn test_plugin_version_equality() {
        let v1 = PluginVersion::new(1, 0, 0);
        let v2 = PluginVersion::new(1, 0, 0);
        let v3 = PluginVersion::new(1, 0, 1);
        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }

    #[test]
    fn test_plugin_version_to_semver() {
        let version = PluginVersion::new(1, 2, 3);
        let semver = version.to_semver().unwrap();
        assert_eq!(semver.major, 1);
        assert_eq!(semver.minor, 2);
        assert_eq!(semver.patch, 3);
    }

    #[test]
    fn test_plugin_version_from_semver() {
        let semver = semver::Version::new(2, 3, 4);
        let version = PluginVersion::from_semver(&semver);
        assert_eq!(version.major, 2);
        assert_eq!(version.minor, 3);
        assert_eq!(version.patch, 4);
    }

    // PluginCapabilities tests
    #[test]
    fn test_plugin_capabilities_default() {
        let caps = PluginCapabilities::default();
        assert!(!caps.network.allow_http);
        assert!(caps.filesystem.read_paths.is_empty());
        assert!(caps.custom.is_empty());
    }

    #[test]
    fn test_plugin_capabilities_from_strings() {
        let strings = vec![
            "network:http".to_string(),
            "filesystem:read".to_string(),
            "custom:feature".to_string(),
        ];
        let caps = PluginCapabilities::from_strings(&strings);
        assert!(caps.network.allow_http);
        assert!(!caps.filesystem.read_paths.is_empty());
        assert!(caps.custom.contains_key("custom:feature"));
    }

    #[test]
    fn test_plugin_capabilities_has_capability() {
        let strings = vec!["network:http".to_string()];
        let caps = PluginCapabilities::from_strings(&strings);
        assert!(caps.has_capability("network:http"));
        assert!(!caps.has_capability("filesystem:write"));
    }

    #[test]
    fn test_plugin_capabilities_has_custom_capability() {
        let strings = vec!["custom:my-feature".to_string()];
        let caps = PluginCapabilities::from_strings(&strings);
        assert!(caps.has_capability("custom:my-feature"));
        assert!(!caps.has_capability("custom:other"));
    }

    // NetworkPermissions tests
    #[test]
    fn test_network_permissions_default() {
        let perms = NetworkPermissions::default();
        assert!(!perms.allow_http);
        assert!(perms.allowed_hosts.is_empty());
        assert_eq!(perms.max_connections, 10);
    }

    // FilesystemPermissions tests
    #[test]
    fn test_filesystem_permissions_default() {
        let perms = FilesystemPermissions::default();
        assert!(perms.read_paths.is_empty());
        assert!(perms.write_paths.is_empty());
        assert!(perms.allow_temp_files);
    }

    // ResourceLimits tests
    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_memory_bytes, 10 * 1024 * 1024);
        assert_eq!(limits.max_cpu_percent, 0.5);
        assert_eq!(limits.max_execution_time_ms, 5000);
        assert_eq!(limits.max_concurrent_executions, 5);
    }

    // PluginContext tests
    #[test]
    fn test_plugin_context_new() {
        let ctx = PluginContext::new(PluginId::new("test"), PluginVersion::new(1, 0, 0));
        assert_eq!(ctx.plugin_id.as_str(), "test");
        assert_eq!(ctx.timeout_ms, 5000);
        assert!(!ctx.request_id.is_empty());
    }

    #[test]
    fn test_plugin_context_with_timeout() {
        let ctx = PluginContext::new(PluginId::new("test"), PluginVersion::new(1, 0, 0))
            .with_timeout(10000);
        assert_eq!(ctx.timeout_ms, 10000);
    }

    #[test]
    fn test_plugin_context_with_env() {
        let ctx = PluginContext::new(PluginId::new("test"), PluginVersion::new(1, 0, 0))
            .with_env("KEY", "VALUE");
        assert_eq!(ctx.environment.get("KEY"), Some(&"VALUE".to_string()));
    }

    #[test]
    fn test_plugin_context_with_custom() {
        let ctx = PluginContext::new(PluginId::new("test"), PluginVersion::new(1, 0, 0))
            .with_custom("custom_key", serde_json::json!({"nested": "value"}));
        assert!(ctx.custom.contains_key("custom_key"));
    }

    // PluginResult tests
    #[test]
    fn test_plugin_result_success() {
        let result: PluginResult<String> = PluginResult::success("data".to_string(), 100);
        assert!(result.is_success());
        assert!(result.success);
        assert_eq!(result.execution_time_ms, 100);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_plugin_result_failure() {
        let result: PluginResult<String> = PluginResult::failure("error occurred", 50);
        assert!(!result.is_success());
        assert!(!result.success);
        assert!(result.data.is_none());
        assert_eq!(result.error(), Some("error occurred"));
    }

    #[test]
    fn test_plugin_result_unwrap() {
        let result: PluginResult<i32> = PluginResult::success(42, 10);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    #[should_panic(expected = "Called unwrap on failed plugin result")]
    fn test_plugin_result_unwrap_failure() {
        let result: PluginResult<i32> = PluginResult::failure("error", 10);
        let _ = result.unwrap();
    }

    #[test]
    fn test_plugin_result_data() {
        let result: PluginResult<String> = PluginResult::success("test".to_string(), 10);
        assert_eq!(result.data(), Some("test".to_string()));
    }

    #[test]
    fn test_plugin_result_data_none() {
        let result: PluginResult<String> = PluginResult::failure("error", 10);
        assert!(result.data().is_none());
    }

    // PluginState tests
    #[test]
    fn test_plugin_state_display() {
        assert_eq!(format!("{}", PluginState::Unloaded), "unloaded");
        assert_eq!(format!("{}", PluginState::Loading), "loading");
        assert_eq!(format!("{}", PluginState::Loaded), "loaded");
        assert_eq!(format!("{}", PluginState::Initializing), "initializing");
        assert_eq!(format!("{}", PluginState::Ready), "ready");
        assert_eq!(format!("{}", PluginState::Executing), "executing");
        assert_eq!(format!("{}", PluginState::Error), "error");
        assert_eq!(format!("{}", PluginState::Unloading), "unloading");
    }

    #[test]
    fn test_plugin_state_is_ready() {
        assert!(PluginState::Ready.is_ready());
        assert!(!PluginState::Loading.is_ready());
        assert!(!PluginState::Error.is_ready());
    }

    #[test]
    fn test_plugin_state_equality() {
        assert_eq!(PluginState::Ready, PluginState::Ready);
        assert_ne!(PluginState::Ready, PluginState::Error);
    }

    #[test]
    fn test_plugin_state_clone() {
        let state = PluginState::Executing;
        let cloned = state;
        assert_eq!(state, cloned);
    }

    // PluginHealth tests
    #[test]
    fn test_plugin_health_default() {
        let health = PluginHealth::default();
        assert_eq!(health.state, PluginState::Unloaded);
        assert!(health.healthy);
        assert_eq!(health.message, "Plugin initialized");
    }

    #[test]
    fn test_plugin_health_healthy() {
        let metrics = PluginMetrics::default();
        let health = PluginHealth::healthy("All good".to_string(), metrics);
        assert!(health.healthy);
        assert_eq!(health.state, PluginState::Ready);
        assert_eq!(health.message, "All good");
    }

    #[test]
    fn test_plugin_health_unhealthy() {
        let metrics = PluginMetrics::default();
        let health = PluginHealth::unhealthy(
            PluginState::Error,
            "Something went wrong".to_string(),
            metrics,
        );
        assert!(!health.healthy);
        assert_eq!(health.state, PluginState::Error);
    }

    // PluginMetrics tests
    #[test]
    fn test_plugin_metrics_default() {
        let metrics = PluginMetrics::default();
        assert_eq!(metrics.total_executions, 0);
        assert_eq!(metrics.successful_executions, 0);
        assert_eq!(metrics.failed_executions, 0);
        assert_eq!(metrics.avg_execution_time_ms, 0.0);
        assert_eq!(metrics.max_execution_time_ms, 0);
        assert_eq!(metrics.memory_usage_bytes, 0);
        assert_eq!(metrics.peak_memory_usage_bytes, 0);
    }

    #[test]
    fn test_plugin_metrics_clone() {
        let mut metrics = PluginMetrics::default();
        metrics.total_executions = 100;
        let cloned = metrics.clone();
        assert_eq!(metrics.total_executions, cloned.total_executions);
    }

    // ResolutionContext tests
    #[test]
    fn test_resolution_context_new() {
        let ctx = ResolutionContext::new();
        assert!(ctx.metadata.is_empty());
        assert!(ctx.request_context.is_none());
    }

    #[test]
    fn test_resolution_context_default() {
        let ctx = ResolutionContext::default();
        assert!(ctx.metadata.is_empty());
    }

    #[test]
    fn test_resolution_context_with_request() {
        let request = RequestMetadata::new("GET", "/api/test");
        let ctx = ResolutionContext::new().with_request(request);
        assert!(ctx.request_context.is_some());
        assert_eq!(ctx.request_context.as_ref().unwrap().method, "GET");
    }

    #[test]
    fn test_resolution_context_with_metadata() {
        let ctx = ResolutionContext::new()
            .with_metadata("key1", "value1")
            .with_metadata("key2", "value2");
        assert_eq!(ctx.metadata.get("key1"), Some(&"value1".to_string()));
        assert_eq!(ctx.metadata.get("key2"), Some(&"value2".to_string()));
    }

    // RequestMetadata tests
    #[test]
    fn test_request_metadata_new() {
        let meta = RequestMetadata::new("POST", "/api/users");
        assert_eq!(meta.method, "POST");
        assert_eq!(meta.path, "/api/users");
        assert!(meta.headers.is_empty());
        assert!(meta.query_params.is_empty());
    }

    #[test]
    fn test_request_metadata_with_header() {
        let meta = RequestMetadata::new("GET", "/api")
            .with_header("Content-Type", "application/json")
            .with_header("Authorization", "Bearer token");
        assert_eq!(meta.headers.len(), 2);
        assert_eq!(meta.headers.get("Content-Type"), Some(&"application/json".to_string()));
    }

    #[test]
    fn test_request_metadata_with_query_param() {
        let meta = RequestMetadata::new("GET", "/api/search")
            .with_query_param("q", "test")
            .with_query_param("limit", "10");
        assert_eq!(meta.query_params.len(), 2);
        assert_eq!(meta.query_params.get("q"), Some(&"test".to_string()));
    }

    #[test]
    fn test_request_metadata_clone() {
        let meta = RequestMetadata::new("GET", "/test").with_header("X-Custom", "value");
        let cloned = meta.clone();
        assert_eq!(meta.method, cloned.method);
        assert_eq!(meta.headers, cloned.headers);
    }

    // PluginError tests (types.rs version)
    #[test]
    fn test_plugin_error_resolution_failed() {
        let err = PluginError::resolution_failed("test message");
        assert!(matches!(err, PluginError::ResolutionFailed { .. }));
        assert!(err.to_string().contains("test message"));
    }

    #[test]
    fn test_plugin_error_invalid_token() {
        let err = PluginError::invalid_token("{{invalid}}");
        assert!(matches!(err, PluginError::InvalidToken { .. }));
        assert!(err.to_string().contains("{{invalid}}"));
    }

    #[test]
    fn test_plugin_error_config_error() {
        let err = PluginError::config_error("missing field");
        assert!(matches!(err, PluginError::ConfigurationError { .. }));
        assert!(err.to_string().contains("missing field"));
    }

    #[test]
    fn test_plugin_error_execution() {
        let err = PluginError::execution("runtime error");
        assert!(matches!(err, PluginError::ExecutionError { .. }));
        assert!(err.to_string().contains("runtime error"));
    }

    #[test]
    fn test_plugin_error_security() {
        let err = PluginError::security("unauthorized access");
        assert!(matches!(err, PluginError::SecurityViolation { .. }));
        assert!(err.to_string().contains("unauthorized access"));
    }

    #[test]
    fn test_plugin_error_wasm() {
        let err = PluginError::wasm("invalid module");
        assert!(matches!(err, PluginError::WasmError { .. }));
        assert!(err.to_string().contains("invalid module"));
    }

    #[test]
    fn test_plugin_error_timeout_display() {
        let err = PluginError::Timeout;
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn test_plugin_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: PluginError = io_err.into();
        assert!(matches!(err, PluginError::InternalError { .. }));
    }

    #[test]
    fn test_plugin_error_from_str() {
        let err: PluginError = "simple error".into();
        assert!(matches!(err, PluginError::InternalError { .. }));
        assert!(err.to_string().contains("simple error"));
    }

    #[test]
    fn test_plugin_error_from_string() {
        let err: PluginError = String::from("string error").into();
        assert!(matches!(err, PluginError::InternalError { .. }));
    }

    #[test]
    fn test_plugin_error_from_vec_strings() {
        let errors = vec!["error 1".to_string(), "error 2".to_string()];
        let err: PluginError = errors.into();
        assert!(matches!(err, PluginError::InternalError { .. }));
        let msg = err.to_string();
        assert!(msg.contains("error 1"));
        assert!(msg.contains("error 2"));
    }

    // PluginInstance tests
    #[test]
    fn test_plugin_instance_new() {
        let author = PluginAuthor::new("Author");
        let info = PluginInfo::new(
            PluginId::new("test"),
            PluginVersion::new(1, 0, 0),
            "Test",
            "Test",
            author,
        );
        let manifest = PluginManifest::new(info);
        let instance = PluginInstance::new(PluginId::new("test"), manifest);

        assert_eq!(instance.id.as_str(), "test");
        assert_eq!(instance.state, PluginState::Unloaded);
        assert!(instance.health.healthy);
    }

    #[test]
    fn test_plugin_instance_set_state() {
        let author = PluginAuthor::new("Author");
        let info = PluginInfo::new(
            PluginId::new("test"),
            PluginVersion::new(1, 0, 0),
            "Test",
            "Test",
            author,
        );
        let manifest = PluginManifest::new(info);
        let mut instance = PluginInstance::new(PluginId::new("test"), manifest);

        instance.set_state(PluginState::Ready);
        assert_eq!(instance.state, PluginState::Ready);
        assert!(instance.is_healthy());
    }

    #[test]
    fn test_plugin_instance_set_state_error() {
        let author = PluginAuthor::new("Author");
        let info = PluginInfo::new(
            PluginId::new("test"),
            PluginVersion::new(1, 0, 0),
            "Test",
            "Test",
            author,
        );
        let manifest = PluginManifest::new(info);
        let mut instance = PluginInstance::new(PluginId::new("test"), manifest);

        instance.set_state(PluginState::Error);
        assert_eq!(instance.state, PluginState::Error);
        assert!(!instance.is_healthy());
    }

    #[test]
    fn test_plugin_instance_is_healthy() {
        let author = PluginAuthor::new("Author");
        let info = PluginInfo::new(
            PluginId::new("test"),
            PluginVersion::new(1, 0, 0),
            "Test",
            "Test",
            author,
        );
        let manifest = PluginManifest::new(info);
        let instance = PluginInstance::new(PluginId::new("test"), manifest);
        assert!(instance.is_healthy());
    }

    #[test]
    fn test_plugin_instance_clone() {
        let author = PluginAuthor::new("Author");
        let info = PluginInfo::new(
            PluginId::new("test"),
            PluginVersion::new(1, 0, 0),
            "Test",
            "Test",
            author,
        );
        let manifest = PluginManifest::new(info);
        let instance = PluginInstance::new(PluginId::new("test"), manifest);
        let cloned = instance.clone();
        assert_eq!(instance.id, cloned.id);
        assert_eq!(instance.state, cloned.state);
    }
}
