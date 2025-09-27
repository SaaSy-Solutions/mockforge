//! # MockForge Plugin Core
//!
//! Core traits and types for MockForge plugins, providing the foundation
//! for extensible functionality like custom token resolvers.

// Public modules
pub mod manifest;
pub mod response;
pub mod runtime;
pub mod template;
pub mod types;

// Re-export the async trait
pub mod async_trait;
pub use async_trait::TokenResolver;

// Re-export types
pub use response::*;
pub use types::*;

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
