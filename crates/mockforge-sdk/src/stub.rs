//! Response stub configuration

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Type alias for a dynamic response function
pub type DynamicResponseFn = Arc<dyn Fn(&RequestContext) -> Value + Send + Sync>;

/// Request context passed to dynamic response functions
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Path parameters extracted from the URL
    pub path_params: HashMap<String, String>,
    /// Query parameters
    pub query_params: HashMap<String, String>,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Request body
    pub body: Option<Value>,
}

/// A response stub for mocking API endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseStub {
    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    pub method: String,
    /// Path pattern (supports {{path_params}})
    pub path: String,
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body (supports templates like {{uuid}}, {{faker.name}})
    pub body: Value,
    /// Optional latency in milliseconds
    pub latency_ms: Option<u64>,
}

impl ResponseStub {
    /// Create a new response stub
    pub fn new(method: impl Into<String>, path: impl Into<String>, body: Value) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            status: 200,
            headers: HashMap::new(),
            body,
            latency_ms: None,
        }
    }

    /// Set the HTTP status code
    pub fn status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    /// Add a response header
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set response latency in milliseconds
    pub fn latency(mut self, ms: u64) -> Self {
        self.latency_ms = Some(ms);
        self
    }
}

/// Dynamic stub with runtime response generation
pub struct DynamicStub {
    /// HTTP method
    pub method: String,
    /// Path pattern
    pub path: String,
    /// HTTP status code (can be dynamic)
    pub status: Arc<RwLock<u16>>,
    /// Response headers (can be dynamic)
    pub headers: Arc<RwLock<HashMap<String, String>>>,
    /// Dynamic response function
    pub response_fn: DynamicResponseFn,
    /// Optional latency in milliseconds
    pub latency_ms: Option<u64>,
}

impl DynamicStub {
    /// Create a new dynamic stub
    pub fn new<F>(method: impl Into<String>, path: impl Into<String>, response_fn: F) -> Self
    where
        F: Fn(&RequestContext) -> Value + Send + Sync + 'static,
    {
        Self {
            method: method.into(),
            path: path.into(),
            status: Arc::new(RwLock::new(200)),
            headers: Arc::new(RwLock::new(HashMap::new())),
            response_fn: Arc::new(response_fn),
            latency_ms: None,
        }
    }

    /// Set the HTTP status code
    pub async fn set_status(&self, status: u16) {
        *self.status.write().await = status;
    }

    /// Get the current status code
    pub async fn get_status(&self) -> u16 {
        *self.status.read().await
    }

    /// Add a response header
    pub async fn add_header(&self, key: String, value: String) {
        self.headers.write().await.insert(key, value);
    }

    /// Remove a response header
    pub async fn remove_header(&self, key: &str) {
        self.headers.write().await.remove(key);
    }

    /// Get all headers (returns a clone)
    ///
    /// For more efficient read-only access, consider using `with_headers()` instead.
    pub async fn get_headers(&self) -> HashMap<String, String> {
        self.headers.read().await.clone()
    }

    /// Access headers without cloning via a callback
    ///
    /// This is more efficient than `get_headers()` when you only need to
    /// read header values without modifying them.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use mockforge_sdk::DynamicStub;
    /// # use serde_json::json;
    /// # async fn example() {
    /// let stub = DynamicStub::new("GET", "/test", |_| json!({}));
    /// stub.add_header("X-Custom".to_string(), "value".to_string()).await;
    ///
    /// // Efficient read-only access
    /// let has_custom = stub.with_headers(|headers| {
    ///     headers.contains_key("X-Custom")
    /// }).await;
    /// # }
    /// ```
    pub async fn with_headers<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&HashMap<String, String>) -> R,
    {
        let headers = self.headers.read().await;
        f(&headers)
    }

    /// Generate a response for a given request context
    pub fn generate_response(&self, ctx: &RequestContext) -> Value {
        (self.response_fn)(ctx)
    }

    /// Set latency
    pub fn with_latency(mut self, ms: u64) -> Self {
        self.latency_ms = Some(ms);
        self
    }
}

/// Builder for creating response stubs
pub struct StubBuilder {
    method: String,
    path: String,
    status: u16,
    headers: HashMap<String, String>,
    body: Value,
    latency_ms: Option<u64>,
}

impl StubBuilder {
    /// Create a new stub builder
    pub fn new(method: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            status: 200,
            headers: HashMap::new(),
            body: Value::Null,
            latency_ms: None,
        }
    }

    /// Set the HTTP status code
    pub fn status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    /// Add a response header
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set the response body
    pub fn body(mut self, body: Value) -> Self {
        self.body = body;
        self
    }

    /// Set response latency in milliseconds
    pub fn latency(mut self, ms: u64) -> Self {
        self.latency_ms = Some(ms);
        self
    }

    /// Build the response stub
    pub fn build(self) -> ResponseStub {
        ResponseStub {
            method: self.method,
            path: self.path,
            status: self.status,
            headers: self.headers,
            body: self.body,
            latency_ms: self.latency_ms,
        }
    }
}
