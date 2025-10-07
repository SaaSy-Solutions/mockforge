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
//! use mockforge_plugin_sdk::prelude::*;
//!
//! #[derive(Debug)]
//! pub struct MyPlugin;
//!
//! #[async_trait]
//! impl AuthPlugin for MyPlugin {
//!     async fn authenticate(
//!         &self,
//!         context: &PluginContext,
//!         credentials: &AuthCredentials,
//!     ) -> PluginResult<AuthResult> {
//!         Ok(AuthResult::authenticated("user123"))
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
}
