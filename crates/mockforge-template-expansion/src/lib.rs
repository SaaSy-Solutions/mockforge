//! Template expansion utilities for request context variables
//!
//! This crate provides Send-safe template expansion that does NOT use `rng()`.
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
    #[must_use]
    pub fn new(method: String, path: String) -> Self {
        Self {
            method,
            path,
            ..Default::default()
        }
    }

    /// Set path parameters
    #[must_use]
    pub fn with_path_params(mut self, params: HashMap<String, Value>) -> Self {
        self.path_params = params;
        self
    }

    /// Set query parameters
    #[must_use]
    pub fn with_query_params(mut self, params: HashMap<String, Value>) -> Self {
        self.query_params = params;
        self
    }

    /// Set headers
    #[must_use]
    pub fn with_headers(mut self, headers: HashMap<String, Value>) -> Self {
        self.headers = headers;
        self
    }

    /// Set body
    #[must_use]
    pub fn with_body(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }

    /// Set multipart form fields
    #[must_use]
    pub fn with_multipart_fields(mut self, fields: HashMap<String, Value>) -> Self {
        self.multipart_fields = fields;
        self
    }

    /// Set multipart file uploads
    #[must_use]
    pub fn with_multipart_files(mut self, files: HashMap<String, String>) -> Self {
        self.multipart_files = files;
        self
    }
}

/// Expand template variables in a prompt string using request context
///
/// This function is Send-safe and does not use `rng()` or any non-Send types.
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
#[must_use]
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
            let placeholder = format!("{{{{{prefix}.{key}}}}}");
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

