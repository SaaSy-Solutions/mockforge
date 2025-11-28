//! # Advanced Template Functions Plugin for MockForge
//!
//! This plugin provides advanced template functions for MockForge responses.
//! It demonstrates more sophisticated template helpers including data transformations,
//! aggregations, and complex data generation.
//!
//! ## Features
//!
//! - Advanced data transformations (grouping, filtering, sorting)
//! - Mathematical operations and aggregations
//! - Date/time manipulation functions
//! - String manipulation and formatting
//! - Collection operations (map, reduce, filter)
//! - JSON manipulation functions

use mockforge_plugin_core::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Mutex;
use rand::Rng;

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedTemplateConfig {
    /// Enable advanced mathematical functions
    pub enable_math: bool,
    /// Enable collection operations
    pub enable_collections: bool,
    /// Enable date/time functions
    pub enable_datetime: bool,
    /// Locale for formatting
    pub locale: String,
}

impl Default for AdvancedTemplateConfig {
    fn default() -> Self {
        Self {
            enable_math: true,
            enable_collections: true,
            enable_datetime: true,
            locale: "en-US".to_string(),
        }
    }
}

/// Advanced Template Plugin
pub struct AdvancedTemplatePlugin {
    config: AdvancedTemplateConfig,
    // Use Mutex for thread-safe RNG access
    // Note: Using a seedable RNG that's Send + Sync
    rng: Mutex<rand::rngs::StdRng>,
}

impl AdvancedTemplatePlugin {
    /// Create a new advanced template plugin
    pub fn new(config: AdvancedTemplateConfig) -> Self {
        use rand::SeedableRng;
        Self {
            config,
            rng: Mutex::new(rand::rngs::StdRng::from_entropy()),
        }
    }

    /// Calculate sum of numbers
    fn sum(&self, args: &[Value]) -> std::result::Result<Value, String> {
        let mut numbers = Vec::new();
        for v in args {
            let num = v.as_f64()
                .or_else(|| v.as_i64().map(|i| i as f64))
                .or_else(|| v.as_u64().map(|u| u as f64))
                .ok_or_else(|| "All arguments must be numbers".to_string())?;
            numbers.push(num);
        }

        let sum: f64 = numbers.iter().sum();
        Ok(json!(sum))
    }

    /// Calculate average of numbers
    fn average(&self, args: &[Value]) -> std::result::Result<Value, String> {
        let mut numbers = Vec::new();
        for v in args {
            let num = v.as_f64()
                .or_else(|| v.as_i64().map(|i| i as f64))
                .or_else(|| v.as_u64().map(|u| u as f64))
                .ok_or_else(|| "All arguments must be numbers".to_string())?;
            numbers.push(num);
        }

        if numbers.is_empty() {
            return Err("At least one number is required".to_string());
        }
        let avg = numbers.iter().sum::<f64>() / numbers.len() as f64;
        Ok(json!(avg))
    }

    /// Format date/time
    fn format_date(&self, args: &[Value]) -> std::result::Result<Value, String> {
        if args.is_empty() {
            return Err("format_date requires at least one argument".to_string());
        }

        let timestamp = args[0]
            .as_i64()
            .or_else(|| args[0].as_u64().map(|u| u as i64))
            .ok_or_else(|| "First argument must be a timestamp".to_string())?;

        let format = args
            .get(1)
            .and_then(|v| v.as_str())
            .unwrap_or("%Y-%m-%d %H:%M:%S");

        let dt = chrono::DateTime::from_timestamp(timestamp, 0)
            .ok_or_else(|| "Invalid timestamp".to_string())?;

        Ok(json!(dt.format(format).to_string()))
    }

    /// Group array by key
    fn group_by(&self, args: &[Value]) -> std::result::Result<Value, String> {
        if args.len() != 2 {
            return Err("group_by requires array and key".to_string());
        }

        let array = args[0]
            .as_array()
            .ok_or_else(|| "First argument must be an array".to_string())?;

        let key = args[1]
            .as_str()
            .ok_or_else(|| "Second argument must be a string key".to_string())?;

        let mut grouped: HashMap<String, Vec<Value>> = HashMap::new();

        for item in array {
            if let Some(obj) = item.as_object() {
                if let Some(value) = obj.get(key) {
                    let group_key = value.to_string();
                    grouped
                        .entry(group_key)
                        .or_insert_with(Vec::new)
                        .push(item.clone());
                }
            }
        }

        Ok(json!(grouped))
    }

