//! Plugin Template
//!
//! A template for creating MockForge plugins

use mockforge_plugin_core::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main plugin struct
#[derive(Debug)]
pub struct PluginTemplate {
    config: PluginConfig,
}

impl PluginTemplate {
    /// Create a new plugin instance
    pub fn new() -> Self {
        Self {
            config: PluginConfig::default(),
        }
    }
}

// Template Plugin Implementation
#[async_trait::async_trait]
impl TemplatePlugin for PluginTemplate {
    async fn execute_function(
        &self,
        function_name: &str,
        args: &[serde_json::Value],
        context: &PluginContext,
    ) -> PluginResult<serde_json::Value> {
        match function_name {
            "example_function" => {
                // TODO: Implement your template function
                if args.is_empty() {
                    return PluginResult::failure("Missing argument".to_string(), 0);
                }

                let input = args[0].as_str().ok_or_else(|| "Argument must be a string")?;

                // Example: Convert to uppercase
                let result = input.to_uppercase();
                PluginResult::success(serde_json::json!(result))
            }
            _ => PluginResult::failure(format!("Unknown function: {}", function_name), 0),
        }
    }

    fn get_functions(&self) -> Vec<TemplateFunction> {
        vec![TemplateFunction {
            name: "example_function".to_string(),
            description: "An example template function".to_string(),
            parameters: vec![FunctionParameter {
                name: "input".to_string(),
                param_type: "string".to_string(),
                required: true,
                description: "Input string".to_string(),
            }],
            return_type: "string".to_string(),
        }]
    }

    fn get_capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            network: NetworkCapabilities {
                allow_http_outbound: false,
                allowed_hosts: vec![],
            },
            filesystem: FilesystemCapabilities {
                allow_read: false,
                allow_write: false,
                allowed_paths: vec![],
            },
            resources: PluginResources {
                max_memory_bytes: 10 * 1024 * 1024,
                max_cpu_time_ms: 100,
            },
        }
    }
}

// Export the plugin (required)
mockforge_plugin_core::export_plugin!(PluginTemplate);

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_creation() {
        let plugin = PluginTemplate::new();
        // Add your tests here
        assert!(true);
    }
}
