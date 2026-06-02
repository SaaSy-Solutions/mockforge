//! k6 script generation for load testing real endpoints

use crate::dynamic_params::{DynamicParamProcessor, DynamicPlaceholder};
use crate::error::{BenchError, Result};
use crate::request_gen::RequestTemplate;
use crate::scenarios::LoadScenario;
use handlebars::Handlebars;
use serde::Serialize;
#[cfg(test)]
use serde_json::json;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Typed template data for `k6_script.hbs`.
///
/// Every field referenced by `{{variable}}` or `{{#if flag}}` in the template
/// is a required field here, so the compiler prevents the Issue-#79 class of
/// bugs (template rendered with missing data).
#[derive(Debug, Clone, Serialize)]
pub struct K6ScriptTemplateData {
    pub base_url: String,
    pub stages: Vec<K6StageData>,
    pub operations: Vec<K6OperationData>,
    pub threshold_percentile: String,
    pub threshold_ms: u64,
    pub max_error_rate: f64,
    pub scenario_name: String,
    pub skip_tls_verify: bool,
    pub has_dynamic_values: bool,
    pub dynamic_imports: Vec<String>,
    pub dynamic_globals: Vec<String>,
    pub security_testing_enabled: bool,
    pub has_custom_headers: bool,
    /// When true, emit `Transfer-Encoding: chunked` on every request that has a
    /// body. NOTE: k6 runs on Go's `net/http`, which decides chunking based on
    /// the body type — a string body has a known length and Go will normally
    /// send `Content-Length`. Setting this header explicitly is the closest
    /// k6-script-level approximation; for true raw chunked traffic, prefer
    /// `curl --data-binary @file -H "Transfer-Encoding: chunked"` or a custom
    /// hyper/reqwest harness.
    pub chunked_request_bodies: bool,
    /// Optional target RPS. When `Some(n)`, the script switches the executor
    /// from `ramping-vus` to `constant-arrival-rate` at `n` requests/sec.
    /// Issue #79.
    pub target_rps: Option<u32>,
    /// When true, the generated script sets `noConnectionReuse: true` on every
    /// request so each one opens a fresh TCP/TLS connection. Used to drive
    /// connections-per-second load. Issue #79.
    pub no_keep_alive: bool,
    /// Total test duration in seconds. Used by the `constant-arrival-rate`
    /// executor (when `target_rps` is set) which needs a single duration
    /// rather than a list of stages. Issue #79 — Srikanth's round-5 reply:
    /// `--rps` was previously deriving duration from the last stage of the
    /// chosen scenario; under `ramp-up` (the default) the last stage has
    /// `target: 0`, which gave `preAllocatedVUs: 0` and 0 requests.
    pub duration_secs: u64,
    /// Max VUs to pre-allocate for the `constant-arrival-rate` executor.
    /// Issue #79 (round 5).
    pub max_vus: u32,
    /// Starting VU count for the `ramping-vus` executor. For
    /// `--scenario constant` this is set to `max_vus` so the test runs at
    /// full concurrency immediately. For ramping scenarios it's 0 so the
    /// stages drive the ramp.
    ///
    /// Issue #79 round 6 follow-up: Srikanth reported that `--vus 5 -d 600s`
    /// took until the ~6-minute mark to reach 5 VUs because `startVUs: 0` +
    /// a single `{duration: '600s', target: 5}` stage made `ramping-vus`
    /// linearly ramp from 0 → 5 across the whole window. Setting startVUs
    /// to the target for `Constant` collapses that ramp.
    pub start_vus: u32,
    /// Issue #79 round 22.3 — fake source IPs to rotate across the
    /// forwarded-IP headers. Pre-round-22.3, `--geo-source-ip` only
    /// applied to the self-test driver; the k6 bench path silently
    /// ignored it. When non-empty, the rendered script picks a
    /// rotating IP per iteration and adds it to every header in
    /// `geo_source_headers` on every request. Empty = no header
    /// injection (preserves prior behaviour).
    pub geo_source_ips: Vec<String>,
    /// Headers to populate with the rotating geo source IP. Default
    /// (when CLI doesn't override) is `X-Forwarded-For`,
    /// `True-Client-IP`, `CF-Connecting-IP`. Empty means no headers
    /// even if `geo_source_ips` is non-empty.
    pub geo_source_headers: Vec<String>,
    /// True iff both `geo_source_ips` and `geo_source_headers` are
    /// non-empty. Pre-computed so the template can use a single
    /// `{{#if has_geo_source}}` guard instead of duplicating the
    /// emptiness check on both lists.
    pub has_geo_source: bool,
    /// JSON-array string of `geo_source_ips` ready for embedding in
    /// the rendered k6 script via `{{{geo_source_ips_json}}}`.
    /// Pre-serialised so Handlebars doesn't have to walk the Vec at
    /// render time.
    pub geo_source_ips_json: String,
    /// JSON-array string of `geo_source_headers` ready for embedding.
    pub geo_source_headers_json: String,
}

