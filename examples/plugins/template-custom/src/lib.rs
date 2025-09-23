//! # Custom Template Functions Plugin for MockForge
//!
//! This plugin provides custom template functions for MockForge responses.
//! It demonstrates how to create domain-specific template helpers.
//!
//! ## Features
//!
//! - Business domain data generation (orders, customers, products)
//! - Custom formatting functions
//! - Dynamic content generation
//! - Template helper functions

use mockforge_plugin_core::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rand::Rng;

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    /// Business domain for data generation
    pub business_domain: String,
    /// Enable advanced functions
    pub enable_advanced_functions: bool,
}

impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            business_domain: "ecommerce".to_string(),
            enable_advanced_functions: true,
        }
    }
}

/// Custom Template Plugin
pub struct CustomTemplatePlugin {
    config: TemplateConfig,
    rng: rand::rngs::ThreadRng,
}

impl CustomTemplatePlugin {
    /// Create a new custom template plugin
    pub fn new(config: TemplateConfig) -> Self {
        Self {
            config,
            rng: rand::thread_rng(),
        }
    }

    /// Generate a random order ID
    fn generate_order_id(&mut self) -> String {
        format!("ORD-{:08X}", self.rng.gen::<u32>())
    }

    /// Generate a customer name
    fn generate_customer_name(&mut self) -> String {
        let first_names = ["John", "Jane", "Bob", "Alice", "Charlie", "Diana", "Eve", "Frank"];
        let last_names = ["Smith", "Johnson", "Brown", "Williams", "Jones", "Garcia", "Miller", "Davis"];

        let first = first_names[self.rng.gen_range(0..first_names.len())];
        let last = last_names[self.rng.gen_range(0..last_names.len())];

        format!("{} {}", first, last)
    }

    /// Generate a product name based on domain
    fn generate_product_name(&mut self) -> String {
        match self.config.business_domain.as_str() {
            "ecommerce" => {
                let products = [
                    "Wireless Headphones",
                    "Smart Watch",
                    "Laptop Computer",
                    "Coffee Maker",
                    "Running Shoes",
                    "Digital Camera",
                    "Bluetooth Speaker",
                    "Fitness Tracker",
                ];
                products[self.rng.gen_range(0..products.len())].to_string()
            }
            "finance" => {
                let products = [
                    "Investment Account",
                    "Credit Card",
                    "Savings Account",
                    "Mortgage Loan",
                    "Auto Insurance",
                    "Health Insurance",
                    "Retirement Plan",
                    "Checking Account",
                ];
                products[self.rng.gen_range(0..products.len())].to_string()
            }
            _ => format!("{} Product", self.config.business_domain)
        }
    }

    /// Format currency amount
    fn format_currency(&self, amount: f64, currency: &str) -> String {
        match currency.to_uppercase().as_str() {
            "USD" => format!("${:.2}", amount),
            "EUR" => format!("€{:.2}", amount),
            "GBP" => format!("£{:.2}", amount),
            "JPY" => format!("¥{:.0}", amount),
            _ => format!("{}{:.2}", currency, amount),
        }
    }

    /// Generate business status
    fn generate_status(&mut self) -> String {
        let statuses = ["pending", "processing", "shipped", "delivered", "cancelled"];
        statuses[self.rng.gen_range(0..statuses.len())].to_string()
    }
}

