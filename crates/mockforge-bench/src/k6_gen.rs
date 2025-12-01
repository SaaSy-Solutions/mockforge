//! k6 script generation for load testing real endpoints

use crate::error::{BenchError, Result};
use crate::request_gen::RequestTemplate;
use crate::scenarios::LoadScenario;
use handlebars::Handlebars;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Configuration for k6 script generation
pub struct K6Config {
    pub target_url: String,
    pub scenario: LoadScenario,
    pub duration_secs: u64,
    pub max_vus: u32,
    pub threshold_percentile: String,
    pub threshold_ms: u64,
    pub max_error_rate: f64,
    pub auth_header: Option<String>,
    pub custom_headers: HashMap<String, String>,
}

/// Generate k6 load test script
pub struct K6ScriptGenerator {
    config: K6Config,
    templates: Vec<RequestTemplate>,
}

impl K6ScriptGenerator {
    /// Create a new k6 script generator
    pub fn new(config: K6Config, templates: Vec<RequestTemplate>) -> Self {
        Self { config, templates }
    }

    /// Generate the k6 script
    pub fn generate(&self) -> Result<String> {
        let handlebars = Handlebars::new();

        let template = include_str!("templates/k6_script.hbs");

        let data = self.build_template_data()?;

        handlebars
            .render_template(template, &data)
            .map_err(|e| BenchError::ScriptGenerationFailed(e.to_string()))
    }

    /// Sanitize a name to be a valid JavaScript identifier
    ///
    /// Replaces invalid characters (dots, spaces, special chars) with underscores.
    /// Ensures the identifier starts with a letter or underscore (not a number).
    ///
    /// Examples:
    /// - "billing.subscriptions.v1" -> "billing_subscriptions_v1"
    /// - "get user" -> "get_user"
    /// - "123invalid" -> "_123invalid"
    fn sanitize_js_identifier(name: &str) -> String {
        let mut result = String::new();
        let mut chars = name.chars().peekable();

        // Ensure it starts with a letter or underscore (not a number)
        if let Some(&first) = chars.peek() {
            if first.is_ascii_digit() {
                result.push('_');
            }
        }

        for ch in chars {
            if ch.is_alphanumeric() || ch == '_' {
                result.push(ch);
            } else {
                // Replace invalid characters with underscore
                // Avoid consecutive underscores
                if !result.ends_with('_') {
                    result.push('_');
                }
            }
        }

        // Remove trailing underscores
        result = result.trim_end_matches('_').to_string();

        // If empty after sanitization, use a default name
        if result.is_empty() {
            result = "operation".to_string();
        }

        result
    }

    /// Build the template data for rendering
    fn build_template_data(&self) -> Result<Value> {
        let stages = self
            .config
            .scenario
            .generate_stages(self.config.duration_secs, self.config.max_vus);

        let operations = self
            .templates
            .iter()
            .enumerate()
            .map(|(idx, template)| {
                let display_name = template.operation.display_name();
                let sanitized_name = Self::sanitize_js_identifier(&display_name);
                json!({
                    "index": idx,
                    "name": sanitized_name,  // Use sanitized name for variable names
                    "display_name": display_name,  // Keep original for comments/display
                    "method": template.operation.method.to_uppercase(),
                    "path": template.generate_path(),
                    "headers": self.build_headers(template),
                    "body": template.body.as_ref().map(|b| b.to_string()),
                    "has_body": template.body.is_some(),
                })
            })
            .collect::<Vec<_>>();

        Ok(json!({
            "base_url": self.config.target_url,
            "stages": stages.iter().map(|s| json!({
                "duration": s.duration,
                "target": s.target,
            })).collect::<Vec<_>>(),
            "operations": operations,
            "threshold_percentile": self.config.threshold_percentile,
            "threshold_ms": self.config.threshold_ms,
            "max_error_rate": self.config.max_error_rate,
            "scenario_name": format!("{:?}", self.config.scenario).to_lowercase(),
        }))
    }