/// Typed template data for `k6_crud_flow.hbs`.
#[derive(Debug, Clone, Serialize)]
pub struct K6CrudFlowTemplateData {
    pub base_url: String,
    pub flows: Vec<Value>,
    pub extract_fields: Vec<String>,
    pub duration_secs: u64,
    pub max_vus: u32,
    pub auth_header: Option<String>,
    pub custom_headers: HashMap<String, String>,
    pub skip_tls_verify: bool,
    pub stages: Vec<K6StageData>,
    pub threshold_percentile: String,
    pub threshold_ms: u64,
    pub max_error_rate: f64,
    /// Raw JSON string for embedding in k6 script (rendered unescaped via `{{{headers}}}`)
    pub headers: String,
    pub dynamic_imports: Vec<String>,
    pub dynamic_globals: Vec<String>,
    pub extracted_values_output_path: String,
    pub error_injection_enabled: bool,
    pub error_rate: f64,
    pub error_types: Vec<String>,
    pub security_testing_enabled: bool,
    pub has_custom_headers: bool,
}

/// A k6 load stage for template rendering.
#[derive(Debug, Clone, Serialize)]
pub struct K6StageData {
    pub duration: String,
    pub target: u32,
}

/// Per-operation data for the `k6_script.hbs` template.
#[derive(Debug, Clone, Serialize)]
pub struct K6OperationData {
    pub index: usize,
    pub name: String,
    pub metric_name: String,
    pub display_name: String,
    pub method: String,
    pub path: Value,
    pub path_is_dynamic: bool,
    pub headers: Value,
    pub body: Option<Value>,
    pub body_is_dynamic: bool,
    pub has_body: bool,
    pub is_get_or_head: bool,
}

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
    pub security_testing_enabled: bool,
    /// Emit `Transfer-Encoding: chunked` on every request body. See
    /// `K6ScriptTemplateData::chunked_request_bodies` for caveats.
    pub chunked_request_bodies: bool,
    /// Target RPS for `constant-arrival-rate` executor. `None` falls back to
    /// the legacy ramping-vus executor.
    pub target_rps: Option<u32>,
    /// When true, set `noConnectionReuse: true` on every request so each one
    /// opens a fresh TCP/TLS connection (drives high CPS).
    pub no_keep_alive: bool,
    /// Round 22.3 — fake source IPs to advertise via forwarded-IP
    /// headers in the rendered k6 script. Empty = no header
    /// injection (preserves pre-22.3 behaviour).
    pub geo_source_ips: Vec<String>,
    /// Which forwarded-IP header(s) to populate when
    /// `geo_source_ips` is non-empty. Empty = no headers even if
    /// `geo_source_ips` is non-empty.
    pub geo_source_headers: Vec<String>,
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

        let value = serde_json::to_value(&data)
            .map_err(|e| BenchError::ScriptGenerationFailed(e.to_string()))?;

        handlebars
            .render_template(template, &value)
            .map_err(|e| BenchError::ScriptGenerationFailed(e.to_string()))
    }

    /// Maximum length for a k6 metric name *base* (the part before any
    /// `_latency` / `_errors` / `_step{N}_*` suffix). k6 enforces a
    /// 128-char limit on the full metric name; the longest suffix used by
    /// our templates is `_step99_errors` (15 chars), so we cap the base at
    /// 128 - 16 = 112 to be safe.
    const K6_METRIC_NAME_BASE_MAX_LEN: usize = 112;

    /// Sanitize a name into a valid k6 metric-name base, capped at
    /// `K6_METRIC_NAME_BASE_MAX_LEN` characters.
    ///
    /// k6 rejects metric names longer than 128 chars, and our templates
    /// append suffixes like `_latency`, `_errors`, `_stepN_latency` —
    /// reserve room for the longest suffix and truncate the base name
    /// when needed. Truncation appends an 8-hex-char hash of the original
    /// name so distinct long names produce distinct metric names.
    ///
    /// Examples:
    /// - "short_name" -> "short_name"
    /// - 200-char OperationId -> "<first-103-chars>_<8-hex-hash>"
    pub fn sanitize_k6_metric_name(name: &str) -> String {
        let sanitized = Self::sanitize_js_identifier(name);
        if sanitized.len() <= Self::K6_METRIC_NAME_BASE_MAX_LEN {
            return sanitized;
        }

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        // Hash the original name (not the sanitized one) so two distinct
        // sources that sanitize to the same string still get different
        // hashes when they exceed the limit.
        name.hash(&mut hasher);
        let hash_suffix = format!("{:08x}", hasher.finish() as u32);

        // Reserve `_<8-hex>` = 9 chars at the end.
        let prefix_len = Self::K6_METRIC_NAME_BASE_MAX_LEN - 9;
        let prefix = &sanitized[..prefix_len];
        // Strip a trailing underscore on the prefix so we don't end up with `__hash`.
        let prefix = prefix.trim_end_matches('_');
        format!("{}_{}", prefix, hash_suffix)
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

    /// Build the typed template data for rendering.
    fn build_template_data(&self) -> Result<K6ScriptTemplateData> {
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
                // metric_name must satisfy k6's 128-char limit AND leave room
                // for suffixes like `_latency` / `_errors` / `_stepN_*`.
                // Long deeply-nested operationIds (e.g. Microsoft Graph) exceed
                // this; sanitize_k6_metric_name truncates with a hash suffix
                // for uniqueness. (See issue #79 — Srikanth's microsoft-graph.yaml run.)
                let metric_name = Self::sanitize_k6_metric_name(&display_name);
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

                let path_value = if processed_path.is_dynamic {
                    processed_path.value
                } else {
                    full_path
                };

                K6OperationData {
                    index: idx,
                    name: sanitized_name,
                    metric_name,
                    display_name,
                    method: k6_method,
                    path: Value::String(path_value),
                    path_is_dynamic: processed_path.is_dynamic,
                    headers: Value::String(self.build_headers_json(template)),
                    body: body_value.map(Value::String),
                    body_is_dynamic,
                    has_body: template.body.is_some(),
                    is_get_or_head,
                }
            })
            .collect::<Vec<_>>();

        // Get required imports and global initializations based on placeholders used
        let required_imports: Vec<String> =
            DynamicParamProcessor::get_required_imports(&all_placeholders)
                .into_iter()
                .map(String::from)
                .collect();
        let required_globals: Vec<String> =
            DynamicParamProcessor::get_required_globals(&all_placeholders)
                .into_iter()
                .map(String::from)
                .collect();
        let has_dynamic_values = !all_placeholders.is_empty();

        Ok(K6ScriptTemplateData {
            base_url: self.config.target_url.clone(),
            stages: stages
                .iter()
                .map(|s| K6StageData {
                    duration: s.duration.clone(),
                    target: s.target,
                })
                .collect(),
            operations,
            threshold_percentile: self.config.threshold_percentile.clone(),
            threshold_ms: self.config.threshold_ms,
            max_error_rate: self.config.max_error_rate,
            scenario_name: format!("{:?}", self.config.scenario).to_lowercase(),
            skip_tls_verify: self.config.skip_tls_verify,
            has_dynamic_values,
            dynamic_imports: required_imports,
            dynamic_globals: required_globals,
            security_testing_enabled: self.config.security_testing_enabled,
            has_custom_headers: !self.config.custom_headers.is_empty(),
            chunked_request_bodies: self.config.chunked_request_bodies,
            target_rps: self.config.target_rps,
            no_keep_alive: self.config.no_keep_alive,
            duration_secs: self.config.duration_secs,
            max_vus: self.config.max_vus,
            // For Constant we want the test at full VU count from t=0; for the
            // ramping scenarios (RampUp / Spike / Stress / Soak) k6 needs to
            // start from 0 and let the stages drive the curve.
            start_vus: match self.config.scenario {
                LoadScenario::Constant => self.config.max_vus,
                _ => 0,
            },
            // Round 22.3 — forward the rotating-geo-IP config from
            // K6Config into template data. `has_geo_source` is the
            // and-gate the template uses to skip the header
            // assignment entirely when either list is empty. The
            // `_json` siblings pre-serialise for `{{{ }}}` triple-
            // brace embedding (raw, no escape) so the script can
            // declare `const GEO_SOURCE_IPS = [...]` directly.
            geo_source_ips: self.config.geo_source_ips.clone(),
            geo_source_headers: self.config.geo_source_headers.clone(),
            has_geo_source: !self.config.geo_source_ips.is_empty()
                && !self.config.geo_source_headers.is_empty(),
            geo_source_ips_json: serde_json::to_string(&self.config.geo_source_ips)
                .unwrap_or_else(|_| "[]".to_string()),
            geo_source_headers_json: serde_json::to_string(&self.config.geo_source_headers)
                .unwrap_or_else(|_| "[]".to_string()),
        })
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

        // Force chunked transfer encoding when requested. Only meaningful for
        // requests with bodies (POST/PUT/PATCH); k6/Go may still send
        // Content-Length in some cases — see the doc on
        // `K6ScriptTemplateData::chunked_request_bodies`.
        if self.config.chunked_request_bodies && template.body.is_some() {
            headers.insert("Transfer-Encoding".to_string(), "chunked".to_string());
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
            security_testing_enabled: false,
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
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
            security_testing_enabled: false,
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
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
    fn test_sanitize_k6_metric_name_short_passthrough() {
        // Names within the limit should pass through unchanged.
        let short = "billing_subscriptions_list";
        let out = K6ScriptGenerator::sanitize_k6_metric_name(short);
        assert_eq!(out, short);
        assert!(K6ScriptGenerator::is_valid_k6_metric_name(&format!("{out}_latency")));
    }

    #[test]
    fn test_sanitize_k6_metric_name_truncates_long_microsoft_graph_id() {
        // Real example from issue #79 (Srikanth's microsoft-graph.yaml run):
        // operationId nested deep enough that the sanitized name + `_latency`
        // exceeds k6's 128-char limit and gets rejected by validate_script.
        let long = "drives.drive.items.driveItem.workbook.worksheets.workbookWorksheet.\
                    charts.workbookChart.axes.categoryAxis.format.line.clear";
        let metric = K6ScriptGenerator::sanitize_k6_metric_name(long);

        // Base must fit within MAX_LEN, leaving room for `_latency` / `_errors`.
        assert!(
            metric.len() <= K6ScriptGenerator::K6_METRIC_NAME_BASE_MAX_LEN,
            "metric base len {} exceeded cap {}",
            metric.len(),
            K6ScriptGenerator::K6_METRIC_NAME_BASE_MAX_LEN
        );

        // Both the bare metric and the suffixed forms must pass k6's validator.
        assert!(K6ScriptGenerator::is_valid_k6_metric_name(&metric));
        assert!(K6ScriptGenerator::is_valid_k6_metric_name(&format!("{metric}_latency")));
        assert!(K6ScriptGenerator::is_valid_k6_metric_name(&format!("{metric}_errors")));
        // Worst-case suffix used by `k6_crud_flow.hbs`.
        assert!(K6ScriptGenerator::is_valid_k6_metric_name(&format!("{metric}_step99_latency")));
    }

    #[test]
    fn test_sanitize_k6_metric_name_distinct_long_names_get_distinct_metrics() {
        // Two long names that share a long common prefix must NOT collide
        // after truncation — the trailing hash makes them distinct.
        let prefix = "a".repeat(150);
        let a = format!("{prefix}.foo");
        let b = format!("{prefix}.bar");
        let ma = K6ScriptGenerator::sanitize_k6_metric_name(&a);
        let mb = K6ScriptGenerator::sanitize_k6_metric_name(&b);
        assert_ne!(ma, mb, "distinct long names produced the same metric name");
    }

    #[test]
    fn test_sanitize_k6_metric_name_truncated_starts_with_letter() {
        // Truncation must preserve the "starts with letter or _" k6 rule.
        let long = format!("{}123end", "x".repeat(120));
        let metric = K6ScriptGenerator::sanitize_k6_metric_name(&long);
        assert!(K6ScriptGenerator::is_valid_k6_metric_name(&metric));
    }

    #[test]
    fn test_microsoft_graph_long_operation_id_passes_validation() {
        // End-to-end: an ApiOperation with a microsoft-graph-style long
        // operationId must produce a script that passes validate_script.
        use crate::spec_parser::ApiOperation;
        use openapiv3::Operation;

        let long_op_id = "drives.drive.items.driveItem.workbook.worksheets.\
            workbookWorksheet.charts.workbookChart.axes.categoryAxis.format.\
            line.clear";

        let operation = ApiOperation {
            method: "post".to_string(),
            path: "/drives/{drive-id}/items/{item-id}/workbook/worksheets/{worksheet-id}/charts/{chart-id}/axes/categoryAxis/format/line/clear".to_string(),
            operation: Operation::default(),
            operation_id: Some(long_op_id.to_string()),
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
            base_path: Some("/v1.0".to_string()),
            scenario: LoadScenario::Constant,
            duration_secs: 30,
            max_vus: 5,
            threshold_percentile: "p(95)".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            auth_header: None,
            custom_headers: HashMap::new(),
            skip_tls_verify: false,
            security_testing_enabled: false,
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
        };
        let generator = K6ScriptGenerator::new(config, vec![template]);
        let script = generator.generate().expect("script generates");

        let errors = K6ScriptGenerator::validate_script(&script);
        assert!(
            errors.is_empty(),
            "validate_script returned errors for long operationId: {errors:#?}"
        );
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
            security_testing_enabled: false,
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
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

    /// Issue #79 (round 5) regression: `--rps` with the default `ramp-up`
    /// scenario produced 0 requests because the script took
    /// `preAllocatedVUs` from the *last* stage's target — and ramp-up's last
    /// stage is the ramp-DOWN to `target: 0`. The fix is to use the
    /// configured `max_vus` directly when `target_rps` is set, and the full
    /// `duration_secs` rather than the last stage's duration.
    #[test]
    fn test_rps_with_ramp_up_uses_full_vu_pool_and_duration() {
        use crate::spec_parser::ApiOperation;
        use openapiv3::Operation;

        let operation = ApiOperation {
            method: "get".to_string(),
            path: "/users".to_string(),
            operation: Operation::default(),
            operation_id: Some("listUsers".to_string()),
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
            scenario: LoadScenario::RampUp,
            duration_secs: 600,
            max_vus: 100,
            threshold_percentile: "p(95)".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            auth_header: None,
            custom_headers: HashMap::new(),
            skip_tls_verify: false,
            security_testing_enabled: false,
            chunked_request_bodies: false,
            target_rps: Some(100),
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
        };

        let generator = K6ScriptGenerator::new(config, vec![template]);
        let script = generator.generate().expect("Should generate script");

        assert!(
            script.contains("constant-arrival-rate"),
            "with --rps set, executor must switch to constant-arrival-rate"
        );
        assert!(
            script.contains("rate: 100,"),
            "constant-arrival-rate must use the configured --rps as `rate`"
        );
        assert!(
            script.contains("duration: '600s'"),
            "duration must come from --duration, not the ramp-down stage; got:\n{}",
            script
        );
        assert!(
            script.contains("preAllocatedVUs: 100,"),
            "preAllocatedVUs must equal --vus, not the last stage's target=0; got:\n{}",
            script
        );
        assert!(
            script.contains("maxVUs: 100,"),
            "maxVUs must equal --vus, not the last stage's target=0; got:\n{}",
            script
        );
        // Make sure the regression — `preAllocatedVUs: 0` from the ramp-down —
        // can never silently come back. Walk the lines so we don't false-
        // positive on the explanatory comment that lives in the template.
        for (idx, line) in script.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("/*") {
                continue;
            }
            assert!(
                !trimmed.starts_with("preAllocatedVUs: 0"),
                "regression at line {}: preAllocatedVUs is 0 — constant-arrival-rate \
                 will run no VUs (issue #79 round 5 ramp-up bug). Line: {:?}",
                idx + 1,
                line,
            );
        }
    }

    /// Companion to the test above: confirm `--cps` flips `noConnectionReuse`
    /// on. Issue #79 (round 5).
    #[test]
    fn test_cps_sets_no_connection_reuse() {
        use crate::spec_parser::ApiOperation;
        use openapiv3::Operation;

        let operation = ApiOperation {
            method: "get".to_string(),
            path: "/u".to_string(),
            operation: Operation::default(),
            operation_id: Some("u".to_string()),
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
            security_testing_enabled: false,
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: true,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
        };
        let script = K6ScriptGenerator::new(config, vec![template]).generate().unwrap();
        assert!(
            script.contains("noConnectionReuse: true"),
            "--cps must set noConnectionReuse: true on the k6 options block"
        );
        assert!(
            script.contains("Total Connections:"),
            "--cps summary must include connection-rate output (Srikanth's round-5 ask)"
        );
        assert!(
            script.contains("Connection Rate:"),
            "--cps summary must include 'Connection Rate:' (Srikanth's round-5 ask)"
        );
    }

    /// Issue #79 round 6 follow-up: Srikanth reported `--vus 5 -d 600s` taking
    /// until the 6-minute mark to reach 5 VUs because the script always set
    /// `startVUs: 0`, so k6's `ramping-vus` linearly ramped 0 → 5 over the
    /// whole window. For `--scenario constant` we now seed startVUs at the
    /// target so the test runs at full concurrency from t=0.
    #[test]
    fn test_constant_scenario_starts_at_target_vus() {
        use crate::spec_parser::ApiOperation;
        use openapiv3::Operation;

        let operation = ApiOperation {
            method: "get".to_string(),
            path: "/u".to_string(),
            operation: Operation::default(),
            operation_id: Some("u".to_string()),
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
            duration_secs: 600,
            max_vus: 5,
            threshold_percentile: "p(95)".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            auth_header: None,
            custom_headers: HashMap::new(),
            skip_tls_verify: false,
            security_testing_enabled: false,
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
        };
        let script = K6ScriptGenerator::new(config, vec![template]).generate().unwrap();
        assert!(
            script.contains("startVUs: 5,"),
            "--scenario constant must seed startVUs at max_vus, not 0; got:\n{}",
            script
        );
        // RampUp should still start at 0 — confirm we didn't break ramps.
        let ramp_config = K6Config {
            target_url: "https://api.example.com".to_string(),
            base_path: None,
            scenario: LoadScenario::RampUp,
            duration_secs: 600,
            max_vus: 5,
            threshold_percentile: "p(95)".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            auth_header: None,
            custom_headers: HashMap::new(),
            skip_tls_verify: false,
            security_testing_enabled: false,
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
        };
        let ramp_template = RequestTemplate {
            operation: ApiOperation {
                method: "get".to_string(),
                path: "/u".to_string(),
                operation: Operation::default(),
                operation_id: Some("u".to_string()),
            },
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: None,
        };
        let ramp_script =
            K6ScriptGenerator::new(ramp_config, vec![ramp_template]).generate().unwrap();
        assert!(
            ramp_script.contains("startVUs: 0,"),
            "--scenario ramp-up must keep startVUs at 0 so stages drive the ramp; got:\n{}",
            ramp_script
        );
    }

    /// Issue #79 round 6 follow-up: srikr's `--rps 100 --vus 5` summary showed
    /// no "Connections opened" line because the client-side connection counter
    /// was reading `http_req_connecting.values.count` — a field that doesn't
    /// exist (k6's Trend metric only emits avg/min/med/max/p90/p95). The fix
    /// adds a dedicated Counter (`mockforge_connections_opened`) that the
    /// template increments whenever `res.timings.connecting > 0`. This test
    /// guards both the metric declaration and the per-request increment so
    /// the connection counter can't silently regress.
    #[test]
    fn test_connections_opened_counter_present() {
        use crate::spec_parser::ApiOperation;
        use openapiv3::Operation;

        let operation = ApiOperation {
            method: "get".to_string(),
            path: "/u".to_string(),
            operation: Operation::default(),
            operation_id: Some("u".to_string()),
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
            security_testing_enabled: false,
            chunked_request_bodies: false,
            target_rps: Some(50),
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
        };
        let script = K6ScriptGenerator::new(config, vec![template]).generate().unwrap();
        assert!(
            script.contains("new Counter('mockforge_connections_opened')"),
            "template must declare the mockforge_connections_opened Counter"
        );
        assert!(
            script.contains("mockforge_connections_opened.add(1)"),
            "template must increment mockforge_connections_opened on new TCP connect"
        );
        assert!(
            script.contains("res.timings.connecting > 0"),
            "template must gate the connection-opened increment on \
             res.timings.connecting > 0 (only fires when a fresh socket was opened)"
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
            security_testing_enabled: false,
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
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
            security_testing_enabled: false,
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
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
            security_testing_enabled: false,
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
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
            security_testing_enabled: false,
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
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
            security_testing_enabled: false,
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
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
            security_testing_enabled: false,
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
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
            security_testing_enabled: false,
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
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
            security_testing_enabled: false,
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
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

    #[test]
    fn test_security_testing_enabled_generates_calling_code() {
        use crate::spec_parser::ApiOperation;
        use openapiv3::Operation;
        use serde_json::json;

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
            skip_tls_verify: false,
            security_testing_enabled: true,
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
        };

        let generator = K6ScriptGenerator::new(config, vec![template]);
        let script = generator.generate().expect("Should generate script");

        // Verify calling code is generated (not just function definitions)
        assert!(
            script.contains("getNextSecurityPayload"),
            "Script should contain getNextSecurityPayload() call when security_testing_enabled is true"
        );
        assert!(
            script.contains("applySecurityPayload"),
            "Script should contain applySecurityPayload() call when security_testing_enabled is true"
        );
        assert!(
            script.contains("secPayloadGroup"),
            "Script should contain secPayloadGroup variable when security_testing_enabled is true"
        );
        assert!(
            script.contains("secBodyPayload"),
            "Script should contain secBodyPayload variable when security_testing_enabled is true"
        );
        // Verify CookieJar skip when Cookie header payload is present
        assert!(
            script.contains("hasSecCookie"),
            "Script should track hasSecCookie for CookieJar conflict avoidance"
        );
        assert!(
            script.contains("secRequestOpts"),
            "Script should use secRequestOpts to conditionally skip CookieJar"
        );
        // Verify mutable headers copy for injection
        assert!(
            script.contains("const requestHeaders = { ..."),
            "Script should spread headers into mutable copy for security payload injection"
        );
        // Verify injectAsPath handling for path-based URI injection
        assert!(
            script.contains("secPayload.injectAsPath"),
            "Script should check injectAsPath for path-based URI injection"
        );
        // Verify formBody handling for form-encoded body delivery
        assert!(
            script.contains("secBodyPayload.formBody"),
            "Script should check formBody for form-encoded body delivery"
        );
        assert!(
            script.contains("application/x-www-form-urlencoded"),
            "Script should set Content-Type for form-encoded body"
        );
        // Verify secPayloadGroup is fetched per-operation (inside operation block), not per-iteration
        let op_comment_pos =
            script.find("// Operation 0:").expect("Should have Operation 0 comment");
        let sec_payload_pos = script
            .find("const secPayloadGroup = typeof getNextSecurityPayload")
            .expect("Should have secPayloadGroup assignment");
        assert!(
            sec_payload_pos > op_comment_pos,
            "secPayloadGroup should be fetched inside operation block (per-operation), not before it (per-iteration)"
        );
    }

    #[test]
    fn test_security_testing_disabled_no_calling_code() {
        use crate::spec_parser::ApiOperation;
        use openapiv3::Operation;
        use serde_json::json;

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
            skip_tls_verify: false,
            security_testing_enabled: false,
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
        };

        let generator = K6ScriptGenerator::new(config, vec![template]);
        let script = generator.generate().expect("Should generate script");

        // Verify calling code is NOT generated
        assert!(
            !script.contains("getNextSecurityPayload"),
            "Script should NOT contain getNextSecurityPayload() when security_testing_enabled is false"
        );
        assert!(
            !script.contains("applySecurityPayload"),
            "Script should NOT contain applySecurityPayload() when security_testing_enabled is false"
        );
        assert!(
            !script.contains("secPayloadGroup"),
            "Script should NOT contain secPayloadGroup variable when security_testing_enabled is false"
        );
        assert!(
            !script.contains("secBodyPayload"),
            "Script should NOT contain secBodyPayload variable when security_testing_enabled is false"
        );
        assert!(
            !script.contains("hasSecCookie"),
            "Script should NOT contain hasSecCookie when security_testing_enabled is false"
        );
        assert!(
            !script.contains("secRequestOpts"),
            "Script should NOT contain secRequestOpts when security_testing_enabled is false"
        );
        assert!(
            !script.contains("injectAsPath"),
            "Script should NOT contain injectAsPath when security_testing_enabled is false"
        );
        assert!(
            !script.contains("formBody"),
            "Script should NOT contain formBody when security_testing_enabled is false"
        );
    }

    /// End-to-end test: simulates the real pipeline of template rendering + enhanced script
    /// injection. This is what actually runs when a user passes `--security-test`.
    /// Verifies that the FINAL script has both function definitions AND calling code.
    #[test]
    fn test_security_e2e_definitions_and_calls_both_present() {
        use crate::security_payloads::{
            SecurityPayloads, SecurityTestConfig, SecurityTestGenerator,
        };
        use crate::spec_parser::ApiOperation;
        use openapiv3::Operation;
        use serde_json::json;

        // Step 1: Generate base script with security_testing_enabled=true (template renders calling code)
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
            skip_tls_verify: false,
            security_testing_enabled: true,
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
        };

        let generator = K6ScriptGenerator::new(config, vec![template]);
        let mut script = generator.generate().expect("Should generate base script");

        // Step 2: Simulate what generate_enhanced_script() does — inject function definitions
        let security_config = SecurityTestConfig::default().enable();
        let payloads = SecurityPayloads::get_payloads(&security_config);
        assert!(!payloads.is_empty(), "Should have built-in payloads");

        let mut additional_code = String::new();
        additional_code
            .push_str(&SecurityTestGenerator::generate_payload_selection(&payloads, false));
        additional_code.push('\n');
        additional_code.push_str(&SecurityTestGenerator::generate_apply_payload(&[]));
        additional_code.push('\n');

        // Insert definitions before 'export const options' (same as generate_enhanced_script)
        if let Some(pos) = script.find("export const options") {
            script.insert_str(
                pos,
                &format!("\n// === Advanced Testing Features ===\n{}\n", additional_code),
            );
        }

        // Step 3: Verify the FINAL script has BOTH definitions AND calls
        // Function definitions (injected by generate_enhanced_script)
        assert!(
            script.contains("function getNextSecurityPayload()"),
            "Final script must contain getNextSecurityPayload function DEFINITION"
        );
        assert!(
            script.contains("function applySecurityPayload("),
            "Final script must contain applySecurityPayload function DEFINITION"
        );
        assert!(
            script.contains("securityPayloads"),
            "Final script must contain securityPayloads array"
        );

        // Calling code (rendered by template)
        assert!(
            script.contains("const secPayloadGroup = typeof getNextSecurityPayload"),
            "Final script must contain secPayloadGroup assignment (template calling code)"
        );
        assert!(
            script.contains("applySecurityPayload(payload, [], secBodyPayload)"),
            "Final script must contain applySecurityPayload CALL with secBodyPayload"
        );
        assert!(
            script.contains("const requestHeaders = { ..."),
            "Final script must spread headers for security payload header injection"
        );
        assert!(
            script.contains("for (const secPayload of secPayloadGroup)"),
            "Final script must loop over secPayloadGroup"
        );
        assert!(
            script.contains("secPayload.injectAsPath"),
            "Final script must check injectAsPath for path-based URI injection"
        );
        assert!(
            script.contains("secBodyPayload.formBody"),
            "Final script must check formBody for form-encoded body delivery"
        );

        // Verify ordering: definitions come BEFORE export default function (which has the calls)
        let def_pos = script.find("function getNextSecurityPayload()").unwrap();
        let call_pos =
            script.find("const secPayloadGroup = typeof getNextSecurityPayload").unwrap();
        let options_pos = script.find("export const options").unwrap();
        let default_fn_pos = script.find("export default function").unwrap();

        assert!(
            def_pos < options_pos,
            "Function definitions must appear before export const options"
        );
        assert!(
            call_pos > default_fn_pos,
            "Calling code must appear inside export default function"
        );
    }

    /// Test that URI security payload injection is generated for GET requests
    #[test]
    fn test_security_uri_injection_for_get_requests() {
        use crate::spec_parser::ApiOperation;
        use openapiv3::Operation;

        let operation = ApiOperation {
            method: "get".to_string(),
            path: "/api/users".to_string(),
            operation: Operation::default(),
            operation_id: Some("listUsers".to_string()),
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
            security_testing_enabled: true,
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
        };

        let generator = K6ScriptGenerator::new(config, vec![template]);
        let script = generator.generate().expect("Should generate script");

        // Verify URI injection code is present for GET requests
        assert!(
            script.contains("requestUrl"),
            "Script should build requestUrl variable for URI payload injection"
        );
        assert!(
            script.contains("secPayload.location === 'uri'"),
            "Script should check for URI-location payloads"
        );
        // URI payloads are URL-encoded for valid HTTP; WAF decodes before inspection
        assert!(
            script.contains("'test=' + encodeURIComponent(secPayload.payload)"),
            "Script should URL-encode security payload in query string for valid HTTP"
        );
        // Verify injectAsPath check for path-based injection
        assert!(
            script.contains("secPayload.injectAsPath"),
            "Script should check injectAsPath for path-based URI injection"
        );
        assert!(
            script.contains("encodeURI(secPayload.payload)"),
            "Script should use encodeURI for path-based injection"
        );
        // Verify the GET request uses requestUrl
        assert!(
            script.contains("http.get(requestUrl,"),
            "GET request should use requestUrl (with URI injection) instead of inline URL"
        );
    }

    /// Test that URI security payload injection is generated for POST requests with body
    #[test]
    fn test_security_uri_injection_for_post_requests() {
        use crate::spec_parser::ApiOperation;
        use openapiv3::Operation;
        use serde_json::json;

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
            skip_tls_verify: false,
            security_testing_enabled: true,
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
        };

        let generator = K6ScriptGenerator::new(config, vec![template]);
        let script = generator.generate().expect("Should generate script");

        // POST with body should get BOTH URI injection AND body injection
        assert!(
            script.contains("requestUrl"),
            "POST script should build requestUrl for URI payload injection"
        );
        assert!(
            script.contains("secPayload.location === 'uri'"),
            "POST script should check for URI-location payloads"
        );
        assert!(
            script.contains("applySecurityPayload(payload, [], secBodyPayload)"),
            "POST script should apply security body payload to request body"
        );
        // Verify the POST request uses requestUrl
        assert!(
            script.contains("http.post(requestUrl,"),
            "POST request should use requestUrl (with URI injection) instead of inline URL"
        );
    }

    /// Test that security is disabled - no URI injection code present
    #[test]
    fn test_no_uri_injection_when_security_disabled() {
        use crate::spec_parser::ApiOperation;
        use openapiv3::Operation;

        let operation = ApiOperation {
            method: "get".to_string(),
            path: "/api/users".to_string(),
            operation: Operation::default(),
            operation_id: Some("listUsers".to_string()),
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
            security_testing_enabled: false,
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
        };

        let generator = K6ScriptGenerator::new(config, vec![template]);
        let script = generator.generate().expect("Should generate script");

        // Verify NO security injection code when disabled
        assert!(
            !script.contains("requestUrl"),
            "Script should NOT have requestUrl when security is disabled"
        );
        assert!(
            !script.contains("secPayloadGroup"),
            "Script should NOT have secPayloadGroup when security is disabled"
        );
        assert!(
            !script.contains("secBodyPayload"),
            "Script should NOT have secBodyPayload when security is disabled"
        );
    }

    /// Test that scripts create a fresh CookieJar per request (not a shared constant)
    #[test]
    fn test_uses_per_request_cookie_jar() {
        use crate::spec_parser::ApiOperation;
        use openapiv3::Operation;

        let operation = ApiOperation {
            method: "get".to_string(),
            path: "/api/users".to_string(),
            operation: Operation::default(),
            operation_id: Some("listUsers".to_string()),
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
            security_testing_enabled: false,
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
        };

        let generator = K6ScriptGenerator::new(config, vec![template]);
        let script = generator.generate().expect("Should generate script");

        // Each request must create a fresh CookieJar to prevent Set-Cookie accumulation
        assert!(
            script.contains("jar: new http.CookieJar()"),
            "Script should create fresh CookieJar per request"
        );
        assert!(
            !script.contains("jar: null"),
            "Script should NOT use jar: null (does not disable default VU cookie jar in k6)"
        );
        assert!(
            !script.contains("EMPTY_JAR"),
            "Script should NOT use shared EMPTY_JAR (accumulates Set-Cookie responses)"
        );
    }
}
