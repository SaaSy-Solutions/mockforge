//! # Token Resolver Plugin Integration
//!
//! This module provides integration between token resolvers and the template engine,
//! allowing plugins to register custom token resolution handlers.

use crate::config::Config;
use async_trait::async_trait;
use mockforge_plugin_core::*;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Plugin-based token resolver that integrates with the template engine
#[derive(Debug)]
pub struct PluginTokenResolver {
    /// Configuration reference
    config: Arc<Config>,
    /// Registered token resolver plugins
    resolvers: Arc<RwLock<HashMap<String, Arc<dyn TokenResolver>>>>,
    /// Plugin loading context
    plugin_context: Option<Arc<mockforge_plugin_loader::PluginLoadContext>>,
}

impl PluginTokenResolver {
    /// Create a new plugin token resolver
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            config,
            resolvers: Arc::new(RwLock::new(HashMap::new())),
            plugin_context: None,
        }
    }

    /// Set the plugin loading context
    pub fn with_plugin_context(mut self, context: Arc<mockforge_plugin_loader::PluginLoadContext>) -> Self {
        self.plugin_context = Some(context);
        self
    }

    /// Register a token resolver plugin
    pub async fn register_resolver(&self, name: &str, resolver: Arc<dyn TokenResolver>) -> Result<(), PluginError> {
        debug!("Registering token resolver plugin: {}", name);

        // Validate the plugin
        resolver.validate()?;

        // Register it
        let mut resolvers = self.resolvers.write().await;
        resolvers.insert(name.to_string(), resolver);

        info!("Successfully registered token resolver plugin: {}", name);
        Ok(())
    }

    /// Unregister a token resolver plugin
    pub async fn unregister_resolver(&self, name: &str) -> Result<(), PluginError> {
        debug!("Unregistering token resolver plugin: {}", name);

        let mut resolvers = self.resolvers.write().await;
        if resolvers.remove(name).is_some() {
            info!("Successfully unregistered token resolver plugin: {}", name);
            Ok(())
        } else {
            Err(PluginError::config_error(&format!("Resolver '{}' not found", name)))
        }
    }

    /// Resolve a token using registered plugins
    pub async fn resolve_token(&self, token: &str, context: &ResolutionContext) -> Option<String> {
        debug!("Attempting to resolve token: {}", token);

        let resolvers = self.resolvers.read().await;
        let mut best_match: Option<(String, usize)> = None;

        // Find the best matching resolver based on prefix priority
        for (name, resolver) in resolvers.iter() {
            if resolver.can_resolve(token) {
                let priority = self.get_resolver_priority(name);

                match best_match {
                    None => best_match = Some((name.clone(), priority)),
                    Some((_, current_priority)) if priority > current_priority => {
                        best_match = Some((name.clone(), priority));
                    }
                    _ => {}
                }
            }
        }

        if let Some((resolver_name, _)) = best_match {
            if let Some(resolver) = resolvers.get(&resolver_name) {
                match resolver.resolve_token(token, context) {
                    Ok(value) => {
                        debug!("Successfully resolved token '{}' using resolver '{}'", token, resolver_name);
                        return Some(value);
                    }
                    Err(e) => {
                        warn!("Token resolution failed for '{}' using resolver '{}': {}",
                              token, resolver_name, e);
                        return None;
                    }
                }
            }
        }

        debug!("No resolver found for token: {}", token);
        None
    }

    /// Get priority for a resolver (higher is better)
    fn get_resolver_priority(&self, resolver_name: &str) -> usize {
        // Priority order: custom > env > db > time > generic
        match resolver_name {
            name if name.contains("custom") => 100,
            name if name.contains("env") => 90,
            name if name.contains("db") || name.contains("database") => 80,
            name if name.contains("time") || name.contains("datetime") => 70,
            name if name.contains("stats") || name.contains("metrics") => 60,
            _ => 50,
        }
    }

    /// Get metadata about registered resolvers
    pub async fn get_resolver_metadata(&self) -> HashMap<String, PluginMetadata> {
        let mut metadata = HashMap::new();
        let resolvers = self.resolvers.read().await;

        for (name, resolver) in resolvers.iter() {
            metadata.insert(name.clone(), resolver.get_metadata());
        }

        metadata
    }

    /// List registered resolver names
    pub async fn list_resolvers(&self) -> Vec<String> {
        let resolvers = self.resolvers.read().await;
        resolvers.keys().cloned().collect()
    }

    /// Check if a token can be resolved
    pub async fn can_resolve_token(&self, token: &str) -> bool {
        let resolvers = self.resolvers.read().await;
        resolvers.values().any(|resolver| resolver.can_resolve(token))
    }
}

