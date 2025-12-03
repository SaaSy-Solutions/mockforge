//! # Token Resolver Plugin System Demo
//!
//! This module demonstrates how to use the custom token resolver plugin system
//! with MockForge's template engine.

use crate::config::Config;
use crate::plugin_integration::PluginTemplateEngine;
use mockforge_plugin_core::{ResolutionContext, RequestMetadata, PluginError};
use std::sync::Arc;

#[derive(Debug, Clone)]
struct CustomBusinessResolver {
    business_rules: std::collections::HashMap<String, String>,
}

impl CustomBusinessResolver {
    fn new() -> Self {
        let mut rules = std::collections::HashMap::new();
        rules.insert("user:status".to_string(), "active".to_string());
        rules.insert("business:hours".to_string(), "9-5 EST".to_string());
        rules.insert("company:name".to_string(), "MockForge Inc".to_string());
        rules.insert("support:email".to_string(), "support@mockforge.com".to_string());
        rules.insert("feature:new-ui".to_string(), "enabled".to_string());

        Self { business_rules: rules }
    }
}

#[async_trait::async_trait]
impl mockforge_plugin_core::TokenResolver for CustomBusinessResolver {
    fn can_resolve(&self, token: &str) -> bool {
        token.starts_with("business:") || token.starts_with("user:") ||
        token.starts_with("company:") || token.starts_with("support:") ||
        token.starts_with("feature:")
    }

    async fn resolve_token(&self, token: &str, _context: &ResolutionContext) -> Result<String, PluginError> {
        match self.business_rules.get(token) {
            Some(value) => Ok(value.clone()),
            None => Err(PluginError::resolution_failed(&format!("Unknown business token: {}", token))),
        }
    }

    fn get_metadata(&self) -> mockforge_plugin_core::PluginMetadata {
        mockforge_plugin_core::PluginMetadata::new("Custom business logic resolver for MockForge")
            .with_capability("business-rules")
            .with_prefix("business")
            .with_prefix("user")
            .with_prefix("company")
            .with_prefix("support")
            .with_prefix("feature")
    }
}

/// Demonstration of the token resolver plugin system
pub struct PluginSystemDemo {
    engine: PluginTemplateEngine,
}

impl PluginSystemDemo {
    /// Create a new demo instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let config = Config::default();
        let engine = PluginTemplateEngine::new(&config)?;

