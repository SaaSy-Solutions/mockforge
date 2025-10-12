//! # MockForge Plugin Loader
//!
//! Secure plugin loading and validation system for MockForge.
//! This crate provides the plugin loader that handles:
//!
//! - Plugin discovery and validation
//! - Security sandboxing and capability checking
//! - WebAssembly module loading and instantiation
//! - Plugin lifecycle management
//!
//! ## Security Features
//!
//! - **WASM Sandboxing**: All plugins run in isolated WebAssembly environments
//! - **Capability Validation**: Strict permission checking before plugin execution
//! - **Resource Limits**: Memory, CPU, and execution time constraints
//! - **Code Signing**: Optional plugin signature verification

use std::path::Path;
use std::path::PathBuf;

// Import types from plugin core
use mockforge_plugin_core::{
    PluginAuthor, PluginId, PluginInfo, PluginInstance, PluginManifest, PluginVersion,
};

pub mod git;
pub mod installer;
pub mod loader;
pub mod metadata;
pub mod registry;
pub mod remote;
pub mod runtime_adapter;
pub mod sandbox;
pub mod signature;
pub mod signature_gen;
pub mod validator;

/// Re-export commonly used types
pub use git::*;
pub use installer::*;
pub use loader::*;
pub use metadata::*;
pub use registry::*;
pub use remote::*;
pub use runtime_adapter::*;
pub use sandbox::*;
pub use signature::*;
pub use signature_gen::*;
pub use validator::*;

/// Plugin loader result type
pub type LoaderResult<T> = std::result::Result<T, PluginLoaderError>;

/// Plugin loader error types
#[derive(Debug, thiserror::Error)]
pub enum PluginLoaderError {
    /// Plugin loading failed
    #[error("Plugin loading error: {message}")]
    LoadError { message: String },

    /// Plugin validation failed
    #[error("Plugin validation error: {message}")]
    ValidationError { message: String },

    /// Security violation during plugin loading
    #[error("Security violation: {violation}")]
    SecurityViolation { violation: String },

    /// Plugin manifest error
    #[error("Plugin manifest error: {message}")]
    ManifestError { message: String },

    /// WebAssembly module error
    #[error("WebAssembly module error: {message}")]
    WasmError { message: String },

    /// File system error
    #[error("File system error: {message}")]
    FsError { message: String },

    /// Plugin already loaded
    #[error("Plugin already loaded: {plugin_id}")]
    AlreadyLoaded { plugin_id: PluginId },

    /// Plugin not found
    #[error("Plugin not found: {plugin_id}")]
    NotFound { plugin_id: PluginId },

    /// Plugin dependency error
    #[error("Plugin dependency error: {message}")]
    DependencyError { message: String },

    /// Resource limit exceeded
    #[error("Resource limit exceeded: {message}")]
    ResourceLimit { message: String },

    /// Plugin execution error
    #[error("Plugin execution error: {message}")]
    ExecutionError { message: String },
}

impl PluginLoaderError {
    /// Create a load error
    pub fn load<S: Into<String>>(message: S) -> Self {
        Self::LoadError {
            message: message.into(),
        }
    }

    /// Create a validation error
    pub fn validation<S: Into<String>>(message: S) -> Self {
        Self::ValidationError {
            message: message.into(),
        }
    }

    /// Create a security violation error
    pub fn security<S: Into<String>>(violation: S) -> Self {
        Self::SecurityViolation {
            violation: violation.into(),
        }
    }

    /// Create a manifest error
    pub fn manifest<S: Into<String>>(message: S) -> Self {
        Self::ManifestError {
            message: message.into(),
        }
    }

    /// Create a WASM error
    pub fn wasm<S: Into<String>>(message: S) -> Self {
        Self::WasmError {
            message: message.into(),
        }
    }

    /// Create a file system error
    pub fn fs<S: Into<String>>(message: S) -> Self {
        Self::FsError {
            message: message.into(),
        }
    }

