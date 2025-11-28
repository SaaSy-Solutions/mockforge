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

/// Builder for creating response stubs
pub struct StubBuilder {
    method: String,
    path: String,
    status: u16,
    headers: HashMap<String, String>,
    body: Value,
    latency_ms: Option<u64>,
    state_machine: Option<StateMachineConfig>,
    fault_injection: Option<StubFaultInjectionConfig>,
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
        self.body = body;
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
            body: self.body,
            latency_ms: self.latency_ms,
            state_machine: self.state_machine,
            fault_injection: self.fault_injection,
        }
    }
}
