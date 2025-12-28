//! Response stub configuration

use mockforge_core::ResourceIdExtract as CoreResourceIdExtract;
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

/// State machine configuration for stateful stub responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachineConfig {
    /// Resource type identifier (e.g., "order", "user", "payment")
    pub resource_type: String,
    /// Resource ID extraction configuration
    #[serde(flatten)]
    pub resource_id_extract: ResourceIdExtractConfig,
    /// Initial state name
    pub initial_state: String,
    /// State-based response mappings (state name -> response override)
    /// If provided, responses will vary based on current state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_responses: Option<HashMap<String, StateResponseOverride>>,
}

/// Resource ID extraction configuration for state machines
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "extract_type", rename_all = "snake_case")]
pub enum ResourceIdExtractConfig {
    /// Extract from path parameter (e.g., "/`orders/{order_id`}" -> extract "`order_id`")
    PathParam {
        /// Path parameter name to extract
        param: String,
    },
    /// Extract from `JSONPath` in request body
    JsonPath {
        /// `JSONPath` expression to extract the resource ID
        path: String,
    },
    /// Extract from header value
    Header {
        /// Header name to extract the resource ID from
        name: String,
    },
    /// Extract from query parameter
    QueryParam {
        /// Query parameter name to extract
        param: String,
    },
}

/// State-based response override
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateResponseOverride {
    /// Optional status code override (if None, uses stub's default status)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    /// Optional body override (if None, uses stub's default body)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
    /// Optional headers override (merged with stub's default headers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

/// Fault injection configuration for per-stub error and latency simulation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StubFaultInjectionConfig {
    /// Enable fault injection for this stub
    #[serde(default)]
    pub enabled: bool,
    /// HTTP error codes to inject (randomly selected if multiple)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_errors: Option<Vec<u16>>,
    /// Probability of injecting HTTP error (0.0-1.0, default: 1.0 if `http_errors` set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_error_probability: Option<f64>,
    /// Inject timeout error (returns 504 Gateway Timeout)
    #[serde(default)]
    pub timeout_error: bool,
    /// Timeout duration in milliseconds (only used if `timeout_error` is true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    /// Probability of timeout error (0.0-1.0, default: 1.0 if `timeout_error` is true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_probability: Option<f64>,
    /// Inject connection error (returns 503 Service Unavailable)
    #[serde(default)]
    pub connection_error: bool,
    /// Probability of connection error (0.0-1.0, default: 1.0 if `connection_error` is true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_error_probability: Option<f64>,
}

impl StubFaultInjectionConfig {
    /// Create a simple HTTP error injection config
    #[must_use]
    pub fn http_error(codes: Vec<u16>) -> Self {
        Self {
            enabled: true,
            http_errors: Some(codes),
            http_error_probability: Some(1.0),
            ..Default::default()
        }
    }

    /// Create a timeout error injection config
    #[must_use]
    pub fn timeout(ms: u64) -> Self {
        Self {
            enabled: true,
            timeout_error: true,
            timeout_ms: Some(ms),
            timeout_probability: Some(1.0),
            ..Default::default()
        }
    }

    /// Create a connection error injection config
    #[must_use]
    pub fn connection_error() -> Self {
        Self {
            enabled: true,
            connection_error: true,
            connection_error_probability: Some(1.0),
            ..Default::default()
        }
    }
}