    /// Create an already loaded error
    pub fn already_loaded(plugin_id: PluginId) -> Self {
        Self::AlreadyLoaded { plugin_id }
    }

    /// Create a not found error
    pub fn not_found(plugin_id: PluginId) -> Self {
        Self::NotFound { plugin_id }
    }

    /// Create a dependency error
    pub fn dependency<S: Into<String>>(message: S) -> Self {
        Self::DependencyError {
            message: message.into(),
        }
    }

    /// Create a resource limit error
    pub fn resource_limit<S: Into<String>>(message: S) -> Self {
        Self::ResourceLimit {
            message: message.into(),
        }
    }

    /// Create an execution error
    pub fn execution<S: Into<String>>(message: S) -> Self {
        Self::ExecutionError {
            message: message.into(),
        }
    }

    /// Check if this is a security-related error
    pub fn is_security_error(&self) -> bool {
        matches!(self, PluginLoaderError::SecurityViolation { .. })
    }
}

/// Plugin loader configuration
#[derive(Debug, Clone)]
pub struct PluginLoaderConfig {
    /// Plugin directories to scan
    pub plugin_dirs: Vec<String>,
    /// Allow unsigned plugins (for development)
    pub allow_unsigned: bool,
    /// Trusted public keys for plugin signing (key IDs)
    pub trusted_keys: Vec<String>,
    /// Key data storage (key_id -> key_bytes)
    pub key_data: std::collections::HashMap<String, Vec<u8>>,
    /// Maximum plugins to load
    pub max_plugins: usize,
    /// Plugin loading timeout
    pub load_timeout_secs: u64,
    /// Enable debug logging
    pub debug_logging: bool,
    /// Skip WASM validation (for testing)
    pub skip_wasm_validation: bool,
}

impl Default for PluginLoaderConfig {
    fn default() -> Self {
        Self {
            plugin_dirs: vec!["~/.mockforge/plugins".to_string(), "./plugins".to_string()],
            allow_unsigned: false,
            trusted_keys: vec!["trusted-dev-key".to_string()],
            key_data: std::collections::HashMap::new(),
            max_plugins: 100,
            load_timeout_secs: 30,
            debug_logging: false,
            skip_wasm_validation: false,
        }
    }
}

/// Plugin loading context
#[derive(Debug, Clone)]
pub struct PluginLoadContext {
    /// Plugin ID
    pub plugin_id: PluginId,
    /// Plugin manifest
    pub manifest: PluginManifest,
    /// Plugin file path
    pub plugin_path: String,
    /// Loading timestamp
    pub load_time: chrono::DateTime<chrono::Utc>,
    /// Loader configuration
    pub config: PluginLoaderConfig,
}

impl PluginLoadContext {
    /// Create new loading context
    pub fn new(
        plugin_id: PluginId,
        manifest: PluginManifest,
        plugin_path: String,
        config: PluginLoaderConfig,
    ) -> Self {
        Self {
            plugin_id,
            manifest,
            plugin_path,
            load_time: chrono::Utc::now(),
            config,
        }
    }
}

/// Plugin loading statistics
#[derive(Debug, Clone, Default)]
pub struct PluginLoadStats {
    /// Total plugins discovered
    pub discovered: usize,
    /// Plugins successfully loaded
    pub loaded: usize,
    /// Plugins that failed to load
    pub failed: usize,
    /// Plugins skipped due to validation
    pub skipped: usize,
    /// Loading start time
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Loading end time
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl PluginLoadStats {
    /// Record loading start
    pub fn start_loading(&mut self) {
        self.start_time = Some(chrono::Utc::now());
    }

    /// Record loading completion
    pub fn finish_loading(&mut self) {
        self.end_time = Some(chrono::Utc::now());
    }

    /// Record successful plugin load
    pub fn record_success(&mut self) {
        self.loaded += 1;
        self.discovered += 1;
    }

    /// Record failed plugin load
    pub fn record_failure(&mut self) {
        self.failed += 1;
        self.discovered += 1;
    }

    /// Record skipped plugin
    pub fn record_skipped(&mut self) {
        self.skipped += 1;
        self.discovered += 1;
    }

    /// Get loading duration
    pub fn duration(&self) -> Option<chrono::Duration> {
        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => Some(end - start),
            _ => None,
        }
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.discovered == 0 {
            1.0 // No plugins discovered means 100% success (no failures)
        } else {
            (self.loaded as f64 / self.discovered as f64) * 100.0
        }
    }

