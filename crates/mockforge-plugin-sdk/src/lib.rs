//! # MockForge Plugin SDK
//!
//! Official SDK for developing MockForge plugins with ease.
//!
//! This SDK provides:
//! - Helper macros for plugin creation
//! - Builder patterns for manifests
//! - Testing utilities
//! - Type-safe plugin development
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use mockforge_plugin_sdk::{export_plugin, prelude::*, Result as PluginCoreResult};
//! use std::collections::HashMap;
//!
//! #[derive(Debug, Default)]
//! pub struct MyPlugin;
//!
//! #[async_trait]
//! impl AuthPlugin for MyPlugin {
//!     fn capabilities(&self) -> PluginCapabilities {
//!         PluginCapabilities::default()
//!     }
//!
//!     async fn initialize(&self, _config: &AuthPluginConfig) -> PluginCoreResult<()> {
//!         Ok(())
//!     }
//!
//!     async fn authenticate(
//!         &self,
//!         _context: &PluginContext,
//!         _request: &AuthRequest,
//!         _config: &AuthPluginConfig,
//!     ) -> PluginCoreResult<PluginResult<AuthResponse>> {
//!         let identity = UserIdentity::new("user123");
//!         let response = AuthResponse::success(identity, HashMap::new());
//!         Ok(PluginResult::success(response, 0))
//!     }
//!
//!     fn validate_config(&self, _config: &AuthPluginConfig) -> PluginCoreResult<()> {
//!         Ok(())
//!     }
//!
//!     fn supported_schemes(&self) -> Vec<String> {
//!         vec!["basic".to_string()]
//!     }
//!
//!     async fn cleanup(&self) -> PluginCoreResult<()> {
//!         Ok(())
//!     }
//! }
//!
//! export_plugin!(MyPlugin);
//! ```

pub mod builders;
pub mod macros;
pub mod prelude;

#[cfg(feature = "testing")]
pub mod testing;

// Re-export core plugin types
pub use mockforge_plugin_core::*;

/// SDK version
pub const SDK_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Recommended WASM target
pub const WASM_TARGET: &str = "wasm32-wasi";

/// Plugin SDK result type
pub type SdkResult<T> = std::result::Result<T, SdkError>;

/// SDK-specific errors
#[derive(Debug, thiserror::Error)]
pub enum SdkError {
    /// Plugin configuration error
    #[error("Plugin configuration error: {0}")]
    ConfigError(String),

    /// Manifest generation error
    #[error("Manifest generation error: {0}")]
    ManifestError(String),

    /// Build error
    #[error("Build error: {0}")]
    BuildError(String),

    /// Template error
    #[error("Template error: {0}")]
    TemplateError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl SdkError {
    /// Create a configuration error
    pub fn config<S: Into<String>>(msg: S) -> Self {
        Self::ConfigError(msg.into())
    }

    /// Create a manifest error
    pub fn manifest<S: Into<String>>(msg: S) -> Self {
        Self::ManifestError(msg.into())
    }

    /// Create a build error
    pub fn build<S: Into<String>>(msg: S) -> Self {
        Self::BuildError(msg.into())
    }

    /// Create a template error
    pub fn template<S: Into<String>>(msg: S) -> Self {
        Self::TemplateError(msg.into())
    }

    /// Create a serialization error
    pub fn serialization<S: Into<String>>(msg: S) -> Self {
        Self::SerializationError(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // SDK constants tests
    #[test]
    fn test_sdk_version() {
        assert!(!SDK_VERSION.is_empty());
    }

    #[test]
    fn test_wasm_target() {
        assert_eq!(WASM_TARGET, "wasm32-wasi");
    }

    // SdkError tests
    #[test]
    fn test_sdk_error_config() {
        let error = SdkError::config("missing field");
        assert_eq!(error.to_string(), "Plugin configuration error: missing field");
    }

    #[test]
    fn test_sdk_error_manifest() {
        let error = SdkError::manifest("invalid version");
        assert_eq!(error.to_string(), "Manifest generation error: invalid version");
    }

    #[test]
    fn test_sdk_error_build() {
        let error = SdkError::build("compilation failed");
        assert_eq!(error.to_string(), "Build error: compilation failed");
    }

    #[test]
    fn test_sdk_error_template() {
        let error = SdkError::template("invalid template");
        assert_eq!(error.to_string(), "Template error: invalid template");
    }

    #[test]
    fn test_sdk_error_serialization() {
        let error = SdkError::serialization("JSON error");
        assert_eq!(error.to_string(), "Serialization error: JSON error");
    }

    #[test]
    fn test_sdk_error_from_io() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let sdk_error: SdkError = io_error.into();
        assert!(matches!(sdk_error, SdkError::IoError(_)));
        assert!(sdk_error.to_string().contains("IO error"));
    }

    #[test]
    fn test_sdk_error_debug() {
        let error = SdkError::config("test");
        let debug = format!("{:?}", error);
        assert!(debug.contains("ConfigError"));
    }

    #[test]
    fn test_sdk_error_config_with_string() {
        let msg = String::from("config error message");
        let error = SdkError::config(msg);
        assert!(error.to_string().contains("config error message"));
    }

    #[test]
    fn test_sdk_error_manifest_with_string() {
        let msg = String::from("manifest error message");
        let error = SdkError::manifest(msg);
        assert!(error.to_string().contains("manifest error message"));
    }

    #[test]
    fn test_sdk_error_build_with_string() {
        let msg = String::from("build error message");
        let error = SdkError::build(msg);
        assert!(error.to_string().contains("build error message"));
    }

    // SdkResult tests
    #[test]
    fn test_sdk_result_ok() {
        let result: SdkResult<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_sdk_result_err() {
        let result: SdkResult<i32> = Err(SdkError::config("test"));
        assert!(result.is_err());
    }

    // Test error variants
    #[test]
    fn test_all_error_variants_display() {
        let errors = vec![
            SdkError::ConfigError("config".to_string()),
            SdkError::ManifestError("manifest".to_string()),
            SdkError::BuildError("build".to_string()),
            SdkError::TemplateError("template".to_string()),
            SdkError::SerializationError("serialization".to_string()),
        ];

        for error in errors {
            let display = error.to_string();
            assert!(!display.is_empty());
        }
    }
}
