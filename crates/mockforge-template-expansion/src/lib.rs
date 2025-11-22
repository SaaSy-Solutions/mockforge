//! Template expansion utilities for request context variables
//!
//! This crate provides Send-safe template expansion that does NOT use rng().
//! It is completely isolated from the templating module in mockforge-core
//! to avoid Send issues in async contexts.
//!
//! The key difference from `mockforge-core::templating` is that this crate
//! only performs simple string replacements based on request context, and
//! does not use any random number generation or other non-Send types.
//!
//! **Important**: This crate does NOT depend on `mockforge-core` to avoid
//! bringing `rng()` into scope. `RequestContext` is duplicated here.

use serde_json::Value;
use std::collections::HashMap;

/// Request context for prompt template expansion
///
/// This is a duplicate of `mockforge_core::ai_response::RequestContext`
/// to avoid depending on `mockforge-core` which has `rng()` in scope.
#[derive(Debug, Clone, Default)]
pub struct RequestContext {
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Request path
    pub path: String,
    /// Path parameters
    pub path_params: HashMap<String, Value>,
    /// Query parameters
    pub query_params: HashMap<String, Value>,
    /// Request headers
    pub headers: HashMap<String, Value>,
    /// Request body (if JSON)
    pub body: Option<Value>,
    /// Multipart form fields (for multipart/form-data requests)
    pub multipart_fields: HashMap<String, Value>,
    /// Multipart file uploads (filename -> file path)
    pub multipart_files: HashMap<String, String>,
}

impl RequestContext {
    /// Create a new request context
    pub fn new(method: String, path: String) -> Self {
        Self {
            method,
            path,
            ..Default::default()
        }
    }

    /// Set path parameters
    pub fn with_path_params(mut self, params: HashMap<String, Value>) -> Self {
        self.path_params = params;
        self
    }

    /// Set query parameters
    pub fn with_query_params(mut self, params: HashMap<String, Value>) -> Self {
        self.query_params = params;
        self
    }

    /// Set headers
    pub fn with_headers(mut self, headers: HashMap<String, Value>) -> Self {
        self.headers = headers;
        self
    }

    /// Set body
    pub fn with_body(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }

    /// Set multipart form fields
    pub fn with_multipart_fields(mut self, fields: HashMap<String, Value>) -> Self {
        self.multipart_fields = fields;
        self
    }

    /// Set multipart file uploads
    pub fn with_multipart_files(mut self, files: HashMap<String, String>) -> Self {
        self.multipart_files = files;
        self
    }
}

/// Expand template variables in a prompt string using request context
///
/// This function is Send-safe and does not use rng() or any non-Send types.
/// It only performs simple string replacements based on the request context.
///
/// # Arguments
/// * `template` - Template string with variables like `{{method}}`, `{{path}}`, `{{query.name}}`, etc.
/// * `context` - Request context containing method, path, query params, headers, body, etc.
///
/// # Returns
/// String with all template variables replaced with actual values from context
///
/// # Example
/// ```
/// use mockforge_template_expansion::{expand_prompt_template, RequestContext};
/// use serde_json::json;
/// use std::collections::HashMap;
///
/// let mut query_params = HashMap::new();
/// query_params.insert("search".to_string(), json!("term"));
///
/// let context = RequestContext::new("GET".to_string(), "/api/search".to_string())
///     .with_query_params(query_params);
///
/// let template = "Search for {{query.search}} on path {{path}}";
/// let expanded = expand_prompt_template(template, &context);
/// assert_eq!(expanded, "Search for term on path /api/search");
/// ```
pub fn expand_prompt_template(template: &str, context: &RequestContext) -> String {
    let mut result = template.to_string();

    // Replace {{method}}
    result = result.replace("{{method}}", &context.method);

    // Replace {{path}}
    result = result.replace("{{path}}", &context.path);

    // Replace {{body.*}} variables
    if let Some(body) = &context.body {
        result = expand_json_variables(&result, "body", body);
    }

    // Replace {{path.*}} variables
    result = expand_map_variables(&result, "path", &context.path_params);

    // Replace {{query.*}} variables
    result = expand_map_variables(&result, "query", &context.query_params);

    // Replace {{headers.*}} variables
    result = expand_map_variables(&result, "headers", &context.headers);

    // Replace {{multipart.*}} variables for form fields
    result = expand_map_variables(&result, "multipart", &context.multipart_fields);

    result
}

/// Expand template variables from a JSON value
///
/// This helper function extracts values from a JSON object and replaces
/// template placeholders like `{{body.field}}` with the actual field value.
fn expand_json_variables(template: &str, prefix: &str, value: &Value) -> String {
    let mut result = template.to_string();

    // Handle object fields
    if let Some(obj) = value.as_object() {
        for (key, val) in obj {
            let placeholder = format!("{{{{{}.{}}}}}", prefix, key);
            let replacement = match val {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Null => "null".to_string(),
                _ => serde_json::to_string(val).unwrap_or_default(),
            };
            result = result.replace(&placeholder, &replacement);
        }
    }

    result
}

/// Expand template variables from a HashMap
///
/// This helper function extracts values from a HashMap and replaces
/// template placeholders like `{{query.name}}` with the actual value.
fn expand_map_variables(template: &str, prefix: &str, map: &std::collections::HashMap<String, Value>) -> String {
    let mut result = template.to_string();

    for (key, val) in map {
        let placeholder = format!("{{{{{}.{}}}}}", prefix, key);
        let replacement = match val {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            _ => serde_json::to_string(val).unwrap_or_default(),
        };
        result = result.replace(&placeholder, &replacement);
    }

    result
}