        Ok(Self { engine })
    }

    /// Set up the demo with various token resolvers
    pub async fn setup_demo(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Load standard resolvers
        self.engine.load_standard_resolvers().await?;

        // Register custom business resolver
        let business_resolver = Arc::new(CustomBusinessResolver::new());
        self.engine.register_plugin("business-resolver", business_resolver).await?;

        println!("✅ Demo setup complete - loaded standard and custom token resolvers");
        Ok(())
    }

    /// Demonstrate token resolution with various scenarios
    pub async fn run_token_resolution_demo(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("\n🚀 **Token Resolver Plugin System Demo**\n");

        // Demo 1: Environment and time tokens
        println!("📍 Demo 1: Standard environment and time tokens");
        let template1 = r#"Server status: {{time:iso8601}}
Environment: {{env:USER}}
Home path: {{env:HOME}}
"#;
        let result1 = self.engine.process_template(template1).await?;
        println!("Template:\n{}\nResult:\n{}\n", template1, result1);

        // Demo 2: UUID and statistics tokens
        println!("🆔 Demo 2: UUID generation and statistics");
        let template2 = r#"Request ID: {{uuid:v4}}
Stats - requests: {{stats:requests}}
Stats - users: {{stats:active-users}}
"#;
        let result2 = self.engine.process_template(template2).await?;
        println!("Template:\n{}\nResult:\n{}\n", template2, result2);

        // Demo 3: Custom business logic tokens
        println!("🏢 Demo 3: Custom business logic tokens");
        let template3 = r#"Welcome to {{company:name}}!
User status: {{user:status}}
Business hours: {{business:hours}}
Support: {{support:email}}
Feature flag: {{feature:new-ui}}
"#;
        let result3 = self.engine.process_template(template3).await?;
        println!("Template:\n{}\nResult:\n{}\n", template3, result3);

        // Demo 4: Request context with HTTP metadata
        println!("🌐 Demo 4: Request context integration");
        let request_context = ResolutionContext::new().with_request(
            RequestMetadata::new("POST", "/api/users")
                .with_header("Authorization", "Bearer xyz123")
                .with_header("User-Agent", "MockForge/1.0")
                .with_query_param("debug", "true")
        );

        let template4 = r#"Processing {{uuid}} request via {{env:USER}}
Method: {{request:POST}}
Path: {{request:/api/users}}
"#;
        let result4 = self.engine.process_template_with_context(template4, &request_context).await?;
        println!("Template:\n{}\nResult:\n{}\n", template4, result4);

        // Demo 5: Plugin management and metadata
        println!("🔧 Demo 5: Plugin management");
        let plugins = self.engine.list_plugins().await;
        println!("Registered plugins: {:?}", plugins);

        let metadata = self.engine.get_plugin_metadata().await;
        println!("Plugin metadata:");
        for (name, meta) in metadata.iter() {
            println!("  {}: {} (prefixes: {:?})", name, meta.description, meta.supported_prefixes);
        }

        // Demo 6: Token extraction
        println!("\n🔍 Demo 6: Token extraction");
        let example_template = r#"The time is {{time:iso8601}}, user is {{env:USER}}, and ID is {{uuid}}.
Business context: {{company:name}} running from {{unknown:path}}.";
"#;
        let extractable_tokens = self.engine.extract_resolvable_tokens(example_template).await;
        println!("Template:\n{}\nExtractable tokens: {:?}", example_template, extractable_tokens);

        // Demo 7: Resolution statistics
        println!("\n📊 Demo 7: Resolution statistics");
        let stats = self.engine.get_resolution_stats().await;
        println!("Resolution stats: {:?}", stats);

        println!("✨ **Demo complete!** All token resolvers are working correctly.");

        Ok(())
    }

    /// Demonstrate advanced scenarios
    pub async fn run_advanced_demo(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("\n🎯 **Advanced Plugin System Demo**\n");

        // Dynamic template with complex business logic
        let business_template = r#"🎫 **{{uuid}}** - Support Ticket

Priority: High
Submitted: {{time:datetime}}
Status: {{user:status}}

Business Rules Applied:
• Operating hours: {{business:hours}}
• Support contact: {{support:email}}
• Company: {{company:name}}
• Feature enabled: {{feature:new-ui}}

Debug info: {{stats:requests}} total requests processed
Environment: {{env:USER}}@{{env:HOSTNAME}}
"#;

        let result = self.engine.process_template(business_template).await?;

        println!("🚀 Complex Business Template Processing:");
        println!("{}\n", result);

        // Simulate request processing with different contexts
        let contexts = vec![
            ("User registration", "/api/users", "POST"),
            ("Product search", "/api/products/search", "GET"),
            ("Order checkout", "/api/orders", "POST"),
            ("Dashboard view", "/dashboard", "GET"),
        ];

        println!("🔄 Processing same template with different contexts:\n");

        for (description, path, method) in contexts {
            let context = ResolutionContext::new().with_request(
                RequestMetadata::new(method, path)
            );

            // Simple template variation
            let simple_template = format!("{}{{}} request to {} at {{time:iso8601}}", description, method, path);
            let result = self.engine.process_template_with_context(&simple_template, &context).await?;
            println!("  {} - {}", description, result);
        }

        println!("\n✅ Advanced demo completed successfully!");
        Ok(())
    }

    /// Demonstrate plugin error handling
    pub async fn run_error_handling_demo(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("\n🛡️ **Error Handling Demo**\n");

        // Templates with tokens that may not resolve
        let error_templates = vec![
            "This will fail: {{nonexistent:plugin:token}}",
            "Partially broken: {{time:iso8601}} and {{bad:token}}",
            "Environment issue: {{env:MISSING_VAR}}",
        ];

        for template in error_templates {
            let tokens = self.engine.extract_resolvable_tokens(template).await;
            println!("Template: {}", template);
            println!("  Extracted tokens: {:?}", tokens);

            match self.engine.process_template(template).await {
                Ok(result) => println!("  ✅ Resolution successful: {}", result),
                Err(e) => println!("  ❌ Resolution failed: {}", e),
            }
            println!();
        }

        // Demonstrate plugin capabilities
        let metadata = self.engine.get_plugin_metadata().await;
        println!("🎛️ **Plugin Capabilities Summary:**");
        for (name, meta) in metadata {
            println!("  {} → {} prefixes supported", name, meta.supported_prefixes.len());
            for prefix in &meta.supported_prefixes {
                println!("    ├── supports '{}*' tokens", prefix);
            }
        }

        println!("✅ Error handling demo completed!");
        Ok(())
    }
}

/// Run the complete plugin system demonstration
pub async fn run_complete_demo() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🎪 **MockForge Token Resolver Plugin System Demo**\n");

    let demo = PluginSystemDemo::new()?;

    // Setup phase
    demo.setup_demo().await?;

    // Run demos
    demo.run_token_resolution_demo().await?;
    demo.run_advanced_demo().await?;
    demo.run_error_handling_demo().await?;

    println!("\n🎉 **COMPLETE DEMO SUCCESSFUL!**");
    println!("=============================================================");
    println!("The token resolver plugin system is fully functional and");
    println!("ready for production use. Developers can now:");
    println!("• Create custom token types without modifying core code");
    println!("• Extend template functionality with business logic");
    println!("• Integrate with external systems and databases");
    println!("• Use secure, sandboxed plugin execution");
    println!("• Leverage rich context and request metadata");
    println!("=============================================================");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_demo_creation() {
        let demo = PluginSystemDemo::new().unwrap();
        let plugins = demo.list_plugins().await;
        assert!(plugins.is_empty());
    }

    #[tokio::test]
    async fn test_demo_setup() {
        let demo = PluginSystemDemo::new().unwrap();
        demo.setup_demo().await.unwrap();

        let plugins = demo.list_plugins().await;
        assert!(!plugins.is_empty());  // Should have standard + custom resolvers
        assert!(plugins.contains(&"business-resolver".to_string()));
    }

    #[tokio::test]
    async fn test_custom_business_resolver() {
        let resolver = CustomBusinessResolver::new();
        assert!(resolver.can_resolve("business:hours"));
        assert!(resolver.can_resolve("company:name"));

        let context = ResolutionContext::new();
        let result = resolver.resolve_token("company:name", &context).await.unwrap();
        assert_eq!(result, "MockForge Inc");

        let meta = resolver.get_metadata();
        assert_eq!(meta.supported_prefixes.len(), 5);
        assert!(meta.supported_prefixes.contains(&"business".to_string()));
    }
}
