//! Response generator plugin interface
//!
//! This module defines the ResponsePlugin trait and related types for implementing
//! custom response generation logic in MockForge. Response plugins can generate
//! complex, dynamic responses based on request context, external data sources,
//! or custom business logic.

use crate::{PluginCapabilities, PluginContext, PluginError, PluginResult, Result};
use axum::http::{HeaderMap, Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Response generator plugin trait
///
/// Implement this trait to create custom response generation logic.
/// Response plugins are called during the mock response generation phase
/// to create dynamic responses based on request context and custom logic.
#[async_trait::async_trait]
pub trait ResponsePlugin: Send + Sync {
    /// Get plugin capabilities (permissions and limits)
    fn capabilities(&self) -> PluginCapabilities;

    /// Initialize the plugin with configuration
    async fn initialize(&self, config: &ResponsePluginConfig) -> Result<()>;

    /// Check if this plugin can handle the given request
    ///
    /// This method is called to determine if the plugin should be used
    /// to generate a response for the current request.
    ///
    /// # Arguments
    /// * `context` - Plugin execution context
    /// * `request` - Request information
    /// * `config` - Plugin configuration
    ///
    /// # Returns
    /// True if this plugin can handle the request
    async fn can_handle(
        &self,
        context: &PluginContext,
        request: &ResponseRequest,
        config: &ResponsePluginConfig,
    ) -> Result<PluginResult<bool>>;

    /// Generate a response for the given request
    ///
    /// This method is called when the plugin has indicated it can handle
    /// the request. It should generate an appropriate response.
    ///
    /// # Arguments
    /// * `context` - Plugin execution context
    /// * `request` - Request information
    /// * `config` - Plugin configuration
    ///
    /// # Returns
    /// Generated response
    async fn generate_response(
        &self,
        context: &PluginContext,
        request: &ResponseRequest,
        config: &ResponsePluginConfig,
    ) -> Result<PluginResult<ResponseData>>;

    /// Get plugin priority (lower numbers = higher priority)
    fn priority(&self) -> i32;

    /// Validate plugin configuration
    fn validate_config(&self, config: &ResponsePluginConfig) -> Result<()>;

    /// Get supported content types
    fn supported_content_types(&self) -> Vec<String>;

    /// Cleanup plugin resources
    async fn cleanup(&self) -> Result<()>;
}

/// Response plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsePluginConfig {
    /// Plugin-specific configuration
    pub config: HashMap<String, Value>,
    /// Enable/disable the plugin
    pub enabled: bool,
    /// Plugin priority (lower numbers = higher priority)
    pub priority: i32,
    /// Content types this plugin handles
    pub content_types: Vec<String>,
    /// URL patterns this plugin matches
    pub url_patterns: Vec<String>,
    /// HTTP methods this plugin handles
    pub methods: Vec<String>,
    /// Custom settings
    pub settings: HashMap<String, Value>,
}

impl Default for ResponsePluginConfig {
    fn default() -> Self {
        Self {
            config: HashMap::new(),
            enabled: true,
            priority: 100,
            content_types: vec!["application/json".to_string()],
            url_patterns: vec!["*".to_string()],
            methods: vec!["GET".to_string(), "POST".to_string()],
            settings: HashMap::new(),
        }
    }
}

/// Response request information
#[derive(Debug, Clone)]
pub struct ResponseRequest {
    /// HTTP method
    pub method: Method,
    /// Request URI
    pub uri: String,
    /// Request path
    pub path: String,
    /// Query parameters
    pub query_params: HashMap<String, String>,
    /// Request headers
    pub headers: HeaderMap,
    /// Request body (if available)
    pub body: Option<Vec<u8>>,
    /// Path parameters (from route matching)
    pub path_params: HashMap<String, String>,
    /// Client IP address
    pub client_ip: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// Request timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Authentication context (if available)
    pub auth_context: Option<HashMap<String, Value>>,
    /// Custom request context
    pub custom: HashMap<String, Value>,
}

