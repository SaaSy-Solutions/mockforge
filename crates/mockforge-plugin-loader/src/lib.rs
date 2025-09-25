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

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

// Import types from plugin core
use mockforge_plugin_core::{
    PluginId, PluginManifest, PluginInfo, PluginVersion, PluginAuthor,
    PluginInstance
};

pub mod loader;
pub mod validator;
pub mod sandbox;
pub mod registry;

/// Re-export commonly used types
pub use loader::*;
pub use validator::*;
pub use sandbox::*;
pub use registry::*;

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
            plugin_dirs: vec![
                "~/.mockforge/plugins".to_string(),
                "./plugins".to_string(),
            ],
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
