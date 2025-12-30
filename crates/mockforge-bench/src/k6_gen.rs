//! k6 script generation for load testing real endpoints

use crate::dynamic_params::{DynamicParamProcessor, DynamicPlaceholder};
use crate::error::{BenchError, Result};
use crate::request_gen::RequestTemplate;
use crate::scenarios::LoadScenario;
use handlebars::Handlebars;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

/// Configuration for k6 script generation
pub struct K6Config {
    pub target_url: String,
    /// API base path prefix (e.g., "/api" or "/v2")
    /// Prepended to all API endpoint paths
    pub base_path: Option<String>,
    pub scenario: LoadScenario,
    pub duration_secs: u64,
    pub max_vus: u32,
    pub threshold_percentile: String,
    pub threshold_ms: u64,
    pub max_error_rate: f64,
    pub auth_header: Option<String>,
    pub custom_headers: HashMap<String, String>,
    pub skip_tls_verify: bool,
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
    pub fn sanitize_js_identifier(name: &str) -> String {
        let mut result = String::new();
        let mut chars = name.chars().peekable();

        // Ensure it starts with a letter or underscore (not a number)
        if let Some(&first) = chars.peek() {
            if first.is_ascii_digit() {
                result.push('_');
            }
        }

        for ch in chars {
            if ch.is_ascii_alphanumeric() || ch == '_' {
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

        // Get the base path (defaults to empty string if not set)
        let base_path = self.config.base_path.as_deref().unwrap_or("");

        // Track all placeholders used across all operations
        let mut all_placeholders: HashSet<DynamicPlaceholder> = HashSet::new();

        let operations = self
            .templates
            .iter()
            .enumerate()
            .map(|(idx, template)| {
                let display_name = template.operation.display_name();
                let sanitized_name = Self::sanitize_js_identifier(&display_name);
                // metric_name must also be sanitized for k6 metric name validation
                // k6 metric names must only contain ASCII letters, numbers, or underscores
                let metric_name = sanitized_name.clone();
                // k6 uses 'del' instead of 'delete' for HTTP DELETE method
                let k6_method = match template.operation.method.to_lowercase().as_str() {
                    "delete" => "del".to_string(),
                    m => m.to_string(),
                };
                // GET and HEAD methods only take 2 arguments in k6: http.get(url, params)
                // Other methods take 3 arguments: http.post(url, body, params)
                let is_get_or_head = matches!(k6_method.as_str(), "get" | "head");

                // Process path for dynamic placeholders
                // Prepend base_path if configured
                let raw_path = template.generate_path();
                let full_path = if base_path.is_empty() {
                    raw_path
                } else {
                    format!("{}{}", base_path, raw_path)
                };
                let processed_path = DynamicParamProcessor::process_path(&full_path);
                all_placeholders.extend(processed_path.placeholders.clone());

                // Process body for dynamic placeholders
                let (body_value, body_is_dynamic) = if let Some(body) = &template.body {
                    let processed_body = DynamicParamProcessor::process_json_body(body);
                    all_placeholders.extend(processed_body.placeholders.clone());
                    (Some(processed_body.value), processed_body.is_dynamic)
                } else {
                    (None, false)
                };

                json!({
                    "index": idx,
                    "name": sanitized_name,  // Use sanitized name for variable names
                    "metric_name": metric_name,  // Use sanitized name for metric name strings (k6 validation)
                    "display_name": display_name,  // Keep original for comments/display
                    "method": k6_method,  // k6 uses lowercase methods (http.get, http.post, http.del)
                    "path": if processed_path.is_dynamic { processed_path.value } else { full_path },
                    "path_is_dynamic": processed_path.is_dynamic,
                    "headers": self.build_headers_json(template),  // Returns JSON string for template
                    "body": body_value,
                    "body_is_dynamic": body_is_dynamic,
                    "has_body": template.body.is_some(),
                    "is_get_or_head": is_get_or_head,  // For correct k6 function signature
                })
            })
            .collect::<Vec<_>>();

        // Get required imports and global initializations based on placeholders used
        let required_imports = DynamicParamProcessor::get_required_imports(&all_placeholders);
        let required_globals = DynamicParamProcessor::get_required_globals(&all_placeholders);
        let has_dynamic_values = !all_placeholders.is_empty();

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
            "skip_tls_verify": self.config.skip_tls_verify,
            "has_dynamic_values": has_dynamic_values,
            "dynamic_imports": required_imports,
            "dynamic_globals": required_globals,
        }))
    }

    /// Build headers for a request template as a JSON string for k6 script
    fn build_headers_json(&self, template: &RequestTemplate) -> String {
        let mut headers = template.get_headers();

        // Add auth header if provided
        if let Some(auth) = &self.config.auth_header {
            headers.insert("Authorization".to_string(), auth.clone());
        }

        // Add custom headers
        for (key, value) in &self.config.custom_headers {
            headers.insert(key.clone(), value.clone());
        }

        // Convert to JSON string for embedding in k6 script
        serde_json::to_string(&headers).unwrap_or_else(|_| "{}".to_string())
    }

    /// Validate the generated k6 script for common issues
    ///
    /// Checks for:
    /// - Invalid metric names (contains dots or special characters)
    /// - Invalid JavaScript variable names
    /// - Missing required k6 imports
    ///
    /// Returns a list of validation errors, empty if all checks pass.
    pub fn validate_script(script: &str) -> Vec<String> {
        let mut errors = Vec::new();

        // Check for required k6 imports
        if !script.contains("import http from 'k6/http'") {
            errors.push("Missing required import: 'k6/http'".to_string());
        }
        if !script.contains("import { check") && !script.contains("import {check") {
            errors.push("Missing required import: 'check' from 'k6'".to_string());
        }
        if !script.contains("import { Rate, Trend") && !script.contains("import {Rate, Trend") {
            errors.push("Missing required import: 'Rate, Trend' from 'k6/metrics'".to_string());
        }

        // Check for invalid metric names in Trend/Rate constructors
        // k6 metric names must only contain ASCII letters, numbers, or underscores
        // and start with a letter or underscore
        let lines: Vec<&str> = script.lines().collect();
        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Check for Trend/Rate constructors with invalid metric names
            if trimmed.contains("new Trend(") || trimmed.contains("new Rate(") {
                // Extract the metric name from the string literal
                // Pattern: new Trend('metric_name') or new Rate("metric_name")
                if let Some(start) = trimmed.find('\'') {
                    if let Some(end) = trimmed[start + 1..].find('\'') {
                        let metric_name = &trimmed[start + 1..start + 1 + end];
                        if !Self::is_valid_k6_metric_name(metric_name) {
                            errors.push(format!(
                                "Line {}: Invalid k6 metric name '{}'. Metric names must only contain ASCII letters, numbers, or underscores and start with a letter or underscore.",
                                line_num + 1,
                                metric_name
                            ));
                        }
                    }
                } else if let Some(start) = trimmed.find('"') {
                    if let Some(end) = trimmed[start + 1..].find('"') {
                        let metric_name = &trimmed[start + 1..start + 1 + end];
                        if !Self::is_valid_k6_metric_name(metric_name) {
                            errors.push(format!(
                                "Line {}: Invalid k6 metric name '{}'. Metric names must only contain ASCII letters, numbers, or underscores and start with a letter or underscore.",
                                line_num + 1,
                                metric_name
                            ));
                        }
                    }
                }
            }

            // Check for invalid JavaScript variable names (containing dots)
            if trimmed.starts_with("const ") || trimmed.starts_with("let ") {
                if let Some(equals_pos) = trimmed.find('=') {
                    let var_decl = &trimmed[..equals_pos];
                    // Check if variable name contains a dot (invalid identifier)
                    // But exclude string literals
                    if var_decl.contains('.')
                        && !var_decl.contains("'")
                        && !var_decl.contains("\"")
                        && !var_decl.trim().starts_with("//")
                    {
                        errors.push(format!(
                            "Line {}: Invalid JavaScript variable name with dot: {}. Variable names cannot contain dots.",
                            line_num + 1,
                            var_decl.trim()
                        ));
                    }
                }
            }
        }

        errors
    }

    /// Check if a string is a valid k6 metric name
    ///
    /// k6 metric names must:
    /// - Only contain ASCII letters, numbers, or underscores
    /// - Start with a letter or underscore (not a number)
    /// - Be at most 128 characters
    fn is_valid_k6_metric_name(name: &str) -> bool {
        if name.is_empty() || name.len() > 128 {
            return false;
        }

        let mut chars = name.chars();

        // First character must be a letter or underscore
        if let Some(first) = chars.next() {
            if !first.is_ascii_alphabetic() && first != '_' {
                return false;
            }
        }

        // Remaining characters must be alphanumeric or underscore
        for ch in chars {
            if !ch.is_ascii_alphanumeric() && ch != '_' {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_k6_config_creation() {
        let config = K6Config {
            target_url: "https://api.example.com".to_string(),
            base_path: None,
            scenario: LoadScenario::RampUp,
            duration_secs: 60,
            max_vus: 10,
            threshold_percentile: "p(95)".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            auth_header: None,
            custom_headers: HashMap::new(),
            skip_tls_verify: false,
        };

        assert_eq!(config.duration_secs, 60);
        assert_eq!(config.max_vus, 10);
    }

    #[test]
    fn test_script_generator_creation() {
        let config = K6Config {
            target_url: "https://api.example.com".to_string(),
            base_path: None,
            scenario: LoadScenario::Constant,
            duration_secs: 30,
            max_vus: 5,
            threshold_percentile: "p(95)".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            auth_header: None,
            custom_headers: HashMap::new(),
            skip_tls_verify: false,
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

        // Test CRUD flow names with dots (issue #79 follow-up)
        assert_eq!(K6ScriptGenerator::sanitize_js_identifier("plans.list"), "plans_list");
        assert_eq!(K6ScriptGenerator::sanitize_js_identifier("plans.create"), "plans_create");
        assert_eq!(
            K6ScriptGenerator::sanitize_js_identifier("plans.update-pricing-schemes"),
            "plans_update_pricing_schemes"
        );
        assert_eq!(K6ScriptGenerator::sanitize_js_identifier("users CRUD"), "users_CRUD");
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
            base_path: None,
            scenario: LoadScenario::Constant,
            duration_secs: 30,
            max_vus: 5,
            threshold_percentile: "p(95)".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            auth_header: None,
            custom_headers: HashMap::new(),
            skip_tls_verify: false,
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

        // Verify metric name strings are sanitized (no dots) - k6 requires valid metric names
        // Metric names must only contain ASCII letters, numbers, or underscores
        assert!(
            script.contains("'billing_subscriptions_v1_latency'"),
            "Metric name strings should be sanitized (no dots) - k6 validation requires valid metric names"
        );
        assert!(
            script.contains("'billing_subscriptions_v1_errors'"),
            "Metric name strings should be sanitized (no dots) - k6 validation requires valid metric names"
        );

        // Verify the original display name is still used in comments and strings (for readability)
        assert!(
            script.contains("billing.subscriptions.v1"),
            "Script should contain original name in comments/strings for readability"
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

    #[test]
    fn test_validate_script_valid() {
        let valid_script = r#"
import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

const test_latency = new Trend('test_latency');
const test_errors = new Rate('test_errors');

export default function() {
    const res = http.get('https://example.com');
    test_latency.add(res.timings.duration);
    test_errors.add(res.status !== 200);
}
"#;

        let errors = K6ScriptGenerator::validate_script(valid_script);
        assert!(errors.is_empty(), "Valid script should have no validation errors");
    }

    #[test]
    fn test_validate_script_invalid_metric_name() {
        let invalid_script = r#"
import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

const test_latency = new Trend('test.latency');
const test_errors = new Rate('test_errors');

export default function() {
    const res = http.get('https://example.com');
    test_latency.add(res.timings.duration);
}
"#;

        let errors = K6ScriptGenerator::validate_script(invalid_script);
        assert!(
            !errors.is_empty(),
            "Script with invalid metric name should have validation errors"
        );
        assert!(
            errors.iter().any(|e| e.contains("Invalid k6 metric name")),
            "Should detect invalid metric name with dot"
        );
    }

    #[test]
    fn test_validate_script_missing_imports() {
        let invalid_script = r#"
const test_latency = new Trend('test_latency');
export default function() {}
"#;

        let errors = K6ScriptGenerator::validate_script(invalid_script);
        assert!(!errors.is_empty(), "Script missing imports should have validation errors");
    }

    #[test]
    fn test_validate_script_metric_name_validation() {
        // Test that validate_script correctly identifies invalid metric names
        // Valid metric names should pass
        let valid_script = r#"
import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';
const test_latency = new Trend('test_latency');
const test_errors = new Rate('test_errors');
export default function() {}
"#;
        let errors = K6ScriptGenerator::validate_script(valid_script);
        assert!(errors.is_empty(), "Valid metric names should pass validation");

        // Invalid metric names should fail
        let invalid_cases = vec![
            ("test.latency", "dot in metric name"),
            ("123test", "starts with number"),
            ("test-latency", "hyphen in metric name"),
            ("test@latency", "special character"),
        ];

        for (invalid_name, description) in invalid_cases {
            let script = format!(
                r#"
import http from 'k6/http';
import {{ check, sleep }} from 'k6';
import {{ Rate, Trend }} from 'k6/metrics';
const test_latency = new Trend('{}');
export default function() {{}}
"#,
                invalid_name
            );
            let errors = K6ScriptGenerator::validate_script(&script);
            assert!(
                !errors.is_empty(),
                "Metric name '{}' ({}) should fail validation",
                invalid_name,
                description
            );
        }
    }

    #[test]
    fn test_skip_tls_verify_with_body() {
        use crate::spec_parser::ApiOperation;
        use openapiv3::Operation;
        use serde_json::json;

        // Create an operation with a request body
        let operation = ApiOperation {
            method: "post".to_string(),
            path: "/api/users".to_string(),
            operation: Operation::default(),
            operation_id: Some("createUser".to_string()),
        };

        let template = RequestTemplate {
            operation,
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: Some(json!({"name": "test"})),
        };

        let config = K6Config {
            target_url: "https://api.example.com".to_string(),
            base_path: None,
            scenario: LoadScenario::Constant,
            duration_secs: 30,
            max_vus: 5,
            threshold_percentile: "p(95)".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            auth_header: None,
            custom_headers: HashMap::new(),
            skip_tls_verify: true,
        };

        let generator = K6ScriptGenerator::new(config, vec![template]);
        let script = generator.generate().expect("Should generate script");

        // Verify the script includes TLS skip option for requests with body
        assert!(
            script.contains("insecureSkipTLSVerify: true"),
            "Script should include insecureSkipTLSVerify option when skip_tls_verify is true"
        );
    }

    #[test]
    fn test_skip_tls_verify_without_body() {
        use crate::spec_parser::ApiOperation;
        use openapiv3::Operation;

        // Create an operation without a request body
        let operation = ApiOperation {
            method: "get".to_string(),
            path: "/api/users".to_string(),
            operation: Operation::default(),
            operation_id: Some("getUsers".to_string()),
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
            base_path: None,
            scenario: LoadScenario::Constant,
            duration_secs: 30,
            max_vus: 5,
            threshold_percentile: "p(95)".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            auth_header: None,
            custom_headers: HashMap::new(),
            skip_tls_verify: true,
        };

        let generator = K6ScriptGenerator::new(config, vec![template]);
        let script = generator.generate().expect("Should generate script");

        // Verify the script includes TLS skip option for requests without body
        assert!(
            script.contains("insecureSkipTLSVerify: true"),
            "Script should include insecureSkipTLSVerify option when skip_tls_verify is true (no body)"
        );
    }

    #[test]
    fn test_no_skip_tls_verify() {
        use crate::spec_parser::ApiOperation;
        use openapiv3::Operation;

        // Create an operation
        let operation = ApiOperation {
            method: "get".to_string(),
            path: "/api/users".to_string(),
            operation: Operation::default(),
            operation_id: Some("getUsers".to_string()),
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
            base_path: None,
            scenario: LoadScenario::Constant,
            duration_secs: 30,
            max_vus: 5,
            threshold_percentile: "p(95)".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            auth_header: None,
            custom_headers: HashMap::new(),
            skip_tls_verify: false,
        };

        let generator = K6ScriptGenerator::new(config, vec![template]);
        let script = generator.generate().expect("Should generate script");

        // Verify the script does NOT include TLS skip option when skip_tls_verify is false
        assert!(
            !script.contains("insecureSkipTLSVerify"),
            "Script should NOT include insecureSkipTLSVerify option when skip_tls_verify is false"
        );
    }

    #[test]
    fn test_skip_tls_verify_multiple_operations() {
        use crate::spec_parser::ApiOperation;
        use openapiv3::Operation;
        use serde_json::json;

        // Create multiple operations - one with body, one without
        let operation1 = ApiOperation {
            method: "get".to_string(),
            path: "/api/users".to_string(),
            operation: Operation::default(),
            operation_id: Some("getUsers".to_string()),
        };

        let operation2 = ApiOperation {
            method: "post".to_string(),
            path: "/api/users".to_string(),
            operation: Operation::default(),
            operation_id: Some("createUser".to_string()),
        };

        let template1 = RequestTemplate {
            operation: operation1,
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: None,
        };

        let template2 = RequestTemplate {
            operation: operation2,
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: Some(json!({"name": "test"})),
        };

        let config = K6Config {
            target_url: "https://api.example.com".to_string(),
            base_path: None,
            scenario: LoadScenario::Constant,
            duration_secs: 30,
            max_vus: 5,
            threshold_percentile: "p(95)".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            auth_header: None,
            custom_headers: HashMap::new(),
            skip_tls_verify: true,
        };

        let generator = K6ScriptGenerator::new(config, vec![template1, template2]);
        let script = generator.generate().expect("Should generate script");

        // Verify the script includes TLS skip option ONCE in global options
        // (k6 only supports insecureSkipTLSVerify as a global option, not per-request)
        let skip_count = script.matches("insecureSkipTLSVerify: true").count();
        assert_eq!(
            skip_count, 1,
            "Script should include insecureSkipTLSVerify exactly once in global options (not per-request)"
        );

        // Verify it appears in the options block, before scenarios
        let options_start = script.find("export const options = {").expect("Should have options");
        let scenarios_start = script.find("scenarios:").expect("Should have scenarios");
        let options_prefix = &script[options_start..scenarios_start];
        assert!(
            options_prefix.contains("insecureSkipTLSVerify: true"),
            "insecureSkipTLSVerify should be in global options block"
        );
    }

    #[test]
    fn test_dynamic_params_in_body() {
        use crate::spec_parser::ApiOperation;
        use openapiv3::Operation;
        use serde_json::json;

        // Create an operation with dynamic placeholders in the body
        let operation = ApiOperation {
            method: "post".to_string(),
            path: "/api/resources".to_string(),
            operation: Operation::default(),
            operation_id: Some("createResource".to_string()),
        };

        let template = RequestTemplate {
            operation,
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: Some(json!({
                "name": "load-test-${__VU}",
                "iteration": "${__ITER}"
            })),
        };

        let config = K6Config {
            target_url: "https://api.example.com".to_string(),
            base_path: None,
            scenario: LoadScenario::Constant,
            duration_secs: 30,
            max_vus: 5,
            threshold_percentile: "p(95)".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            auth_header: None,
            custom_headers: HashMap::new(),
            skip_tls_verify: false,
        };

        let generator = K6ScriptGenerator::new(config, vec![template]);
        let script = generator.generate().expect("Should generate script");

        // Verify the script contains dynamic body indication
        assert!(
            script.contains("Dynamic body with runtime placeholders"),
            "Script should contain comment about dynamic body"
        );

        // Verify the script contains the __VU variable reference
        assert!(
            script.contains("__VU"),
            "Script should contain __VU reference for dynamic VU-based values"
        );

        // Verify the script contains the __ITER variable reference
        assert!(
            script.contains("__ITER"),
            "Script should contain __ITER reference for dynamic iteration values"
        );
    }

    #[test]
    fn test_dynamic_params_with_uuid() {
        use crate::spec_parser::ApiOperation;
        use openapiv3::Operation;
        use serde_json::json;

        // Create an operation with UUID placeholder
        let operation = ApiOperation {
            method: "post".to_string(),
            path: "/api/resources".to_string(),
            operation: Operation::default(),
            operation_id: Some("createResource".to_string()),
        };

        let template = RequestTemplate {
            operation,
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: Some(json!({
                "id": "${__UUID}"
            })),
        };

        let config = K6Config {
            target_url: "https://api.example.com".to_string(),
            base_path: None,
            scenario: LoadScenario::Constant,
            duration_secs: 30,
            max_vus: 5,
            threshold_percentile: "p(95)".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            auth_header: None,
            custom_headers: HashMap::new(),
            skip_tls_verify: false,
        };

        let generator = K6ScriptGenerator::new(config, vec![template]);
        let script = generator.generate().expect("Should generate script");

        // As of k6 v1.0.0+, webcrypto is globally available - no import needed
        // Verify the script does NOT include the old experimental webcrypto import
        assert!(
            !script.contains("k6/experimental/webcrypto"),
            "Script should NOT include deprecated k6/experimental/webcrypto import"
        );

        // Verify crypto.randomUUID() is in the generated code
        assert!(
            script.contains("crypto.randomUUID()"),
            "Script should contain crypto.randomUUID() for UUID placeholder"
        );
    }

    #[test]
    fn test_dynamic_params_with_counter() {
        use crate::spec_parser::ApiOperation;
        use openapiv3::Operation;
        use serde_json::json;

        // Create an operation with COUNTER placeholder
        let operation = ApiOperation {
            method: "post".to_string(),
            path: "/api/resources".to_string(),
            operation: Operation::default(),
            operation_id: Some("createResource".to_string()),
        };

        let template = RequestTemplate {
            operation,
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: Some(json!({
                "sequence": "${__COUNTER}"
            })),
        };

        let config = K6Config {
            target_url: "https://api.example.com".to_string(),
            base_path: None,
            scenario: LoadScenario::Constant,
            duration_secs: 30,
            max_vus: 5,
            threshold_percentile: "p(95)".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            auth_header: None,
            custom_headers: HashMap::new(),
            skip_tls_verify: false,
        };

        let generator = K6ScriptGenerator::new(config, vec![template]);
        let script = generator.generate().expect("Should generate script");

        // Verify the script includes the global counter initialization
        assert!(
            script.contains("let globalCounter = 0"),
            "Script should include globalCounter initialization when COUNTER placeholder is used"
        );

        // Verify globalCounter++ is in the generated code
        assert!(
            script.contains("globalCounter++"),
            "Script should contain globalCounter++ for COUNTER placeholder"
        );
    }

    #[test]
    fn test_static_body_no_dynamic_marker() {
        use crate::spec_parser::ApiOperation;
        use openapiv3::Operation;
        use serde_json::json;

        // Create an operation with static body (no placeholders)
        let operation = ApiOperation {
            method: "post".to_string(),
            path: "/api/resources".to_string(),
            operation: Operation::default(),
            operation_id: Some("createResource".to_string()),
        };

        let template = RequestTemplate {
            operation,
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: Some(json!({
                "name": "static-value",
                "count": 42
            })),
        };

        let config = K6Config {
            target_url: "https://api.example.com".to_string(),
            base_path: None,
            scenario: LoadScenario::Constant,
            duration_secs: 30,
            max_vus: 5,
            threshold_percentile: "p(95)".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            auth_header: None,
            custom_headers: HashMap::new(),
            skip_tls_verify: false,
        };

        let generator = K6ScriptGenerator::new(config, vec![template]);
        let script = generator.generate().expect("Should generate script");

        // Verify the script does NOT contain dynamic body marker
        assert!(
            !script.contains("Dynamic body with runtime placeholders"),
            "Script should NOT contain dynamic body comment for static body"
        );

        // Verify it does NOT include unnecessary crypto imports
        assert!(
            !script.contains("webcrypto"),
            "Script should NOT include webcrypto import for static body"
        );

        // Verify it does NOT include global counter
        assert!(
            !script.contains("let globalCounter"),
            "Script should NOT include globalCounter for static body"
        );
    }
}
