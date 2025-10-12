//! AI-assisted response generation for dynamic mock endpoints
//!
//! This module provides configuration and utilities for generating
//! dynamic mock responses using LLMs based on request context.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// AI response generation mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AiResponseMode {
    /// Static response (no AI)
    Static,
    /// Generate response using LLM
    Intelligent,
    /// Use static template enhanced with LLM
    Hybrid,
}

impl Default for AiResponseMode {
    fn default() -> Self {
        Self::Static
    }
}

/// Configuration for AI-assisted response generation per endpoint
/// This is parsed from the `x-mockforge-ai` OpenAPI extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponseConfig {
    /// Whether AI response generation is enabled
    #[serde(default)]
    pub enabled: bool,

    /// Response generation mode
    #[serde(default)]
    pub mode: AiResponseMode,

    /// Prompt template for LLM generation
    /// Supports template variables: {{body.field}}, {{path.param}}, {{query.param}}, {{headers.name}}
    pub prompt: Option<String>,

    /// Additional context for generation
    pub context: Option<String>,

    /// Temperature for LLM (0.0 to 2.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Max tokens for LLM response
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,

    /// Schema that the response should conform to (JSON Schema)
    pub schema: Option<Value>,

    /// Enable caching for identical requests
    #[serde(default = "default_true")]
    pub cache_enabled: bool,
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> usize {
    1024
}

fn default_true() -> bool {
    true
}

impl Default for AiResponseConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: AiResponseMode::Static,
            prompt: None,
            context: None,
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            schema: None,
            cache_enabled: true,
        }
    }
}

impl AiResponseConfig {
    /// Create a new AI response configuration
    pub fn new(enabled: bool, mode: AiResponseMode, prompt: String) -> Self {
        Self {
            enabled,
            mode,
            prompt: Some(prompt),
            ..Default::default()
        }
    }

    /// Check if AI generation is enabled and configured
    pub fn is_active(&self) -> bool {
        self.enabled && self.mode != AiResponseMode::Static && self.prompt.is_some()
    }
}

/// Request context for prompt template expansion
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
}

/// Expand template variables in a prompt string using request context
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

    result
}

/// Expand template variables from a JSON value
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
fn expand_map_variables(template: &str, prefix: &str, map: &HashMap<String, Value>) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_ai_response_config_default() {
        let config = AiResponseConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.mode, AiResponseMode::Static);
        assert!(!config.is_active());
    }

    #[test]
    fn test_ai_response_config_is_active() {
        let config =
            AiResponseConfig::new(true, AiResponseMode::Intelligent, "Test prompt".to_string());
        assert!(config.is_active());

        let config_disabled = AiResponseConfig {
            enabled: false,
            mode: AiResponseMode::Intelligent,
            prompt: Some("Test".to_string()),
            ..Default::default()
        };
        assert!(!config_disabled.is_active());
    }

    #[test]
    fn test_request_context_builder() {
        let mut path_params = HashMap::new();
        path_params.insert("id".to_string(), json!("123"));

        let context = RequestContext::new("POST".to_string(), "/users/123".to_string())
            .with_path_params(path_params)
            .with_body(json!({"name": "John"}));

        assert_eq!(context.method, "POST");
        assert_eq!(context.path, "/users/123");
        assert_eq!(context.path_params.get("id"), Some(&json!("123")));
        assert_eq!(context.body, Some(json!({"name": "John"})));
    }

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
}