#[async_trait]
impl TokenResolver for PluginTokenResolver {
    fn can_resolve(&self, token: &str) -> bool {
        // This is a sync check - in practice, use can_resolve_token
        // For now, do a simple prefix check
        token.starts_with("{{") && token.ends_with("}}")
    }

    fn resolve_token(&self, token: &str, context: &ResolutionContext) -> Result<String, PluginError> {
        // For async resolution, use the async method
        // This is a limitation of the trait - we'd need to make it async
        Err(PluginError::resolution_failed("Use async resolve_token method for plugin resolution"))
    }

    fn get_metadata(&self) -> PluginMetadata {
        PluginMetadata::new("Plugin-based token resolver with registered plugins")
            .with_capability("token-resolution")
            .with_capability("plugin-integration")
    }
}

/// Integration point for template engine to use plugin resolvers
pub struct TemplatePluginIntegration {
    /// The plugin token resolver
    resolver: Arc<PluginTokenResolver>,
}

impl TemplatePluginIntegration {
    /// Create a new template plugin integration
    pub fn new(resolver: Arc<PluginTokenResolver>) -> Self {
        Self { resolver }
    }

    /// Process a template string with plugin resolution
    pub async fn process_template(
        &self,
        template: &str,
        context: &ResolutionContext,
    ) -> Result<String, PluginError> {
        let mut result = template.to_string();

        // Find all token patterns {{...}}
        let token_pattern = regex::Regex::new(r"\{\{([^}]+)\}\}").map_err(|e| {
            PluginError::config_error(&format!("Invalid regex pattern: {}", e))
        })?;

        // Replace tokens asynchronously
        for capture in token_pattern.captures_iter(template) {
            let token = capture.get(1).unwrap().as_str();

            if let Some(resolved) = self.resolver.resolve_token(token, context).await {
                let token_pattern = format!("{{{{{}}}}}", regex::escape(token));
                if let Ok(replace_regex) = regex::Regex::new(&token_pattern) {
                    result = replace_regex.replace(&result, &resolved).to_string();
                }
            }
        }

        Ok(result)
    }

    /// Extract tokens that need resolution
    pub async fn extract_tokens(&self, template: &str) -> Vec<String> {
        let mut tokens = Vec::new();

        let token_pattern = regex::Regex::new(r"\{\{([^}]+)\}\}").unwrap_or_else(|_| regex::Regex::new(r"\{\{([^}]+)\}\}").unwrap());

        for capture in token_pattern.captures_iter(template) {
            let token = capture.get(1).unwrap().as_str().to_string();
            if self.resolver.can_resolve_token(&token).await {
                tokens.push(token);
            }
        }

        tokens
    }

    /// Get resolution statistics
    pub async fn get_resolution_stats(&self) -> HashMap<String, usize> {
        let resolvers = self.resolver.get_resolver_metadata().await;
        let mut stats = HashMap::new();
        stats.insert("total_resolvers".to_string(), resolvers.len());

        // Count resolvers by type
        let mut types = HashMap::new();
        for metadata in resolvers.values() {
            for prefix in &metadata.supported_prefixes {
                *types.entry(prefix.clone()).or_insert(0) += 1;
            }
        }

        for (prefix, count) in types {
            stats.insert(format!("{}_resolvers", prefix), count);
        }

        stats
    }
}

/// Factory function to create standard token resolvers
pub struct TokenResolverFactory;

impl TokenResolverFactory {
    /// Create an environment variable resolver
    pub fn create_env_resolver() -> Result<Arc<dyn TokenResolver>, PluginError> {
        Ok(Arc::new(EnvironmentResolver::new()?))
    }

    /// Create a time-based resolver
    pub fn create_time_resolver() -> Arc<dyn TokenResolver> {
        Arc::new(TimeResolver::new())
    }

    /// Create a UUID resolver
    pub fn create_uuid_resolver() -> Arc<dyn TokenResolver> {
        Arc::new(UuidResolver::new())
    }

    /// Create a stats resolver
    pub fn create_stats_resolver(stats_store: Option<HashMap<String, serde_json::Value>>) -> Arc<dyn TokenResolver> {
        Arc::new(StatsResolver::new(stats_store.unwrap_or_default()))
    }
}

