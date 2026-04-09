//! Custom conformance test authoring via YAML
//!
//! Allows users to define additional conformance checks beyond the built-in
//! OpenAPI 3.0.0 feature set. Custom checks are grouped under a "Custom"
//! category in the conformance report.

use crate::error::{BenchError, Result};
use serde::Deserialize;
use std::path::Path;

/// Top-level YAML configuration for custom conformance checks
#[derive(Debug, Deserialize)]
pub struct CustomConformanceConfig {
    /// List of custom checks to run
    pub custom_checks: Vec<CustomCheck>,
}

/// A single custom conformance check
#[derive(Debug, Deserialize)]
pub struct CustomCheck {
    /// Check name (should start with "custom:" for report aggregation)
    pub name: String,
    /// Request path (e.g., "/api/users")
    pub path: String,
    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    pub method: String,
    /// Expected HTTP status code
    pub expected_status: u16,
    /// Optional request body (JSON string)
    #[serde(default)]
    pub body: Option<String>,
    /// Optional expected response headers (name -> regex pattern)
    #[serde(default)]
    pub expected_headers: std::collections::HashMap<String, String>,
    /// Optional expected body fields with type validation
    #[serde(default)]
    pub expected_body_fields: Vec<ExpectedBodyField>,
    /// Optional request headers
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
}

/// Expected field in the response body with type checking
#[derive(Debug, Deserialize)]
pub struct ExpectedBodyField {
    /// Field name in the JSON response
    pub name: String,
    /// Expected JSON type: "string", "integer", "number", "boolean", "array", "object"
    #[serde(rename = "type")]
    pub field_type: String,
}

