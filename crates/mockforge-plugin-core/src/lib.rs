//! # MockForge Plugin Core
//!
//! Core traits, types, and runtime interfaces for the MockForge plugin system.
//!
//! This crate provides the foundational abstractions for building MockForge plugins,
//! including custom authentication handlers, data sources, response generators, and
//! template token resolvers.
//!
//! ## Overview
//!
//! MockForge uses a WebAssembly-based plugin system that allows developers to extend
//! its functionality without modifying the core application. Plugins are sandboxed for
//! security and can be loaded/unloaded at runtime.
//!
//! ## Plugin Types
//!
//! The plugin system supports several categories of plugins:
//!
//! - **Authentication Plugins**: Custom authentication and authorization logic
//! - **Data Source Plugins**: Connect to external data sources for realistic test data
//! - **Response Plugins**: Generate custom responses based on request data
//! - **Template Plugins**: Custom token resolvers for the template system
//!
//! ## Quick Start
//!
//! To create a plugin, implement one or more of the plugin traits:
//!
//! ```rust,ignore
//! use mockforge_plugin_core::{TokenResolver, ResolutionContext, PluginError};
//!
//! pub struct MyPlugin;
//!
//! #[async_trait::async_trait]
//! impl TokenResolver for MyPlugin {
//!     async fn can_resolve(&self, token: &str) -> bool {
//!         token.starts_with("my_")
//!     }
//!
//!     async fn resolve_token(
//!         &self,
//!         token: &str,
//!         context: &ResolutionContext,
//!     ) -> Result<String, PluginError> {
//!         // Custom resolution logic
//!         Ok(format!("resolved: {}", token))
//!     }
//!
//!     async fn get_metadata(&self) -> PluginMetadata {
//!         PluginMetadata::new("My custom plugin")
//!             .with_capability("token_resolver")
//!             .with_prefix("my_")
//!     }
//! }
//! ```
//!
//! ## Key Types
//!
//! - [`PluginId`]: Unique identifier for plugins
//! - [`PluginVersion`]: Semantic version information
//! - [`PluginManifest`]: Plugin metadata and dependencies
//! - [`PluginError`]: Common error types
//! - [`ResolutionContext`]: Context for token resolution
//!
//! ## Features
//!
//! - Type-safe plugin interfaces
//! - Comprehensive error handling
//! - Built-in validation and health checks
//! - Async/await support
//! - Security sandboxing via WebAssembly
//!
//! ## For Plugin Developers
//!
//! For a more convenient development experience, consider using the
//! [`mockforge-plugin-sdk`](https://docs.rs/mockforge-plugin-sdk) crate, which provides
//! helper macros, testing utilities, and additional conveniences.
//!
//! ## Documentation
//!
//! - [Plugin Development Guide](https://docs.mockforge.dev/plugins)
//! - [API Reference](https://docs.rs/mockforge-plugin-core)
//! - [Example Plugins](https://github.com/SaaSy-Solutions/mockforge/tree/main/examples/plugins)

// Public modules
pub mod auth;
pub mod client_generator;
pub mod datasource;
pub mod error;
pub mod manifest;
pub mod plugins;
pub mod response;
pub mod runtime;
pub mod template;
pub mod types;

// Re-export the async trait
pub mod async_trait;
pub use async_trait::TokenResolver;

// Re-export types
pub use auth::*;
pub use client_generator::{
    ClientGenerationResult, ClientGeneratorConfig, ClientGeneratorPlugin,
    ClientGeneratorPluginConfig, GeneratedFile, GenerationMetadata, OpenApiSpec,
};
pub use datasource::{
    DataConnection, DataQuery, DataResult, DataSourcePlugin, DataSourcePluginConfig,
};
pub use plugins::{ReactClientGenerator, VueClientGenerator};
pub use response::{
    ResponseData, ResponseModifierConfig, ResponseModifierPlugin, ResponsePlugin,
    ResponsePluginConfig, ResponseRequest,
};
pub use template::*;
pub use types::*;

// Re-export helper modules with qualified names to avoid ambiguity
pub use datasource::helpers as datasource_helpers;
pub use response::helpers as response_helpers;

// Re-export common types for backwards compatibility
pub use types::{
    PluginAuthor, PluginHealth, PluginId, PluginInfo, PluginManifest, PluginMetadata, PluginState,
    PluginVersion,
};

// Additional utility traits (commented out as we're using the async trait)
// pub trait SyncTokenResolver {
//     /// Check if this resolver can handle a given token
//     fn can_resolve(&self, token: &str) -> bool;
//
//     /// Resolve a token to its value synchronously
//     fn resolve_token(&self, token: &str, context: &ResolutionContext) -> Result<String, PluginError>;
//
//     /// Get plugin metadata
//     fn get_metadata(&self) -> PluginMetadata;
//
//     /// Validate plugin configuration
//     fn validate(&self) -> Result<(), PluginError> {
//         Ok(())
//     }
// }

// Re-export additional types for backwards compatibility
pub use types::{PluginError, PluginInstance, RequestMetadata, ResolutionContext, Result};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_id() {
        let id = PluginId::new("test-plugin");
        assert_eq!(id.as_str(), "test-plugin");
    }

    #[test]
    fn test_plugin_version() {
        let version = PluginVersion::new(1, 2, 3);
        assert_eq!(version.to_string(), "1.2.3");
    }

    #[test]
    fn test_plugin_info() {
        let id = PluginId::new("example");
        let version = PluginVersion::new(1, 0, 0);
        let author = PluginAuthor {
            name: "Author".to_string(),
            email: Some("author@example.com".to_string()),
        };
        let info = PluginInfo {
            id: id.clone(),
            version: version.clone(),
            name: "Example Plugin".to_string(),
            description: "Description".to_string(),
            author: author.clone(),
        };

        assert_eq!(info.id.as_str(), "example");
        assert_eq!(info.name, "Example Plugin");
        assert_eq!(info.description, "Description");
        assert_eq!(info.author.name, "Author");
        assert_eq!(info.author.email, Some("author@example.com".to_string()));
    }

    #[test]
    fn test_resolution_context() {
        let context = ResolutionContext::new();
        assert!(!context.environment.is_empty());
        assert!(context.request_context.is_none());
    }

    #[test]
    fn test_request_metadata() {
        let request =
            RequestMetadata::new("GET", "/api/users").with_header("Accept", "application/json");

        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/users");
        assert_eq!(request.headers.get("Accept"), Some(&"application/json".to_string()));
    }
}

// Include client generator tests
#[cfg(test)]
mod client_generator_tests;
