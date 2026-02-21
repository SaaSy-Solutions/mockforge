//! Template plugin interface
//!
//! This module defines the TemplatePlugin trait and related types for implementing
//! custom template functions and data generators in MockForge. Template plugins
//! extend the templating system with custom functions, filters, and data sources.

use crate::{PluginCapabilities, PluginContext, PluginResult, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Template plugin trait
///
/// Implement this trait to create custom template functions and data generators.
/// Template plugins are called during template expansion to provide custom
/// functions, filters, and data sources.
#[async_trait::async_trait]
pub trait TemplatePlugin: Send + Sync {
    /// Get plugin capabilities (permissions and limits)
    fn capabilities(&self) -> PluginCapabilities;

    /// Initialize the plugin with configuration
    async fn initialize(&self, config: &TemplatePluginConfig) -> Result<()>;

    /// Register template functions
    ///
    /// This method is called during plugin initialization to register custom
    /// template functions. The plugin should return a map of function names
    /// to function metadata.
    ///
    /// # Arguments
    /// * `context` - Plugin execution context
    /// * `config` - Plugin configuration
    ///
    /// # Returns
    /// Map of function names to function metadata
    async fn register_functions(
        &self,
        context: &PluginContext,
        config: &TemplatePluginConfig,
    ) -> Result<PluginResult<HashMap<String, TemplateFunction>>>;

    /// Execute a template function
    ///
    /// This method is called when a registered template function is invoked
    /// during template expansion.
    ///
    /// # Arguments
    /// * `context` - Plugin execution context
    /// * `function_name` - Name of the function being called
    /// * `args` - Function arguments
    /// * `config` - Plugin configuration
    ///
    /// # Returns
    /// Function execution result
    async fn execute_function(
        &self,
        context: &PluginContext,
        function_name: &str,
        args: &[Value],
        config: &TemplatePluginConfig,
    ) -> Result<PluginResult<Value>>;

    /// Provide data sources
    ///
    /// This method can be called to retrieve data that can be used in templates.
    /// The plugin can provide dynamic data sources that are refreshed periodically.
    ///
    /// # Arguments
    /// * `context` - Plugin execution context
    /// * `data_source` - Name of the requested data source
    /// * `config` - Plugin configuration
    ///
    /// # Returns
    /// Data source content
    async fn get_data_source(
        &self,
        context: &PluginContext,
        data_source: &str,
        config: &TemplatePluginConfig,
    ) -> Result<PluginResult<Value>>;

    /// Validate plugin configuration
    fn validate_config(&self, config: &TemplatePluginConfig) -> Result<()>;

    /// Get list of available data sources
    fn available_data_sources(&self) -> Vec<String>;

    /// Cleanup plugin resources
    async fn cleanup(&self) -> Result<()>;
}

/// Template plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplatePluginConfig {
    /// Plugin-specific configuration
    pub config: HashMap<String, Value>,
    /// Enable/disable the plugin
    pub enabled: bool,
    /// Function prefix (to avoid conflicts)
    pub function_prefix: Option<String>,
    /// Data source refresh interval in seconds
    pub data_refresh_interval_secs: Option<u64>,
    /// Custom settings
    pub settings: HashMap<String, Value>,
}

impl Default for TemplatePluginConfig {
    fn default() -> Self {
        Self {
            config: HashMap::new(),
            enabled: true,
            function_prefix: None,
            data_refresh_interval_secs: Some(300), // 5 minutes
            settings: HashMap::new(),
        }
    }
}

/// Template function metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateFunction {
    /// Function name
    pub name: String,
    /// Function description
    pub description: String,
    /// Function parameters
    pub parameters: Vec<FunctionParameter>,
    /// Return type description
    pub return_type: String,
    /// Examples of usage
    pub examples: Vec<String>,
    /// Function category/tag
    pub category: Option<String>,
    /// Whether function is pure (same inputs = same outputs)
    pub pure: bool,
}

impl TemplateFunction {
    /// Create a new template function
    pub fn new<S: Into<String>>(name: S, description: S, return_type: S) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: Vec::new(),
            return_type: return_type.into(),
            examples: Vec::new(),
            category: None,
            pure: true,
        }
    }

    /// Add a parameter
    pub fn with_parameter(mut self, param: FunctionParameter) -> Self {
        self.parameters.push(param);
        self
    }

    /// Add multiple parameters
    pub fn with_parameters(mut self, params: Vec<FunctionParameter>) -> Self {
        self.parameters.extend(params);
        self
    }

    /// Add an example
    pub fn with_example<S: Into<String>>(mut self, example: S) -> Self {
        self.examples.push(example.into());
        self
    }

    /// Set category
    pub fn with_category<S: Into<String>>(mut self, category: S) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Mark as impure
    pub fn impure(mut self) -> Self {
        self.pure = false;
        self
    }

    /// Get parameter count
    pub fn param_count(&self) -> usize {
        self.parameters.len()
    }

    /// Check if function has variable arguments
    pub fn has_var_args(&self) -> bool {
        self.parameters.iter().any(|p| p.var_args)
    }
}

/// Function parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionParameter {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub param_type: String,
    /// Parameter description
    pub description: String,
    /// Whether parameter is required
    pub required: bool,
    /// Default value (if optional)
    pub default_value: Option<Value>,
    /// Whether this parameter accepts variable arguments
    pub var_args: bool,
}

impl FunctionParameter {
    /// Create a required parameter
    pub fn required<S: Into<String>>(name: S, param_type: S, description: S) -> Self {
        Self {
            name: name.into(),
            param_type: param_type.into(),
            description: description.into(),
            required: true,
            default_value: None,
            var_args: false,
        }
    }

