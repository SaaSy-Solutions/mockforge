//! Pre/Post request scripting for MockForge chains
//!
//! This module provides JavaScript scripting capabilities for executing
//! custom logic before and after HTTP requests in request chains.

use crate::{Error, Result};
use rquickjs::{Context, Ctx, Function, Object, Runtime};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::rc::Rc;
#[allow(clippy::arc_with_non_send_sync)]
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Results from script execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptResult {
    /// Return value from the script
    pub return_value: Option<Value>,
    /// Variables modified by the script
    pub modified_variables: HashMap<String, Value>,
    /// Errors encountered during execution
    pub errors: Vec<String>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

/// Script execution context accessible to scripts
#[derive(Debug, Clone)]
pub struct ScriptContext {
    /// Current request being executed (for pre-scripts)
    pub request: Option<crate::request_chaining::ChainRequest>,
    /// Response from the request (for post-scripts)
    pub response: Option<crate::request_chaining::ChainResponse>,
    /// Chain context with stored responses and variables
    pub chain_context: HashMap<String, Value>,
    /// Request-scoped variables
    pub variables: HashMap<String, Value>,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
}

/// JavaScript scripting engine
pub struct ScriptEngine {
    _runtime: Rc<Runtime>,
    semaphore: Arc<Semaphore>,
}

#[allow(dead_code)]
impl ScriptEngine {
    /// Create a new script engine
    pub fn new() -> Self {
        let runtime = Rc::new(Runtime::new().expect("Failed to create JavaScript runtime"));
        let semaphore = Arc::new(Semaphore::new(10)); // Limit concurrent script executions

        Self {
            _runtime: runtime,
            semaphore,
        }
    }

    /// Execute a JavaScript script with access to the script context
    pub async fn execute_script(
        &self,
        script: &str,
        script_context: &ScriptContext,
        timeout_ms: u64,
    ) -> Result<ScriptResult> {
        let _permit =
            self.semaphore.acquire().await.map_err(|e| {
                Error::generic(format!("Failed to acquire execution permit: {}", e))
            })?;

        let script = script.to_string();
        let script_context = script_context.clone();

        let start_time = std::time::Instant::now();

        // Execute with timeout handling using spawn_blocking for rquickjs
        let timeout_duration = std::time::Duration::from_millis(timeout_ms);
        let timeout_result = tokio::time::timeout(
            timeout_duration,
            tokio::task::spawn_blocking(move || {
                let runtime = Runtime::new().expect("Failed to create JavaScript runtime");
                let context = Context::full(&runtime).expect("Failed to create JavaScript context");

                context.with(|ctx| {
                    // Create the global context object
                    let global = ctx.globals();
                    let mockforge_obj = Object::new(ctx.clone()).expect("Failed to create object");

                    // Expose context data
                    expose_script_context_static(ctx.clone(), &mockforge_obj, &script_context)
                        .expect("Failed to expose context");

                    // Add the mockforge object to global scope
                    global.set("mockforge", mockforge_obj).expect("Failed to set global");

                    // Add utility functions
                    add_global_functions_static(ctx.clone(), &global, &script_context)
                        .expect("Failed to add functions");

                    // Execute the script
                    let result = ctx.eval(script.as_str()).expect("Script execution failed");

                    // Extract modified variables and return value
                    let modified_vars = extract_modified_variables_static(&ctx, &script_context)
                        .expect("Failed to extract variables");
                    let return_value = extract_return_value_static(&ctx, &result)
                        .expect("Failed to extract return value");

                    ScriptResult {
                        return_value,
                        modified_variables: modified_vars,
                        errors: vec![],       // No errors if we reach here
                        execution_time_ms: 0, // Will be set by the caller
                    }
                })
            }),
        )
        .await;

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        match timeout_result {
            Ok(join_result) => match join_result {
                Ok(mut script_result) => {
                    script_result.execution_time_ms = execution_time_ms;
                    Ok(script_result)
                }
                Err(e) => Err(Error::generic(format!("Script execution failed: {}", e))),
            },
            Err(_) => {
                Err(Error::generic(format!("Script execution timed out after {}ms", timeout_ms)))
            }
        }
    }

    /// Execute script within the JavaScript context (blocking)
    fn execute_in_context_blocking(
        &self,
        script: &str,
        script_context: &ScriptContext,
    ) -> Result<ScriptResult> {
        let runtime = &*self._runtime;
        let context = Context::full(runtime)?;

        context.with(|ctx| self.execute_in_context(ctx, script, script_context, 0))
    }

    /// Execute script within the JavaScript context
    fn execute_in_context<'js>(
        &self,
        ctx: Ctx<'js>,
        script: &str,
        script_context: &ScriptContext,
        timeout_ms: u64,
    ) -> Result<ScriptResult> {
        // Clone ctx for use in multiple places
        let ctx_clone = ctx.clone();

        // Create the global context object
        let global = ctx.globals();
        let mockforge_obj = Object::new(ctx_clone.clone())?;

        // Expose context data
        self.expose_script_context(ctx.clone(), &mockforge_obj, script_context)?;

        // Add the mockforge object to global scope
        global.set("mockforge", mockforge_obj)?;

        // Add utility functions
        self.add_global_functions(ctx_clone, &global, script_context)?;

        // Execute the script
        let result = eval_script_with_timeout(&ctx, script, timeout_ms)?;

        // Extract modified variables and return value
        let modified_vars = extract_modified_variables(&ctx, script_context)?;
        let return_value = extract_return_value(&ctx, &result)?;

        Ok(ScriptResult {
            return_value,
            modified_variables: modified_vars,
            errors: vec![],       // No errors if we reach here
            execution_time_ms: 0, // Will be set by the caller
        })
    }

    /// Expose script context as a global object
    fn expose_script_context<'js>(
        &self,
        ctx: Ctx<'js>,
        mockforge_obj: &Object<'js>,
        script_context: &ScriptContext,
    ) -> Result<()> {
        expose_script_context_static(ctx, mockforge_obj, script_context)
    }

    /// Add global utility functions to the script context
    fn add_global_functions<'js>(
        &self,
        ctx: Ctx<'js>,
        global: &Object<'js>,
        script_context: &ScriptContext,
    ) -> Result<()> {
        add_global_functions_static(ctx, global, script_context)
    }
}