/// Expand template variables from a `HashMap`
///
/// This helper function extracts values from a `HashMap` and replaces
/// template placeholders like `{{query.name}}` with the actual value.
fn expand_map_variables(template: &str, prefix: &str, map: &HashMap<String, Value>) -> String {
    let mut result = template.to_string();

    for (key, val) in map {
        let placeholder = format!("{{{{{prefix}.{key}}}}}");
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
/// This function is Send-safe and does not use `rng()` or any non-Send types.
/// It only uses `expand_prompt_template` which performs simple string replacements.
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
#[must_use]
pub fn expand_templates_in_json(value: Value, context: &RequestContext) -> Value {
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
        Value::Array(arr) => {
            Value::Array(arr.into_iter().map(|v| expand_templates_in_json(v, context)).collect())
        }
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

    // ==================== RequestContext Tests ====================

    #[test]
    fn test_request_context_new() {
        let ctx = RequestContext::new("POST".to_string(), "/api/test".to_string());
        assert_eq!(ctx.method, "POST");
        assert_eq!(ctx.path, "/api/test");
        assert!(ctx.path_params.is_empty());
        assert!(ctx.query_params.is_empty());
        assert!(ctx.headers.is_empty());
        assert!(ctx.body.is_none());
        assert!(ctx.multipart_fields.is_empty());
        assert!(ctx.multipart_files.is_empty());
    }

    #[test]
    fn test_request_context_default() {
        let ctx = RequestContext::default();
        assert_eq!(ctx.method, "");
        assert_eq!(ctx.path, "");
        assert!(ctx.path_params.is_empty());
        assert!(ctx.query_params.is_empty());
        assert!(ctx.headers.is_empty());
        assert!(ctx.body.is_none());
        assert!(ctx.multipart_fields.is_empty());
        assert!(ctx.multipart_files.is_empty());
    }

    #[test]
    fn test_request_context_with_path_params() {
        let mut params = HashMap::new();
        params.insert("id".to_string(), json!("123"));
        params.insert("name".to_string(), json!("test"));

        let ctx = RequestContext::new("GET".to_string(), "/users".to_string())
            .with_path_params(params.clone());

        assert_eq!(ctx.path_params.len(), 2);
        assert_eq!(ctx.path_params.get("id"), Some(&json!("123")));
        assert_eq!(ctx.path_params.get("name"), Some(&json!("test")));
    }

    #[test]
    fn test_request_context_with_query_params() {
        let mut params = HashMap::new();
        params.insert("page".to_string(), json!(1));
        params.insert("limit".to_string(), json!(10));

        let ctx =
            RequestContext::new("GET".to_string(), "/items".to_string()).with_query_params(params);

        assert_eq!(ctx.query_params.len(), 2);
        assert_eq!(ctx.query_params.get("page"), Some(&json!(1)));
    }

    #[test]
    fn test_request_context_with_headers() {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), json!("application/json"));
        headers.insert("authorization".to_string(), json!("Bearer token123"));

        let ctx = RequestContext::new("POST".to_string(), "/api".to_string()).with_headers(headers);

        assert_eq!(ctx.headers.len(), 2);
        assert_eq!(ctx.headers.get("content-type"), Some(&json!("application/json")));
    }

    #[test]
    fn test_request_context_with_body() {
        let body = json!({"key": "value", "count": 42});
        let ctx =
            RequestContext::new("POST".to_string(), "/data".to_string()).with_body(body.clone());

        assert_eq!(ctx.body, Some(body));
    }

    #[test]
    fn test_request_context_with_multipart_fields() {
        let mut fields = HashMap::new();
        fields.insert("username".to_string(), json!("testuser"));
        fields.insert("email".to_string(), json!("test@example.com"));

        let ctx = RequestContext::new("POST".to_string(), "/upload".to_string())
            .with_multipart_fields(fields);

        assert_eq!(ctx.multipart_fields.len(), 2);
        assert_eq!(ctx.multipart_fields.get("username"), Some(&json!("testuser")));
    }

    #[test]
    fn test_request_context_with_multipart_files() {
        let mut files = HashMap::new();
        files.insert("document".to_string(), "/tmp/doc.pdf".to_string());
        files.insert("image".to_string(), "/tmp/photo.jpg".to_string());

        let ctx = RequestContext::new("POST".to_string(), "/upload".to_string())
            .with_multipart_files(files);

        assert_eq!(ctx.multipart_files.len(), 2);
        assert_eq!(ctx.multipart_files.get("document"), Some(&"/tmp/doc.pdf".to_string()));
    }

    #[test]
    fn test_request_context_builder_chain() {
        let mut path_params = HashMap::new();
        path_params.insert("id".to_string(), json!("456"));

        let mut query_params = HashMap::new();
        query_params.insert("verbose".to_string(), json!(true));

        let mut headers = HashMap::new();
        headers.insert("x-custom".to_string(), json!("value"));

        let mut multipart_fields = HashMap::new();
        multipart_fields.insert("field1".to_string(), json!("data"));

        let mut multipart_files = HashMap::new();
        multipart_files.insert("file1".to_string(), "/path/to/file".to_string());

        let ctx = RequestContext::new("PUT".to_string(), "/resource/456".to_string())
            .with_path_params(path_params)
            .with_query_params(query_params)
            .with_headers(headers)
            .with_body(json!({"update": true}))
            .with_multipart_fields(multipart_fields)
            .with_multipart_files(multipart_files);

        assert_eq!(ctx.method, "PUT");
        assert_eq!(ctx.path, "/resource/456");
        assert_eq!(ctx.path_params.len(), 1);
        assert_eq!(ctx.query_params.len(), 1);
        assert_eq!(ctx.headers.len(), 1);
        assert!(ctx.body.is_some());
        assert_eq!(ctx.multipart_fields.len(), 1);
        assert_eq!(ctx.multipart_files.len(), 1);
    }

    // ==================== expand_prompt_template Tests ====================

    #[test]
    fn test_expand_prompt_template_basic() {
        let context = RequestContext::new("GET".to_string(), "/users".to_string());
        let template = "Method: {{method}}, Path: {{path}}";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, "Method: GET, Path: /users");
    }

    #[test]
    fn test_expand_prompt_template_empty_template() {
        let context = RequestContext::new("GET".to_string(), "/test".to_string());
        let expanded = expand_prompt_template("", &context);
        assert_eq!(expanded, "");
    }

    #[test]
    fn test_expand_prompt_template_no_variables() {
        let context = RequestContext::new("GET".to_string(), "/test".to_string());
        let template = "This is a plain string with no variables";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, template);
    }

    #[test]
    fn test_expand_prompt_template_missing_variable() {
        let context = RequestContext::new("GET".to_string(), "/test".to_string());
        let template = "Value is {{query.nonexistent}}";
        let expanded = expand_prompt_template(template, &context);
        // Missing variables should remain in the string
        assert_eq!(expanded, "Value is {{query.nonexistent}}");
    }

    #[test]
    fn test_expand_prompt_template_multiple_occurrences() {
        let context = RequestContext::new("GET".to_string(), "/api".to_string());
        let template = "{{method}} to {{path}}, again {{method}} to {{path}}";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, "GET to /api, again GET to /api");
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
    fn test_expand_prompt_template_body_with_number() {
        let body = json!({
            "count": 42,
            "price": 19.99
        });
        let context = RequestContext::new("POST".to_string(), "/order".to_string()).with_body(body);

        let template = "Count: {{body.count}}, Price: {{body.price}}";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, "Count: 42, Price: 19.99");
    }

    #[test]
    fn test_expand_prompt_template_body_with_boolean() {
        let body = json!({
            "active": true,
            "deleted": false
        });
        let context =
            RequestContext::new("POST".to_string(), "/status".to_string()).with_body(body);

        let template = "Active: {{body.active}}, Deleted: {{body.deleted}}";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, "Active: true, Deleted: false");
    }

    #[test]
    fn test_expand_prompt_template_body_with_null() {
        let body = json!({
            "value": null
        });
        let context = RequestContext::new("POST".to_string(), "/data".to_string()).with_body(body);

        let template = "Value: {{body.value}}";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, "Value: null");
    }

    #[test]
    fn test_expand_prompt_template_body_with_nested_object() {
        let body = json!({
            "nested": {"inner": "value"}
        });
        let context = RequestContext::new("POST".to_string(), "/data".to_string()).with_body(body);

        let template = "Nested: {{body.nested}}";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, r#"Nested: {"inner":"value"}"#);
    }

    #[test]
    fn test_expand_prompt_template_body_with_array() {
        let body = json!({
            "items": [1, 2, 3]
        });
        let context = RequestContext::new("POST".to_string(), "/data".to_string()).with_body(body);

        let template = "Items: {{body.items}}";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, "Items: [1,2,3]");
    }

    #[test]
    fn test_expand_prompt_template_no_body() {
        let context = RequestContext::new("GET".to_string(), "/test".to_string());
        let template = "Body field: {{body.field}}";
        let expanded = expand_prompt_template(template, &context);
        // Body is None, so placeholder remains
        assert_eq!(expanded, "Body field: {{body.field}}");
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
    fn test_expand_prompt_template_query_params_boolean() {
        let mut query_params = HashMap::new();
        query_params.insert("verbose".to_string(), json!(true));
        query_params.insert("debug".to_string(), json!(false));

        let context = RequestContext::new("GET".to_string(), "/api".to_string())
            .with_query_params(query_params);

        let template = "Verbose: {{query.verbose}}, Debug: {{query.debug}}";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, "Verbose: true, Debug: false");
    }

    #[test]
    fn test_expand_prompt_template_query_params_null() {
        let mut query_params = HashMap::new();
        query_params.insert("filter".to_string(), json!(null));

        let context = RequestContext::new("GET".to_string(), "/api".to_string())
            .with_query_params(query_params);

        let template = "Filter: {{query.filter}}";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, "Filter: null");
    }

    #[test]
    fn test_expand_prompt_template_query_params_array() {
        let mut query_params = HashMap::new();
        query_params.insert("tags".to_string(), json!(["a", "b", "c"]));

        let context = RequestContext::new("GET".to_string(), "/api".to_string())
            .with_query_params(query_params);

        let template = "Tags: {{query.tags}}";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, r#"Tags: ["a","b","c"]"#);
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
    fn test_expand_prompt_template_multipart_fields() {
        let mut multipart_fields = HashMap::new();
        multipart_fields.insert("username".to_string(), json!("testuser"));
        multipart_fields.insert("description".to_string(), json!("A test file"));

        let context = RequestContext::new("POST".to_string(), "/upload".to_string())
            .with_multipart_fields(multipart_fields);

        let template = "User: {{multipart.username}}, Desc: {{multipart.description}}";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, "User: testuser, Desc: A test file");
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
    fn test_expand_prompt_template_empty_context() {
        let context = RequestContext::default();
        let template = "Method: {{method}}, Path: {{path}}";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, "Method: , Path: ");
    }

    // ==================== expand_templates_in_json Tests ====================

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
    fn test_expand_templates_in_json_primitives_unchanged() {
        let context = RequestContext::new("GET".to_string(), "/test".to_string());

        // Numbers should remain unchanged
        let num_value = json!(42);
        let expanded_num = expand_templates_in_json(num_value, &context);
        assert_eq!(expanded_num, json!(42));

        // Booleans should remain unchanged
        let bool_value = json!(true);
        let expanded_bool = expand_templates_in_json(bool_value, &context);
        assert_eq!(expanded_bool, json!(true));

        // Null should remain unchanged
        let null_value = json!(null);
        let expanded_null = expand_templates_in_json(null_value, &context);
        assert_eq!(expanded_null, json!(null));

        // Float should remain unchanged
        let float_value = json!(3.125);
        let expanded_float = expand_templates_in_json(float_value, &context);
        assert_eq!(expanded_float, json!(3.125));
    }

    #[test]
    fn test_expand_templates_in_json_empty_string() {
        let context = RequestContext::new("GET".to_string(), "/test".to_string());
        let value = json!("");
        let expanded = expand_templates_in_json(value, &context);
        assert_eq!(expanded, json!(""));
    }

    #[test]
    fn test_expand_templates_in_json_empty_array() {
        let context = RequestContext::new("GET".to_string(), "/test".to_string());
        let value = json!([]);
        let expanded = expand_templates_in_json(value, &context);
        assert_eq!(expanded, json!([]));
    }

    #[test]
    fn test_expand_templates_in_json_empty_object() {
        let context = RequestContext::new("GET".to_string(), "/test".to_string());
        let value = json!({});
        let expanded = expand_templates_in_json(value, &context);
        assert_eq!(expanded, json!({}));
    }

    #[test]
    fn test_expand_templates_in_json_deeply_nested() {
        let context = RequestContext::new("POST".to_string(), "/deep".to_string());
        let value = json!({
            "level1": {
                "level2": {
                    "level3": {
                        "method": "{{method}}",
                        "path": "{{path}}"
                    }
                }
            }
        });

        let expanded = expand_templates_in_json(value, &context);
        assert_eq!(expanded["level1"]["level2"]["level3"]["method"], "POST");
        assert_eq!(expanded["level1"]["level2"]["level3"]["path"], "/deep");
    }

    #[test]
    fn test_expand_templates_in_json_mixed_array() {
        let context = RequestContext::new("DELETE".to_string(), "/resource".to_string());
        let value = json!([
            "{{method}}",
            42,
            true,
            null,
            {"nested": "{{path}}"},
            ["{{method}}", 123]
        ]);

        let expanded = expand_templates_in_json(value, &context);
        assert_eq!(expanded[0], "DELETE");
        assert_eq!(expanded[1], 42);
        assert_eq!(expanded[2], true);
        assert_eq!(expanded[3], json!(null));
        assert_eq!(expanded[4]["nested"], "/resource");
        assert_eq!(expanded[5][0], "DELETE");
        assert_eq!(expanded[5][1], 123);
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

    #[test]
    fn test_expand_templates_in_json_normalize_all_request_prefixes() {
        let mut query_params = HashMap::new();
        query_params.insert("search".to_string(), json!("test"));

        let mut path_params = HashMap::new();
        path_params.insert("id".to_string(), json!("123"));

        let mut headers = HashMap::new();
        headers.insert("auth".to_string(), json!("token"));

        let body = json!({"field": "value"});

        let context = RequestContext::new("GET".to_string(), "/items/123".to_string())
            .with_query_params(query_params)
            .with_path_params(path_params)
            .with_headers(headers)
            .with_body(body);

        let value = json!({
            "method": "{{request.method}}",
            "path": "{{request.path}}",
            "query_search": "{{request.query.search}}",
            "path_id": "{{request.path.id}}",
            "header_auth": "{{request.headers.auth}}",
            "body_field": "{{request.body.field}}"
        });

        let expanded = expand_templates_in_json(value, &context);
        assert_eq!(expanded["method"], "GET");
        assert_eq!(expanded["path"], "/items/123");
        assert_eq!(expanded["query_search"], "test");
        assert_eq!(expanded["path_id"], "123");
        assert_eq!(expanded["header_auth"], "token");
        assert_eq!(expanded["body_field"], "value");
    }

    #[test]
    fn test_expand_templates_in_json_string_without_template() {
        let context = RequestContext::new("GET".to_string(), "/test".to_string());
        let value = json!("plain string without any templates");
        let expanded = expand_templates_in_json(value, &context);
        assert_eq!(expanded, json!("plain string without any templates"));
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_special_characters_in_values() {
        let mut query_params = HashMap::new();
        query_params
            .insert("special".to_string(), json!("value with \"quotes\" and \\backslashes"));

        let context = RequestContext::new("GET".to_string(), "/test?foo=bar&baz=qux".to_string())
            .with_query_params(query_params);

        let template = "Path: {{path}}, Special: {{query.special}}";
        let expanded = expand_prompt_template(template, &context);
        assert!(expanded.contains("/test?foo=bar&baz=qux"));
        assert!(expanded.contains("value with \"quotes\" and \\backslashes"));
    }

    #[test]
    fn test_unicode_in_values() {
        let body = json!({
            "message": "Hello 世界! 🌍",
            "emoji": "🚀✨"
        });
        let context =
            RequestContext::new("POST".to_string(), "/unicode".to_string()).with_body(body);

        let template = "Message: {{body.message}}, Emoji: {{body.emoji}}";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, "Message: Hello 世界! 🌍, Emoji: 🚀✨");
    }

    #[test]
    fn test_whitespace_handling() {
        let context =
            RequestContext::new("  GET  ".to_string(), "  /path with spaces  ".to_string());
        let template = "Method: '{{method}}', Path: '{{path}}'";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, "Method: '  GET  ', Path: '  /path with spaces  '");
    }

    #[test]
    fn test_empty_string_values() {
        let mut query_params = HashMap::new();
        query_params.insert("empty".to_string(), json!(""));

        let context = RequestContext::new("GET".to_string(), "/test".to_string())
            .with_query_params(query_params);

        let template = "Empty value: '{{query.empty}}'";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, "Empty value: ''");
    }

    #[test]
    fn test_request_context_debug() {
        let ctx = RequestContext::new("GET".to_string(), "/test".to_string());
        let debug_str = format!("{:?}", ctx);
        assert!(debug_str.contains("RequestContext"));
        assert!(debug_str.contains("GET"));
        assert!(debug_str.contains("/test"));
    }

    #[test]
    fn test_request_context_clone() {
        let mut query_params = HashMap::new();
        query_params.insert("key".to_string(), json!("value"));

        let ctx = RequestContext::new("POST".to_string(), "/clone-test".to_string())
            .with_query_params(query_params)
            .with_body(json!({"data": 123}));

        let cloned = ctx.clone();
        assert_eq!(cloned.method, ctx.method);
        assert_eq!(cloned.path, ctx.path);
        assert_eq!(cloned.query_params, ctx.query_params);
        assert_eq!(cloned.body, ctx.body);
    }

    #[test]
    fn test_expand_prompt_template_body_non_object() {
        // Test when body is a string (not an object)
        let context = RequestContext::new("POST".to_string(), "/data".to_string())
            .with_body(json!("just a string"));

        let template = "Body: {{body}}, method: {{method}}";
        let expanded = expand_prompt_template(template, &context);
        // Body placeholders that don't match object fields should remain
        assert_eq!(expanded, "Body: {{body}}, method: POST");
    }

    #[test]
    fn test_expand_prompt_template_body_array_top_level() {
        // Test when body is an array at the top level (not an object)
        let context = RequestContext::new("POST".to_string(), "/data".to_string())
            .with_body(json!([1, 2, 3]));

        let template = "Array body: {{body.item}}, path: {{path}}";
        let expanded = expand_prompt_template(template, &context);
        // Since body is not an object, the placeholder should remain
        assert_eq!(expanded, "Array body: {{body.item}}, path: /data");
    }

    #[test]
    fn test_expand_prompt_template_body_primitive() {
        // Test when body is a primitive value (number)
        let context =
            RequestContext::new("POST".to_string(), "/data".to_string()).with_body(json!(42));

        let template = "Number body: {{body.value}}, method: {{method}}";
        let expanded = expand_prompt_template(template, &context);
        // Since body is not an object, the placeholder should remain
        assert_eq!(expanded, "Number body: {{body.value}}, method: POST");
    }

    #[test]
    fn test_expand_json_variables_non_object() {
        // Direct test of expand_json_variables with non-object values
        let template = "Value: {{test.field}}, other: {{other}}";

        // Test with string
        let result = expand_json_variables(template, "test", &json!("string value"));
        assert_eq!(result, "Value: {{test.field}}, other: {{other}}");

        // Test with number
        let result = expand_json_variables(template, "test", &json!(123));
        assert_eq!(result, "Value: {{test.field}}, other: {{other}}");

        // Test with boolean
        let result = expand_json_variables(template, "test", &json!(true));
        assert_eq!(result, "Value: {{test.field}}, other: {{other}}");

        // Test with null
        let result = expand_json_variables(template, "test", &json!(null));
        assert_eq!(result, "Value: {{test.field}}, other: {{other}}");

        // Test with array
        let result = expand_json_variables(template, "test", &json!([1, 2, 3]));
        assert_eq!(result, "Value: {{test.field}}, other: {{other}}");
    }

    #[test]
    fn test_expand_map_variables_empty_map() {
        let template = "Query: {{query.param}}, path: {{path.id}}";
        let empty_map = HashMap::new();

        let result = expand_map_variables(template, "query", &empty_map);
        assert_eq!(result, "Query: {{query.param}}, path: {{path.id}}");
    }

    #[test]
    fn test_expand_map_variables_complex_types() {
        let mut map = HashMap::new();
        map.insert("nested".to_string(), json!({"inner": "value"}));
        map.insert("array".to_string(), json!([1, 2, 3]));

        let template = "Nested: {{test.nested}}, Array: {{test.array}}";
        let result = expand_map_variables(template, "test", &map);
        assert_eq!(result, r#"Nested: {"inner":"value"}, Array: [1,2,3]"#);
    }

    #[test]
    fn test_expand_templates_in_json_with_request_body_prefix() {
        let body = json!({"field": "value"});
        let context = RequestContext::new("POST".to_string(), "/api".to_string()).with_body(body);

        let value = json!({"msg": "{{request.body.field}}"});
        let expanded = expand_templates_in_json(value, &context);
        assert_eq!(expanded["msg"], "value");
    }

    #[test]
    fn test_key_with_special_chars_in_placeholder() {
        // Test that keys with special characters work correctly
        let mut query_params = HashMap::new();
        query_params.insert("user-id".to_string(), json!("12345"));
        query_params.insert("session_token".to_string(), json!("abc123"));

        let context = RequestContext::new("GET".to_string(), "/api".to_string())
            .with_query_params(query_params);

        let template = "User: {{query.user-id}}, Token: {{query.session_token}}";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, "User: 12345, Token: abc123");
    }

    #[test]
    fn test_large_number_values() {
        let mut query_params = HashMap::new();
        query_params.insert("big".to_string(), json!(9999999999i64));
        query_params.insert("float".to_string(), json!(1.23456789));

        let context = RequestContext::new("GET".to_string(), "/api".to_string())
            .with_query_params(query_params);

        let template = "Big: {{query.big}}, Float: {{query.float}}";
        let expanded = expand_prompt_template(template, &context);
        assert!(expanded.contains("9999999999"));
        assert!(expanded.contains("1.23456789"));
    }

    #[test]
    fn test_multiple_template_variables_same_name_different_prefix() {
        let mut path_params = HashMap::new();
        path_params.insert("id".to_string(), json!("path-123"));

        let mut query_params = HashMap::new();
        query_params.insert("id".to_string(), json!("query-456"));

        let body = json!({"id": "body-789"});

        let context = RequestContext::new("POST".to_string(), "/resource".to_string())
            .with_path_params(path_params)
            .with_query_params(query_params)
            .with_body(body);

        let template = "Path ID: {{path.id}}, Query ID: {{query.id}}, Body ID: {{body.id}}";
        let expanded = expand_prompt_template(template, &context);
        assert_eq!(expanded, "Path ID: path-123, Query ID: query-456, Body ID: body-789");
    }

    #[test]
    fn test_partial_placeholder_should_not_expand() {
        let context = RequestContext::new("GET".to_string(), "/test".to_string());

        // Test incomplete placeholders
        let template = "{{method} {{method}} {method}} {{metho {{method";
        let expanded = expand_prompt_template(template, &context);
        // Only {{method}} should be expanded
        assert!(expanded.contains("GET"));
        assert!(expanded.contains("{{method}"));
    }
}