impl CustomConformanceConfig {
    /// Parse a custom conformance config from a YAML file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            BenchError::Other(format!(
                "Failed to read custom conformance file '{}': {}",
                path.display(),
                e
            ))
        })?;
        serde_yaml::from_str(&content).map_err(|e| {
            BenchError::Other(format!(
                "Failed to parse custom conformance YAML '{}': {}",
                path.display(),
                e
            ))
        })
    }

    /// Generate a k6 `group('Custom', ...)` block for all custom checks.
    ///
    /// `base_url` is the JS expression for the base URL (e.g., `"BASE_URL"`).
    /// `custom_headers` are additional headers to inject into every request.
    pub fn generate_k6_group(&self, base_url: &str, custom_headers: &[(String, String)]) -> String {
        let mut script = String::with_capacity(4096);
        script.push_str("  group('Custom', function () {\n");

        for check in &self.custom_checks {
            script.push_str("    {\n");

            // Build headers object
            let mut all_headers: Vec<(String, String)> = Vec::new();
            // Add check-specific headers
            for (k, v) in &check.headers {
                all_headers.push((k.clone(), v.clone()));
            }
            // Add global custom headers (check-specific take priority)
            for (k, v) in custom_headers {
                if !check.headers.contains_key(k) {
                    all_headers.push((k.clone(), v.clone()));
                }
            }
            // If posting JSON body, add Content-Type
            if check.body.is_some()
                && !all_headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            {
                all_headers.push(("Content-Type".to_string(), "application/json".to_string()));
            }

            let headers_js = if all_headers.is_empty() {
                "{}".to_string()
            } else {
                let entries: Vec<String> = all_headers
                    .iter()
                    .map(|(k, v)| format!("'{}': '{}'", k, v.replace('\'', "\\'")))
                    .collect();
                format!("{{ {} }}", entries.join(", "))
            };

            let method = check.method.to_uppercase();
            let url = format!("${{{}}}{}", base_url, check.path);
            let escaped_name = check.name.replace('\'', "\\'");

            match method.as_str() {
                "GET" | "HEAD" | "OPTIONS" | "DELETE" => {
                    let k6_method = match method.as_str() {
                        "DELETE" => "del",
                        other => &other.to_lowercase(),
                    };
                    if all_headers.is_empty() {
                        script
                            .push_str(&format!("      let res = http.{}(`{}`);\n", k6_method, url));
                    } else {
                        script.push_str(&format!(
                            "      let res = http.{}(`{}`, {{ headers: {} }});\n",
                            k6_method, url, headers_js
                        ));
                    }
                }
                _ => {
                    // POST, PUT, PATCH
                    let k6_method = method.to_lowercase();
                    let body_expr = match &check.body {
                        Some(b) => format!("'{}'", b.replace('\'', "\\'")),
                        None => "null".to_string(),
                    };
                    script.push_str(&format!(
                        "      let res = http.{}(`{}`, {}, {{ headers: {} }});\n",
                        k6_method, url, body_expr, headers_js
                    ));
                }
            }

            // Status check with failure detail capture
            script.push_str(&format!(
                "      {{ let ok = check(res, {{ '{}': (r) => r.status === {} }}); if (!ok) __captureFailure('{}', res, 'status === {}'); }}\n",
                escaped_name, check.expected_status, escaped_name, check.expected_status
            ));

            // Header checks with failure detail capture.
            // k6 canonicalizes response header names (e.g. `X-XSS-Protection` ->
            // `X-Xss-Protection`), so match header names case-insensitively.
            for (header_name, pattern) in &check.expected_headers {
                let header_check_name = format!("{}:header:{}", escaped_name, header_name);
                let escaped_pattern = pattern.replace('\\', "\\\\").replace('\'', "\\'");
                let header_lower = header_name.to_lowercase();
                script.push_str(&format!(
                    "      {{ let ok = check(res, {{ '{}': (r) => {{ const _hk = Object.keys(r.headers || {{}}).find(k => k.toLowerCase() === '{}'); return new RegExp('{}').test(_hk ? r.headers[_hk] : ''); }} }}); if (!ok) __captureFailure('{}', res, 'header {} matches /{}/ '); }}\n",
                    header_check_name,
                    header_lower,
                    escaped_pattern,
                    header_check_name,
                    header_name,
                    escaped_pattern
                ));
            }

            // Body field checks
            for field in &check.expected_body_fields {
                let field_check_name =
                    format!("{}:body:{}:{}", escaped_name, field.name, field.field_type);
                // Generate JS expression to access the field value, supporting
                // nested paths like "results.name" and "items[].id"
                let accessor = generate_field_accessor(&field.name);
                let type_check = match field.field_type.as_str() {
                    "string" => format!("typeof ({}) === 'string'", accessor),
                    "integer" => format!("Number.isInteger({})", accessor),
                    "number" => format!("typeof ({}) === 'number'", accessor),
                    "boolean" => format!("typeof ({}) === 'boolean'", accessor),
                    "array" => format!("Array.isArray({})", accessor),
                    "object" => format!(
                        "typeof ({}) === 'object' && !Array.isArray({})",
                        accessor, accessor
                    ),
                    _ => format!("({}) !== undefined", accessor),
                };
                script.push_str(&format!(
                    "      {{ let ok = check(res, {{ '{}': (r) => {{ try {{ return {}; }} catch(e) {{ return false; }} }} }}); if (!ok) __captureFailure('{}', res, 'body field {} is {}'); }}\n",
                    field_check_name, type_check, field_check_name, field.name, field.field_type
                ));
            }

            script.push_str("    }\n");
        }

        script.push_str("  });\n\n");
        script
    }
}