/// Environment variable token resolver
#[derive(Debug)]
struct EnvironmentResolver {
    // No state needed
}

impl EnvironmentResolver {
    fn new() -> Result<Self, PluginError> {
        Ok(Self {})
    }
}

#[async_trait]
impl TokenResolver for EnvironmentResolver {
    fn can_resolve(&self, token: &str) -> bool {
        token.starts_with("env:") || token.starts_with("ENV:")
    }

    fn resolve_token(&self, token: &str, _context: &ResolutionContext) -> Result<String, PluginError> {
        let env_key = if token.starts_with("env:") {
            &token[4..]
        } else {
            &token[4..]
        };

        std::env::var(env_key).map_err(|_| {
            PluginError::resolution_failed(&format!("Environment variable '{}' not found", env_key))
        })
    }

    fn get_metadata(&self) -> PluginMetadata {
        PluginMetadata::new("Environment variable resolver for config and secrets")
            .with_capability("env-vars")
            .with_prefix("env")
    }
}

/// Time-based token resolver
#[derive(Debug)]
struct TimeResolver {
    // No state needed
}

impl TimeResolver {
    fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl TokenResolver for TimeResolver {
    fn can_resolve(&self, token: &str) -> bool {
        token.starts_with("time:") || token.starts_with("datetime:")
    }

    fn resolve_token(&self, token: &str, _context: &ResolutionContext) -> Result<String, PluginError> {
        let time_spec = if token.starts_with("time:") {
            &token[5..]
        } else {
            &token[9..]
        };

        let now = chrono::Utc::now();

        match time_spec {
            "iso8601" | "rfc3339" => Ok(now.to_rfc3339()),
            "timestamp" => Ok(now.timestamp().to_string()),
            "unix" => Ok(now.timestamp().to_string()),
            "date" => Ok(now.format("%Y-%m-%d").to_string()),
            "time" => Ok(now.format("%H:%M:%S").to_string()),
            "datetime" => Ok(now.format("%Y-%m-%d %H:%M:%S").to_string()),
            "business-hours" => {
                let hour = now.hour();
                if hour >= 9 && hour < 17 {
                    Ok("Open".to_string())
                } else {
                    Ok("Closed".to_string())
                }
            }
            _ => Err(PluginError::invalid_token(token)),
        }
    }

    fn get_metadata(&self) -> PluginMetadata {
        PluginMetadata::new("Time and date resolver for dynamic timestamps")
            .with_capability("datetime")
            .with_prefix("time")
            .with_prefix("datetime")
    }
}

/// UUID token resolver
#[derive(Debug)]
struct UuidResolver {
    // No state needed
}

impl UuidResolver {
    fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl TokenResolver for UuidResolver {
    fn can_resolve(&self, token: &str) -> bool {
        token == "uuid:v4" || token == "uuid" || token == "id" || token == "session:id"
    }

    fn resolve_token(&self, token: &str, _context: &ResolutionContext) -> Result<String, PluginError> {
        match token {
            "uuid:v4" | "uuid" | "id" | "session:id" => {
                Ok(uuid::Uuid::new_v4().to_string())
            }
            _ => Err(PluginError::invalid_token(token)),
        }
    }

    fn get_metadata(&self) -> PluginMetadata {
        PluginMetadata::new("UUID generator for unique identifiers")
            .with_capability("uuid-generation")
            .with_prefix("uuid")
            .with_prefix("id")
    }
}

/// Statistics token resolver
#[derive(Debug)]
struct StatsResolver {
    stats_store: HashMap<String, serde_json::Value>,
}

impl StatsResolver {
    fn new(stats_store: HashMap<String, serde_json::Value>) -> Self {
        Self { stats_store }
    }

    fn increment_stat(&mut self, key: &str) {
        let value = self.stats_store.entry(key.to_string())
            .or_insert(serde_json::Value::Number(0.into()));

        if let Some(num) = value.as_i64() {
            *value = serde_json::Value::Number((num + 1).into());
        }
    }
}

#[async_trait]
impl TokenResolver for StatsResolver {
    fn can_resolve(&self, token: &str) -> bool {
        token.starts_with("stats:") ||
        token.starts_with("metrics:") ||
        token.starts_with("counter:")
    }