    /// Get total number of plugins processed
    pub fn total_plugins(&self) -> usize {
        self.loaded + self.failed + self.skipped
    }
}

/// Plugin discovery result
#[derive(Debug, Clone)]
pub struct PluginDiscovery {
    /// Plugin ID
    pub plugin_id: PluginId,
    /// Plugin manifest
    pub manifest: PluginManifest,
    /// Plugin file path
    pub path: String,
    /// Whether plugin is valid
    pub is_valid: bool,
    /// Validation errors (if any)
    pub errors: Vec<String>,
}

impl PluginDiscovery {
    /// Create successful discovery
    pub fn success(plugin_id: PluginId, manifest: PluginManifest, path: String) -> Self {
        Self {
            plugin_id,
            manifest,
            path,
            is_valid: true,
            errors: Vec::new(),
        }
    }

    /// Create failed discovery
    pub fn failure(plugin_id: PluginId, path: String, errors: Vec<String>) -> Self {
        let plugin_id_clone = PluginId(plugin_id.0.clone());
        Self {
            plugin_id,
            manifest: PluginManifest::new(PluginInfo::new(
                plugin_id_clone,
                PluginVersion::new(0, 0, 0),
                "Unknown",
                "Plugin failed to load",
                PluginAuthor::new("unknown"),
            )),
            path,
            is_valid: false,
            errors,
        }
    }

    /// Check if discovery was successful
    pub fn is_success(&self) -> bool {
        self.is_valid
    }

    /// Get first error (if any)
    pub fn first_error(&self) -> Option<&str> {
        self.errors.first().map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_loader_error_types() {
        let load_error = PluginLoaderError::LoadError {
            message: "test error".to_string(),
        };
        assert!(matches!(load_error, PluginLoaderError::LoadError { .. }));

        let validation_error = PluginLoaderError::ValidationError {
            message: "validation failed".to_string(),
        };
        assert!(matches!(validation_error, PluginLoaderError::ValidationError { .. }));
    }

    #[test]
    fn test_plugin_discovery_success() {
        let plugin_id = PluginId("test-plugin".to_string());
        let manifest = PluginManifest::new(PluginInfo::new(
            plugin_id.clone(),
            PluginVersion::new(1, 0, 0),
            "Test Plugin",
            "A test plugin",
            PluginAuthor::new("test-author"),
        ));

        let result = PluginDiscovery::success(plugin_id, manifest, "/path/to/plugin".to_string());

        assert!(result.is_success());
        assert!(result.first_error().is_none());
    }

    #[test]
    fn test_plugin_discovery_failure() {
        let plugin_id = PluginId("failing-plugin".to_string());
        let errors = vec!["Error 1".to_string(), "Error 2".to_string()];

        let result =
            PluginDiscovery::failure(plugin_id, "/path/to/plugin".to_string(), errors.clone());

        assert!(!result.is_success());
        assert_eq!(result.first_error(), Some("Error 1"));
        assert_eq!(result.errors.len(), 2);
    }

    #[test]
    fn test_module_exports() {
        // Verify main types are accessible
        let _ = std::marker::PhantomData::<PluginLoader>;
        let _ = std::marker::PhantomData::<PluginRegistry>;
        let _ = std::marker::PhantomData::<PluginValidator>;
        // Compilation test - if this compiles, the types are properly defined
    }
}