/// A response stub for mocking API endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseStub {
    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    pub method: String,
    /// Path pattern (supports {{`path_params`}})
    pub path: String,
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body (supports templates like {{uuid}}, {{faker.name}})
    pub body: Value,
    /// Optional latency in milliseconds
    pub latency_ms: Option<u64>,
    /// Optional state machine configuration for stateful behavior
    /// When set, responses will vary based on resource state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_machine: Option<StateMachineConfig>,
    /// Optional fault injection configuration for error simulation
    /// When set, can inject errors, timeouts, or connection failures
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fault_injection: Option<StubFaultInjectionConfig>,
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
            state_machine: None,
            fault_injection: None,
        }
    }

    /// Set the HTTP status code
    #[must_use]
    pub const fn status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    /// Add a response header
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set response latency in milliseconds
    #[must_use]
    pub const fn latency(mut self, ms: u64) -> Self {
        self.latency_ms = Some(ms);
        self
    }

    /// Set state machine configuration for stateful behavior
    #[must_use]
    pub fn with_state_machine(mut self, config: StateMachineConfig) -> Self {
        self.state_machine = Some(config);
        self
    }

    /// Check if this stub has state machine configuration
    #[must_use]
    pub const fn has_state_machine(&self) -> bool {
        self.state_machine.is_some()
    }

    /// Get state machine configuration
    #[must_use]
    pub const fn state_machine(&self) -> Option<&StateMachineConfig> {
        self.state_machine.as_ref()
    }

    /// Apply state-based response override if state machine is configured
    ///
    /// This method checks if the stub has state machine configuration and applies
    /// state-based response overrides based on the current state.
    ///
    /// Returns a modified stub with state-specific overrides applied, or the original
    /// stub if no state machine config or no override for current state.
    #[must_use]
    pub fn apply_state_override(&self, current_state: &str) -> Self {
        let mut stub = self.clone();

        if let Some(ref state_machine) = self.state_machine {
            if let Some(ref state_responses) = state_machine.state_responses {
                if let Some(override_config) = state_responses.get(current_state) {
                    // Apply status override
                    if let Some(status) = override_config.status {
                        stub.status = status;
                    }

                    // Apply body override
                    if let Some(ref body) = override_config.body {
                        stub.body = body.clone();
                    }

                    // Merge headers
                    if let Some(ref headers) = override_config.headers {
                        for (key, value) in headers {
                            stub.headers.insert(key.clone(), value.clone());
                        }
                    }
                }
            }
        }

        stub
    }

    /// Set fault injection configuration
    #[must_use]
    pub fn with_fault_injection(mut self, config: StubFaultInjectionConfig) -> Self {
        self.fault_injection = Some(config);
        self
    }

    /// Check if this stub has fault injection configured
    #[must_use]
    pub fn has_fault_injection(&self) -> bool {
        self.fault_injection.as_ref().is_some_and(|f| f.enabled)
    }

    /// Get fault injection configuration
    #[must_use]
    pub const fn fault_injection(&self) -> Option<&StubFaultInjectionConfig> {
        self.fault_injection.as_ref()
    }
}

impl ResourceIdExtractConfig {
    /// Convert to core's `ResourceIdExtract` enum
    #[must_use]
    pub fn to_core(&self) -> CoreResourceIdExtract {
        match self {
            Self::PathParam { param } => CoreResourceIdExtract::PathParam {
                param: param.clone(),
            },
            Self::JsonPath { path } => CoreResourceIdExtract::JsonPath { path: path.clone() },
            Self::Header { name } => CoreResourceIdExtract::Header { name: name.clone() },
            Self::QueryParam { param } => CoreResourceIdExtract::QueryParam {
                param: param.clone(),
            },
        }
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
    #[must_use]
    pub fn generate_response(&self, ctx: &RequestContext) -> Value {
        (self.response_fn)(ctx)
    }

    /// Set latency
    #[must_use]
    pub const fn with_latency(mut self, ms: u64) -> Self {
        self.latency_ms = Some(ms);
        self
    }
}

/// Builder for creating `ResponseStub` instances with a fluent API
///
/// Provides a convenient way to construct response stubs with method chaining.
///
/// # Examples
///
/// ```rust
/// use mockforge_sdk::StubBuilder;
/// use serde_json::json;
///
/// let stub = StubBuilder::new("GET", "/api/users")
///     .status(200)
///     .header("Content-Type", "application/json")
///     .body(json!({"users": []}))
///     .latency(100)
///     .build();
/// ```
pub struct StubBuilder {
    method: String,
    path: String,
    status: u16,
    headers: HashMap<String, String>,
    body: Option<Value>,
    latency_ms: Option<u64>,
    state_machine: Option<StateMachineConfig>,
    fault_injection: Option<StubFaultInjectionConfig>,
}

impl StubBuilder {
    /// Create a new stub builder
    ///
    /// # Arguments
    /// * `method` - HTTP method (GET, POST, PUT, DELETE, etc.)
    /// * `path` - URL path pattern
    pub fn new(method: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            status: 200,
            headers: HashMap::new(),
            body: None,
            latency_ms: None,
            state_machine: None,
            fault_injection: None,
        }
    }