impl TemplatePlugin for CustomTemplatePlugin {
    fn execute_function(
        &mut self,
        function_name: &str,
        args: &[TemplateArg],
        _context: &PluginContext,
    ) -> PluginResult<String> {
        match function_name {
            "order_id" => {
                if !args.is_empty() {
                    return PluginResult::failure(
                        "order_id function takes no arguments".to_string(),
                        0,
                    );
                }
                PluginResult::success(self.generate_order_id(), 0)
            }

            "customer_name" => {
                if !args.is_empty() {
                    return PluginResult::failure(
                        "customer_name function takes no arguments".to_string(),
                        0,
                    );
                }
                PluginResult::success(self.generate_customer_name(), 0)
            }

            "product_name" => {
                if !args.is_empty() {
                    return PluginResult::failure(
                        "product_name function takes no arguments".to_string(),
                        0,
                    );
                }
                PluginResult::success(self.generate_product_name(), 0)
            }

            "currency" => {
                if args.len() != 2 {
                    return PluginResult::failure(
                        "currency function requires amount and currency code".to_string(),
                        0,
                    );
                }

                let amount = match &args[0] {
                    TemplateArg::Number(n) => *n,
                    TemplateArg::String(s) => s.parse().unwrap_or(0.0),
                    _ => {
                        return PluginResult::failure(
                            "currency amount must be a number".to_string(),
                            0,
                        );
                    }
                };

                let currency = match &args[1] {
                    TemplateArg::String(s) => s.clone(),
                    _ => {
                        return PluginResult::failure(
                            "currency code must be a string".to_string(),
                            0,
                        );
                    }
                };

                PluginResult::success(self.format_currency(amount, &currency), 0)
            }

            "business_status" => {
                if !args.is_empty() {
                    return PluginResult::failure(
                        "business_status function takes no arguments".to_string(),
                        0,
                    );
                }
                PluginResult::success(self.generate_status(), 0)
            }

            "domain_data" => {
                if args.len() != 1 {
                    return PluginResult::failure(
                        "domain_data function requires a data type".to_string(),
                        0,
                    );
                }

                let data_type = match &args[0] {
                    TemplateArg::String(s) => s.clone(),
                    _ => {
                        return PluginResult::failure(
                            "domain_data type must be a string".to_string(),
                            0,
                        );
                    }
                };

                let result = match data_type.as_str() {
                    "order" => {
                        let order_id = self.generate_order_id();
                        let customer = self.generate_customer_name();
                        let product = self.generate_product_name();
                        let amount: f64 = self.rng.gen_range(10.0..1000.0);
                        let status = self.generate_status();

                        serde_json::json!({
                            "id": order_id,
                            "customer": customer,
                            "product": product,
                            "amount": self.format_currency(amount, "USD"),
                            "status": status,
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        }).to_string()
                    }
                    "customer" => {
                        let name = self.generate_customer_name();
                        let id = format!("CUST-{:06X}", self.rng.gen::<u32>());

                        serde_json::json!({
                            "id": id,
                            "name": name,
                            "email": format!("{}.{}@example.com",
                                name.split_whitespace().next().unwrap_or("user").to_lowercase(),
                                name.split_whitespace().last().unwrap_or("user").to_lowercase()
                            ),
                            "created_at": chrono::Utc::now().to_rfc3339()
                        }).to_string()
                    }
                    _ => {
                        return PluginResult::failure(
                            format!("Unknown domain data type: {}", data_type),
                            0,
                        );
                    }
                };

                PluginResult::success(result, 0)
            }

            _ => {
                PluginResult::failure(
                    format!("Unknown template function: {}", function_name),
                    0,
                )
            }
        }
    }

    fn get_available_functions(&self) -> Vec<TemplateFunction> {
        let mut functions = vec![
            TemplateFunction {
                name: "order_id".to_string(),
                description: "Generate a random order ID".to_string(),
                args: vec![],
                return_type: "string".to_string(),
            },
            TemplateFunction {
                name: "customer_name".to_string(),
                description: "Generate a random customer name".to_string(),
                args: vec![],
                return_type: "string".to_string(),
            },
            TemplateFunction {
                name: "product_name".to_string(),
                description: "Generate a product name for the configured business domain".to_string(),
                args: vec![],
                return_type: "string".to_string(),
            },
            TemplateFunction {
                name: "business_status".to_string(),
                description: "Generate a random business status".to_string(),
                args: vec![],
                return_type: "string".to_string(),
            },
        ];

        if self.config.enable_advanced_functions {
            functions.extend(vec![
                TemplateFunction {
                    name: "currency".to_string(),
                    description: "Format a number as currency".to_string(),
                    args: vec![
                        TemplateArg::String("amount".to_string()),
                        TemplateArg::String("currency_code".to_string()),
                    ],
                    return_type: "string".to_string(),
                },
                TemplateFunction {
                    name: "domain_data".to_string(),
                    description: "Generate domain-specific data objects".to_string(),
                    args: vec![TemplateArg::String("data_type".to_string())],
                    return_type: "json".to_string(),
                },
            ]);
        }

        functions
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
                max_memory_bytes: 15 * 1024 * 1024, // 15MB
                max_cpu_time_ms: 150, // 150ms per function call
            },
            custom: HashMap::new(),
        }
    }

    fn health_check(&self) -> PluginHealth {
        PluginHealth::healthy(
            "Custom template plugin is healthy".to_string(),
            PluginMetrics::default(),
        )
    }
}