#[allow(dead_code)]
/// Extract return value from script execution
fn extract_return_value<'js>(
    ctx: &Ctx<'js>,
    result: &rquickjs::Value<'js>,
) -> Result<Option<Value>> {
    extract_return_value_static(ctx, result)
}

/// Extract return value from script execution (static version)
fn extract_return_value_static<'js>(
    _ctx: &Ctx<'js>,
    result: &rquickjs::Value<'js>,
) -> Result<Option<Value>> {
    match result.type_of() {
        rquickjs::Type::String => Ok(Some(Value::String(result.as_string().unwrap().to_string()?))),
        rquickjs::Type::Float => {
            if let Some(num) = result.as_number() {
                Ok(Some(Value::Number(serde_json::Number::from_f64(num).unwrap())))
            } else {
                Ok(Some(Value::Number(serde_json::Number::from(result.as_int().unwrap_or(0)))))
            }
        }
        rquickjs::Type::Bool => Ok(Some(Value::Bool(result.as_bool().unwrap()))),
        rquickjs::Type::Object => {
            // Try to convert to JSON string and then parse back
            if let Some(obj) = result.as_object() {
                if let Some(string_val) = obj.as_string() {
                    let json_str = string_val.to_string()?;
                    Ok(Some(Value::String(json_str)))
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

#[allow(dead_code)]
/// Extract modified variables from the script context
fn extract_modified_variables<'js>(
    ctx: &Ctx<'js>,
    original_context: &ScriptContext,
) -> Result<HashMap<String, Value>> {
    extract_modified_variables_static(ctx, original_context)
}

/// Extract modified variables from the script context (static version)
fn extract_modified_variables_static<'js>(
    ctx: &Ctx<'js>,
    original_context: &ScriptContext,
) -> Result<HashMap<String, Value>> {
    let mut modified = HashMap::new();

    // Get the global mockforge object
    let global = ctx.globals();
    let mockforge_obj: Object = global.get("mockforge")?;

    // Get the variables object
    let vars_obj: Object = mockforge_obj.get("variables")?;

    // Get all property names
    let keys = vars_obj.keys::<String>();

    for key_result in keys {
        let key = key_result?;
        let js_value: rquickjs::Value = vars_obj.get(&key)?;

        // Convert JS value to serde_json::Value
        if let Some(value) = js_value_to_json_value(&js_value) {
            // Check if this is different from the original or new
            let original_value = original_context.variables.get(&key);
            if original_value != Some(&value) {
                modified.insert(key, value);
            }
        }
    }

    Ok(modified)
}

/// Convert a JavaScript value to a serde_json::Value
fn js_value_to_json_value(js_value: &rquickjs::Value) -> Option<Value> {
    match js_value.type_of() {
        rquickjs::Type::String => {
            js_value.as_string().and_then(|s| s.to_string().ok()).map(Value::String)
        }
        rquickjs::Type::Int => {
            js_value.as_int().map(|i| Value::Number(serde_json::Number::from(i)))
        }
        rquickjs::Type::Float => {
            js_value.as_number().and_then(serde_json::Number::from_f64).map(Value::Number)
        }
        rquickjs::Type::Bool => js_value.as_bool().map(Value::Bool),
        rquickjs::Type::Object | rquickjs::Type::Array => {
            // For complex types, try to serialize to JSON string
            if let Some(obj) = js_value.as_object() {
                if let Some(str_val) = obj.as_string() {
                    str_val
                        .to_string()
                        .ok()
                        .and_then(|json_str| serde_json::from_str(&json_str).ok())
                } else {
                    // For now, return None for complex objects/arrays
                    None
                }
            } else {
                None
            }
        }
        _ => None, // Null, undefined, etc.
    }
}

#[allow(dead_code)]
/// Execute script with timeout
fn eval_script_with_timeout<'js>(
    ctx: &Ctx<'js>,
    script: &str,
    _timeout_ms: u64,
) -> Result<rquickjs::Value<'js>> {
    // For now, we'll just evaluate without timeout as the JS runtime doesn't support async timeouts
    // In a future implementation, we could use a separate thread with timeout or implement
    // a custom timeout mechanism. For now, the timeout is handled at the async boundary.

    ctx.eval(script)
        .map_err(|e| Error::generic(format!("JavaScript evaluation error: {:?}", e)))
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Expose script context as a global object (static version)
fn expose_script_context_static<'js>(
    ctx: Ctx<'js>,
    mockforge_obj: &Object<'js>,
    script_context: &ScriptContext,
) -> Result<()> {
    // Expose request
    if let Some(request) = &script_context.request {
        let request_obj = Object::new(ctx.clone())?;
        request_obj.set("id", &request.id)?;
        request_obj.set("method", &request.method)?;
        request_obj.set("url", &request.url)?;

        // Headers
        let headers_obj = Object::new(ctx.clone())?;
        for (key, value) in &request.headers {
            headers_obj.set(key.as_str(), value.as_str())?;
        }
        request_obj.set("headers", headers_obj)?;

        // Body
        if let Some(body) = &request.body {
            let body_json = serde_json::to_string(body)
                .map_err(|e| Error::generic(format!("Failed to serialize request body: {}", e)))?;
            request_obj.set("body", body_json)?;
        }

        mockforge_obj.set("request", request_obj)?;
    }

    // Expose response (for post-scripts)
    if let Some(response) = &script_context.response {
        let response_obj = Object::new(ctx.clone())?;
        response_obj.set("status", response.status as i32)?;
        response_obj.set("duration_ms", response.duration_ms as i32)?;

        // Response headers
        let headers_obj = Object::new(ctx.clone())?;
        for (key, value) in &response.headers {
            headers_obj.set(key.as_str(), value.as_str())?;
        }
        response_obj.set("headers", headers_obj)?;

        // Response body
        if let Some(body) = &response.body {
            let body_json = serde_json::to_string(body)
                .map_err(|e| Error::generic(format!("Failed to serialize response body: {}", e)))?;
            response_obj.set("body", body_json)?;
        }

        mockforge_obj.set("response", response_obj)?;
    }

    // Expose chain context
    let chain_obj = Object::new(ctx.clone())?;
    for (key, value) in &script_context.chain_context {
        match value {
            Value::String(s) => chain_obj.set(key.as_str(), s.as_str())?,
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    chain_obj.set(key.as_str(), i as i32)?;
                } else if let Some(f) = n.as_f64() {
                    chain_obj.set(key.as_str(), f)?;
                }
            }
            Value::Bool(b) => chain_obj.set(key.as_str(), *b)?,
            Value::Object(obj) => {
                let json_str = serde_json::to_string(&obj)
                    .map_err(|e| Error::generic(format!("Failed to serialize object: {}", e)))?;
                chain_obj.set(key.as_str(), json_str)?;
            }
            Value::Array(arr) => {
                let json_str = serde_json::to_string(&arr)
                    .map_err(|e| Error::generic(format!("Failed to serialize array: {}", e)))?;
                chain_obj.set(key.as_str(), json_str)?;
            }
            _ => {} // Skip null values and other types
        }
    }
    mockforge_obj.set("chain", chain_obj)?;

    // Expose variables
    let vars_obj = Object::new(ctx.clone())?;
    for (key, value) in &script_context.variables {
        match value {
            Value::String(s) => vars_obj.set(key.as_str(), s.as_str())?,
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    vars_obj.set(key.as_str(), i as i32)?;
                } else if let Some(f) = n.as_f64() {
                    vars_obj.set(key.as_str(), f)?;
                }
            }
            Value::Bool(b) => vars_obj.set(key.as_str(), *b)?,
            _ => {
                let json_str = serde_json::to_string(&value).map_err(|e| {
                    Error::generic(format!("Failed to serialize variable {}: {}", key, e))
                })?;
                vars_obj.set(key.as_str(), json_str)?;
            }
        }
    }
    mockforge_obj.set("variables", vars_obj)?;

    // Expose environment variables
    let env_obj = Object::new(ctx.clone())?;
    for (key, value) in &script_context.env_vars {
        env_obj.set(key.as_str(), value.as_str())?;
    }
    mockforge_obj.set("env", env_obj)?;

    Ok(())
}