    /// Set the HTTP status code
    #[must_use]
    pub const fn status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    /// Add a response header
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set the response body
    #[must_use]
    pub fn body(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }

    /// Set response latency in milliseconds
    #[must_use]
    pub const fn latency(mut self, ms: u64) -> Self {
        self.latency_ms = Some(ms);
        self
    }

    /// Set state machine configuration
    #[must_use]
    pub fn state_machine(mut self, config: StateMachineConfig) -> Self {
        self.state_machine = Some(config);
        self
    }

    /// Set fault injection configuration
    #[must_use]
    pub fn fault_injection(mut self, config: StubFaultInjectionConfig) -> Self {
        self.fault_injection = Some(config);
        self
    }

    /// Build the response stub
    #[must_use]
    pub fn build(self) -> ResponseStub {
        ResponseStub {
            method: self.method,
            path: self.path,
            status: self.status,
            headers: self.headers,
            body: self.body.unwrap_or(Value::Null),
            latency_ms: self.latency_ms,
            state_machine: self.state_machine,
            fault_injection: self.fault_injection,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ==================== RequestContext Tests ====================

    #[test]
    fn test_request_context_creation() {
        let ctx = RequestContext {
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            path_params: HashMap::from([("id".to_string(), "123".to_string())]),
            query_params: HashMap::from([("page".to_string(), "1".to_string())]),
            headers: HashMap::from([("content-type".to_string(), "application/json".to_string())]),
            body: Some(json!({"name": "test"})),
        };

        assert_eq!(ctx.method, "GET");
        assert_eq!(ctx.path, "/api/users");
        assert_eq!(ctx.path_params.get("id"), Some(&"123".to_string()));
    }

    #[test]
    fn test_request_context_clone() {
        let ctx = RequestContext {
            method: "POST".to_string(),
            path: "/api/users".to_string(),
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: None,
        };

        let cloned = ctx.clone();
        assert_eq!(ctx.method, cloned.method);
        assert_eq!(ctx.path, cloned.path);
    }

    // ==================== StateMachineConfig Tests ====================

    #[test]
    fn test_state_machine_config_serialize() {
        let config = StateMachineConfig {
            resource_type: "order".to_string(),
            resource_id_extract: ResourceIdExtractConfig::PathParam {
                param: "order_id".to_string(),
            },
            initial_state: "pending".to_string(),
            state_responses: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("order"));
        assert!(json.contains("pending"));
    }

    #[test]
    fn test_state_machine_config_with_responses() {
        let mut responses = HashMap::new();
        responses.insert(
            "confirmed".to_string(),
            StateResponseOverride {
                status: Some(200),
                body: Some(json!({"status": "confirmed"})),
                headers: None,
            },
        );

        let config = StateMachineConfig {
            resource_type: "order".to_string(),
            resource_id_extract: ResourceIdExtractConfig::PathParam {
                param: "order_id".to_string(),
            },
            initial_state: "pending".to_string(),
            state_responses: Some(responses),
        };

        assert!(config.state_responses.is_some());
    }

    // ==================== ResourceIdExtractConfig Tests ====================

    #[test]
    fn test_resource_id_extract_path_param() {
        let config = ResourceIdExtractConfig::PathParam {
            param: "user_id".to_string(),
        };
        let core = config.to_core();
        match core {
            CoreResourceIdExtract::PathParam { param } => assert_eq!(param, "user_id"),
            _ => panic!("Expected PathParam"),
        }
    }

    #[test]
    fn test_resource_id_extract_json_path() {
        let config = ResourceIdExtractConfig::JsonPath {
            path: "$.data.id".to_string(),
        };
        let core = config.to_core();
        match core {
            CoreResourceIdExtract::JsonPath { path } => assert_eq!(path, "$.data.id"),
            _ => panic!("Expected JsonPath"),
        }
    }

    #[test]
    fn test_resource_id_extract_header() {
        let config = ResourceIdExtractConfig::Header {
            name: "X-Resource-Id".to_string(),
        };
        let core = config.to_core();
        match core {
            CoreResourceIdExtract::Header { name } => assert_eq!(name, "X-Resource-Id"),
            _ => panic!("Expected Header"),
        }
    }

    #[test]
    fn test_resource_id_extract_query_param() {
        let config = ResourceIdExtractConfig::QueryParam {
            param: "id".to_string(),
        };
        let core = config.to_core();
        match core {
            CoreResourceIdExtract::QueryParam { param } => assert_eq!(param, "id"),
            _ => panic!("Expected QueryParam"),
        }
    }

    // ==================== StateResponseOverride Tests ====================

    #[test]
    fn test_state_response_override_status_only() {
        let override_config = StateResponseOverride {
            status: Some(404),
            body: None,
            headers: None,
        };
        assert_eq!(override_config.status, Some(404));
    }

    #[test]
    fn test_state_response_override_full() {
        let mut headers = HashMap::new();
        headers.insert("X-Custom".to_string(), "value".to_string());

        let override_config = StateResponseOverride {
            status: Some(200),
            body: Some(json!({"data": "test"})),
            headers: Some(headers),
        };

        assert_eq!(override_config.status, Some(200));
        assert!(override_config.body.is_some());
        assert!(override_config.headers.is_some());
    }

    // ==================== StubFaultInjectionConfig Tests ====================

    #[test]
    fn test_stub_fault_injection_default() {
        let config = StubFaultInjectionConfig::default();
        assert!(!config.enabled);
        assert!(config.http_errors.is_none());
        assert!(!config.timeout_error);
        assert!(!config.connection_error);
    }

    #[test]
    fn test_stub_fault_injection_http_error() {
        let config = StubFaultInjectionConfig::http_error(vec![500, 502, 503]);
        assert!(config.enabled);
        assert_eq!(config.http_errors, Some(vec![500, 502, 503]));
        assert_eq!(config.http_error_probability, Some(1.0));
    }

    #[test]
    fn test_stub_fault_injection_timeout() {
        let config = StubFaultInjectionConfig::timeout(5000);
        assert!(config.enabled);
        assert!(config.timeout_error);
        assert_eq!(config.timeout_ms, Some(5000));
        assert_eq!(config.timeout_probability, Some(1.0));
    }

    #[test]
    fn test_stub_fault_injection_connection_error() {
        let config = StubFaultInjectionConfig::connection_error();
        assert!(config.enabled);
        assert!(config.connection_error);
        assert_eq!(config.connection_error_probability, Some(1.0));
    }

    // ==================== ResponseStub Tests ====================

    #[test]
    fn test_response_stub_new() {
        let stub = ResponseStub::new("GET", "/api/users", json!({"users": []}));
        assert_eq!(stub.method, "GET");
        assert_eq!(stub.path, "/api/users");
        assert_eq!(stub.status, 200);
        assert!(stub.headers.is_empty());
        assert!(stub.latency_ms.is_none());
    }

    #[test]
    fn test_response_stub_status() {
        let stub = ResponseStub::new("GET", "/api/users", json!({})).status(404);
        assert_eq!(stub.status, 404);
    }

    #[test]
    fn test_response_stub_header() {
        let stub = ResponseStub::new("GET", "/api/users", json!({}))
            .header("Content-Type", "application/json")
            .header("X-Custom", "value");

        assert_eq!(stub.headers.get("Content-Type"), Some(&"application/json".to_string()));
        assert_eq!(stub.headers.get("X-Custom"), Some(&"value".to_string()));
    }

    #[test]
    fn test_response_stub_latency() {
        let stub = ResponseStub::new("GET", "/api/users", json!({})).latency(100);
        assert_eq!(stub.latency_ms, Some(100));
    }

    #[test]
    fn test_response_stub_with_state_machine() {
        let state_config = StateMachineConfig {
            resource_type: "user".to_string(),
            resource_id_extract: ResourceIdExtractConfig::PathParam {
                param: "user_id".to_string(),
            },
            initial_state: "active".to_string(),
            state_responses: None,
        };

        let stub = ResponseStub::new("GET", "/api/users/{user_id}", json!({}))
            .with_state_machine(state_config);

        assert!(stub.has_state_machine());
        assert!(stub.state_machine().is_some());
    }

    #[test]
    fn test_response_stub_no_state_machine() {
        let stub = ResponseStub::new("GET", "/api/users", json!({}));
        assert!(!stub.has_state_machine());
        assert!(stub.state_machine().is_none());
    }

    #[test]
    fn test_response_stub_apply_state_override() {
        let mut state_responses = HashMap::new();
        state_responses.insert(
            "inactive".to_string(),
            StateResponseOverride {
                status: Some(403),
                body: Some(json!({"error": "User is inactive"})),
                headers: Some(HashMap::from([("X-State".to_string(), "inactive".to_string())])),
            },
        );

        let state_config = StateMachineConfig {
            resource_type: "user".to_string(),
            resource_id_extract: ResourceIdExtractConfig::PathParam {
                param: "user_id".to_string(),
            },
            initial_state: "active".to_string(),
            state_responses: Some(state_responses),
        };

        let stub = ResponseStub::new("GET", "/api/users/{user_id}", json!({"status": "ok"}))
            .with_state_machine(state_config);

        // Apply inactive state override
        let overridden = stub.apply_state_override("inactive");
        assert_eq!(overridden.status, 403);
        assert_eq!(overridden.body, json!({"error": "User is inactive"}));
        assert_eq!(overridden.headers.get("X-State"), Some(&"inactive".to_string()));
    }

    #[test]
    fn test_response_stub_apply_state_override_no_match() {
        let state_config = StateMachineConfig {
            resource_type: "user".to_string(),
            resource_id_extract: ResourceIdExtractConfig::PathParam {
                param: "user_id".to_string(),
            },
            initial_state: "active".to_string(),
            state_responses: None,
        };

        let stub = ResponseStub::new("GET", "/api/users/{user_id}", json!({"original": true}))
            .status(200)
            .with_state_machine(state_config);

        // State override with no matching state should return original
        let overridden = stub.apply_state_override("unknown");
        assert_eq!(overridden.status, 200);
        assert_eq!(overridden.body, json!({"original": true}));
    }

    #[test]
    fn test_response_stub_with_fault_injection() {
        let fault_config = StubFaultInjectionConfig::http_error(vec![500]);
        let stub =
            ResponseStub::new("GET", "/api/users", json!({})).with_fault_injection(fault_config);

        assert!(stub.has_fault_injection());
        assert!(stub.fault_injection().is_some());
    }

    #[test]
    fn test_response_stub_no_fault_injection() {
        let stub = ResponseStub::new("GET", "/api/users", json!({}));
        assert!(!stub.has_fault_injection());
    }

    #[test]
    fn test_response_stub_serialize() {
        let stub = ResponseStub::new("POST", "/api/orders", json!({"id": 1}))
            .status(201)
            .header("Location", "/api/orders/1")
            .latency(50);

        let json = serde_json::to_string(&stub).unwrap();
        assert!(json.contains("POST"));
        assert!(json.contains("/api/orders"));
        assert!(json.contains("201"));
    }

    // ==================== DynamicStub Tests ====================

    #[test]
    fn test_dynamic_stub_new() {
        let stub = DynamicStub::new("GET", "/api/users", |ctx| json!({"path": ctx.path.clone()}));

        assert_eq!(stub.method, "GET");
        assert_eq!(stub.path, "/api/users");
    }

    #[tokio::test]
    async fn test_dynamic_stub_status() {
        let stub = DynamicStub::new("GET", "/test", |_| json!({}));
        assert_eq!(stub.get_status().await, 200);

        stub.set_status(404).await;
        assert_eq!(stub.get_status().await, 404);
    }

    #[tokio::test]
    async fn test_dynamic_stub_headers() {
        let stub = DynamicStub::new("GET", "/test", |_| json!({}));

        stub.add_header("X-Custom".to_string(), "value".to_string()).await;

        let headers = stub.get_headers().await;
        assert_eq!(headers.get("X-Custom"), Some(&"value".to_string()));

        stub.remove_header("X-Custom").await;
        let headers = stub.get_headers().await;
        assert!(headers.get("X-Custom").is_none());
    }

    #[tokio::test]
    async fn test_dynamic_stub_with_headers() {
        let stub = DynamicStub::new("GET", "/test", |_| json!({}));
        stub.add_header("X-Test".to_string(), "test-value".to_string()).await;

        let has_header = stub.with_headers(|headers| headers.contains_key("X-Test")).await;
        assert!(has_header);
    }

    #[test]
    fn test_dynamic_stub_generate_response() {
        let stub = DynamicStub::new("GET", "/api/users/{id}", |ctx| {
            let id = ctx.path_params.get("id").cloned().unwrap_or_default();
            json!({"user_id": id})
        });

        let ctx = RequestContext {
            method: "GET".to_string(),
            path: "/api/users/123".to_string(),
            path_params: HashMap::from([("id".to_string(), "123".to_string())]),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: None,
        };

        let response = stub.generate_response(&ctx);
        assert_eq!(response, json!({"user_id": "123"}));
    }

    #[test]
    fn test_dynamic_stub_with_latency() {
        let stub = DynamicStub::new("GET", "/test", |_| json!({})).with_latency(100);
        assert_eq!(stub.latency_ms, Some(100));
    }

    // ==================== StubBuilder Tests ====================

    #[test]
    fn test_stub_builder_basic() {
        let stub = StubBuilder::new("GET", "/api/users").body(json!({"users": []})).build();

        assert_eq!(stub.method, "GET");
        assert_eq!(stub.path, "/api/users");
        assert_eq!(stub.status, 200);
    }

    #[test]
    fn test_stub_builder_status() {
        let stub = StubBuilder::new("GET", "/api/users").status(404).build();

        assert_eq!(stub.status, 404);
    }

    #[test]
    fn test_stub_builder_headers() {
        let stub = StubBuilder::new("GET", "/api/users")
            .header("Content-Type", "application/json")
            .header("X-Custom", "value")
            .build();

        assert_eq!(stub.headers.len(), 2);
    }

    #[test]
    fn test_stub_builder_latency() {
        let stub = StubBuilder::new("GET", "/api/users").latency(500).build();

        assert_eq!(stub.latency_ms, Some(500));
    }

    #[test]
    fn test_stub_builder_state_machine() {
        let config = StateMachineConfig {
            resource_type: "order".to_string(),
            resource_id_extract: ResourceIdExtractConfig::PathParam {
                param: "order_id".to_string(),
            },
            initial_state: "pending".to_string(),
            state_responses: None,
        };

        let stub = StubBuilder::new("GET", "/api/orders/{order_id}").state_machine(config).build();

        assert!(stub.state_machine.is_some());
    }

    #[test]
    fn test_stub_builder_fault_injection() {
        let fault = StubFaultInjectionConfig::http_error(vec![500]);

        let stub = StubBuilder::new("GET", "/api/users").fault_injection(fault).build();

        assert!(stub.fault_injection.is_some());
    }

    #[test]
    fn test_stub_builder_full_chain() {
        let stub = StubBuilder::new("POST", "/api/orders")
            .status(201)
            .header("Location", "/api/orders/1")
            .body(json!({"id": 1, "status": "created"}))
            .latency(100)
            .build();

        assert_eq!(stub.method, "POST");
        assert_eq!(stub.path, "/api/orders");
        assert_eq!(stub.status, 201);
        assert_eq!(stub.headers.get("Location"), Some(&"/api/orders/1".to_string()));
        assert_eq!(stub.latency_ms, Some(100));
    }
}
