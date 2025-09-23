//! Plugin manifest and metadata handling
//!
//! This module defines the plugin manifest format and provides utilities
//! for loading, validating, and managing plugin metadata.

// Sub-modules
pub mod models;
pub mod schema;
pub mod loader;

// Re-export main types and functions for convenience
pub use models::{PluginManifest, PluginInfo, PluginAuthor, PluginDependency};
pub use schema::{ConfigSchema, ConfigProperty, PropertyType, PropertyValidation};
pub use loader::ManifestLoader;