/// Plugin factory function
#[no_mangle]
pub extern "C" fn create_template_plugin(config_json: *const u8, config_len: usize) -> *mut CustomTemplatePlugin {
    let config_bytes = unsafe {
        std::slice::from_raw_parts(config_json, config_len)
    };

    let config_str = match std::str::from_utf8(config_bytes) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let config: TemplateConfig = match serde_json::from_str(config_str) {
        Ok(c) => c,
        Err(_) => return std::ptr::null_mut(),
    };

    let plugin = Box::new(CustomTemplatePlugin::new(config));
    Box::into_raw(plugin)
}

/// Plugin cleanup function
#[no_mangle]
pub extern "C" fn destroy_template_plugin(plugin: *mut CustomTemplatePlugin) {
    if !plugin.is_null() {
        unsafe {
            let _ = Box::from_raw(plugin);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_id_generation() {
        let config = TemplateConfig::default();
        let mut plugin = CustomTemplatePlugin::new(config);

        let order_id = plugin.generate_order_id();
        assert!(order_id.starts_with("ORD-"));
        assert_eq!(order_id.len(), 12); // "ORD-" + 8 hex chars
    }

    #[test]
    fn test_customer_name_generation() {
        let config = TemplateConfig::default();
        let mut plugin = CustomTemplatePlugin::new(config);

        let name = plugin.generate_customer_name();
        assert!(name.contains(" "));
        assert!(name.split_whitespace().count() == 2);
    }

    #[test]
    fn test_currency_formatting() {
        let config = TemplateConfig::default();
        let plugin = CustomTemplatePlugin::new(config);

        assert_eq!(plugin.format_currency(123.45, "USD"), "$123.45");
        assert_eq!(plugin.format_currency(99.99, "EUR"), "€99.99");
        assert_eq!(plugin.format_currency(1000.0, "JPY"), "¥1000");
    }

    #[test]
    fn test_template_functions() {
        let config = TemplateConfig::default();
        let mut plugin = CustomTemplatePlugin::new(config);
        let context = PluginContext::new("GET".to_string(), "/test".to_string(), HashMap::new(), None);

        // Test order_id function
        let result = plugin.execute_function("order_id", &[], &context);
        assert!(result.success);
        assert!(result.data.as_ref().unwrap().starts_with("ORD-"));

        // Test currency function
        let args = vec![
            TemplateArg::Number(123.45),
            TemplateArg::String("USD".to_string()),
        ];
        let result = plugin.execute_function("currency", &args, &context);
        assert!(result.success);
        assert_eq!(result.data.as_ref().unwrap(), "$123.45");

        // Test unknown function
        let result = plugin.execute_function("unknown", &[], &context);
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("Unknown template function"));
    }

    #[test]
    fn test_available_functions() {
        let config = TemplateConfig::default();
        let plugin = CustomTemplatePlugin::new(config);

        let functions = plugin.get_available_functions();
        assert!(functions.len() >= 4); // Basic functions

        let function_names: Vec<_> = functions.iter().map(|f| f.name.as_str()).collect();
        assert!(function_names.contains(&"order_id"));
        assert!(function_names.contains(&"customer_name"));
        assert!(function_names.contains(&"currency"));
        assert!(function_names.contains(&"domain_data"));
    }
}