/// Generate a JavaScript expression to access a field in a parsed JSON body.
///
/// Supports three path formats:
/// - Simple key: `"name"` → `JSON.parse(r.body)['name']`
/// - Dot-notation: `"config.enabled"` → `JSON.parse(r.body)['config']['enabled']`
/// - Array bracket: `"items[].id"` → `JSON.parse(r.body)['items'][0]['id']`
fn generate_field_accessor(field_name: &str) -> String {
    // Split on dots, handling [] array notation
    let parts: Vec<&str> = field_name.split('.').collect();
    let mut expr = String::from("JSON.parse(r.body)");

    for part in &parts {
        if let Some(arr_name) = part.strip_suffix("[]") {
            // Array field — access the array then index first element
            expr.push_str(&format!("['{}'][0]", arr_name));
        } else {
            expr.push_str(&format!("['{}']", part));
        }
    }

    expr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_custom_yaml() {
        let yaml = r#"
custom_checks:
  - name: "custom:pets-returns-200"
    path: /pets
    method: GET
    expected_status: 200
  - name: "custom:create-product"
    path: /api/products
    method: POST
    expected_status: 201
    body: '{"sku": "TEST-001", "name": "Test"}'
    expected_body_fields:
      - name: id
        type: integer
    expected_headers:
      content-type: "application/json"
"#;
        let config: CustomConformanceConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.custom_checks.len(), 2);
        assert_eq!(config.custom_checks[0].name, "custom:pets-returns-200");
        assert_eq!(config.custom_checks[0].expected_status, 200);
        assert_eq!(config.custom_checks[1].expected_body_fields.len(), 1);
        assert_eq!(config.custom_checks[1].expected_body_fields[0].name, "id");
        assert_eq!(config.custom_checks[1].expected_body_fields[0].field_type, "integer");
    }

    #[test]
    fn test_generate_k6_group_get() {
        let config = CustomConformanceConfig {
            custom_checks: vec![CustomCheck {
                name: "custom:test-get".to_string(),
                path: "/api/test".to_string(),
                method: "GET".to_string(),
                expected_status: 200,
                body: None,
                expected_headers: std::collections::HashMap::new(),
                expected_body_fields: vec![],
                headers: std::collections::HashMap::new(),
            }],
        };

        let script = config.generate_k6_group("BASE_URL", &[]);
        assert!(script.contains("group('Custom'"));
        assert!(script.contains("http.get(`${BASE_URL}/api/test`)"));
        assert!(script.contains("'custom:test-get': (r) => r.status === 200"));
    }

    #[test]
    fn test_generate_k6_group_post_with_body() {
        let config = CustomConformanceConfig {
            custom_checks: vec![CustomCheck {
                name: "custom:create".to_string(),
                path: "/api/items".to_string(),
                method: "POST".to_string(),
                expected_status: 201,
                body: Some(r#"{"name": "test"}"#.to_string()),
                expected_headers: std::collections::HashMap::new(),
                expected_body_fields: vec![ExpectedBodyField {
                    name: "id".to_string(),
                    field_type: "integer".to_string(),
                }],
                headers: std::collections::HashMap::new(),
            }],
        };

        let script = config.generate_k6_group("BASE_URL", &[]);
        assert!(script.contains("http.post("));
        assert!(script.contains("'custom:create': (r) => r.status === 201"));
        assert!(script.contains("custom:create:body:id:integer"));
        assert!(script.contains("Number.isInteger"));
    }

    #[test]
    fn test_generate_k6_group_with_header_checks() {
        let mut expected_headers = std::collections::HashMap::new();
        expected_headers.insert("content-type".to_string(), "application/json".to_string());

        let config = CustomConformanceConfig {
            custom_checks: vec![CustomCheck {
                name: "custom:header-check".to_string(),
                path: "/api/test".to_string(),
                method: "GET".to_string(),
                expected_status: 200,
                body: None,
                expected_headers,
                expected_body_fields: vec![],
                headers: std::collections::HashMap::new(),
            }],
        };

        let script = config.generate_k6_group("BASE_URL", &[]);
        assert!(script.contains("custom:header-check:header:content-type"));
        assert!(script.contains("new RegExp('application/json')"));
    }

    #[test]
    fn test_generate_k6_group_with_custom_headers() {
        let config = CustomConformanceConfig {
            custom_checks: vec![CustomCheck {
                name: "custom:auth-test".to_string(),
                path: "/api/secure".to_string(),
                method: "GET".to_string(),
                expected_status: 200,
                body: None,
                expected_headers: std::collections::HashMap::new(),
                expected_body_fields: vec![],
                headers: std::collections::HashMap::new(),
            }],
        };

        let custom_headers = vec![("Authorization".to_string(), "Bearer token123".to_string())];
        let script = config.generate_k6_group("BASE_URL", &custom_headers);
        assert!(script.contains("'Authorization': 'Bearer token123'"));
    }

    #[test]
    fn test_failure_capture_emitted() {
        let config = CustomConformanceConfig {
            custom_checks: vec![CustomCheck {
                name: "custom:capture-test".to_string(),
                path: "/api/test".to_string(),
                method: "GET".to_string(),
                expected_status: 200,
                body: None,
                expected_headers: {
                    let mut m = std::collections::HashMap::new();
                    m.insert("X-Rate-Limit".to_string(), ".*".to_string());
                    m
                },
                expected_body_fields: vec![ExpectedBodyField {
                    name: "id".to_string(),
                    field_type: "integer".to_string(),
                }],
                headers: std::collections::HashMap::new(),
            }],
        };

        let script = config.generate_k6_group("BASE_URL", &[]);
        // Status check should call __captureFailure on failure
        assert!(
            script.contains("__captureFailure('custom:capture-test', res, 'status === 200')"),
            "Status check should emit __captureFailure"
        );
        // Header check should call __captureFailure on failure
        assert!(
            script.contains("__captureFailure('custom:capture-test:header:X-Rate-Limit'"),
            "Header check should emit __captureFailure"
        );
        // Body field check should call __captureFailure on failure
        assert!(
            script.contains("__captureFailure('custom:capture-test:body:id:integer'"),
            "Body field check should emit __captureFailure"
        );
    }

    #[test]
    fn test_from_file_nonexistent() {
        let result = CustomConformanceConfig::from_file(Path::new("/nonexistent/file.yaml"));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Failed to read custom conformance file"));
    }

    #[test]
    fn test_generate_k6_group_delete() {
        let config = CustomConformanceConfig {
            custom_checks: vec![CustomCheck {
                name: "custom:delete-item".to_string(),
                path: "/api/items/1".to_string(),
                method: "DELETE".to_string(),
                expected_status: 204,
                body: None,
                expected_headers: std::collections::HashMap::new(),
                expected_body_fields: vec![],
                headers: std::collections::HashMap::new(),
            }],
        };

        let script = config.generate_k6_group("BASE_URL", &[]);
        assert!(script.contains("http.del("));
        assert!(script.contains("r.status === 204"));
    }

    #[test]
    fn test_field_accessor_simple() {
        assert_eq!(generate_field_accessor("name"), "JSON.parse(r.body)['name']");
    }

    #[test]
    fn test_field_accessor_nested_dot() {
        assert_eq!(
            generate_field_accessor("config.enabled"),
            "JSON.parse(r.body)['config']['enabled']"
        );
    }

    #[test]
    fn test_field_accessor_array_bracket() {
        assert_eq!(generate_field_accessor("items[].id"), "JSON.parse(r.body)['items'][0]['id']");
    }

    #[test]
    fn test_field_accessor_deep_nested() {
        assert_eq!(generate_field_accessor("a.b.c"), "JSON.parse(r.body)['a']['b']['c']");
    }

    #[test]
    fn test_generate_k6_nested_body_fields() {
        let config = CustomConformanceConfig {
            custom_checks: vec![CustomCheck {
                name: "custom:nested".to_string(),
                path: "/api/data".to_string(),
                method: "GET".to_string(),
                expected_status: 200,
                body: None,
                expected_headers: std::collections::HashMap::new(),
                expected_body_fields: vec![
                    ExpectedBodyField {
                        name: "count".to_string(),
                        field_type: "integer".to_string(),
                    },
                    ExpectedBodyField {
                        name: "results[].name".to_string(),
                        field_type: "string".to_string(),
                    },
                ],
                headers: std::collections::HashMap::new(),
            }],
        };

        let script = config.generate_k6_group("BASE_URL", &[]);
        // Simple field should use direct bracket access
        assert!(script.contains("JSON.parse(r.body)['count']"));
        // Nested array field should use [0] for array traversal
        assert!(script.contains("JSON.parse(r.body)['results'][0]['name']"));
    }
}