impl ResponseRequest {
    /// Create from axum request components
    pub fn from_axum(
        method: Method,
        uri: axum::http::Uri,
        headers: HeaderMap,
        body: Option<Vec<u8>>,
        path_params: HashMap<String, String>,
    ) -> Self {
        let query_params = uri
            .query()
            .map(|q| url::form_urlencoded::parse(q.as_bytes()).into_owned().collect())
            .unwrap_or_default();

        let client_ip = headers
            .get("x-forwarded-for")
            .or_else(|| headers.get("x-real-ip"))
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let user_agent =
            headers.get("user-agent").and_then(|h| h.to_str().ok()).map(|s| s.to_string());

        Self {
            method,
            uri: uri.to_string(),
            path: uri.path().to_string(),
            query_params,
            headers,
            body,
            path_params,
            client_ip,
            user_agent,
            timestamp: chrono::Utc::now(),
            auth_context: None,
            custom: HashMap::new(),
        }
    }

    /// Get header value
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|h| h.to_str().ok())
    }

    /// Get query parameter value
    pub fn query_param(&self, name: &str) -> Option<&str> {
        self.query_params.get(name).map(|s| s.as_str())
    }

    /// Get path parameter value
    pub fn path_param(&self, name: &str) -> Option<&str> {
        self.path_params.get(name).map(|s| s.as_str())
    }

    /// Get authentication context value
    pub fn auth_value(&self, key: &str) -> Option<&Value> {
        self.auth_context.as_ref()?.get(key)
    }

    /// Get custom context value
    pub fn custom_value(&self, key: &str) -> Option<&Value> {
        self.custom.get(key)
    }

    /// Check if request matches URL pattern
    pub fn matches_url_pattern(&self, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        // Simple glob matching (can be enhanced with proper glob library)
        if pattern.contains('*') {
            let regex_pattern = pattern.replace('.', r"\.").replace('*', ".*");
            regex::Regex::new(&format!("^{}$", regex_pattern))
                .map(|re| re.is_match(&self.path))
                .unwrap_or(false)
        } else {
            self.path == pattern
        }
    }

    /// Check if request method is supported
    pub fn matches_method(&self, methods: &[String]) -> bool {
        methods.iter().any(|m| m == "*" || m == &self.method.to_string())
    }
}

/// Response data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseData {
    /// HTTP status code
    pub status_code: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: Vec<u8>,
    /// Content type
    pub content_type: String,
    /// Response metadata
    pub metadata: HashMap<String, Value>,
    /// Cache control directives
    pub cache_control: Option<String>,
    /// Custom response data
    pub custom: HashMap<String, Value>,
}

impl ResponseData {
    /// Create a new response
    pub fn new(status_code: u16, content_type: String, body: Vec<u8>) -> Self {
        Self {
            status_code,
            headers: HashMap::new(),
            body,
            content_type,
            metadata: HashMap::new(),
            cache_control: None,
            custom: HashMap::new(),
        }
    }

    /// Create JSON response
    pub fn json<T: Serialize>(status_code: u16, data: &T) -> Result<Self> {
        let body = serde_json::to_vec(data)
            .map_err(|e| PluginError::execution(format!("JSON serialization error: {}", e)))?;

        Ok(Self::new(status_code, "application/json".to_string(), body))
    }

    /// Create text response
    pub fn text<S: Into<String>>(status_code: u16, text: S) -> Self {
        Self::new(status_code, "text/plain".to_string(), text.into().into_bytes())
    }

    /// Create HTML response
    pub fn html<S: Into<String>>(status_code: u16, html: S) -> Self {
        Self::new(status_code, "text/html".to_string(), html.into().into_bytes())
    }

    /// Create XML response
    pub fn xml<S: Into<String>>(status_code: u16, xml: S) -> Self {
        Self::new(status_code, "application/xml".to_string(), xml.into().into_bytes())
    }