    /// Build headers for a request template
    fn build_headers(&self, template: &RequestTemplate) -> Value {
        let mut headers = template.get_headers();

        // Add auth header if provided
        if let Some(auth) = &self.config.auth_header {
            headers.insert("Authorization".to_string(), auth.clone());
        }

        // Add custom headers
        for (key, value) in &self.config.custom_headers {
            headers.insert(key.clone(), value.clone());
        }

        json!(headers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_k6_config_creation() {
        let config = K6Config {
            target_url: "https://api.example.com".to_string(),
            scenario: LoadScenario::RampUp,
            duration_secs: 60,
            max_vus: 10,
            threshold_percentile: "p95".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            auth_header: None,
            custom_headers: HashMap::new(),
        };

        assert_eq!(config.duration_secs, 60);
        assert_eq!(config.max_vus, 10);
    }

    #[test]
    fn test_script_generator_creation() {
        let config = K6Config {
            target_url: "https://api.example.com".to_string(),
            scenario: LoadScenario::Constant,
            duration_secs: 30,
            max_vus: 5,
            threshold_percentile: "p95".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            auth_header: None,
            custom_headers: HashMap::new(),
        };

        let templates = vec![];
        let generator = K6ScriptGenerator::new(config, templates);

        assert_eq!(generator.templates.len(), 0);
    }

    #[test]
    fn test_sanitize_js_identifier() {
        // Test case from issue #79: names with dots
        assert_eq!(
            K6ScriptGenerator::sanitize_js_identifier("billing.subscriptions.v1"),
            "billing_subscriptions_v1"
        );

        // Test other invalid characters
        assert_eq!(K6ScriptGenerator::sanitize_js_identifier("get user"), "get_user");

        // Test names starting with numbers
        assert_eq!(K6ScriptGenerator::sanitize_js_identifier("123invalid"), "_123invalid");

        // Test already valid identifiers
        assert_eq!(K6ScriptGenerator::sanitize_js_identifier("getUsers"), "getUsers");

        // Test with multiple consecutive invalid chars
        assert_eq!(K6ScriptGenerator::sanitize_js_identifier("test...name"), "test_name");

        // Test empty string (should return default)
        assert_eq!(K6ScriptGenerator::sanitize_js_identifier(""), "operation");

        // Test with special characters
        assert_eq!(K6ScriptGenerator::sanitize_js_identifier("test@name#value"), "test_name_value");
    }

    #[test]
    fn test_script_generation_with_dots_in_name() {
        use crate::spec_parser::ApiOperation;
        use openapiv3::Operation;

        // Create an operation with a name containing dots (like in issue #79)
        let operation = ApiOperation {
            method: "get".to_string(),
            path: "/billing/subscriptions".to_string(),
            operation: Operation::default(),
            operation_id: Some("billing.subscriptions.v1".to_string()),
        };

        let template = RequestTemplate {
            operation,
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: None,
        };

        let config = K6Config {
            target_url: "https://api.example.com".to_string(),
            scenario: LoadScenario::Constant,
            duration_secs: 30,
            max_vus: 5,
            threshold_percentile: "p95".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            auth_header: None,
            custom_headers: HashMap::new(),
        };

        let generator = K6ScriptGenerator::new(config, vec![template]);
        let script = generator.generate().expect("Should generate script");

        // Verify the script contains sanitized variable names (no dots in variable identifiers)
        assert!(
            script.contains("const billing_subscriptions_v1_latency"),
            "Script should contain sanitized variable name for latency"
        );
        assert!(
            script.contains("const billing_subscriptions_v1_errors"),
            "Script should contain sanitized variable name for errors"
        );

        // Verify variable names do NOT contain dots (check the actual variable identifier, not string literals)
        // The pattern "const billing.subscriptions" would indicate a variable name with dots
        assert!(
            !script.contains("const billing.subscriptions"),
            "Script should not contain variable names with dots - this would cause 'Unexpected token .' error"
        );

        // Verify metric name strings CAN contain dots (they're just strings, not identifiers)
        assert!(
            script.contains("'billing.subscriptions.v1_latency'"),
            "Metric name strings can contain dots (they're string literals)"
        );

        // Verify the original display name is still used in comments and strings
        assert!(
            script.contains("billing.subscriptions.v1"),
            "Script should contain original name in comments/strings"
        );

        // Most importantly: verify the variable usage doesn't have dots
        assert!(
            script.contains("billing_subscriptions_v1_latency.add"),
            "Variable usage should use sanitized name"
        );
        assert!(
            script.contains("billing_subscriptions_v1_errors.add"),
            "Variable usage should use sanitized name"
        );
    }
}