    fn resolve_token(&self, token: &str, _context: &ResolutionContext) -> Result<String, PluginError> {
        let stat_key = if let Some(key) = token.strip_prefix("stats:") {
            key
        } else if let Some(key) = token.strip_prefix("metrics:") {
            key
        } else if let Some(key) = token.strip_prefix("counter:") {
            key
        } else {
            return Err(PluginError::invalid_token(token));
        };

        match stat_key {
            "requests" | "active-users" | "total-users" => {
                // Mock values for common stats
                match stat_key {
                    "requests" => Ok("15423".to_string()),
                    "active-users" => Ok("89".to_string()),
                    "total-users" => Ok("1274".to_string()),
                    _ => Ok("0".to_string()),
                }
            }
            key if self.stats_store.contains_key(key) => {
                if let Some(value) = self.stats_store.get(key) {
                    Ok(value.to_string())
                } else {
                    Err(PluginError::resolution_failed(&format!("Stat '{}' not found", key)))
                }
            }
            _ => Err(PluginError::resolution_failed(&format!("Unknown stat '{}'", stat_key))),
        }
    }

    fn get_metadata(&self) -> PluginMetadata {
        PluginMetadata::new("Statistics and metrics resolver")
            .with_capability("stats")
            .with_prefix("stats")
            .with_prefix("metrics")
    }
}

/// Convenience functions for creating standard resolvers
pub mod presets {
    use super::*;

    /// Create all standard token resolvers
    pub async fn create_standard_resolvers() -> Vec<(String, Arc<dyn TokenResolver>)> {
        vec![
            ("env-resolver".to_string(), TokenResolverFactory::create_env_resolver().unwrap()),
            ("time-resolver".to_string(), TokenResolverFactory::create_time_resolver()),
            ("uuid-resolver".to_string(), TokenResolverFactory::create_uuid_resolver()),
            ("stats-resolver".to_string(), TokenResolverFactory::create_stats_resolver(None)),
        ]
    }

    /// Create minimal resolvers for basic functionality
    pub fn create_minimal_resolvers() -> Vec<(String, Arc<dyn TokenResolver>)> {
        vec![
            ("env-resolver".to_string(), TokenResolverFactory::create_env_resolver().unwrap()),
            ("time-resolver".to_string(), TokenResolverFactory::create_time_resolver()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_token_resolver_creation() {
        let config = Arc::new(Config::default());
        let resolver = PluginTokenResolver::new(config);
        assert!(resolver.list_resolvers().is_empty());
    }

    #[tokio::test]
    async fn test_env_resolver() {
        let resolver = EnvironmentResolver::new().unwrap();

        // Test that it can resolve env tokens
        assert!(resolver.can_resolve("env:PATH"));
        assert!(!resolver.can_resolve("db:users"));

        // Test metadata
        let metadata = resolver.get_metadata();
        assert_eq!(metadata.description, "Environment variable resolver for config and secrets");
        assert!(metadata.supported_prefixes.contains(&"env".to_string()));
    }

    #[tokio::test]
    async fn test_time_resolver() {
        let resolver = TimeResolver::new();

        // Test that it can resolve time tokens
        assert!(resolver.can_resolve("time:iso8601"));
        assert!(resolver.can_resolve("datetime:date"));
        assert!(!resolver.can_resolve("env:PATH"));

        // Test resolution
        let context = ResolutionContext::new();
        let result = resolver.resolve_token("time:iso8601", &context).unwrap();
        assert!(!result.is_empty());
        assert!(result.contains("T")); // ISO 8601 format should contain 'T'
    }

    #[tokio::test]
    async fn test_uuid_resolver() {
        let resolver = UuidResolver::new();

        // Test that it can resolve UUID tokens
        assert!(resolver.can_resolve("uuid:v4"));
        assert!(resolver.can_resolve("id"));
        assert!(!resolver.can_resolve("env:PATH"));

        // Test resolution produces valid UUID
        let context = ResolutionContext::new();
        let result = resolver.resolve_token("uuid:v4", &context).unwrap();
        assert!(uuid::Uuid::parse_str(&result).is_ok());
    }

    #[tokio::test]
    async fn test_stats_resolver() {
        let resolver = StatsResolver::new(HashMap::new());

        // Test that it can resolve stats tokens
        assert!(resolver.can_resolve("stats:requests"));
        assert!(resolver.can_resolve("metrics:active-users"));
        assert!(!resolver.can_resolve("env:PATH"));

        // Test resolution
        let context = ResolutionContext::new();
        let result = resolver.resolve_token("stats:requests", &context).unwrap();
        assert!(result == "15423"); // Mock value
    }
}