    /// Add header
    pub fn with_header<S: Into<String>>(mut self, key: S, value: S) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Add multiple headers
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers.extend(headers);
        self
    }

    /// Add metadata
    pub fn with_metadata<S: Into<String>>(mut self, key: S, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Set cache control
    pub fn with_cache_control<S: Into<String>>(mut self, cache_control: S) -> Self {
        self.cache_control = Some(cache_control.into());
        self
    }

    /// Add custom data
    pub fn with_custom<S: Into<String>>(mut self, key: S, value: Value) -> Self {
        self.custom.insert(key.into(), value);
        self
    }

    /// Convert to axum response
    pub fn to_axum_response(self) -> Result<axum::response::Response> {
        use axum::http::HeaderValue;
        use axum::response::Response;

        let mut response = Response::new(axum::body::Body::from(self.body));
        *response.status_mut() = StatusCode::from_u16(self.status_code)
            .map_err(|_| PluginError::execution("Invalid status code"))?;

        // Add headers
        for (key, value) in self.headers {
            if let (Ok(header_name), Ok(header_value)) =
                (key.parse::<axum::http::HeaderName>(), value.parse::<HeaderValue>())
            {
                response.headers_mut().insert(header_name, header_value);
            }
        }

        // Set content type if not already set
        if !response.headers().contains_key("content-type") {
            if let Ok(header_value) = self.content_type.parse::<HeaderValue>() {
                response.headers_mut().insert("content-type", header_value);
            }
        }

        // Set cache control if specified
        if let Some(cache_control) = self.cache_control {
            if let Ok(header_value) = cache_control.parse::<HeaderValue>() {
                response.headers_mut().insert("cache-control", header_value);
            }
        }

        Ok(response)
    }

    /// Get body as string (if valid UTF-8)
    pub fn body_as_string(&self) -> Option<String> {
        String::from_utf8(self.body.clone()).ok()
    }

    /// Get body as JSON value
    pub fn body_as_json(&self) -> Option<Value> {
        serde_json::from_slice(&self.body).ok()
    }
}

/// Response plugin registry entry
pub struct ResponsePluginEntry {
    /// Plugin ID
    pub plugin_id: crate::PluginId,
    /// Plugin instance
    pub plugin: std::sync::Arc<dyn ResponsePlugin>,
    /// Plugin configuration
    pub config: ResponsePluginConfig,
    /// Plugin capabilities
    pub capabilities: PluginCapabilities,
}

impl ResponsePluginEntry {
    /// Create new plugin entry
    pub fn new(
        plugin_id: crate::PluginId,
        plugin: std::sync::Arc<dyn ResponsePlugin>,
        config: ResponsePluginConfig,
    ) -> Self {
        let capabilities = plugin.capabilities();
        Self {
            plugin_id,
            plugin,
            config,
            capabilities,
        }
    }

    /// Check if plugin is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get plugin priority
    pub fn priority(&self) -> i32 {
        self.config.priority
    }

    /// Check if plugin can handle the request
    pub fn can_handle_request(&self, request: &ResponseRequest) -> bool {
        self.is_enabled()
            && request.matches_method(&self.config.methods)
            && self
                .config
                .url_patterns
                .iter()
                .any(|pattern| request.matches_url_pattern(pattern))
    }
}

/// Response modifier plugin trait
///
/// Implement this trait to modify responses after they have been generated.
/// This allows plugins to transform, enhance, or filter responses before
/// they are sent to the client.
///
/// Use cases include:
/// - Adding custom headers or metadata
/// - Compressing response bodies
/// - Encrypting sensitive data
/// - Filtering or redacting content
/// - Adding CORS headers
/// - Response validation
#[async_trait::async_trait]
pub trait ResponseModifierPlugin: Send + Sync {
    /// Get plugin capabilities (permissions and limits)
    fn capabilities(&self) -> PluginCapabilities;

    /// Initialize the plugin with configuration
    async fn initialize(&self, config: &ResponseModifierConfig) -> Result<()>;

    /// Check if this plugin should modify the given response
    ///
    /// # Arguments
    /// * `context` - Plugin execution context
    /// * `request` - Original request information
    /// * `response` - Current response data
    /// * `config` - Plugin configuration
    ///
    /// # Returns
    /// True if this plugin should modify the response
    async fn should_modify(
        &self,
        context: &PluginContext,
        request: &ResponseRequest,
        response: &ResponseData,
        config: &ResponseModifierConfig,
    ) -> Result<PluginResult<bool>>;