    /// Create an optional parameter
    pub fn optional<S: Into<String>>(name: S, param_type: S, description: S) -> Self {
        Self {
            name: name.into(),
            param_type: param_type.into(),
            description: description.into(),
            required: false,
            default_value: None,
            var_args: false,
        }
    }

    /// Create an optional parameter with default value
    pub fn with_default<S: Into<String>>(
        name: S,
        param_type: S,
        description: S,
        default: Value,
    ) -> Self {
        Self {
            name: name.into(),
            param_type: param_type.into(),
            description: description.into(),
            required: false,
            default_value: Some(default),
            var_args: false,
        }
    }

    /// Create a variable arguments parameter
    pub fn var_args<S: Into<String>>(name: S, param_type: S, description: S) -> Self {
        Self {
            name: name.into(),
            param_type: param_type.into(),
            description: description.into(),
            required: false,
            default_value: None,
            var_args: true,
        }
    }
}

/// Template execution context
#[derive(Debug, Clone)]
pub struct TemplateExecutionContext {
    /// Template being processed
    pub template: String,
    /// Current position in template
    pub position: usize,
    /// Available variables
    pub variables: HashMap<String, Value>,
    /// Request context (if available)
    pub request_context: Option<HashMap<String, Value>>,
    /// Custom context data
    pub custom: HashMap<String, Value>,
}

impl TemplateExecutionContext {
    /// Create new execution context
    pub fn new<S: Into<String>>(template: S) -> Self {
        Self {
            template: template.into(),
            position: 0,
            variables: HashMap::new(),
            request_context: None,
            custom: HashMap::new(),
        }
    }

    /// Set position in template
    pub fn with_position(mut self, position: usize) -> Self {
        self.position = position;
        self
    }

    /// Add variable
    pub fn with_variable<S: Into<String>>(mut self, key: S, value: Value) -> Self {
        self.variables.insert(key.into(), value);
        self
    }

    /// Add request context
    pub fn with_request_context(mut self, context: HashMap<String, Value>) -> Self {
        self.request_context = Some(context);
        self
    }

    /// Add custom data
    pub fn with_custom<S: Into<String>>(mut self, key: S, value: Value) -> Self {
        self.custom.insert(key.into(), value);
        self
    }

    /// Get variable value
    pub fn get_variable(&self, key: &str) -> Option<&Value> {
        self.variables.get(key)
    }

    /// Get request context value
    pub fn get_request_value(&self, key: &str) -> Option<&Value> {
        self.request_context.as_ref()?.get(key)
    }

    /// Get custom value
    pub fn get_custom_value(&self, key: &str) -> Option<&Value> {
        self.custom.get(key)
    }
}

/// Template function registry entry
pub struct TemplateFunctionEntry {
    /// Plugin ID that provides this function
    pub plugin_id: crate::PluginId,
    /// Function metadata
    pub function: TemplateFunction,
    /// Plugin instance
    pub plugin: std::sync::Arc<dyn TemplatePlugin>,
    /// Function configuration
    pub config: TemplatePluginConfig,
}

impl TemplateFunctionEntry {
    /// Create new function entry
    pub fn new(
        plugin_id: crate::PluginId,
        function: TemplateFunction,
        plugin: std::sync::Arc<dyn TemplatePlugin>,
        config: TemplatePluginConfig,
    ) -> Self {
        Self {
            plugin_id,
            function,
            plugin,
            config,
        }
    }

    /// Get full function name (with prefix if configured)
    pub fn full_name(&self) -> String {
        if let Some(prefix) = &self.config.function_prefix {
            format!("{}_{}", prefix, self.function.name)
        } else {
            self.function.name.clone()
        }
    }

    /// Check if function is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

/// Helper trait for creating template plugins
pub trait TemplatePluginFactory: Send + Sync {
    /// Create a new template plugin instance
    fn create_plugin(&self) -> Result<Box<dyn TemplatePlugin>>;
}

/// Built-in template functions that plugins can use as helpers
pub mod builtin {
    use super::*;

    /// Generate a random UUID
    pub fn uuid_v4() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// Get current timestamp in RFC3339 format
    pub fn now_rfc3339() -> String {
        chrono::Utc::now().to_rfc3339()
    }

    /// Generate random integer in range
    pub fn random_int(min: i64, max: i64) -> i64 {
        use rand::Rng;
        rand::rng().random_range(min..=max)
    }

    /// Generate random float between 0 and 1
    pub fn random_float() -> f64 {
        rand::random::<f64>()
    }

    /// URL encode a string
    pub fn url_encode(input: &str) -> String {
        urlencoding::encode(input).to_string()
    }

    /// URL decode a string
    pub fn url_decode(input: &str) -> Result<String> {
        urlencoding::decode(input)
            .map(|s| s.to_string())
            .map_err(|e| crate::PluginError::execution(format!("URL decode error: {}", e)))
    }

    /// JSON stringify a value
    pub fn json_stringify(value: &Value) -> Result<String> {
        serde_json::to_string(value)
            .map_err(|e| crate::PluginError::execution(format!("JSON stringify error: {}", e)))
    }

    /// JSON parse a string
    pub fn json_parse(input: &str) -> Result<Value> {
        serde_json::from_str(input)
            .map_err(|e| crate::PluginError::execution(format!("JSON parse error: {}", e)))
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_module_compiles() {
        // Basic compilation test
    }
}