/// Expand template variables in a JSON value recursively using request context
///
/// This function is Send-safe and does not use rng() or any non-Send types.
/// It only uses expand_prompt_template which performs simple string replacements.
///
/// # Arguments
/// * `value` - JSON value to process
/// * `context` - Request context for template variable expansion
///
/// # Returns
/// New JSON value with all template tokens expanded
///
/// # Example
/// ```
/// use mockforge_template_expansion::{expand_templates_in_json, RequestContext};
/// use serde_json::json;
///
/// let context = RequestContext::new("GET".to_string(), "/api/users".to_string());
/// let value = json!({
///     "message": "Request to {{path}}",
///     "method": "{{method}}"
/// });
///
/// let expanded = expand_templates_in_json(value, &context);
/// assert_eq!(expanded["message"], "Request to /api/users");
/// assert_eq!(expanded["method"], "GET");
/// ```
pub fn expand_templates_in_json(
    value: Value,
    context: &RequestContext,
) -> Value {
    match value {
        Value::String(s) => {
            // Normalize {{request.query.name}} to {{query.name}} format for compatibility
            let normalized = s
                .replace("{{request.query.", "{{query.")
                .replace("{{request.path.", "{{path.")
                .replace("{{request.headers.", "{{headers.")
                .replace("{{request.body.", "{{body.")
                .replace("{{request.method}}", "{{method}}")
                .replace("{{request.path}}", "{{path}}");
            // Use expand_prompt_template which is Send-safe and doesn't use rng()
            Value::String(expand_prompt_template(&normalized, context))
        }
        Value::Array(arr) => Value::Array(
            arr.into_iter()
                .map(|v| expand_templates_in_json(v, context))
                .collect(),
        ),
        Value::Object(obj) => Value::Object(
            obj.into_iter()
                .map(|(k, v)| (k, expand_templates_in_json(v, context)))
                .collect(),
        ),
        _ => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_expand_prompt_template_basic() {
        let context = RequestContext::new("GET".to_string(), "/users".to_string());
        let template = "Method: {{method}}, Path: {{path}}";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, "Method: GET, Path: /users");
    }

    #[test]
    fn test_expand_prompt_template_body() {
        let body = json!({
            "message": "Hello",
            "user": "Alice"
        });
        let context = RequestContext::new("POST".to_string(), "/chat".to_string()).with_body(body);

        let template = "User {{body.user}} says: {{body.message}}";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, "User Alice says: Hello");
    }

    #[test]
    fn test_expand_prompt_template_path_params() {
        let mut path_params = HashMap::new();
        path_params.insert("id".to_string(), json!("456"));
        path_params.insert("name".to_string(), json!("test"));

        let context = RequestContext::new("GET".to_string(), "/users/456".to_string())
            .with_path_params(path_params);

        let template = "Get user {{path.id}} with name {{path.name}}";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, "Get user 456 with name test");
    }

    #[test]
    fn test_expand_prompt_template_query_params() {
        let mut query_params = HashMap::new();
        query_params.insert("search".to_string(), json!("term"));
        query_params.insert("limit".to_string(), json!(10));

        let context = RequestContext::new("GET".to_string(), "/search".to_string())
            .with_query_params(query_params);

        let template = "Search for {{query.search}} with limit {{query.limit}}";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, "Search for term with limit 10");
    }

    #[test]
    fn test_expand_prompt_template_headers() {
        let mut headers = HashMap::new();
        headers.insert("user-agent".to_string(), json!("TestClient/1.0"));

        let context =
            RequestContext::new("GET".to_string(), "/api".to_string()).with_headers(headers);

        let template = "Request from {{headers.user-agent}}";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, "Request from TestClient/1.0");
    }

    #[test]
    fn test_expand_prompt_template_complex() {
        let mut path_params = HashMap::new();
        path_params.insert("id".to_string(), json!("789"));

        let mut query_params = HashMap::new();
        query_params.insert("format".to_string(), json!("json"));

        let body = json!({"action": "update", "value": 42});

        let context = RequestContext::new("PUT".to_string(), "/api/items/789".to_string())
            .with_path_params(path_params)
            .with_query_params(query_params)
            .with_body(body);

        let template = "{{method}} item {{path.id}} with action {{body.action}} and value {{body.value}} in format {{query.format}}";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, "PUT item 789 with action update and value 42 in format json");
    }

    #[test]
    fn test_expand_templates_in_json() {
        let context = RequestContext::new("GET".to_string(), "/api/users".to_string());
        let value = json!({
            "message": "Request to {{path}}",
            "method": "{{method}}",
            "nested": {
                "path": "{{path}}"
            },
            "array": ["{{method}}", "{{path}}"]
        });

        let expanded = expand_templates_in_json(value, &context);
        assert_eq!(expanded["message"], "Request to /api/users");
        assert_eq!(expanded["method"], "GET");
        assert_eq!(expanded["nested"]["path"], "/api/users");
        assert_eq!(expanded["array"][0], "GET");
        assert_eq!(expanded["array"][1], "/api/users");
    }

    #[test]
    fn test_expand_templates_in_json_normalize_request_prefix() {
        let context = RequestContext::new("POST".to_string(), "/api/data".to_string());
        let value = json!({
            "message": "{{request.method}} {{request.path}}",
            "query": "{{request.query.name}}"
        });

        let expanded = expand_templates_in_json(value, &context);
        assert_eq!(expanded["message"], "POST /api/data");
        // Note: query.name won't be expanded since it's not in context, but normalization should work
    }
}