    /// Modify the response
    ///
    /// This method is called when the plugin has indicated it should modify
    /// the response. It receives the current response and returns a modified version.
    ///
    /// # Arguments
    /// * `context` - Plugin execution context
    /// * `request` - Original request information
    /// * `response` - Current response data to modify
    /// * `config` - Plugin configuration
    ///
    /// # Returns
    /// Modified response data
    async fn modify_response(
        &self,
        context: &PluginContext,
        request: &ResponseRequest,
        response: ResponseData,
        config: &ResponseModifierConfig,
    ) -> Result<PluginResult<ResponseData>>;

    /// Get plugin priority (lower numbers = higher priority, executed first)
    fn priority(&self) -> i32;

    /// Validate plugin configuration
    fn validate_config(&self, config: &ResponseModifierConfig) -> Result<()>;

    /// Cleanup plugin resources
    async fn cleanup(&self) -> Result<()>;
}

/// Response modifier plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseModifierConfig {
    /// Plugin-specific configuration
    pub config: HashMap<String, Value>,
    /// Enable/disable the plugin
    pub enabled: bool,
    /// Plugin priority (lower numbers = higher priority, executed first)
    pub priority: i32,
    /// Content types this plugin modifies
    pub content_types: Vec<String>,
    /// URL patterns this plugin matches
    pub url_patterns: Vec<String>,
    /// HTTP methods this plugin handles
    pub methods: Vec<String>,
    /// Status codes this plugin modifies (empty = all)
    pub status_codes: Vec<u16>,
    /// Custom settings
    pub settings: HashMap<String, Value>,
}

impl Default for ResponseModifierConfig {
    fn default() -> Self {
        Self {
            config: HashMap::new(),
            enabled: true,
            priority: 100,
            content_types: vec!["application/json".to_string()],
            url_patterns: vec!["*".to_string()],
            methods: vec!["GET".to_string(), "POST".to_string()],
            status_codes: vec![], // Empty means all status codes
            settings: HashMap::new(),
        }
    }
}

/// Helper trait for creating response plugins
pub trait ResponsePluginFactory: Send + Sync {
    /// Create a new response plugin instance
    fn create_plugin(&self) -> Result<Box<dyn ResponsePlugin>>;
}

/// Helper trait for creating response modifier plugins
pub trait ResponseModifierPluginFactory: Send + Sync {
    /// Create a new response modifier plugin instance
    fn create_plugin(&self) -> Result<Box<dyn ResponseModifierPlugin>>;
}

/// Built-in response helpers
pub mod helpers {
    use super::*;

    /// Create a standard error response
    pub fn error_response(status_code: u16, message: &str) -> ResponseData {
        let error_data = serde_json::json!({
            "error": {
                "message": message,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "status_code": status_code
            }
        });

        ResponseData::json(status_code, &error_data)
            .unwrap_or_else(|_| ResponseData::text(status_code, format!("Error: {}", message)))
    }

    /// Create a success response with data
    pub fn success_response<T: Serialize>(data: &T) -> Result<ResponseData> {
        ResponseData::json(200, data)
    }

    /// Create a redirect response
    pub fn redirect_response(location: &str, permanent: bool) -> ResponseData {
        let status_code = if permanent { 301 } else { 302 };
        ResponseData::new(
            status_code,
            "text/plain".to_string(),
            format!("Redirecting to: {}", location).into_bytes(),
        )
        .with_header("location", location)
    }

    /// Create a not found response
    pub fn not_found_response(message: Option<&str>) -> ResponseData {
        let message = message.unwrap_or("Resource not found");
        error_response(404, message)
    }

    /// Create an unauthorized response
    pub fn unauthorized_response(message: Option<&str>) -> ResponseData {
        let message = message.unwrap_or("Unauthorized");
        error_response(401, message)
    }

    /// Create a forbidden response
    pub fn forbidden_response(message: Option<&str>) -> ResponseData {
        let message = message.unwrap_or("Forbidden");
        error_response(403, message)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_module_compiles() {
        // Basic compilation test
    }
}