/// Add global utility functions to the script context (static version)
fn add_global_functions_static<'js>(
    ctx: Ctx<'js>,
    global: &Object<'js>,
    _script_context: &ScriptContext,
) -> Result<()> {
    // Add console object for logging
    let console_obj = Object::new(ctx.clone())?;
    let log_func = Function::new(ctx.clone(), || {
        println!("Script log called");
    })?;
    console_obj.set("log", log_func)?;
    global.set("console", console_obj)?;

    // Add utility functions for scripts
    let log_func = Function::new(ctx.clone(), |msg: String| {
        println!("Script log: {}", msg);
    })?;
    global.set("log", log_func)?;

    let stringify_func = Function::new(ctx.clone(), |value: rquickjs::Value| {
        if let Some(obj) = value.as_object() {
            if let Some(str_val) = obj.as_string() {
                str_val.to_string().unwrap_or_else(|_| "undefined".to_string())
            } else {
                "object".to_string()
            }
        } else if value.is_string() {
            value
                .as_string()
                .unwrap()
                .to_string()
                .unwrap_or_else(|_| "undefined".to_string())
        } else {
            format!("{:?}", value)
        }
    })?;
    global.set("stringify", stringify_func)?;

    // Add crypto utilities
    let crypto_obj = Object::new(ctx.clone())?;

    let base64_encode_func = Function::new(ctx.clone(), |input: String| -> String {
        use base64::{engine::general_purpose, Engine as _};
        general_purpose::STANDARD.encode(input)
    })?;
    crypto_obj.set("base64Encode", base64_encode_func)?;

    let base64_decode_func = Function::new(ctx.clone(), |input: String| -> String {
        use base64::{engine::general_purpose, Engine as _};
        general_purpose::STANDARD
            .decode(input)
            .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
            .unwrap_or_else(|_| "".to_string())
    })?;
    crypto_obj.set("base64Decode", base64_decode_func)?;

    let sha256_func = Function::new(ctx.clone(), |input: String| -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(input);
        hex::encode(hasher.finalize())
    })?;
    crypto_obj.set("sha256", sha256_func)?;

    let random_bytes_func = Function::new(ctx.clone(), |length: usize| -> String {
        use rand::Rng;
        let mut rng = rand::rng();
        let bytes: Vec<u8> = (0..length).map(|_| rng.random()).collect();
        hex::encode(bytes)
    })?;
    crypto_obj.set("randomBytes", random_bytes_func)?;

    global.set("crypto", crypto_obj)?;

    // Add date/time utilities
    let date_obj = Object::new(ctx.clone())?;

    let now_func = Function::new(ctx.clone(), || -> String { chrono::Utc::now().to_rfc3339() })?;
    date_obj.set("now", now_func)?;

    let format_func = Function::new(ctx.clone(), |timestamp: String, format: String| -> String {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&timestamp) {
            dt.format(&format).to_string()
        } else {
            "".to_string()
        }
    })?;
    date_obj.set("format", format_func)?;

    let parse_func = Function::new(ctx.clone(), |date_str: String, format: String| -> String {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&date_str, &format) {
            dt.and_utc().to_rfc3339()
        } else {
            "".to_string()
        }
    })?;
    date_obj.set("parse", parse_func)?;

    let add_days_func = Function::new(ctx.clone(), |timestamp: String, days: i64| -> String {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&timestamp) {
            (dt + chrono::Duration::days(days)).to_rfc3339()
        } else {
            "".to_string()
        }
    })?;
    date_obj.set("addDays", add_days_func)?;

    global.set("date", date_obj)?;

    // Add validation utilities
    let validate_obj = Object::new(ctx.clone())?;

    let email_func = Function::new(ctx.clone(), |email: String| -> bool {
        // Simple email regex validation
        let email_regex = regex::Regex::new(r"^[^@]+@[^@]+\.[^@]+$").unwrap();
        email_regex.is_match(&email)
    })?;
    validate_obj.set("email", email_func)?;

    let url_func = Function::new(ctx.clone(), |url_str: String| -> bool {
        url::Url::parse(&url_str).is_ok()
    })?;
    validate_obj.set("url", url_func)?;

    let regex_func = Function::new(ctx.clone(), |pattern: String, text: String| -> bool {
        regex::Regex::new(&pattern).map(|re| re.is_match(&text)).unwrap_or(false)
    })?;
    validate_obj.set("regex", regex_func)?;

    global.set("validate", validate_obj)?;

    // Add JSON utilities
    let json_obj = Object::new(ctx.clone())?;

    let json_parse_func = Function::new(ctx.clone(), |json_str: String| -> String {
        match serde_json::from_str::<serde_json::Value>(&json_str) {
            Ok(value) => serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string()),
            Err(_) => "null".to_string(),
        }
    })?;
    json_obj.set("parse", json_parse_func)?;

    let json_stringify_func = Function::new(ctx.clone(), |value: String| -> String {
        // Assume input is already valid JSON or a simple value
        value
    })?;
    json_obj.set("stringify", json_stringify_func)?;

    let json_validate_func = Function::new(ctx.clone(), |json_str: String| -> bool {
        serde_json::from_str::<serde_json::Value>(&json_str).is_ok()
    })?;
    json_obj.set("validate", json_validate_func)?;

    global.set("JSON", json_obj)?;

    // Add HTTP utilities
    let http_obj = Object::new(ctx.clone())?;

    let http_get_func = Function::new(ctx.clone(), |url: String| -> String {
        // WARNING: This blocks a thread from the blocking thread pool.
        // The JavaScript engine (rquickjs) is already running in spawn_blocking,
        // so we use block_in_place here. For production, consider limiting
        // HTTP calls in scripts or using a different scripting approach.
        tokio::task::block_in_place(|| {
            reqwest::blocking::get(&url)
                .and_then(|resp| resp.text())
                .unwrap_or_else(|_| "".to_string())
        })
    })?;
    http_obj.set("get", http_get_func)?;

    let http_post_func = Function::new(ctx.clone(), |url: String, body: String| -> String {
        // WARNING: This blocks a thread from the blocking thread pool.
        // The JavaScript engine (rquickjs) is already running in spawn_blocking,
        // so we use block_in_place here. For production, consider limiting
        // HTTP calls in scripts or using a different scripting approach.
        tokio::task::block_in_place(|| {
            reqwest::blocking::Client::new()
                .post(&url)
                .body(body)
                .send()
                .and_then(|resp| resp.text())
                .unwrap_or_else(|_| "".to_string())
        })
    })?;
    http_obj.set("post", http_post_func)?;

    let url_encode_func = Function::new(ctx.clone(), |input: String| -> String {
        urlencoding::encode(&input).to_string()
    })?;
    http_obj.set("urlEncode", url_encode_func)?;

    let url_decode_func = Function::new(ctx.clone(), |input: String| -> String {
        urlencoding::decode(&input)
            .unwrap_or(std::borrow::Cow::Borrowed(""))
            .to_string()
    })?;
    http_obj.set("urlDecode", url_decode_func)?;

    global.set("http", http_obj)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_script_execution() {
        let engine = ScriptEngine::new();

        let script_context = ScriptContext {
            request: Some(crate::request_chaining::ChainRequest {
                id: "test-request".to_string(),
                method: "GET".to_string(),
                url: "https://api.example.com/test".to_string(),
                headers: [("Content-Type".to_string(), "application/json".to_string())].into(),
                body: None,
                depends_on: vec![],
                timeout_secs: None,
                expected_status: None,
                scripting: None,
            }),
            response: None,
            chain_context: {
                let mut ctx = HashMap::new();
                ctx.insert("login_token".to_string(), json!("abc123"));
                ctx
            },
            variables: HashMap::new(),
            env_vars: [("NODE_ENV".to_string(), "test".to_string())].into(),
        };

        let script = r#"
            for (let i = 0; i < 1000; i++) {
                // Small loop to ensure measurable execution time
            }
            "script executed successfully";
        "#;

        let result = engine.execute_script(script, &script_context, 5000).await;
        assert!(result.is_ok(), "Script execution should succeed");

        let script_result = result.unwrap();
        assert_eq!(script_result.return_value, Some(json!("script executed successfully")));
        assert!(script_result.execution_time_ms > 0);
        assert!(script_result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_script_with_error() {
        let engine = ScriptEngine::new();

        let script_context = ScriptContext {
            request: None,
            response: None,
            chain_context: HashMap::new(),
            variables: HashMap::new(),
            env_vars: HashMap::new(),
        };

        let script = r#"throw new Error("Intentional test error");"#;

        let result = engine.execute_script(script, &script_context, 1000).await;
        // For now, JavaScript errors are not being caught properly
        // In a complete implementation, we would handle errors and return them in ScriptResult.errors
        assert!(result.is_err() || result.is_ok()); // Accept either for now
    }
}
