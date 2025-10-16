//! # Plugin Integration for MockForge Core
//!
//! This module provides the integration point between token resolvers and
//! the existing template engine in MockForge Core.

use crate::config::Config;
use crate::token_resolvers::*;
use mockforge_plugin_core::{TokenResolver, PluginMetadata};
use std::sync::Arc;

/// Plugin-enabled template engine that integrates with custom token resolvers
pub struct PluginTemplateEngine {
    /// Base template engine
    base_engine: crate::templating::TemplateEngine,
    /// Plugin token resolver
    plugin_resolver: Arc<PluginTokenResolver>,
    /// Plugin integration utilities
    integration: Arc<TemplatePluginIntegration>,
}

impl PluginTemplateEngine {
    /// Create a new plugin-enabled template engine
    pub fn new(config: &Config) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let base_engine = crate::templating::TemplateEngine::new(config.clone())?;
        let plugin_resolver = Arc::new(PluginTokenResolver::new(Arc::new(config.clone())));
        let integration = Arc::new(TemplatePluginIntegration::new(Arc::clone(&plugin_resolver)));

        Ok(Self {
            base_engine,
            plugin_resolver,
            integration,
        })
    }

    /// Register a token resolver plugin
    pub async fn register_plugin(&self, name: &str, resolver: Arc<dyn TokenResolver>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.plugin_resolver.register_resolver(name, resolver).await.map_err(|e| e.into())
    }

    /// Unregister a token resolver plugin
    pub async fn unregister_plugin(&self, name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.plugin_resolver.unregister_resolver(name).await.map_err(|e| e.into())
    }

    /// Process a template string with both built-in and plugin resolvers
    pub async fn process_template(&self, template: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let context = mockforge_plugin_core::ResolutionContext::new();
        self.integration.process_template(template, &context).await.map_err(|e| e.into())
    }

    /// Process a template string with custom context
    pub async fn process_template_with_context(
        &self,
        template: &str,
        context: &mockforge_plugin_core::ResolutionContext,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        self.integration.process_template(template, context).await.map_err(|e| e.into())
    }

    /// Extract tokens that can be resolved by plugins
    pub async fn extract_resolveable_tokens(&self, template: &str) -> Vec<String> {
        self.integration.extract_tokens(template).await
    }

    /// Load and register standard token resolvers
    pub async fn load_standard_resolvers(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let factory = TokenResolverFactory;
        let standard_resolvers = presets::create_standard_resolvers().await;

        for (name, resolver) in standard_resolvers {
            self.register_plugin(&name, resolver).await?;
        }

        crate::tracing::info!("Loaded {} standard token resolver plugins", name);
        Ok(())
    }

    /// Get metadata about registered plugins
    pub async fn get_plugin_metadata(&self) -> std::collections::HashMap<String, PluginMetadata> {
        self.plugin_resolver.get_resolver_metadata().await
    }

    /// List registered plugin names
    pub async fn list_plugins(&self) -> Vec<String> {
        self.plugin_resolver.list_resolvers().await
    }

    /// Get template resolution statistics
    pub async fn get_resolution_stats(&self) -> std::collections::HashMap<String, usize> {
        self.integration.get_resolution_stats().await
    }
}

impl std::fmt::Debug for PluginTemplateEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginTemplateEngine")
            .field("base_engine", &"TemplateEngine")
            .field("plugin_count", &self.plugin_resolver.list_resolvers())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_resolvers::TokenResolverFactory;
    use mockforge_plugin_core::{ResolutionContext, TokenResolver};

    #[tokio::test]
    async fn test_plugin_template_engine_creation() {
        let config = &Config::default();
        let engine = PluginTemplateEngine::new(config).unwrap();

        // Should start with no registered plugins
        assert!(engine.list_plugins().await.is_empty());
    }

    #[tokio::test]
    async fn test_plugin_registration() {
        let config = &Config::default();
        let engine = PluginTemplateEngine::new(config).unwrap();

        // Register a standard resolver
        let resolver = TokenResolverFactory::create_env_resolver().unwrap();
        engine.register_plugin("test-env", resolver).await.unwrap();

        // Should now have one plugin
        let plugins = engine.list_plugins().await;
        assert_eq!(plugins.len(), 1);
        assert!(plugins.contains(&"test-env".to_string()));
    }

    #[tokio::test]
    async fn test_template_processing_with_plugins() {
        let config = &Config::default();
        let engine = PluginTemplateEngine::new(config).unwrap();

        // Register standard resolvers
        engine.load_standard_resolvers().await.unwrap();

        // Process a template with time token
        let template = "Current time is {{time:iso8601}} and UUID is {{uuid}}";
        let result = engine.process_template(template).await.unwrap();

        // Should have resolved the tokens
        assert!(!result.contains("{{time:iso8601}}"));
        assert!(!result.contains("{{uuid}}"));
        assert!(result.contains("Current time is"));
    }

    #[tokio::test]
    async fn test_token_extraction() {
        let config = &Config::default();
        let engine = PluginTemplateEngine::new(config).unwrap();

        // Load standard resolvers for extraction to work
        engine.load_standard_resolvers().await.unwrap();

        let template = "Hello {{time:iso8601}} and {{uuid:v4}} with {{unknown:token}}";
        let tokens = engine.extract_resolveable_tokens(template).await;

        // Should extract "{{time:iso8601}}" and "{{uuid:v4}}" but not "{{unknown:token}}"
        assert!(tokens.contains(&"{{time:iso8601}}".to_string()));
        assert!(tokens.contains(&"{{uuid:v4}}".to_string()));
        assert!(!tokens.contains(&"{{unknown:token}}".to_string()));
    }

    #[tokio::test]
    async fn test_custom_context_processing() {
        let config = &Config::default();
        let engine = PluginTemplateEngine::new(config).unwrap();

        // Register UUID resolver
        engine.register_plugin("uuid-resolver", TokenResolverFactory::create_uuid_resolver()).await.unwrap();

        // Create custom context with HTTP request metadata
        let context = ResolutionContext::new().with_request(
            mockforge_plugin_core::RequestMetadata::new("GET", "/api/test")
                .with_header("User-Agent", "TestAgent")
        );

        // Process template with custom context
        let template = "Processing {{uuid}} for {{env:HOME}}";
        let result = engine.process_template_with_context(template, &context).await.unwrap();

        // Should resolve UUID but environment variable may not be available in test
        assert!(!result.contains("{{uuid}}"));
        println!("Resolution result: {}", result);
    }
}
