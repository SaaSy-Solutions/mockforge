//! Async Token Resolver Trait Definition
//!
//! This module defines the async trait for token resolvers,
//! providing a proper async interface for plugin-based token resolution.

use crate::types::{PluginError, PluginMetadata, ResolutionContext};
use async_trait::async_trait;

/// Async token resolver trait for plugins
#[async_trait]
pub trait TokenResolver: Send + Sync {
    /// Check if this resolver can handle a given token
    fn can_resolve(&self, token: &str) -> bool;

    /// Resolve a token to its value asynchronously
    async fn resolve_token(
        &self,
        token: &str,
        context: &ResolutionContext,
    ) -> Result<String, PluginError>;

    /// Get plugin metadata
    fn get_metadata(&self) -> PluginMetadata;

    /// Validate plugin configuration
    fn validate(&self) -> Result<(), PluginError> {
        Ok(())
    }
}
