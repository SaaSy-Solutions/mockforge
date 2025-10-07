//! Template plugin template

pub const TEMPLATE_TEMPLATE: &str = r#"//! {{plugin_name}} - Template Plugin
//!
//! This plugin provides custom template functions for MockForge.

use mockforge_plugin_sdk::prelude::*;

#[derive(Debug)]
pub struct Plugin {
    config: Option<serde_json::Value>,
}

impl Default for Plugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin {
    pub fn new() -> Self {
        Self { config: None }
    }
}

#[async_trait]
impl TemplatePlugin for Plugin {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            name: "{{plugin_name}}".to_string(),
            version: "0.1.0".to_string(),
            description: "Custom template functions plugin".to_string(),
        }
    }

    async fn initialize(&mut self, config: serde_json::Value) -> PluginResult<()> {
        self.config = Some(config);
        Ok(())
    }

    fn register_functions(&self) -> HashMap<String, TemplateFunction> {
        let mut functions = HashMap::new();

        // Register custom template functions
        functions.insert(
            "uppercase".to_string(),
            TemplateFunction {
                name: "uppercase".to_string(),
                description: "Convert text to uppercase".to_string(),
                parameters: vec![
                    FunctionParameter {
                        name: "text".to_string(),
                        param_type: "string".to_string(),
                        required: true,
                        description: "Text to convert".to_string(),
                    },
                ],
            },
        );

        functions.insert(
            "format_date".to_string(),
            TemplateFunction {
                name: "format_date".to_string(),
                description: "Format a date string".to_string(),
                parameters: vec![
                    FunctionParameter {
                        name: "date".to_string(),
                        param_type: "string".to_string(),
                        required: true,
                        description: "Date to format".to_string(),
                    },
                    FunctionParameter {
                        name: "format".to_string(),
                        param_type: "string".to_string(),
                        required: false,
                        description: "Date format (defaults to ISO)".to_string(),
                    },
                ],
            },
        );

        functions.insert(
            "random_number".to_string(),
            TemplateFunction {
                name: "random_number".to_string(),
                description: "Generate a random number".to_string(),
                parameters: vec![
                    FunctionParameter {
                        name: "min".to_string(),
                        param_type: "number".to_string(),
                        required: false,
                        description: "Minimum value (default: 0)".to_string(),
                    },
                    FunctionParameter {
                        name: "max".to_string(),
                        param_type: "number".to_string(),
                        required: false,
                        description: "Maximum value (default: 100)".to_string(),
                    },
                ],
            },
        );

        functions
    }

    async fn execute_function(
        &self,
        context: &PluginContext,
        function_name: &str,
        parameters: HashMap<String, serde_json::Value>,
    ) -> PluginResult<serde_json::Value> {
        // TODO: Implement your custom template functions here

        match function_name {
            "uppercase" => {
                let text = parameters
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| PluginError::InvalidInput("Missing 'text' parameter".to_string()))?;

                Ok(json!(text.to_uppercase()))
            }

            "format_date" => {
                let date = parameters
                    .get("date")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| PluginError::InvalidInput("Missing 'date' parameter".to_string()))?;

                let format = parameters
                    .get("format")
                    .and_then(|v| v.as_str())
                    .unwrap_or("ISO");

                // Simple date formatting example (in real plugin, use chrono)
                Ok(json!(format!("{} (format: {})", date, format)))
            }

            "random_number" => {
                use std::time::{SystemTime, UNIX_EPOCH};

                let min = parameters
                    .get("min")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);

                let max = parameters
                    .get("max")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(100);

                // Simple pseudo-random (in real plugin, use rand crate)
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
                let random = (timestamp % (max - min) as u128) as i64 + min;

                Ok(json!(random))
            }

            _ => Err(PluginError::InvalidInput(
                format!("Unknown function: {}", function_name)
            )),
        }
    }

    async fn get_data_source(
        &self,
        context: &PluginContext,
        source_name: &str,
    ) -> PluginResult<HashMap<String, serde_json::Value>> {
        // TODO: Implement data source providers if needed
        // This is optional - only implement if your plugin provides data sources

        match source_name {
            "example_data" => {
                let mut data = HashMap::new();
                data.insert("key1".to_string(), json!("value1"));
                data.insert("key2".to_string(), json!(42));
                Ok(data)
            }
            _ => Err(PluginError::InvalidInput(
                format!("Unknown data source: {}", source_name)
            )),
        }
    }

    async fn validate_config(&self, config: &serde_json::Value) -> PluginResult<()> {
        if !config.is_object() {
            return Err(PluginError::ConfigError(
                "Configuration must be an object".to_string()
            ));
        }
        Ok(())
    }

    async fn cleanup(&mut self) -> PluginResult<()> {
        self.config = None;
        Ok(())
    }
}

// Export the plugin
export_plugin!(Plugin);

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_uppercase_function() {
        let plugin = Plugin::new();
        let mut harness = TestHarness::new();
        let context = harness.create_context("{{plugin_id}}", "test-request");

        let mut params = HashMap::new();
        params.insert("text".to_string(), json!("hello world"));

        let result = plugin.execute_function(&context, "uppercase", params).await;

        assert_plugin_ok!(result);
        if let Ok(output) = result {
            assert_eq!(output, json!("HELLO WORLD"));
        }
    }

    #[tokio::test]
    async fn test_random_number_function() {
        let plugin = Plugin::new();
        let mut harness = TestHarness::new();
        let context = harness.create_context("{{plugin_id}}", "test-request");

        let mut params = HashMap::new();
        params.insert("min".to_string(), json!(10));
        params.insert("max".to_string(), json!(20));

        let result = plugin.execute_function(&context, "random_number", params).await;

        assert_plugin_ok!(result);
        if let Ok(output) = result {
            let num = output.as_i64().unwrap();
            assert!(num >= 10 && num <= 20);
        }
    }

    #[tokio::test]
    async fn test_register_functions() {
        let plugin = Plugin::new();
        let functions = plugin.register_functions();

        assert!(functions.contains_key("uppercase"));
        assert!(functions.contains_key("format_date"));
        assert!(functions.contains_key("random_number"));
    }

    #[tokio::test]
    async fn test_get_data_source() {
        let plugin = Plugin::new();
        let mut harness = TestHarness::new();
        let context = harness.create_context("{{plugin_id}}", "test-request");

        let result = plugin.get_data_source(&context, "example_data").await;

        assert_plugin_ok!(result);
        if let Ok(data) = result {
            assert!(data.contains_key("key1"));
            assert!(data.contains_key("key2"));
        }
    }
}
"#;