    /// Sort array
    fn sort(&self, args: &[Value]) -> std::result::Result<Value, String> {
        if args.is_empty() {
            return Err("sort requires an array".to_string());
        }

        let mut array = args[0]
            .as_array()
            .ok_or_else(|| "First argument must be an array".to_string())?
            .clone();

        let key = args.get(1).and_then(|v| v.as_str());

        if let Some(sort_key) = key {
            // Sort by object key
            array.sort_by(|a, b| {
                let a_val = a.as_object().and_then(|o| o.get(sort_key));
                let b_val = b.as_object().and_then(|o| o.get(sort_key));
                match (a_val, b_val) {
                    (Some(a), Some(b)) => {
                        // Simple string comparison
                        a.to_string().cmp(&b.to_string())
                    }
                    _ => std::cmp::Ordering::Equal,
                }
            });
        } else {
            // Simple value sort
            array.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
        }

        Ok(json!(array))
    }

    /// Generate UUID
    fn uuid(&self) -> std::result::Result<Value, String> {
        Ok(json!(uuid::Uuid::new_v4().to_string()))
    }

    /// Generate random number in range
    fn random_int(&self, args: &[Value]) -> std::result::Result<Value, String> {
        let min = args
            .get(0)
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let max = args
            .get(1)
            .and_then(|v| v.as_i64())
            .unwrap_or(100);

        let mut rng = self.rng.lock().unwrap();
        let value = rng.gen_range(min..=max);
        Ok(json!(value))
    }
}

#[::async_trait::async_trait]
impl TemplatePlugin for AdvancedTemplatePlugin {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            network: NetworkPermissions {
                allow_http: false,
                allowed_hosts: vec![],
                max_connections: 10,
            },
            filesystem: FilesystemPermissions {
                read_paths: vec![],
                write_paths: vec![],
                allow_temp_files: false,
            },
            resources: ResourceLimits {
                max_memory_bytes: 15 * 1024 * 1024, // 15MB
                max_cpu_percent: 0.5,
                max_execution_time_ms: 200,
                max_concurrent_executions: 5,
            },
            custom: HashMap::new(),
        }
    }

    async fn initialize(&self, _config: &TemplatePluginConfig) -> Result<()> {
        Ok(())
    }

    async fn register_functions(
        &self,
        _context: &PluginContext,
        _config: &TemplatePluginConfig,
    ) -> Result<PluginResult<HashMap<String, TemplateFunction>>> {
        let mut functions = HashMap::new();

        if self.config.enable_math {
            functions.insert(
                "sum".to_string(),
                TemplateFunction::new("sum", "Calculate sum of numbers", "number")
                    .with_parameter(FunctionParameter::var_args("numbers", "number", "Numbers to sum"))
                    .with_example("{{sum 1 2 3 4}}")
                    .with_category("math"),
            );

            functions.insert(
                "average".to_string(),
                TemplateFunction::new("average", "Calculate average of numbers", "number")
                    .with_parameter(FunctionParameter::var_args("numbers", "number", "Numbers to average"))
                    .with_example("{{average 10 20 30}}")
                    .with_category("math"),
            );
        }

        if self.config.enable_datetime {
            functions.insert(
                "format_date".to_string(),
                TemplateFunction::new("format_date", "Format timestamp as date string", "string")
                    .with_parameter(FunctionParameter::required("timestamp", "number", "Unix timestamp"))
                    .with_parameter(FunctionParameter::optional("format", "string", "Date format string"))
                    .with_example("{{format_date 1640995200 \"%Y-%m-%d\"}}")
                    .with_category("datetime"),
            );
        }

        if self.config.enable_collections {
            functions.insert(
                "group_by".to_string(),
                TemplateFunction::new("group_by", "Group array by key", "object")
                    .with_parameter(FunctionParameter::required("array", "array", "Array to group"))
                    .with_parameter(FunctionParameter::required("key", "string", "Key to group by"))
                    .with_example("{{group_by users \"role\"}}")
                    .with_category("collection"),
            );

            functions.insert(
                "sort".to_string(),
                TemplateFunction::new("sort", "Sort array", "array")
                    .with_parameter(FunctionParameter::required("array", "array", "Array to sort"))
                    .with_parameter(FunctionParameter::optional("key", "string", "Object key to sort by"))
                    .with_example("{{sort items \"price\"}}")
                    .with_category("collection"),
            );
        }

        functions.insert(
            "uuid".to_string(),
            TemplateFunction::new("uuid", "Generate UUID v4", "string")
                .with_example("{{uuid}}")
                .with_category("generator")
                .impure(),
        );

        functions.insert(
            "random_int".to_string(),
            TemplateFunction::new("random_int", "Generate random integer in range", "number")
                .with_parameter(FunctionParameter::optional("min", "number", "Minimum value"))
                .with_parameter(FunctionParameter::optional("max", "number", "Maximum value"))
                .with_example("{{random_int 1 100}}")
                .with_category("generator")
                .impure(),
        );

        Ok(PluginResult::success(functions, 0))
    }

    async fn execute_function(
        &self,
        _context: &PluginContext,
        function_name: &str,
        args: &[Value],
        _config: &TemplatePluginConfig,
    ) -> Result<PluginResult<Value>> {
        let result: std::result::Result<Value, String> = match function_name {
            "sum" if self.config.enable_math => self.sum(args),
            "average" if self.config.enable_math => self.average(args),
            "format_date" if self.config.enable_datetime => self.format_date(args),
            "group_by" if self.config.enable_collections => self.group_by(args),
            "sort" if self.config.enable_collections => self.sort(args),
            "uuid" => self.uuid(),
            "random_int" => self.random_int(args),
            _ => Err(format!("Unknown function: {}", function_name)),
        };

        match result {
            Ok(value) => Ok(PluginResult::success(value, 0)),
            Err(e) => Ok(PluginResult::failure(e, 0)),
        }
    }

    async fn get_data_source(
        &self,
        _context: &PluginContext,
        _data_source: &str,
        _config: &TemplatePluginConfig,
    ) -> Result<PluginResult<Value>> {
        Ok(PluginResult::failure("No data sources available".to_string(), 0))
    }

    fn validate_config(&self, _config: &TemplatePluginConfig) -> Result<()> {
        Ok(())
    }

    fn available_data_sources(&self) -> Vec<String> {
        vec![]
    }

    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }
}

