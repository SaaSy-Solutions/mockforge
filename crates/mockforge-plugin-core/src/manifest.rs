//! Plugin manifest and metadata handling
//!
//! This module defines the plugin manifest format and provides utilities
//! for loading, validating, and managing plugin metadata.

// Sub-modules
pub mod loader;
pub mod models;
pub mod schema;

// Re-export main types and functions for convenience
pub use loader::ManifestLoader;
pub use models::{PluginAuthor, PluginDependency, PluginInfo, PluginManifest};
pub use schema::{ConfigProperty, ConfigSchema, PropertyType, PropertyValidation};
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Test that main types are accessible
        let _ = std::marker::PhantomData::<PluginManifest>;
        let _ = std::marker::PhantomData::<ManifestLoader>;
        let _ = std::marker::PhantomData::<ConfigSchema>;
        assert!(true);
    }
}