/// Plugin factory function
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
#[no_mangle]
pub unsafe extern "C" fn create_template_plugin(
    config_json: *const u8,
    config_len: usize,
) -> *mut AdvancedTemplatePlugin {
    let config_bytes = std::slice::from_raw_parts(config_json, config_len);

    let config_str = match std::str::from_utf8(config_bytes) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let template_config: AdvancedTemplateConfig = match serde_json::from_str(config_str) {
        Ok(c) => c,
        Err(_) => {
            // Use defaults if parsing fails
            AdvancedTemplateConfig::default()
        }
    };

    let plugin = Box::new(AdvancedTemplatePlugin::new(template_config));
    Box::into_raw(plugin)
}

/// Plugin cleanup function
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
#[no_mangle]
pub unsafe extern "C" fn destroy_template_plugin(plugin: *mut AdvancedTemplatePlugin) {
    if !plugin.is_null() {
        let _ = Box::from_raw(plugin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_plugin_core::{PluginId, PluginVersion};

    fn create_test_context() -> PluginContext {
        PluginContext::new(
            PluginId::new("template-advanced"),
            PluginVersion::new(1, 0, 0),
        )
    }

    #[tokio::test]
    async fn test_sum_function() {
        let config = AdvancedTemplateConfig::default();
        let plugin = AdvancedTemplatePlugin::new(config);
        let context = create_test_context();
        let config = TemplatePluginConfig::default();

        let args = vec![json!(1), json!(2), json!(3)];
        let result = plugin.execute_function(&context, "sum", &args, &config).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.success);
        assert_eq!(result.data.unwrap(), json!(6));
    }

    #[tokio::test]
    async fn test_average_function() {
        let config = AdvancedTemplateConfig::default();
        let plugin = AdvancedTemplatePlugin::new(config);
        let context = create_test_context();
        let config = TemplatePluginConfig::default();

        let args = vec![json!(10), json!(20), json!(30)];
        let result = plugin.execute_function(&context, "average", &args, &config).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.success);
        assert_eq!(result.data.unwrap(), json!(20.0));
    }

    #[tokio::test]
    async fn test_register_functions() {
        let config = AdvancedTemplateConfig::default();
        let plugin = AdvancedTemplatePlugin::new(config);
        let context = create_test_context();
        let config = TemplatePluginConfig::default();

        let result = plugin.register_functions(&context, &config).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.success);
        let functions = result.data.unwrap();
        assert!(functions.contains_key("sum"));
        assert!(functions.contains_key("average"));
        assert!(functions.contains_key("uuid"));
    }

    #[test]
    fn test_plugin_config_defaults() {
        let config = AdvancedTemplateConfig::default();
        assert!(config.enable_math);
        assert!(config.enable_collections);
        assert!(config.enable_datetime);
        assert_eq!(config.locale, "en-US");
    }

    #[test]
    fn test_capabilities() {
        let config = AdvancedTemplateConfig::default();
        let plugin = AdvancedTemplatePlugin::new(config);
        let caps = plugin.capabilities();

        assert!(!caps.network.allow_http);
        assert_eq!(caps.resources.max_memory_bytes, 15 * 1024 * 1024);
    }
}
