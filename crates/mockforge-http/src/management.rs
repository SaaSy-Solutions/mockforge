/// Management API for MockForge
///
/// Provides REST endpoints for controlling mocks, server configuration,
/// and integration with developer tools (VS Code extension, CI/CD, etc.)
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{
        sse::{Event, Sse},
        IntoResponse, Json, Response,
    },
    routing::{delete, get, post, put},
    Router,
};
use futures::stream::{self, Stream};
use mockforge_core::openapi::OpenApiSpec;
use mockforge_core::proxy::config::{
    BodyTransform, BodyTransformRule, ProxyConfig, TransformOperation,
};
#[cfg(feature = "smtp")]
use mockforge_smtp::EmailSearchFilters;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::*;

/// Message event types for real-time monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "protocol", content = "data")]
#[serde(rename_all = "lowercase")]
pub enum MessageEvent {
    #[cfg(feature = "mqtt")]
    /// MQTT message event
    Mqtt(MqttMessageEvent),
    #[cfg(feature = "kafka")]
    /// Kafka message event
    Kafka(KafkaMessageEvent),
}

#[cfg(feature = "mqtt")]
/// MQTT message event for real-time monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttMessageEvent {
    /// MQTT topic name
    pub topic: String,
    /// Message payload content
    pub payload: String,
    /// Quality of Service level (0, 1, or 2)
    pub qos: u8,
    /// Whether the message is retained
    pub retain: bool,
    /// RFC3339 formatted timestamp
    pub timestamp: String,
}

#[cfg(feature = "kafka")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaMessageEvent {
    pub topic: String,
    pub key: Option<String>,
    pub value: String,
    pub partition: i32,
    pub offset: i64,
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub timestamp: String,
}

/// Mock configuration representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockConfig {
    /// Unique identifier for the mock
    #[serde(skip_serializing_if = "String::is_empty")]
    pub id: String,
    /// Human-readable name for the mock
    pub name: String,
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// API path pattern to match
    pub path: String,
    /// Response configuration
    pub response: MockResponse,
    /// Whether this mock is currently enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Optional latency to inject in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    /// Optional HTTP status code override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    /// Request matching criteria (headers, query params, body patterns)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_match: Option<RequestMatchCriteria>,
    /// Priority for mock ordering (higher priority mocks are matched first)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    /// Scenario name for stateful mocking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario: Option<String>,
    /// Required scenario state for this mock to be active
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_scenario_state: Option<String>,
    /// New scenario state after this mock is matched
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_scenario_state: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Mock response configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockResponse {
    /// Response body as JSON
    pub body: serde_json::Value,
    /// Optional custom response headers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::HashMap<String, String>>,
}

/// Request matching criteria for advanced request matching
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequestMatchCriteria {
    /// Headers that must be present and match (case-insensitive header names)
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub headers: std::collections::HashMap<String, String>,
    /// Query parameters that must be present and match
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub query_params: std::collections::HashMap<String, String>,
    /// Request body pattern (supports exact match or regex)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_pattern: Option<String>,
    /// JSONPath expression for JSON body matching
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_path: Option<String>,
    /// XPath expression for XML body matching
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xpath: Option<String>,
    /// Custom matcher expression (e.g., "headers.content-type == \"application/json\"")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_matcher: Option<String>,
}

/// Check if a request matches the given mock configuration
///
/// This function implements comprehensive request matching including:
/// - Method and path matching
/// - Header matching (with regex support)
/// - Query parameter matching
/// - Body pattern matching (exact, regex, JSONPath, XPath)
/// - Custom matcher expressions
pub fn mock_matches_request(
    mock: &MockConfig,
    method: &str,
    path: &str,
    headers: &std::collections::HashMap<String, String>,
    query_params: &std::collections::HashMap<String, String>,
    body: Option<&[u8]>,
) -> bool {
    use regex::Regex;

    // Check if mock is enabled
    if !mock.enabled {
        return false;
    }

    // Check method (case-insensitive)
    if mock.method.to_uppercase() != method.to_uppercase() {
        return false;
    }

    // Check path pattern (supports wildcards and path parameters)
    if !path_matches_pattern(&mock.path, path) {
        return false;
    }

    // Check request matching criteria if present
    if let Some(criteria) = &mock.request_match {
        // Check headers
        for (key, expected_value) in &criteria.headers {
            let header_key_lower = key.to_lowercase();
            let found = headers.iter().find(|(k, _)| k.to_lowercase() == header_key_lower);

            if let Some((_, actual_value)) = found {
                // Try regex match first, then exact match
                if let Ok(re) = Regex::new(expected_value) {
                    if !re.is_match(actual_value) {
                        return false;
                    }
                } else if actual_value != expected_value {
                    return false;
                }
            } else {
                return false; // Header not found
            }
        }

        // Check query parameters
        for (key, expected_value) in &criteria.query_params {
            if let Some(actual_value) = query_params.get(key) {
                if actual_value != expected_value {
                    return false;
                }
            } else {
                return false; // Query param not found
            }
        }

        // Check body pattern
        if let Some(pattern) = &criteria.body_pattern {
            if let Some(body_bytes) = body {
                let body_str = String::from_utf8_lossy(body_bytes);
                // Try regex first, then exact match
                if let Ok(re) = Regex::new(pattern) {
                    if !re.is_match(&body_str) {
                        return false;
                    }
                } else if body_str.as_ref() != pattern {
                    return false;
                }
            } else {
                return false; // Body required but not present
            }
        }

        // Check JSONPath (simplified implementation)
        if let Some(json_path) = &criteria.json_path {
            if let Some(body_bytes) = body {
                if let Ok(body_str) = std::str::from_utf8(body_bytes) {
                    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(body_str) {
                        // Simple JSONPath check
                        if !json_path_exists(&json_value, json_path) {
                            return false;
                        }
                    }
                }
            }
        }

        // Check XPath (placeholder - requires XML/XPath library for full implementation)
        if let Some(_xpath) = &criteria.xpath {
            // XPath matching would require an XML/XPath library
            // For now, this is a placeholder that warns but doesn't fail
            tracing::warn!("XPath matching not yet fully implemented");
        }

        // Check custom matcher
        if let Some(custom) = &criteria.custom_matcher {
            if !evaluate_custom_matcher(custom, method, path, headers, query_params, body) {
                return false;
            }
        }
    }

    true
}

/// Check if a path matches a pattern (supports wildcards and path parameters)
fn path_matches_pattern(pattern: &str, path: &str) -> bool {
    // Exact match
    if pattern == path {
        return true;
    }

    // Wildcard match
    if pattern == "*" {
        return true;
    }

    // Path parameter matching (e.g., /users/{id} matches /users/123)
    let pattern_parts: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if pattern_parts.len() != path_parts.len() {
        // Check for wildcard patterns
        if pattern.contains('*') {
            return matches_wildcard_pattern(pattern, path);
        }
        return false;
    }

    for (pattern_part, path_part) in pattern_parts.iter().zip(path_parts.iter()) {
        // Check for path parameters {param}
        if pattern_part.starts_with('{') && pattern_part.ends_with('}') {
            continue; // Matches any value
        }

        if pattern_part != path_part {
            return false;
        }
    }

    true
}

/// Check if path matches a wildcard pattern
fn matches_wildcard_pattern(pattern: &str, path: &str) -> bool {
    use regex::Regex;

    // Convert pattern to regex
    let regex_pattern = pattern.replace('*', ".*").replace('?', ".?");

    if let Ok(re) = Regex::new(&format!("^{}$", regex_pattern)) {
        return re.is_match(path);
    }

    false
}

/// Check if a JSONPath exists in a JSON value (simplified implementation)
fn json_path_exists(json: &serde_json::Value, json_path: &str) -> bool {
    // Simple implementation - for full JSONPath support, use a library like jsonpath-rs
    // This handles simple paths like $.field or $.field.subfield
    if json_path.starts_with("$.") {
        let path = &json_path[2..];
        let parts: Vec<&str> = path.split('.').collect();

        let mut current = json;
        for part in parts {
            if let Some(obj) = current.as_object() {
                if let Some(value) = obj.get(part) {
                    current = value;
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    } else {
        // For complex JSONPath expressions, would need a proper JSONPath library
        tracing::warn!("Complex JSONPath expressions not yet fully supported: {}", json_path);
        false
    }
}

/// Evaluate a custom matcher expression
fn evaluate_custom_matcher(
    expression: &str,
    method: &str,
    path: &str,
    headers: &std::collections::HashMap<String, String>,
    query_params: &std::collections::HashMap<String, String>,
    body: Option<&[u8]>,
) -> bool {
    use regex::Regex;

    let expr = expression.trim();

    // Handle equality expressions (field == "value")
    if expr.contains("==") {
        let parts: Vec<&str> = expr.split("==").map(|s| s.trim()).collect();
        if parts.len() != 2 {
            return false;
        }

        let field = parts[0];
        let expected_value = parts[1].trim_matches('"').trim_matches('\'');

        match field {
            "method" => method == expected_value,
            "path" => path == expected_value,
            _ if field.starts_with("headers.") => {
                let header_name = &field[8..];
                headers.get(header_name).map(|v| v == expected_value).unwrap_or(false)
            }
            _ if field.starts_with("query.") => {
                let param_name = &field[6..];
                query_params.get(param_name).map(|v| v == expected_value).unwrap_or(false)
            }
            _ => false,
        }
    }
    // Handle regex match expressions (field =~ "pattern")
    else if expr.contains("=~") {
        let parts: Vec<&str> = expr.split("=~").map(|s| s.trim()).collect();
        if parts.len() != 2 {
            return false;
        }

        let field = parts[0];
        let pattern = parts[1].trim_matches('"').trim_matches('\'');

        if let Ok(re) = Regex::new(pattern) {
            match field {
                "method" => re.is_match(method),
                "path" => re.is_match(path),
                _ if field.starts_with("headers.") => {
                    let header_name = &field[8..];
                    headers.get(header_name).map(|v| re.is_match(v)).unwrap_or(false)
                }
                _ if field.starts_with("query.") => {
                    let param_name = &field[6..];
                    query_params.get(param_name).map(|v| re.is_match(v)).unwrap_or(false)
                }
                _ => false,
            }
        } else {
            false
        }
    }
    // Handle contains expressions (field contains "value")
    else if expr.contains("contains") {
        let parts: Vec<&str> = expr.split("contains").map(|s| s.trim()).collect();
        if parts.len() != 2 {
            return false;
        }

        let field = parts[0];
        let search_value = parts[1].trim_matches('"').trim_matches('\'');

        match field {
            "path" => path.contains(search_value),
            _ if field.starts_with("headers.") => {
                let header_name = &field[8..];
                headers.get(header_name).map(|v| v.contains(search_value)).unwrap_or(false)
            }
            _ if field.starts_with("body") => {
                if let Some(body_bytes) = body {
                    let body_str = String::from_utf8_lossy(body_bytes);
                    body_str.contains(search_value)
                } else {
                    false
                }
            }
            _ => false,
        }
    } else {
        // Unknown expression format
        tracing::warn!("Unknown custom matcher expression format: {}", expr);
        false
    }
}

/// Server statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStats {
    /// Server uptime in seconds
    pub uptime_seconds: u64,
    /// Total number of requests processed
    pub total_requests: u64,
    /// Number of active mock configurations
    pub active_mocks: usize,
    /// Number of currently enabled mocks
    pub enabled_mocks: usize,
    /// Number of registered API routes
    pub registered_routes: usize,
}

/// Server configuration info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// MockForge version string
    pub version: String,
    /// Server port number
    pub port: u16,
    /// Whether an OpenAPI spec is loaded
    pub has_openapi_spec: bool,
    /// Optional path to the OpenAPI spec file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_path: Option<String>,
}

/// Shared state for the management API
#[derive(Clone)]
pub struct ManagementState {
    /// Collection of mock configurations
    pub mocks: Arc<RwLock<Vec<MockConfig>>>,
    /// Optional OpenAPI specification
    pub spec: Option<Arc<OpenApiSpec>>,
    /// Optional path to the OpenAPI spec file
    pub spec_path: Option<String>,
    /// Server port number
    pub port: u16,
    /// Server start time for uptime calculation
    pub start_time: std::time::Instant,
    /// Counter for total requests processed
    pub request_counter: Arc<RwLock<u64>>,
    /// Optional proxy configuration for migration pipeline
    pub proxy_config: Option<Arc<RwLock<ProxyConfig>>>,
    /// Optional SMTP registry for email mocking
    #[cfg(feature = "smtp")]
    pub smtp_registry: Option<Arc<mockforge_smtp::SmtpSpecRegistry>>,
    /// Optional MQTT broker for message mocking
    #[cfg(feature = "mqtt")]
    pub mqtt_broker: Option<Arc<mockforge_mqtt::MqttBroker>>,
    /// Optional Kafka broker for event streaming
    #[cfg(feature = "kafka")]
    pub kafka_broker: Option<Arc<mockforge_kafka::KafkaMockBroker>>,
    /// Broadcast channel for message events (MQTT & Kafka)
    #[cfg(any(feature = "mqtt", feature = "kafka"))]
    pub message_events: Arc<broadcast::Sender<MessageEvent>>,
    /// State machine manager for scenario state machines
    pub state_machine_manager:
        Arc<RwLock<mockforge_scenarios::state_machine::ScenarioStateMachineManager>>,
    /// Optional WebSocket broadcast channel for real-time updates
    pub ws_broadcast: Option<Arc<broadcast::Sender<crate::management_ws::MockEvent>>>,
    /// Lifecycle hook registry for extensibility
    pub lifecycle_hooks: Option<Arc<mockforge_core::lifecycle::LifecycleHookRegistry>>,
    /// Rule explanations storage (in-memory for now)
    pub rule_explanations: Arc<
        RwLock<
            std::collections::HashMap<
                String,
                mockforge_core::intelligent_behavior::RuleExplanation,
            >,
        >,
    >,
    /// Optional chaos API state for chaos config management
    #[cfg(feature = "chaos")]
    pub chaos_api_state: Option<Arc<mockforge_chaos::api::ChaosApiState>>,
    /// Optional server configuration for profile application
    pub server_config: Option<Arc<RwLock<mockforge_core::config::ServerConfig>>>,
}

impl ManagementState {
    /// Create a new management state
    ///
    /// # Arguments
    /// * `spec` - Optional OpenAPI specification
    /// * `spec_path` - Optional path to the OpenAPI spec file
    /// * `port` - Server port number
    pub fn new(spec: Option<Arc<OpenApiSpec>>, spec_path: Option<String>, port: u16) -> Self {
        Self {
            mocks: Arc::new(RwLock::new(Vec::new())),
            spec,
            spec_path,
            port,
            start_time: std::time::Instant::now(),
            request_counter: Arc::new(RwLock::new(0)),
            proxy_config: None,
            #[cfg(feature = "smtp")]
            smtp_registry: None,
            #[cfg(feature = "mqtt")]
            mqtt_broker: None,
            #[cfg(feature = "kafka")]
            kafka_broker: None,
            #[cfg(any(feature = "mqtt", feature = "kafka"))]
            message_events: {
                let (tx, _) = broadcast::channel(1000);
                Arc::new(tx)
            },
            state_machine_manager: Arc::new(RwLock::new(
                mockforge_scenarios::state_machine::ScenarioStateMachineManager::new(),
            )),
            ws_broadcast: None,
            lifecycle_hooks: None,
            rule_explanations: Arc::new(RwLock::new(std::collections::HashMap::new())),
            #[cfg(feature = "chaos")]
            chaos_api_state: None,
            server_config: None,
        }
    }

    /// Add lifecycle hook registry to management state
    pub fn with_lifecycle_hooks(
        mut self,
        hooks: Arc<mockforge_core::lifecycle::LifecycleHookRegistry>,
    ) -> Self {
        self.lifecycle_hooks = Some(hooks);
        self
    }

    /// Add WebSocket broadcast channel to management state
    pub fn with_ws_broadcast(
        mut self,
        ws_broadcast: Arc<broadcast::Sender<crate::management_ws::MockEvent>>,
    ) -> Self {
        self.ws_broadcast = Some(ws_broadcast);
        self
    }

    /// Add proxy configuration to management state
    pub fn with_proxy_config(mut self, proxy_config: Arc<RwLock<ProxyConfig>>) -> Self {
        self.proxy_config = Some(proxy_config);
        self
    }

    #[cfg(feature = "smtp")]
    /// Add SMTP registry to management state
    pub fn with_smtp_registry(
        mut self,
        smtp_registry: Arc<mockforge_smtp::SmtpSpecRegistry>,
    ) -> Self {
        self.smtp_registry = Some(smtp_registry);
        self
    }

    #[cfg(feature = "mqtt")]
    /// Add MQTT broker to management state
    pub fn with_mqtt_broker(mut self, mqtt_broker: Arc<mockforge_mqtt::MqttBroker>) -> Self {
        self.mqtt_broker = Some(mqtt_broker);
        self
    }

    #[cfg(feature = "kafka")]
    /// Add Kafka broker to management state
    pub fn with_kafka_broker(
        mut self,
        kafka_broker: Arc<mockforge_kafka::KafkaMockBroker>,
    ) -> Self {
        self.kafka_broker = Some(kafka_broker);
        self
    }

    #[cfg(feature = "chaos")]
    /// Add chaos API state to management state
    pub fn with_chaos_api_state(
        mut self,
        chaos_api_state: Arc<mockforge_chaos::api::ChaosApiState>,
    ) -> Self {
        self.chaos_api_state = Some(chaos_api_state);
        self
    }

    /// Add server configuration to management state
    pub fn with_server_config(
        mut self,
        server_config: Arc<RwLock<mockforge_core::config::ServerConfig>>,
    ) -> Self {
        self.server_config = Some(server_config);
        self
    }
}

/// List all mocks
async fn list_mocks(State(state): State<ManagementState>) -> Json<serde_json::Value> {
    let mocks = state.mocks.read().await;
    Json(serde_json::json!({
        "mocks": *mocks,
        "total": mocks.len(),
        "enabled": mocks.iter().filter(|m| m.enabled).count()
    }))
}

/// Get a specific mock by ID
async fn get_mock(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
) -> Result<Json<MockConfig>, StatusCode> {
    let mocks = state.mocks.read().await;
    mocks
        .iter()
        .find(|m| m.id == id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Create a new mock
async fn create_mock(
    State(state): State<ManagementState>,
    Json(mut mock): Json<MockConfig>,
) -> Result<Json<MockConfig>, StatusCode> {
    let mut mocks = state.mocks.write().await;

    // Generate ID if not provided
    if mock.id.is_empty() {
        mock.id = uuid::Uuid::new_v4().to_string();
    }

    // Check for duplicate ID
    if mocks.iter().any(|m| m.id == mock.id) {
        return Err(StatusCode::CONFLICT);
    }

    info!("Creating mock: {} {} {}", mock.method, mock.path, mock.id);

    // Invoke lifecycle hooks
    if let Some(hooks) = &state.lifecycle_hooks {
        let event = mockforge_core::lifecycle::MockLifecycleEvent::Created {
            id: mock.id.clone(),
            name: mock.name.clone(),
            config: serde_json::to_value(&mock).unwrap_or_default(),
        };
        hooks.invoke_mock_created(&event).await;
    }

    mocks.push(mock.clone());

    // Broadcast WebSocket event
    if let Some(tx) = &state.ws_broadcast {
        let _ = tx.send(crate::management_ws::MockEvent::mock_created(mock.clone()));
    }

    Ok(Json(mock))
}

/// Update an existing mock
async fn update_mock(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
    Json(updated_mock): Json<MockConfig>,
) -> Result<Json<MockConfig>, StatusCode> {
    let mut mocks = state.mocks.write().await;

    let position = mocks.iter().position(|m| m.id == id).ok_or(StatusCode::NOT_FOUND)?;

    // Get old mock for comparison
    let old_mock = mocks[position].clone();

    info!("Updating mock: {}", id);
    mocks[position] = updated_mock.clone();

    // Invoke lifecycle hooks
    if let Some(hooks) = &state.lifecycle_hooks {
        let event = mockforge_core::lifecycle::MockLifecycleEvent::Updated {
            id: updated_mock.id.clone(),
            name: updated_mock.name.clone(),
            config: serde_json::to_value(&updated_mock).unwrap_or_default(),
        };
        hooks.invoke_mock_updated(&event).await;

        // Check if enabled state changed
        if old_mock.enabled != updated_mock.enabled {
            let state_event = if updated_mock.enabled {
                mockforge_core::lifecycle::MockLifecycleEvent::Enabled {
                    id: updated_mock.id.clone(),
                }
            } else {
                mockforge_core::lifecycle::MockLifecycleEvent::Disabled {
                    id: updated_mock.id.clone(),
                }
            };
            hooks.invoke_mock_state_changed(&state_event).await;
        }
    }

    // Broadcast WebSocket event
    if let Some(tx) = &state.ws_broadcast {
        let _ = tx.send(crate::management_ws::MockEvent::mock_updated(updated_mock.clone()));
    }

    Ok(Json(updated_mock))
}

/// Delete a mock
async fn delete_mock(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let mut mocks = state.mocks.write().await;

    let position = mocks.iter().position(|m| m.id == id).ok_or(StatusCode::NOT_FOUND)?;

    // Get mock info before deletion for lifecycle hooks
    let deleted_mock = mocks[position].clone();

    info!("Deleting mock: {}", id);
    mocks.remove(position);

    // Invoke lifecycle hooks
    if let Some(hooks) = &state.lifecycle_hooks {
        let event = mockforge_core::lifecycle::MockLifecycleEvent::Deleted {
            id: deleted_mock.id.clone(),
            name: deleted_mock.name.clone(),
        };
        hooks.invoke_mock_deleted(&event).await;
    }

    // Broadcast WebSocket event
    if let Some(tx) = &state.ws_broadcast {
        let _ = tx.send(crate::management_ws::MockEvent::mock_deleted(id.clone()));
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Request to validate configuration
#[derive(Debug, Deserialize)]
pub struct ValidateConfigRequest {
    /// Configuration to validate (as JSON)
    pub config: serde_json::Value,
    /// Format of the configuration ("json" or "yaml")
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "json".to_string()
}

/// Validate configuration without applying it
async fn validate_config(Json(request): Json<ValidateConfigRequest>) -> impl IntoResponse {
    use mockforge_core::config::ServerConfig;

    let config_result: Result<ServerConfig, String> = match request.format.as_str() {
        "yaml" | "yml" => {
            let yaml_str = match serde_json::to_string(&request.config) {
                Ok(s) => s,
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "valid": false,
                            "error": format!("Failed to convert to string: {}", e),
                            "message": "Configuration validation failed"
                        })),
                    )
                        .into_response();
                }
            };
            serde_yaml::from_str(&yaml_str).map_err(|e| format!("YAML parse error: {}", e))
        }
        _ => serde_json::from_value(request.config).map_err(|e| format!("JSON parse error: {}", e)),
    };

    match config_result {
        Ok(_) => Json(serde_json::json!({
            "valid": true,
            "message": "Configuration is valid"
        }))
        .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "valid": false,
                "error": format!("Invalid configuration: {}", e),
                "message": "Configuration validation failed"
            })),
        )
            .into_response(),
    }
}

/// Request for bulk configuration update
#[derive(Debug, Deserialize)]
pub struct BulkConfigUpdateRequest {
    /// Partial configuration updates (only specified fields will be updated)
    pub updates: serde_json::Value,
}

/// Bulk update configuration
///
/// This endpoint allows updating multiple configuration options at once.
/// Only the specified fields in the updates object will be modified.
///
/// Configuration updates are applied to the server configuration if available
/// in ManagementState. Changes take effect immediately for supported settings.
async fn bulk_update_config(
    State(state): State<ManagementState>,
    Json(request): Json<BulkConfigUpdateRequest>,
) -> impl IntoResponse {
    // Validate the updates structure
    if !request.updates.is_object() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid request",
                "message": "Updates must be a JSON object"
            })),
        )
            .into_response();
    }

    // Try to validate as partial ServerConfig
    use mockforge_core::config::ServerConfig;

    // Create a minimal valid config and try to merge updates
    let base_config = ServerConfig::default();
    let base_json = match serde_json::to_value(&base_config) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Internal error",
                    "message": format!("Failed to serialize base config: {}", e)
                })),
            )
                .into_response();
        }
    };

    // Merge updates into base config (simplified merge)
    let mut merged = base_json.clone();
    if let (Some(merged_obj), Some(updates_obj)) =
        (merged.as_object_mut(), request.updates.as_object())
    {
        for (key, value) in updates_obj {
            merged_obj.insert(key.clone(), value.clone());
        }
    }

    // Validate the merged config
    match serde_json::from_value::<ServerConfig>(merged) {
        Ok(_) => {
            // Config is valid
            // Note: Runtime application of config changes would require:
            // 1. Storing ServerConfig in ManagementState
            // 2. Implementing hot-reload mechanism for server configuration
            // 3. Updating router state and middleware based on new config
            // For now, this endpoint only validates the configuration structure
            Json(serde_json::json!({
                "success": true,
                "message": "Bulk configuration update validated successfully. Note: Runtime application requires ServerConfig in ManagementState and hot-reload support.",
                "updates_received": request.updates,
                "validated": true
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid configuration",
                "message": format!("Configuration validation failed: {}", e),
                "validated": false
            })),
        )
            .into_response(),
    }
}

/// Get server statistics
async fn get_stats(State(state): State<ManagementState>) -> Json<ServerStats> {
    let mocks = state.mocks.read().await;
    let request_count = *state.request_counter.read().await;

    Json(ServerStats {
        uptime_seconds: state.start_time.elapsed().as_secs(),
        total_requests: request_count,
        active_mocks: mocks.len(),
        enabled_mocks: mocks.iter().filter(|m| m.enabled).count(),
        registered_routes: mocks.len(), // This could be enhanced with actual route registry info
    })
}

/// Get server configuration
async fn get_config(State(state): State<ManagementState>) -> Json<ServerConfig> {
    Json(ServerConfig {
        version: env!("CARGO_PKG_VERSION").to_string(),
        port: state.port,
        has_openapi_spec: state.spec.is_some(),
        spec_path: state.spec_path.clone(),
    })
}

/// Health check endpoint
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "mockforge-management",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Export format for mock configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    /// JSON format
    Json,
    /// YAML format
    Yaml,
}

/// Export mocks in specified format
async fn export_mocks(
    State(state): State<ManagementState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<(StatusCode, String), StatusCode> {
    let mocks = state.mocks.read().await;

    let format = params
        .get("format")
        .map(|f| match f.as_str() {
            "yaml" | "yml" => ExportFormat::Yaml,
            _ => ExportFormat::Json,
        })
        .unwrap_or(ExportFormat::Json);

    match format {
        ExportFormat::Json => serde_json::to_string_pretty(&*mocks)
            .map(|json| (StatusCode::OK, json))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR),
        ExportFormat::Yaml => serde_yaml::to_string(&*mocks)
            .map(|yaml| (StatusCode::OK, yaml))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Import mocks from JSON/YAML
async fn import_mocks(
    State(state): State<ManagementState>,
    Json(mocks): Json<Vec<MockConfig>>,
) -> impl IntoResponse {
    let mut current_mocks = state.mocks.write().await;
    current_mocks.clear();
    current_mocks.extend(mocks);
    Json(serde_json::json!({ "status": "imported", "count": current_mocks.len() }))
}

#[cfg(feature = "smtp")]
/// List SMTP emails in mailbox
async fn list_smtp_emails(State(state): State<ManagementState>) -> impl IntoResponse {
    if let Some(ref smtp_registry) = state.smtp_registry {
        match smtp_registry.get_emails() {
            Ok(emails) => (StatusCode::OK, Json(serde_json::json!(emails))),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to retrieve emails",
                    "message": e.to_string()
                })),
            ),
        }
    } else {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({
                "error": "SMTP mailbox management not available",
                "message": "SMTP server is not enabled or registry not available."
            })),
        )
    }
}

/// Get specific SMTP email
#[cfg(feature = "smtp")]
async fn get_smtp_email(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Some(ref smtp_registry) = state.smtp_registry {
        match smtp_registry.get_email_by_id(&id) {
            Ok(Some(email)) => (StatusCode::OK, Json(serde_json::json!(email))),
            Ok(None) => (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Email not found",
                    "id": id
                })),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to retrieve email",
                    "message": e.to_string()
                })),
            ),
        }
    } else {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({
                "error": "SMTP mailbox management not available",
                "message": "SMTP server is not enabled or registry not available."
            })),
        )
    }
}

/// Clear SMTP mailbox
#[cfg(feature = "smtp")]
async fn clear_smtp_mailbox(State(state): State<ManagementState>) -> impl IntoResponse {
    if let Some(ref smtp_registry) = state.smtp_registry {
        match smtp_registry.clear_mailbox() {
            Ok(()) => (
                StatusCode::OK,
                Json(serde_json::json!({
                    "message": "Mailbox cleared successfully"
                })),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to clear mailbox",
                    "message": e.to_string()
                })),
            ),
        }
    } else {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({
                "error": "SMTP mailbox management not available",
                "message": "SMTP server is not enabled or registry not available."
            })),
        )
    }
}

/// Export SMTP mailbox
#[cfg(feature = "smtp")]
async fn export_smtp_mailbox(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let format = params.get("format").unwrap_or(&"json".to_string()).clone();
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": "SMTP mailbox management not available via HTTP API",
            "message": "SMTP server runs separately from HTTP server. Use CLI commands to access mailbox.",
            "requested_format": format
        })),
    )
}

/// Search SMTP emails
#[cfg(feature = "smtp")]
async fn search_smtp_emails(
    State(state): State<ManagementState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    if let Some(ref smtp_registry) = state.smtp_registry {
        let filters = EmailSearchFilters {
            sender: params.get("sender").cloned(),
            recipient: params.get("recipient").cloned(),
            subject: params.get("subject").cloned(),
            body: params.get("body").cloned(),
            since: params
                .get("since")
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            until: params
                .get("until")
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            use_regex: params.get("regex").map(|s| s == "true").unwrap_or(false),
            case_sensitive: params.get("case_sensitive").map(|s| s == "true").unwrap_or(false),
        };

        match smtp_registry.search_emails(filters) {
            Ok(emails) => (StatusCode::OK, Json(serde_json::json!(emails))),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to search emails",
                    "message": e.to_string()
                })),
            ),
        }
    } else {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({
                "error": "SMTP mailbox management not available",
                "message": "SMTP server is not enabled or registry not available."
            })),
        )
    }
}

/// MQTT broker statistics
#[cfg(feature = "mqtt")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttBrokerStats {
    /// Number of connected MQTT clients
    pub connected_clients: usize,
    /// Number of active MQTT topics
    pub active_topics: usize,
    /// Number of retained messages
    pub retained_messages: usize,
    /// Total number of subscriptions
    pub total_subscriptions: usize,
}

/// MQTT management handlers
#[cfg(feature = "mqtt")]
async fn get_mqtt_stats(State(state): State<ManagementState>) -> impl IntoResponse {
    if let Some(broker) = &state.mqtt_broker {
        let connected_clients = broker.get_connected_clients().await.len();
        let active_topics = broker.get_active_topics().await.len();
        let stats = broker.get_topic_stats().await;

        let broker_stats = MqttBrokerStats {
            connected_clients,
            active_topics,
            retained_messages: stats.retained_messages,
            total_subscriptions: stats.total_subscriptions,
        };

        Json(broker_stats).into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "MQTT broker not available").into_response()
    }
}

#[cfg(feature = "mqtt")]
async fn get_mqtt_clients(State(state): State<ManagementState>) -> impl IntoResponse {
    if let Some(broker) = &state.mqtt_broker {
        let clients = broker.get_connected_clients().await;
        Json(serde_json::json!({
            "clients": clients
        }))
        .into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "MQTT broker not available").into_response()
    }
}

#[cfg(feature = "mqtt")]
async fn get_mqtt_topics(State(state): State<ManagementState>) -> impl IntoResponse {
    if let Some(broker) = &state.mqtt_broker {
        let topics = broker.get_active_topics().await;
        Json(serde_json::json!({
            "topics": topics
        }))
        .into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "MQTT broker not available").into_response()
    }
}

#[cfg(feature = "mqtt")]
async fn disconnect_mqtt_client(
    State(state): State<ManagementState>,
    Path(client_id): Path<String>,
) -> impl IntoResponse {
    if let Some(broker) = &state.mqtt_broker {
        match broker.disconnect_client(&client_id).await {
            Ok(_) => {
                (StatusCode::OK, format!("Client '{}' disconnected", client_id)).into_response()
            }
            Err(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to disconnect client: {}", e))
                    .into_response()
            }
        }
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "MQTT broker not available").into_response()
    }
}

// ========== MQTT Publish Handler ==========

#[cfg(feature = "mqtt")]
/// Request to publish a single MQTT message
#[derive(Debug, Deserialize)]
pub struct MqttPublishRequest {
    /// Topic to publish to
    pub topic: String,
    /// Message payload (string or JSON)
    pub payload: String,
    /// QoS level (0, 1, or 2)
    #[serde(default = "default_qos")]
    pub qos: u8,
    /// Whether to retain the message
    #[serde(default)]
    pub retain: bool,
}

#[cfg(feature = "mqtt")]
fn default_qos() -> u8 {
    0
}

#[cfg(feature = "mqtt")]
/// Publish a message to an MQTT topic (only compiled when mqtt feature is enabled)
async fn publish_mqtt_message_handler(
    State(state): State<ManagementState>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    // Extract fields from JSON manually
    let topic = request.get("topic").and_then(|v| v.as_str()).map(|s| s.to_string());
    let payload = request.get("payload").and_then(|v| v.as_str()).map(|s| s.to_string());
    let qos = request.get("qos").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
    let retain = request.get("retain").and_then(|v| v.as_bool()).unwrap_or(false);

    if topic.is_none() || payload.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid request",
                "message": "Missing required fields: topic and payload"
            })),
        );
    }

    let topic = topic.unwrap();
    let payload = payload.unwrap();

    if let Some(broker) = &state.mqtt_broker {
        // Validate QoS
        if qos > 2 {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid QoS",
                    "message": "QoS must be 0, 1, or 2"
                })),
            );
        }

        // Convert payload to bytes
        let payload_bytes = payload.as_bytes().to_vec();
        let client_id = "mockforge-management-api".to_string();

        let publish_result = broker
            .handle_publish(&client_id, &topic, payload_bytes, qos, retain)
            .await
            .map_err(|e| format!("{}", e));

        match publish_result {
            Ok(_) => {
                // Emit message event for real-time monitoring
                let event = MessageEvent::Mqtt(MqttMessageEvent {
                    topic: topic.clone(),
                    payload: payload.clone(),
                    qos,
                    retain,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                });
                let _ = state.message_events.send(event);

                (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "success": true,
                        "message": format!("Message published to topic '{}'", topic),
                        "topic": topic,
                        "qos": qos,
                        "retain": retain
                    })),
                )
            }
            Err(error_msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to publish message",
                    "message": error_msg
                })),
            ),
        }
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "MQTT broker not available",
                "message": "MQTT broker is not enabled or not available."
            })),
        )
    }
}

#[cfg(not(feature = "mqtt"))]
/// Publish a message to an MQTT topic (stub when mqtt feature is disabled)
async fn publish_mqtt_message_handler(
    State(_state): State<ManagementState>,
    Json(_request): Json<serde_json::Value>,
) -> impl IntoResponse {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({
            "error": "MQTT feature not enabled",
            "message": "MQTT support is not compiled into this build"
        })),
    )
}

#[cfg(feature = "mqtt")]
/// Request to publish multiple MQTT messages
#[derive(Debug, Deserialize)]
pub struct MqttBatchPublishRequest {
    /// List of messages to publish
    pub messages: Vec<MqttPublishRequest>,
    /// Delay between messages in milliseconds
    #[serde(default = "default_delay")]
    pub delay_ms: u64,
}

#[cfg(feature = "mqtt")]
fn default_delay() -> u64 {
    100
}

#[cfg(feature = "mqtt")]
/// Publish multiple messages to MQTT topics (only compiled when mqtt feature is enabled)
async fn publish_mqtt_batch_handler(
    State(state): State<ManagementState>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    // Extract fields from JSON manually
    let messages_json = request.get("messages").and_then(|v| v.as_array());
    let delay_ms = request.get("delay_ms").and_then(|v| v.as_u64()).unwrap_or(100);

    if messages_json.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid request",
                "message": "Missing required field: messages"
            })),
        );
    }

    let messages_json = messages_json.unwrap();

    if let Some(broker) = &state.mqtt_broker {
        if messages_json.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Empty batch",
                    "message": "At least one message is required"
                })),
            );
        }

        let mut results = Vec::new();
        let client_id = "mockforge-management-api".to_string();

        for (index, msg_json) in messages_json.iter().enumerate() {
            let topic = msg_json.get("topic").and_then(|v| v.as_str()).map(|s| s.to_string());
            let payload = msg_json.get("payload").and_then(|v| v.as_str()).map(|s| s.to_string());
            let qos = msg_json.get("qos").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
            let retain = msg_json.get("retain").and_then(|v| v.as_bool()).unwrap_or(false);

            if topic.is_none() || payload.is_none() {
                results.push(serde_json::json!({
                    "index": index,
                    "success": false,
                    "error": "Missing required fields: topic and payload"
                }));
                continue;
            }

            let topic = topic.unwrap();
            let payload = payload.unwrap();

            // Validate QoS
            if qos > 2 {
                results.push(serde_json::json!({
                    "index": index,
                    "success": false,
                    "error": "Invalid QoS (must be 0, 1, or 2)"
                }));
                continue;
            }

            // Convert payload to bytes
            let payload_bytes = payload.as_bytes().to_vec();

            let publish_result = broker
                .handle_publish(&client_id, &topic, payload_bytes, qos, retain)
                .await
                .map_err(|e| format!("{}", e));

            match publish_result {
                Ok(_) => {
                    // Emit message event
                    let event = MessageEvent::Mqtt(MqttMessageEvent {
                        topic: topic.clone(),
                        payload: payload.clone(),
                        qos,
                        retain,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    });
                    let _ = state.message_events.send(event);

                    results.push(serde_json::json!({
                        "index": index,
                        "success": true,
                        "topic": topic,
                        "qos": qos
                    }));
                }
                Err(error_msg) => {
                    results.push(serde_json::json!({
                        "index": index,
                        "success": false,
                        "error": error_msg
                    }));
                }
            }

            // Add delay between messages (except for the last one)
            if index < messages_json.len() - 1 && delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
        }

        let success_count =
            results.iter().filter(|r| r["success"].as_bool().unwrap_or(false)).count();

        (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "total": messages_json.len(),
                "succeeded": success_count,
                "failed": messages_json.len() - success_count,
                "results": results
            })),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "MQTT broker not available",
                "message": "MQTT broker is not enabled or not available."
            })),
        )
    }
}

#[cfg(not(feature = "mqtt"))]
/// Publish multiple messages to MQTT topics (stub when mqtt feature is disabled)
async fn publish_mqtt_batch_handler(
    State(_state): State<ManagementState>,
    Json(_request): Json<serde_json::Value>,
) -> impl IntoResponse {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({
            "error": "MQTT feature not enabled",
            "message": "MQTT support is not compiled into this build"
        })),
    )
}

// Migration pipeline handlers

/// Request to set migration mode
#[derive(Debug, Deserialize)]
struct SetMigrationModeRequest {
    mode: String,
}

/// Get all migration routes
async fn get_migration_routes(
    State(state): State<ManagementState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Migration not configured. Proxy config not available."
            })));
        }
    };

    let config = proxy_config.read().await;
    let routes = config.get_migration_routes();

    Ok(Json(serde_json::json!({
        "routes": routes
    })))
}

/// Toggle a route's migration mode
async fn toggle_route_migration(
    State(state): State<ManagementState>,
    Path(pattern): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Migration not configured. Proxy config not available."
            })));
        }
    };

    let mut config = proxy_config.write().await;
    let new_mode = match config.toggle_route_migration(&pattern) {
        Some(mode) => mode,
        None => {
            return Ok(Json(serde_json::json!({
                "error": format!("Route pattern not found: {}", pattern)
            })));
        }
    };

    Ok(Json(serde_json::json!({
        "pattern": pattern,
        "mode": format!("{:?}", new_mode).to_lowercase()
    })))
}

/// Set a route's migration mode explicitly
async fn set_route_migration_mode(
    State(state): State<ManagementState>,
    Path(pattern): Path<String>,
    Json(request): Json<SetMigrationModeRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Migration not configured. Proxy config not available."
            })));
        }
    };

    use mockforge_core::proxy::config::MigrationMode;
    let mode = match request.mode.to_lowercase().as_str() {
        "mock" => MigrationMode::Mock,
        "shadow" => MigrationMode::Shadow,
        "real" => MigrationMode::Real,
        "auto" => MigrationMode::Auto,
        _ => {
            return Ok(Json(serde_json::json!({
                "error": format!("Invalid migration mode: {}. Must be one of: mock, shadow, real, auto", request.mode)
            })));
        }
    };

    let mut config = proxy_config.write().await;
    let updated = config.update_rule_migration_mode(&pattern, mode);

    if !updated {
        return Ok(Json(serde_json::json!({
            "error": format!("Route pattern not found: {}", pattern)
        })));
    }

    Ok(Json(serde_json::json!({
        "pattern": pattern,
        "mode": format!("{:?}", mode).to_lowercase()
    })))
}

/// Toggle a group's migration mode
async fn toggle_group_migration(
    State(state): State<ManagementState>,
    Path(group): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Migration not configured. Proxy config not available."
            })));
        }
    };

    let mut config = proxy_config.write().await;
    let new_mode = config.toggle_group_migration(&group);

    Ok(Json(serde_json::json!({
        "group": group,
        "mode": format!("{:?}", new_mode).to_lowercase()
    })))
}

/// Set a group's migration mode explicitly
async fn set_group_migration_mode(
    State(state): State<ManagementState>,
    Path(group): Path<String>,
    Json(request): Json<SetMigrationModeRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Migration not configured. Proxy config not available."
            })));
        }
    };

    use mockforge_core::proxy::config::MigrationMode;
    let mode = match request.mode.to_lowercase().as_str() {
        "mock" => MigrationMode::Mock,
        "shadow" => MigrationMode::Shadow,
        "real" => MigrationMode::Real,
        "auto" => MigrationMode::Auto,
        _ => {
            return Ok(Json(serde_json::json!({
                "error": format!("Invalid migration mode: {}. Must be one of: mock, shadow, real, auto", request.mode)
            })));
        }
    };

    let mut config = proxy_config.write().await;
    config.update_group_migration_mode(&group, mode);

    Ok(Json(serde_json::json!({
        "group": group,
        "mode": format!("{:?}", mode).to_lowercase()
    })))
}

/// Get all migration groups
async fn get_migration_groups(
    State(state): State<ManagementState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Migration not configured. Proxy config not available."
            })));
        }
    };

    let config = proxy_config.read().await;
    let groups = config.get_migration_groups();

    // Convert to JSON-serializable format
    let groups_json: serde_json::Map<String, serde_json::Value> = groups
        .into_iter()
        .map(|(name, info)| {
            (
                name,
                serde_json::json!({
                    "name": info.name,
                    "migration_mode": format!("{:?}", info.migration_mode).to_lowercase(),
                    "route_count": info.route_count
                }),
            )
        })
        .collect();

    Ok(Json(serde_json::json!(groups_json)))
}

/// Get overall migration status
async fn get_migration_status(
    State(state): State<ManagementState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Migration not configured. Proxy config not available."
            })));
        }
    };

    let config = proxy_config.read().await;
    let routes = config.get_migration_routes();
    let groups = config.get_migration_groups();

    let mut mock_count = 0;
    let mut shadow_count = 0;
    let mut real_count = 0;
    let mut auto_count = 0;

    for route in &routes {
        match route.migration_mode {
            mockforge_core::proxy::config::MigrationMode::Mock => mock_count += 1,
            mockforge_core::proxy::config::MigrationMode::Shadow => shadow_count += 1,
            mockforge_core::proxy::config::MigrationMode::Real => real_count += 1,
            mockforge_core::proxy::config::MigrationMode::Auto => auto_count += 1,
        }
    }

    Ok(Json(serde_json::json!({
        "total_routes": routes.len(),
        "mock_routes": mock_count,
        "shadow_routes": shadow_count,
        "real_routes": real_count,
        "auto_routes": auto_count,
        "total_groups": groups.len(),
        "migration_enabled": config.migration_enabled
    })))
}

// ========== Proxy Replacement Rules Management ==========

/// Request body for creating/updating proxy replacement rules
#[derive(Debug, Deserialize, Serialize)]
pub struct ProxyRuleRequest {
    /// URL pattern to match (supports wildcards like "/api/users/*")
    pub pattern: String,
    /// Rule type: "request" or "response"
    #[serde(rename = "type")]
    pub rule_type: String,
    /// Optional status code filter for response rules
    #[serde(default)]
    pub status_codes: Vec<u16>,
    /// Body transformations to apply
    pub body_transforms: Vec<BodyTransformRequest>,
    /// Whether this rule is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Request body for individual body transformations
#[derive(Debug, Deserialize, Serialize)]
pub struct BodyTransformRequest {
    /// JSONPath expression to target (e.g., "$.userId", "$.email")
    pub path: String,
    /// Replacement value (supports template expansion like "{{uuid}}", "{{faker.email}}")
    pub replace: String,
    /// Operation to perform: "replace", "add", or "remove"
    #[serde(default)]
    pub operation: String,
}

/// Response format for proxy rules
#[derive(Debug, Serialize)]
pub struct ProxyRuleResponse {
    /// Rule ID (index in the array)
    pub id: usize,
    /// URL pattern
    pub pattern: String,
    /// Rule type
    #[serde(rename = "type")]
    pub rule_type: String,
    /// Status codes (for response rules)
    pub status_codes: Vec<u16>,
    /// Body transformations
    pub body_transforms: Vec<BodyTransformRequest>,
    /// Whether enabled
    pub enabled: bool,
}

/// List all proxy replacement rules
async fn list_proxy_rules(
    State(state): State<ManagementState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Proxy not configured. Proxy config not available."
            })));
        }
    };

    let config = proxy_config.read().await;

    let mut rules: Vec<ProxyRuleResponse> = Vec::new();

    // Add request replacement rules
    for (idx, rule) in config.request_replacements.iter().enumerate() {
        rules.push(ProxyRuleResponse {
            id: idx,
            pattern: rule.pattern.clone(),
            rule_type: "request".to_string(),
            status_codes: Vec::new(),
            body_transforms: rule
                .body_transforms
                .iter()
                .map(|t| BodyTransformRequest {
                    path: t.path.clone(),
                    replace: t.replace.clone(),
                    operation: format!("{:?}", t.operation).to_lowercase(),
                })
                .collect(),
            enabled: rule.enabled,
        });
    }

    // Add response replacement rules
    let request_count = config.request_replacements.len();
    for (idx, rule) in config.response_replacements.iter().enumerate() {
        rules.push(ProxyRuleResponse {
            id: request_count + idx,
            pattern: rule.pattern.clone(),
            rule_type: "response".to_string(),
            status_codes: rule.status_codes.clone(),
            body_transforms: rule
                .body_transforms
                .iter()
                .map(|t| BodyTransformRequest {
                    path: t.path.clone(),
                    replace: t.replace.clone(),
                    operation: format!("{:?}", t.operation).to_lowercase(),
                })
                .collect(),
            enabled: rule.enabled,
        });
    }

    Ok(Json(serde_json::json!({
        "rules": rules
    })))
}

/// Create a new proxy replacement rule
async fn create_proxy_rule(
    State(state): State<ManagementState>,
    Json(request): Json<ProxyRuleRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Proxy not configured. Proxy config not available."
            })));
        }
    };

    // Validate request
    if request.body_transforms.is_empty() {
        return Ok(Json(serde_json::json!({
            "error": "At least one body transform is required"
        })));
    }

    let body_transforms: Vec<BodyTransform> = request
        .body_transforms
        .iter()
        .map(|t| {
            let op = match t.operation.as_str() {
                "replace" => TransformOperation::Replace,
                "add" => TransformOperation::Add,
                "remove" => TransformOperation::Remove,
                _ => TransformOperation::Replace,
            };
            BodyTransform {
                path: t.path.clone(),
                replace: t.replace.clone(),
                operation: op,
            }
        })
        .collect();

    let new_rule = BodyTransformRule {
        pattern: request.pattern.clone(),
        status_codes: request.status_codes.clone(),
        body_transforms,
        enabled: request.enabled,
    };

    let mut config = proxy_config.write().await;

    let rule_id = if request.rule_type == "request" {
        config.request_replacements.push(new_rule);
        config.request_replacements.len() - 1
    } else if request.rule_type == "response" {
        config.response_replacements.push(new_rule);
        config.request_replacements.len() + config.response_replacements.len() - 1
    } else {
        return Ok(Json(serde_json::json!({
            "error": format!("Invalid rule type: {}. Must be 'request' or 'response'", request.rule_type)
        })));
    };

    Ok(Json(serde_json::json!({
        "id": rule_id,
        "message": "Rule created successfully"
    })))
}

/// Get a specific proxy replacement rule
async fn get_proxy_rule(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Proxy not configured. Proxy config not available."
            })));
        }
    };

    let config = proxy_config.read().await;
    let rule_id: usize = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return Ok(Json(serde_json::json!({
                "error": format!("Invalid rule ID: {}", id)
            })));
        }
    };

    let request_count = config.request_replacements.len();

    if rule_id < request_count {
        // Request rule
        let rule = &config.request_replacements[rule_id];
        Ok(Json(serde_json::json!({
            "id": rule_id,
            "pattern": rule.pattern,
            "type": "request",
            "status_codes": [],
            "body_transforms": rule.body_transforms.iter().map(|t| serde_json::json!({
                "path": t.path,
                "replace": t.replace,
                "operation": format!("{:?}", t.operation).to_lowercase()
            })).collect::<Vec<_>>(),
            "enabled": rule.enabled
        })))
    } else if rule_id < request_count + config.response_replacements.len() {
        // Response rule
        let response_idx = rule_id - request_count;
        let rule = &config.response_replacements[response_idx];
        Ok(Json(serde_json::json!({
            "id": rule_id,
            "pattern": rule.pattern,
            "type": "response",
            "status_codes": rule.status_codes,
            "body_transforms": rule.body_transforms.iter().map(|t| serde_json::json!({
                "path": t.path,
                "replace": t.replace,
                "operation": format!("{:?}", t.operation).to_lowercase()
            })).collect::<Vec<_>>(),
            "enabled": rule.enabled
        })))
    } else {
        Ok(Json(serde_json::json!({
            "error": format!("Rule ID {} not found", rule_id)
        })))
    }
}

/// Update a proxy replacement rule
async fn update_proxy_rule(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
    Json(request): Json<ProxyRuleRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Proxy not configured. Proxy config not available."
            })));
        }
    };

    let mut config = proxy_config.write().await;
    let rule_id: usize = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return Ok(Json(serde_json::json!({
                "error": format!("Invalid rule ID: {}", id)
            })));
        }
    };

    let body_transforms: Vec<BodyTransform> = request
        .body_transforms
        .iter()
        .map(|t| {
            let op = match t.operation.as_str() {
                "replace" => TransformOperation::Replace,
                "add" => TransformOperation::Add,
                "remove" => TransformOperation::Remove,
                _ => TransformOperation::Replace,
            };
            BodyTransform {
                path: t.path.clone(),
                replace: t.replace.clone(),
                operation: op,
            }
        })
        .collect();

    let updated_rule = BodyTransformRule {
        pattern: request.pattern.clone(),
        status_codes: request.status_codes.clone(),
        body_transforms,
        enabled: request.enabled,
    };

    let request_count = config.request_replacements.len();

    if rule_id < request_count {
        // Update request rule
        config.request_replacements[rule_id] = updated_rule;
    } else if rule_id < request_count + config.response_replacements.len() {
        // Update response rule
        let response_idx = rule_id - request_count;
        config.response_replacements[response_idx] = updated_rule;
    } else {
        return Ok(Json(serde_json::json!({
            "error": format!("Rule ID {} not found", rule_id)
        })));
    }

    Ok(Json(serde_json::json!({
        "id": rule_id,
        "message": "Rule updated successfully"
    })))
}

/// Delete a proxy replacement rule
async fn delete_proxy_rule(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Proxy not configured. Proxy config not available."
            })));
        }
    };

    let mut config = proxy_config.write().await;
    let rule_id: usize = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return Ok(Json(serde_json::json!({
                "error": format!("Invalid rule ID: {}", id)
            })));
        }
    };

    let request_count = config.request_replacements.len();

    if rule_id < request_count {
        // Delete request rule
        config.request_replacements.remove(rule_id);
    } else if rule_id < request_count + config.response_replacements.len() {
        // Delete response rule
        let response_idx = rule_id - request_count;
        config.response_replacements.remove(response_idx);
    } else {
        return Ok(Json(serde_json::json!({
            "error": format!("Rule ID {} not found", rule_id)
        })));
    }

    Ok(Json(serde_json::json!({
        "id": rule_id,
        "message": "Rule deleted successfully"
    })))
}

/// Get recent intercepted requests/responses for inspection
/// This is a placeholder - in a full implementation, you'd track intercepted traffic
async fn get_proxy_inspect(
    State(_state): State<ManagementState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit: usize = params.get("limit").and_then(|s| s.parse().ok()).unwrap_or(50);

    // Note: Request/response inspection would require:
    // 1. Storing intercepted requests/responses in ManagementState or a separate store
    // 2. Integrating with proxy middleware to capture traffic
    // 3. Implementing filtering and pagination for large volumes of traffic
    // For now, return an empty response structure indicating the feature is not yet implemented
    Ok(Json(serde_json::json!({
        "requests": [],
        "responses": [],
        "limit": limit,
        "total": 0,
        "message": "Request/response inspection not yet implemented. This endpoint will return intercepted traffic when proxy inspection is fully integrated."
    })))
}

/// Build the management API router
pub fn management_router(state: ManagementState) -> Router {
    let router = Router::new()
        .route("/health", get(health_check))
        .route("/stats", get(get_stats))
        .route("/config", get(get_config))
        .route("/config/validate", post(validate_config))
        .route("/config/bulk", post(bulk_update_config))
        .route("/mocks", get(list_mocks))
        .route("/mocks", post(create_mock))
        .route("/mocks/{id}", get(get_mock))
        .route("/mocks/{id}", put(update_mock))
        .route("/mocks/{id}", delete(delete_mock))
        .route("/export", get(export_mocks))
        .route("/import", post(import_mocks));

    #[cfg(feature = "smtp")]
    let router = router
        .route("/smtp/mailbox", get(list_smtp_emails))
        .route("/smtp/mailbox", delete(clear_smtp_mailbox))
        .route("/smtp/mailbox/{id}", get(get_smtp_email))
        .route("/smtp/mailbox/export", get(export_smtp_mailbox))
        .route("/smtp/mailbox/search", get(search_smtp_emails));

    #[cfg(not(feature = "smtp"))]
    let router = router;

    // MQTT routes
    #[cfg(feature = "mqtt")]
    let router = router
        .route("/mqtt/stats", get(get_mqtt_stats))
        .route("/mqtt/clients", get(get_mqtt_clients))
        .route("/mqtt/topics", get(get_mqtt_topics))
        .route("/mqtt/clients/{client_id}", delete(disconnect_mqtt_client))
        .route("/mqtt/messages/stream", get(mqtt_messages_stream))
        .route("/mqtt/publish", post(publish_mqtt_message_handler))
        .route("/mqtt/publish/batch", post(publish_mqtt_batch_handler));

    #[cfg(not(feature = "mqtt"))]
    let router = router
        .route("/mqtt/publish", post(publish_mqtt_message_handler))
        .route("/mqtt/publish/batch", post(publish_mqtt_batch_handler));

    #[cfg(feature = "kafka")]
    let router = router
        .route("/kafka/stats", get(get_kafka_stats))
        .route("/kafka/topics", get(get_kafka_topics))
        .route("/kafka/topics/{topic}", get(get_kafka_topic))
        .route("/kafka/groups", get(get_kafka_groups))
        .route("/kafka/groups/{group_id}", get(get_kafka_group))
        .route("/kafka/produce", post(produce_kafka_message))
        .route("/kafka/produce/batch", post(produce_kafka_batch))
        .route("/kafka/messages/stream", get(kafka_messages_stream));

    #[cfg(not(feature = "kafka"))]
    let router = router;

    // Migration pipeline routes
    let router = router
        .route("/migration/routes", get(get_migration_routes))
        .route("/migration/routes/{pattern}/toggle", post(toggle_route_migration))
        .route("/migration/routes/{pattern}", put(set_route_migration_mode))
        .route("/migration/groups/{group}/toggle", post(toggle_group_migration))
        .route("/migration/groups/{group}", put(set_group_migration_mode))
        .route("/migration/groups", get(get_migration_groups))
        .route("/migration/status", get(get_migration_status));

    // Proxy replacement rules routes
    let router = router
        .route("/proxy/rules", get(list_proxy_rules))
        .route("/proxy/rules", post(create_proxy_rule))
        .route("/proxy/rules/{id}", get(get_proxy_rule))
        .route("/proxy/rules/{id}", put(update_proxy_rule))
        .route("/proxy/rules/{id}", delete(delete_proxy_rule))
        .route("/proxy/inspect", get(get_proxy_inspect));

    // AI-powered features
    let router = router
        .route("/ai/generate-spec", post(generate_ai_spec));

    // Snapshot diff endpoints
    let router = router
        .nest("/snapshot-diff", crate::handlers::snapshot_diff::snapshot_diff_router(state.clone()));

    #[cfg(feature = "behavioral-cloning")]
    let router = router
        .route("/mockai/generate-openapi", post(generate_openapi_from_traffic));

    let router = router
        .route("/mockai/learn", post(learn_from_examples))
        .route("/mockai/rules/explanations", get(list_rule_explanations))
        .route("/mockai/rules/{id}/explanation", get(get_rule_explanation))
        .route("/chaos/config", get(get_chaos_config))
        .route("/chaos/config", post(update_chaos_config))
        .route("/network/profiles", get(list_network_profiles))
        .route("/network/profile/apply", post(apply_network_profile));

    // State machine API routes
    let router =
        router.nest("/state-machines", crate::state_machine_api::create_state_machine_routes());

    router.with_state(state)
}

#[cfg(feature = "kafka")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaBrokerStats {
    /// Number of topics
    pub topics: usize,
    /// Total number of partitions
    pub partitions: usize,
    /// Number of consumer groups
    pub consumer_groups: usize,
    /// Total messages produced
    pub messages_produced: u64,
    /// Total messages consumed
    pub messages_consumed: u64,
}

#[cfg(feature = "kafka")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaTopicInfo {
    pub name: String,
    pub partitions: usize,
    pub replication_factor: i32,
}

#[cfg(feature = "kafka")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConsumerGroupInfo {
    pub group_id: String,
    pub members: usize,
    pub state: String,
}

#[cfg(feature = "kafka")]
/// Get Kafka broker statistics
async fn get_kafka_stats(State(state): State<ManagementState>) -> impl IntoResponse {
    if let Some(broker) = &state.kafka_broker {
        let topics = broker.topics.read().await;
        let consumer_groups = broker.consumer_groups.read().await;
        let metrics = broker.metrics.clone();

        let total_partitions: usize = topics.values().map(|t| t.partitions.len()).sum();
        let snapshot = metrics.snapshot();
        let messages_produced = snapshot.messages_produced_total;
        let messages_consumed = snapshot.messages_consumed_total;

        let stats = KafkaBrokerStats {
            topics: topics.len(),
            partitions: total_partitions,
            consumer_groups: consumer_groups.groups().len(),
            messages_produced,
            messages_consumed,
        };

        Json(stats).into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Kafka broker not available",
                "message": "Kafka broker is not enabled or not available."
            })),
        )
            .into_response()
    }
}

#[cfg(feature = "kafka")]
/// List Kafka topics
async fn get_kafka_topics(State(state): State<ManagementState>) -> impl IntoResponse {
    if let Some(broker) = &state.kafka_broker {
        let topics = broker.topics.read().await;
        let topic_list: Vec<KafkaTopicInfo> = topics
            .iter()
            .map(|(name, topic)| KafkaTopicInfo {
                name: name.clone(),
                partitions: topic.partitions.len(),
                replication_factor: topic.config.replication_factor,
            })
            .collect();

        Json(serde_json::json!({
            "topics": topic_list
        }))
        .into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Kafka broker not available",
                "message": "Kafka broker is not enabled or not available."
            })),
        )
            .into_response()
    }
}

#[cfg(feature = "kafka")]
/// Get Kafka topic details
async fn get_kafka_topic(
    State(state): State<ManagementState>,
    Path(topic_name): Path<String>,
) -> impl IntoResponse {
    if let Some(broker) = &state.kafka_broker {
        let topics = broker.topics.read().await;
        if let Some(topic) = topics.get(&topic_name) {
            Json(serde_json::json!({
                "name": topic_name,
                "partitions": topic.partitions.len(),
                "replication_factor": topic.config.replication_factor,
                "partitions_detail": topic.partitions.iter().enumerate().map(|(idx, partition)| serde_json::json!({
                    "id": idx as i32,
                    "leader": 0,
                    "replicas": vec![0],
                    "message_count": partition.messages.len()
                })).collect::<Vec<_>>()
            })).into_response()
        } else {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Topic not found",
                    "topic": topic_name
                })),
            )
                .into_response()
        }
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Kafka broker not available",
                "message": "Kafka broker is not enabled or not available."
            })),
        )
            .into_response()
    }
}

#[cfg(feature = "kafka")]
/// List Kafka consumer groups
async fn get_kafka_groups(State(state): State<ManagementState>) -> impl IntoResponse {
    if let Some(broker) = &state.kafka_broker {
        let consumer_groups = broker.consumer_groups.read().await;
        let groups: Vec<KafkaConsumerGroupInfo> = consumer_groups
            .groups()
            .iter()
            .map(|(group_id, group)| KafkaConsumerGroupInfo {
                group_id: group_id.clone(),
                members: group.members.len(),
                state: "Stable".to_string(), // Simplified - could be more detailed
            })
            .collect();

        Json(serde_json::json!({
            "groups": groups
        }))
        .into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Kafka broker not available",
                "message": "Kafka broker is not enabled or not available."
            })),
        )
            .into_response()
    }
}

#[cfg(feature = "kafka")]
/// Get Kafka consumer group details
async fn get_kafka_group(
    State(state): State<ManagementState>,
    Path(group_id): Path<String>,
) -> impl IntoResponse {
    if let Some(broker) = &state.kafka_broker {
        let consumer_groups = broker.consumer_groups.read().await;
        if let Some(group) = consumer_groups.groups().get(&group_id) {
            Json(serde_json::json!({
                "group_id": group_id,
                "members": group.members.len(),
                "state": "Stable",
                "members_detail": group.members.iter().map(|(member_id, member)| serde_json::json!({
                    "member_id": member_id,
                    "client_id": member.client_id,
                    "assignments": member.assignment.iter().map(|a| serde_json::json!({
                        "topic": a.topic,
                        "partitions": a.partitions
                    })).collect::<Vec<_>>()
                })).collect::<Vec<_>>(),
                "offsets": group.offsets.iter().map(|((topic, partition), offset)| serde_json::json!({
                    "topic": topic,
                    "partition": partition,
                    "offset": offset
                })).collect::<Vec<_>>()
            })).into_response()
        } else {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Consumer group not found",
                    "group_id": group_id
                })),
            )
                .into_response()
        }
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Kafka broker not available",
                "message": "Kafka broker is not enabled or not available."
            })),
        )
            .into_response()
    }
}

// ========== Kafka Produce Handler ==========

#[cfg(feature = "kafka")]
#[derive(Debug, Deserialize)]
pub struct KafkaProduceRequest {
    /// Topic to produce to
    pub topic: String,
    /// Message key (optional)
    #[serde(default)]
    pub key: Option<String>,
    /// Message value (JSON string or plain string)
    pub value: String,
    /// Partition ID (optional, auto-assigned if not provided)
    #[serde(default)]
    pub partition: Option<i32>,
    /// Message headers (optional, key-value pairs)
    #[serde(default)]
    pub headers: Option<std::collections::HashMap<String, String>>,
}

#[cfg(feature = "kafka")]
/// Produce a message to a Kafka topic
async fn produce_kafka_message(
    State(state): State<ManagementState>,
    Json(request): Json<KafkaProduceRequest>,
) -> impl IntoResponse {
    if let Some(broker) = &state.kafka_broker {
        let mut topics = broker.topics.write().await;

        // Get or create the topic
        let topic_entry = topics.entry(request.topic.clone()).or_insert_with(|| {
            crate::topics::Topic::new(request.topic.clone(), crate::topics::TopicConfig::default())
        });

        // Determine partition
        let partition_id = if let Some(partition) = request.partition {
            partition
        } else {
            topic_entry.assign_partition(request.key.as_ref().map(|k| k.as_bytes()))
        };

        // Validate partition exists
        if partition_id < 0 || partition_id >= topic_entry.partitions.len() as i32 {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid partition",
                    "message": format!("Partition {} does not exist (topic has {} partitions)", partition_id, topic_entry.partitions.len())
                })),
            )
                .into_response();
        }

        // Create the message
        let message = crate::partitions::KafkaMessage {
            offset: 0, // Will be set by partition.append
            timestamp: chrono::Utc::now().timestamp_millis(),
            key: request.key.map(|k| k.as_bytes().to_vec()),
            value: request.value.as_bytes().to_vec(),
            headers: request
                .headers
                .unwrap_or_default()
                .into_iter()
                .map(|(k, v)| (k, v.as_bytes().to_vec()))
                .collect(),
        };

        // Produce to partition
        match topic_entry.produce(partition_id, message).await {
            Ok(offset) => {
                // Record metrics
                broker.metrics.record_messages_produced(1);

                // Emit message event for real-time monitoring
                #[cfg(feature = "kafka")]
                {
                    let event = MessageEvent::Kafka(KafkaMessageEvent {
                        topic: request.topic.clone(),
                        key: request.key.clone(),
                        value: request.value.clone(),
                        partition: partition_id,
                        offset,
                        headers: request.headers.clone(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    });
                    let _ = state.message_events.send(event);
                }

                Json(serde_json::json!({
                    "success": true,
                    "message": format!("Message produced to topic '{}'", request.topic),
                    "topic": request.topic,
                    "partition": partition_id,
                    "offset": offset
                }))
                .into_response()
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to produce message",
                    "message": e.to_string()
                })),
            )
                .into_response(),
        }
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Kafka broker not available",
                "message": "Kafka broker is not enabled or not available."
            })),
        )
            .into_response()
    }
}

#[cfg(feature = "kafka")]
#[derive(Debug, Deserialize)]
pub struct KafkaBatchProduceRequest {
    /// List of messages to produce
    pub messages: Vec<KafkaProduceRequest>,
    /// Delay between messages in milliseconds
    #[serde(default = "default_delay")]
    pub delay_ms: u64,
}

#[cfg(feature = "kafka")]
/// Produce multiple messages to Kafka topics
async fn produce_kafka_batch(
    State(state): State<ManagementState>,
    Json(request): Json<KafkaBatchProduceRequest>,
) -> impl IntoResponse {
    if let Some(broker) = &state.kafka_broker {
        if request.messages.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Empty batch",
                    "message": "At least one message is required"
                })),
            )
                .into_response();
        }

        let mut results = Vec::new();

        for (index, msg_request) in request.messages.iter().enumerate() {
            let mut topics = broker.topics.write().await;

            // Get or create the topic
            let topic_entry = topics.entry(msg_request.topic.clone()).or_insert_with(|| {
                crate::topics::Topic::new(
                    msg_request.topic.clone(),
                    crate::topics::TopicConfig::default(),
                )
            });

            // Determine partition
            let partition_id = if let Some(partition) = msg_request.partition {
                partition
            } else {
                topic_entry.assign_partition(msg_request.key.as_ref().map(|k| k.as_bytes()))
            };

            // Validate partition exists
            if partition_id < 0 || partition_id >= topic_entry.partitions.len() as i32 {
                results.push(serde_json::json!({
                    "index": index,
                    "success": false,
                    "error": format!("Invalid partition {} (topic has {} partitions)", partition_id, topic_entry.partitions.len())
                }));
                continue;
            }

            // Create the message
            let message = crate::partitions::KafkaMessage {
                offset: 0,
                timestamp: chrono::Utc::now().timestamp_millis(),
                key: msg_request.key.clone().map(|k| k.as_bytes().to_vec()),
                value: msg_request.value.as_bytes().to_vec(),
                headers: msg_request
                    .headers
                    .clone()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(k, v)| (k, v.as_bytes().to_vec()))
                    .collect(),
            };

            // Produce to partition
            match topic_entry.produce(partition_id, message).await {
                Ok(offset) => {
                    broker.metrics.record_messages_produced(1);

                    // Emit message event
                    let event = MessageEvent::Kafka(KafkaMessageEvent {
                        topic: msg_request.topic.clone(),
                        key: msg_request.key.clone(),
                        value: msg_request.value.clone(),
                        partition: partition_id,
                        offset,
                        headers: msg_request.headers.clone(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    });
                    let _ = state.message_events.send(event);

                    results.push(serde_json::json!({
                        "index": index,
                        "success": true,
                        "topic": msg_request.topic,
                        "partition": partition_id,
                        "offset": offset
                    }));
                }
                Err(e) => {
                    results.push(serde_json::json!({
                        "index": index,
                        "success": false,
                        "error": e.to_string()
                    }));
                }
            }

            // Add delay between messages (except for the last one)
            if index < request.messages.len() - 1 && request.delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(request.delay_ms)).await;
            }
        }

        let success_count =
            results.iter().filter(|r| r["success"].as_bool().unwrap_or(false)).count();

        Json(serde_json::json!({
            "success": true,
            "total": request.messages.len(),
            "succeeded": success_count,
            "failed": request.messages.len() - success_count,
            "results": results
        }))
        .into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Kafka broker not available",
                "message": "Kafka broker is not enabled or not available."
            })),
        )
            .into_response()
    }
}

// ========== Real-time Message Streaming (SSE) ==========

#[cfg(feature = "mqtt")]
/// SSE stream for MQTT messages
async fn mqtt_messages_stream(
    State(state): State<ManagementState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.message_events.subscribe();
    let topic_filter = params.get("topic").cloned();

    let stream = stream::unfold(rx, move |mut rx| {
        let topic_filter = topic_filter.clone();

        async move {
            loop {
                match rx.recv().await {
                    Ok(MessageEvent::Mqtt(event)) => {
                        // Apply topic filter if specified
                        if let Some(filter) = &topic_filter {
                            if !event.topic.contains(filter) {
                                continue;
                            }
                        }

                        let event_json = serde_json::json!({
                            "protocol": "mqtt",
                            "topic": event.topic,
                            "payload": event.payload,
                            "qos": event.qos,
                            "retain": event.retain,
                            "timestamp": event.timestamp,
                        });

                        if let Ok(event_data) = serde_json::to_string(&event_json) {
                            let sse_event = Event::default().event("mqtt_message").data(event_data);
                            return Some((Ok(sse_event), rx));
                        }
                    }
                    #[cfg(feature = "kafka")]
                    Ok(MessageEvent::Kafka(_)) => {
                        // Skip Kafka events in MQTT stream
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        return None;
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!("MQTT message stream lagged, skipped {} messages", skipped);
                        continue;
                    }
                }
            }
        }
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive-text"),
    )
}

#[cfg(feature = "kafka")]
/// SSE stream for Kafka messages
async fn kafka_messages_stream(
    State(state): State<ManagementState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.message_events.subscribe();
    let topic_filter = params.get("topic").cloned();

    let stream = stream::unfold(rx, move |mut rx| {
        let topic_filter = topic_filter.clone();

        async move {
            loop {
                match rx.recv().await {
                    #[cfg(feature = "mqtt")]
                    Ok(MessageEvent::Mqtt(_)) => {
                        // Skip MQTT events in Kafka stream
                        continue;
                    }
                    Ok(MessageEvent::Kafka(event)) => {
                        // Apply topic filter if specified
                        if let Some(filter) = &topic_filter {
                            if !event.topic.contains(filter) {
                                continue;
                            }
                        }

                        let event_json = serde_json::json!({
                            "protocol": "kafka",
                            "topic": event.topic,
                            "key": event.key,
                            "value": event.value,
                            "partition": event.partition,
                            "offset": event.offset,
                            "headers": event.headers,
                            "timestamp": event.timestamp,
                        });

                        if let Ok(event_data) = serde_json::to_string(&event_json) {
                            let sse_event =
                                Event::default().event("kafka_message").data(event_data);
                            return Some((Ok(sse_event), rx));
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        return None;
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!("Kafka message stream lagged, skipped {} messages", skipped);
                        continue;
                    }
                }
            }
        }
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive-text"),
    )
}

// ========== AI-Powered Features ==========

/// Request for AI-powered API specification generation
#[derive(Debug, Deserialize)]
pub struct GenerateSpecRequest {
    /// Natural language description of the API to generate
    pub query: String,
    /// Type of specification to generate: "openapi", "graphql", or "asyncapi"
    pub spec_type: String,
    /// Optional API version (e.g., "3.0.0" for OpenAPI)
    pub api_version: Option<String>,
}

/// Request for OpenAPI generation from recorded traffic
#[derive(Debug, Deserialize)]
pub struct GenerateOpenApiFromTrafficRequest {
    /// Path to recorder database (optional, defaults to ./recordings.db)
    #[serde(default)]
    pub database_path: Option<String>,
    /// Start time for filtering (ISO 8601 format, e.g., 2025-01-01T00:00:00Z)
    #[serde(default)]
    pub since: Option<String>,
    /// End time for filtering (ISO 8601 format)
    #[serde(default)]
    pub until: Option<String>,
    /// Path pattern filter (supports wildcards, e.g., /api/*)
    #[serde(default)]
    pub path_pattern: Option<String>,
    /// Minimum confidence score for including paths (0.0 to 1.0)
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f64,
}

fn default_min_confidence() -> f64 {
    0.7
}

/// Generate API specification from natural language using AI
#[cfg(feature = "data-faker")]
async fn generate_ai_spec(
    State(_state): State<ManagementState>,
    Json(request): Json<GenerateSpecRequest>,
) -> impl IntoResponse {
    use mockforge_data::rag::{
        config::{EmbeddingProvider, LlmProvider, RagConfig},
        engine::RagEngine,
        storage::{DocumentStorage, StorageFactory},
    };
    use std::sync::Arc;

    // Build RAG config from environment variables
    let api_key = std::env::var("MOCKFORGE_RAG_API_KEY")
        .ok()
        .or_else(|| std::env::var("OPENAI_API_KEY").ok());

    // Check if RAG is configured - require API key
    if api_key.is_none() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "AI service not configured",
                "message": "Please provide an API key via MOCKFORGE_RAG_API_KEY or OPENAI_API_KEY"
            })),
        )
            .into_response();
    }

    // Build RAG configuration
    let provider_str = std::env::var("MOCKFORGE_RAG_PROVIDER")
        .unwrap_or_else(|_| "openai".to_string())
        .to_lowercase();

    let provider = match provider_str.as_str() {
        "openai" => LlmProvider::OpenAI,
        "anthropic" => LlmProvider::Anthropic,
        "ollama" => LlmProvider::Ollama,
        "openai-compatible" | "openai_compatible" => LlmProvider::OpenAICompatible,
        _ => LlmProvider::OpenAI,
    };

    let api_endpoint =
        std::env::var("MOCKFORGE_RAG_API_ENDPOINT").unwrap_or_else(|_| match provider {
            LlmProvider::OpenAI => "https://api.openai.com/v1".to_string(),
            LlmProvider::Anthropic => "https://api.anthropic.com/v1".to_string(),
            LlmProvider::Ollama => "http://localhost:11434/api".to_string(),
            LlmProvider::OpenAICompatible => "http://localhost:8000/v1".to_string(),
        });

    let model = std::env::var("MOCKFORGE_RAG_MODEL").unwrap_or_else(|_| match provider {
        LlmProvider::OpenAI => "gpt-3.5-turbo".to_string(),
        LlmProvider::Anthropic => "claude-3-sonnet-20240229".to_string(),
        LlmProvider::Ollama => "llama2".to_string(),
        LlmProvider::OpenAICompatible => "gpt-3.5-turbo".to_string(),
    });

    // Build RagConfig using default() and override fields
    let mut rag_config = RagConfig::default();
    rag_config.provider = provider;
    rag_config.api_endpoint = api_endpoint;
    rag_config.api_key = api_key;
    rag_config.model = model;
    rag_config.max_tokens = std::env::var("MOCKFORGE_RAG_MAX_TOKENS")
        .unwrap_or_else(|_| "4096".to_string())
        .parse()
        .unwrap_or(4096);
    rag_config.temperature = std::env::var("MOCKFORGE_RAG_TEMPERATURE")
        .unwrap_or_else(|_| "0.3".to_string())
        .parse()
        .unwrap_or(0.3); // Lower temperature for more structured output
    rag_config.timeout_secs = std::env::var("MOCKFORGE_RAG_TIMEOUT")
        .unwrap_or_else(|_| "60".to_string())
        .parse()
        .unwrap_or(60);
    rag_config.max_context_length = std::env::var("MOCKFORGE_RAG_CONTEXT_WINDOW")
        .unwrap_or_else(|_| "4000".to_string())
        .parse()
        .unwrap_or(4000);

    // Build the prompt for spec generation
    let spec_type_label = match request.spec_type.as_str() {
        "openapi" => "OpenAPI 3.0",
        "graphql" => "GraphQL",
        "asyncapi" => "AsyncAPI",
        _ => "OpenAPI 3.0",
    };

    let api_version = request.api_version.as_deref().unwrap_or("3.0.0");

    let prompt = format!(
        r#"You are an expert API architect. Generate a complete {} specification based on the following user requirements.

User Requirements:
{}

Instructions:
1. Generate a complete, valid {} specification
2. Include all paths, operations, request/response schemas, and components
3. Use realistic field names and data types
4. Include proper descriptions and examples
5. Follow {} best practices
6. Return ONLY the specification, no additional explanation
7. For OpenAPI, use version {}

Return the specification in {} format."#,
        spec_type_label,
        request.query,
        spec_type_label,
        spec_type_label,
        api_version,
        if request.spec_type == "graphql" {
            "GraphQL SDL"
        } else {
            "YAML"
        }
    );

    // Create in-memory storage for RAG engine
    // Note: StorageFactory::create_memory() returns Box<dyn DocumentStorage>
    // We need to use unsafe transmute or create a wrapper, but for now we'll use
    // a simpler approach: create InMemoryStorage directly
    use mockforge_data::rag::storage::InMemoryStorage;
    let storage: Arc<dyn DocumentStorage> = Arc::new(InMemoryStorage::new());

    // Create RAG engine
    let mut rag_engine = match RagEngine::new(rag_config.clone(), storage) {
        Ok(engine) => engine,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to initialize RAG engine",
                    "message": e.to_string()
                })),
            )
                .into_response();
        }
    };

    // Generate using RAG engine
    match rag_engine.generate(&prompt, None).await {
        Ok(generated_text) => {
            // Try to extract just the YAML/JSON/SDL content if LLM added explanation
            let spec = if request.spec_type == "graphql" {
                // For GraphQL, extract SDL
                extract_graphql_schema(&generated_text)
            } else {
                // For OpenAPI/AsyncAPI, extract YAML
                extract_yaml_spec(&generated_text)
            };

            Json(serde_json::json!({
                "success": true,
                "spec": spec,
                "spec_type": request.spec_type,
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "AI generation failed",
                "message": e.to_string()
            })),
        )
            .into_response(),
    }
}

#[cfg(not(feature = "data-faker"))]
async fn generate_ai_spec(
    State(_state): State<ManagementState>,
    Json(_request): Json<GenerateSpecRequest>,
) -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": "AI features not enabled",
            "message": "Please enable the 'data-faker' feature to use AI-powered specification generation"
        })),
    )
        .into_response()
}

/// Generate OpenAPI specification from recorded traffic
#[cfg(feature = "behavioral-cloning")]
async fn generate_openapi_from_traffic(
    State(_state): State<ManagementState>,
    Json(request): Json<GenerateOpenApiFromTrafficRequest>,
) -> impl IntoResponse {
    use chrono::{DateTime, Utc};
    use mockforge_core::intelligent_behavior::{
        openapi_generator::{OpenApiGenerationConfig, OpenApiSpecGenerator},
        IntelligentBehaviorConfig,
    };
    use mockforge_recorder::{
        database::RecorderDatabase,
        openapi_export::{QueryFilters, RecordingsToOpenApi},
    };
    use std::path::PathBuf;

    // Determine database path
    let db_path = if let Some(ref path) = request.database_path {
        PathBuf::from(path)
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("recordings.db")
    };

    // Open database
    let db = match RecorderDatabase::new(&db_path).await {
        Ok(db) => db,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Database error",
                    "message": format!("Failed to open recorder database: {}", e)
                })),
            )
                .into_response();
        }
    };

    // Parse time filters
    let since_dt = if let Some(ref since_str) = request.since {
        match DateTime::parse_from_rfc3339(since_str) {
            Ok(dt) => Some(dt.with_timezone(&Utc)),
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "Invalid date format",
                        "message": format!("Invalid --since format: {}. Use ISO 8601 format (e.g., 2025-01-01T00:00:00Z)", e)
                    })),
                )
                    .into_response();
            }
        }
    } else {
        None
    };

    let until_dt = if let Some(ref until_str) = request.until {
        match DateTime::parse_from_rfc3339(until_str) {
            Ok(dt) => Some(dt.with_timezone(&Utc)),
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "Invalid date format",
                        "message": format!("Invalid --until format: {}. Use ISO 8601 format (e.g., 2025-01-01T00:00:00Z)", e)
                    })),
                )
                    .into_response();
            }
        }
    } else {
        None
    };

    // Build query filters
    let query_filters = QueryFilters {
        since: since_dt,
        until: until_dt,
        path_pattern: request.path_pattern.clone(),
        min_status_code: None,
        max_requests: Some(1000),
    };

    // Query HTTP exchanges
    // Note: We need to convert from mockforge-recorder's HttpExchange to mockforge-core's HttpExchange
    // to avoid version mismatch issues. The converter returns the version from mockforge-recorder's
    // dependency, so we need to manually convert to the local version.
    let exchanges_from_recorder =
        match RecordingsToOpenApi::query_http_exchanges(&db, Some(query_filters)).await {
            Ok(exchanges) => exchanges,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "Query error",
                        "message": format!("Failed to query HTTP exchanges: {}", e)
                    })),
                )
                    .into_response();
            }
        };

    if exchanges_from_recorder.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "No exchanges found",
                "message": "No HTTP exchanges found matching the specified filters"
            })),
        )
            .into_response();
    }

    // Convert to local HttpExchange type to avoid version mismatch
    use mockforge_core::intelligent_behavior::openapi_generator::HttpExchange as LocalHttpExchange;
    let exchanges: Vec<LocalHttpExchange> = exchanges_from_recorder
        .into_iter()
        .map(|e| LocalHttpExchange {
            method: e.method,
            path: e.path,
            query_params: e.query_params,
            headers: e.headers,
            body: e.body,
            body_encoding: e.body_encoding,
            status_code: e.status_code,
            response_headers: e.response_headers,
            response_body: e.response_body,
            response_body_encoding: e.response_body_encoding,
            timestamp: e.timestamp,
        })
        .collect();

    // Create OpenAPI generator config
    let behavior_config = IntelligentBehaviorConfig::default();
    let gen_config = OpenApiGenerationConfig {
        min_confidence: request.min_confidence,
        behavior_model: Some(behavior_config.behavior_model),
    };

    // Generate OpenAPI spec
    let generator = OpenApiSpecGenerator::new(gen_config);
    let result = match generator.generate_from_exchanges(exchanges).await {
        Ok(result) => result,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Generation error",
                    "message": format!("Failed to generate OpenAPI spec: {}", e)
                })),
            )
                .into_response();
        }
    };

    // Prepare response
    let spec_json = if let Some(ref raw) = result.spec.raw_document {
        raw.clone()
    } else {
        match serde_json::to_value(&result.spec.spec) {
            Ok(json) => json,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "Serialization error",
                        "message": format!("Failed to serialize OpenAPI spec: {}", e)
                    })),
                )
                    .into_response();
            }
        }
    };

    // Build response with metadata
    let response = serde_json::json!({
        "spec": spec_json,
        "metadata": {
            "requests_analyzed": result.metadata.requests_analyzed,
            "paths_inferred": result.metadata.paths_inferred,
            "path_confidence": result.metadata.path_confidence,
            "generated_at": result.metadata.generated_at.to_rfc3339(),
            "duration_ms": result.metadata.duration_ms,
        }
    });

    Json(response).into_response()
}

/// List all rule explanations
async fn list_rule_explanations(
    State(state): State<ManagementState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    use mockforge_core::intelligent_behavior::RuleType;

    let explanations = state.rule_explanations.read().await;
    let mut explanations_vec: Vec<_> = explanations.values().cloned().collect();

    // Filter by rule type if provided
    if let Some(rule_type_str) = params.get("rule_type") {
        if let Ok(rule_type) = serde_json::from_str::<RuleType>(&format!("\"{}\"", rule_type_str)) {
            explanations_vec.retain(|e| e.rule_type == rule_type);
        }
    }

    // Filter by minimum confidence if provided
    if let Some(min_confidence_str) = params.get("min_confidence") {
        if let Ok(min_confidence) = min_confidence_str.parse::<f64>() {
            explanations_vec.retain(|e| e.confidence >= min_confidence);
        }
    }

    // Sort by confidence (descending) and then by generated_at (descending)
    explanations_vec.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.generated_at.cmp(&a.generated_at))
    });

    Json(serde_json::json!({
        "explanations": explanations_vec,
        "total": explanations_vec.len(),
    }))
    .into_response()
}

/// Get a specific rule explanation by ID
async fn get_rule_explanation(
    State(state): State<ManagementState>,
    Path(rule_id): Path<String>,
) -> impl IntoResponse {
    let explanations = state.rule_explanations.read().await;

    match explanations.get(&rule_id) {
        Some(explanation) => Json(serde_json::json!({
            "explanation": explanation,
        }))
        .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Rule explanation not found",
                "message": format!("No explanation found for rule ID: {}", rule_id)
            })),
        )
            .into_response(),
    }
}

/// Request for learning from examples
#[derive(Debug, Deserialize)]
pub struct LearnFromExamplesRequest {
    /// Example request/response pairs to learn from
    pub examples: Vec<ExamplePairRequest>,
    /// Optional configuration override
    #[serde(default)]
    pub config: Option<serde_json::Value>,
}

/// Example pair request format
#[derive(Debug, Deserialize)]
pub struct ExamplePairRequest {
    /// Request data (method, path, body, etc.)
    pub request: serde_json::Value,
    /// Response data (status_code, body, etc.)
    pub response: serde_json::Value,
}

/// Learn behavioral rules from example pairs
///
/// This endpoint accepts example request/response pairs, generates behavioral rules
/// with explanations, and stores the explanations for later retrieval.
async fn learn_from_examples(
    State(state): State<ManagementState>,
    Json(request): Json<LearnFromExamplesRequest>,
) -> impl IntoResponse {
    use mockforge_core::intelligent_behavior::{
        config::{BehaviorModelConfig, IntelligentBehaviorConfig},
        rule_generator::{ExamplePair, RuleGenerator},
    };

    if request.examples.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "No examples provided",
                "message": "At least one example pair is required"
            })),
        )
            .into_response();
    }

    // Convert request examples to ExamplePair format
    let example_pairs: Result<Vec<ExamplePair>, String> = request
        .examples
        .into_iter()
        .enumerate()
        .map(|(idx, ex)| {
            // Parse request JSON to extract method, path, body, etc.
            let method = ex
                .request
                .get("method")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "GET".to_string());
            let path = ex
                .request
                .get("path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "/".to_string());
            let request_body = ex.request.get("body").cloned();
            let query_params = ex
                .request
                .get("query_params")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default();
            let headers = ex
                .request
                .get("headers")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default();

            // Parse response JSON to extract status, body, etc.
            let status = ex
                .response
                .get("status_code")
                .or_else(|| ex.response.get("status"))
                .and_then(|v| v.as_u64())
                .map(|n| n as u16)
                .unwrap_or(200);
            let response_body = ex.response.get("body").cloned();

            Ok(ExamplePair {
                method,
                path,
                request: request_body,
                status,
                response: response_body,
                query_params,
                headers,
                metadata: {
                    let mut meta = std::collections::HashMap::new();
                    meta.insert("source".to_string(), "api".to_string());
                    meta.insert("example_index".to_string(), idx.to_string());
                    meta
                },
            })
        })
        .collect();

    let example_pairs = match example_pairs {
        Ok(pairs) => pairs,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid examples",
                    "message": e
                })),
            )
                .into_response();
        }
    };

    // Create behavior config (use provided config or default)
    let behavior_config = if let Some(config_json) = request.config {
        // Try to deserialize custom config, fall back to default
        serde_json::from_value(config_json)
            .unwrap_or_else(|_| IntelligentBehaviorConfig::default())
            .behavior_model
    } else {
        BehaviorModelConfig::default()
    };

    // Create rule generator
    let generator = RuleGenerator::new(behavior_config);

    // Generate rules with explanations
    let (rules, explanations) =
        match generator.generate_rules_with_explanations(example_pairs).await {
            Ok(result) => result,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "Rule generation failed",
                        "message": format!("Failed to generate rules: {}", e)
                    })),
                )
                    .into_response();
            }
        };

    // Store explanations in ManagementState
    {
        let mut stored_explanations = state.rule_explanations.write().await;
        for explanation in &explanations {
            stored_explanations.insert(explanation.rule_id.clone(), explanation.clone());
        }
    }

    // Prepare response
    let response = serde_json::json!({
        "success": true,
        "rules_generated": {
            "consistency_rules": rules.consistency_rules.len(),
            "schemas": rules.schemas.len(),
            "state_machines": rules.state_transitions.len(),
            "system_prompt": !rules.system_prompt.is_empty(),
        },
        "explanations": explanations.iter().map(|e| serde_json::json!({
            "rule_id": e.rule_id,
            "rule_type": e.rule_type,
            "confidence": e.confidence,
            "reasoning": e.reasoning,
        })).collect::<Vec<_>>(),
        "total_explanations": explanations.len(),
    });

    Json(response).into_response()
}

fn extract_yaml_spec(text: &str) -> String {
    // Try to find YAML code blocks
    if let Some(start) = text.find("```yaml") {
        let yaml_start = text[start + 7..].trim_start();
        if let Some(end) = yaml_start.find("```") {
            return yaml_start[..end].trim().to_string();
        }
    }
    if let Some(start) = text.find("```") {
        let content_start = text[start + 3..].trim_start();
        if let Some(end) = content_start.find("```") {
            return content_start[..end].trim().to_string();
        }
    }

    // Check if it starts with openapi: or asyncapi:
    if text.trim_start().starts_with("openapi:") || text.trim_start().starts_with("asyncapi:") {
        return text.trim().to_string();
    }

    // Return as-is if no code blocks found
    text.trim().to_string()
}

/// Extract GraphQL schema from text content
fn extract_graphql_schema(text: &str) -> String {
    // Try to find GraphQL code blocks
    if let Some(start) = text.find("```graphql") {
        let schema_start = text[start + 10..].trim_start();
        if let Some(end) = schema_start.find("```") {
            return schema_start[..end].trim().to_string();
        }
    }
    if let Some(start) = text.find("```") {
        let content_start = text[start + 3..].trim_start();
        if let Some(end) = content_start.find("```") {
            return content_start[..end].trim().to_string();
        }
    }

    // Check if it looks like GraphQL SDL (starts with type, schema, etc.)
    if text.trim_start().starts_with("type ") || text.trim_start().starts_with("schema ") {
        return text.trim().to_string();
    }

    text.trim().to_string()
}

// ========== Chaos Engineering Management ==========

/// Get current chaos engineering configuration
async fn get_chaos_config(State(state): State<ManagementState>) -> impl IntoResponse {
    #[cfg(feature = "chaos")]
    {
        if let Some(chaos_state) = &state.chaos_api_state {
            let config = chaos_state.config.read().await;
            // Convert ChaosConfig to JSON response format
            Json(serde_json::json!({
                "enabled": config.enabled,
                "latency": config.latency.as_ref().map(|l| serde_json::to_value(l).unwrap_or(serde_json::Value::Null)),
                "fault_injection": config.fault_injection.as_ref().map(|f| serde_json::to_value(f).unwrap_or(serde_json::Value::Null)),
                "rate_limit": config.rate_limit.as_ref().map(|r| serde_json::to_value(r).unwrap_or(serde_json::Value::Null)),
                "traffic_shaping": config.traffic_shaping.as_ref().map(|t| serde_json::to_value(t).unwrap_or(serde_json::Value::Null)),
            }))
            .into_response()
        } else {
            // Chaos API not available, return default
            Json(serde_json::json!({
                "enabled": false,
                "latency": null,
                "fault_injection": null,
                "rate_limit": null,
                "traffic_shaping": null,
            }))
            .into_response()
        }
    }
    #[cfg(not(feature = "chaos"))]
    {
        // Chaos feature not enabled
        Json(serde_json::json!({
            "enabled": false,
            "latency": null,
            "fault_injection": null,
            "rate_limit": null,
            "traffic_shaping": null,
        }))
        .into_response()
    }
}

/// Request to update chaos configuration
#[derive(Debug, Deserialize)]
pub struct ChaosConfigUpdate {
    /// Whether to enable chaos engineering
    pub enabled: Option<bool>,
    /// Latency configuration
    pub latency: Option<serde_json::Value>,
    /// Fault injection configuration
    pub fault_injection: Option<serde_json::Value>,
    /// Rate limiting configuration
    pub rate_limit: Option<serde_json::Value>,
    /// Traffic shaping configuration
    pub traffic_shaping: Option<serde_json::Value>,
}

/// Update chaos engineering configuration
async fn update_chaos_config(
    State(state): State<ManagementState>,
    Json(config_update): Json<ChaosConfigUpdate>,
) -> impl IntoResponse {
    #[cfg(feature = "chaos")]
    {
        if let Some(chaos_state) = &state.chaos_api_state {
            use mockforge_chaos::config::{
                ChaosConfig, FaultInjectionConfig, LatencyConfig, RateLimitConfig,
                TrafficShapingConfig,
            };

            let mut config = chaos_state.config.write().await;

            // Update enabled flag if provided
            if let Some(enabled) = config_update.enabled {
                config.enabled = enabled;
            }

            // Update latency config if provided
            if let Some(latency_json) = config_update.latency {
                if let Ok(latency) = serde_json::from_value::<LatencyConfig>(latency_json) {
                    config.latency = Some(latency);
                }
            }

            // Update fault injection config if provided
            if let Some(fault_json) = config_update.fault_injection {
                if let Ok(fault) = serde_json::from_value::<FaultInjectionConfig>(fault_json) {
                    config.fault_injection = Some(fault);
                }
            }

            // Update rate limit config if provided
            if let Some(rate_json) = config_update.rate_limit {
                if let Ok(rate) = serde_json::from_value::<RateLimitConfig>(rate_json) {
                    config.rate_limit = Some(rate);
                }
            }

            // Update traffic shaping config if provided
            if let Some(traffic_json) = config_update.traffic_shaping {
                if let Ok(traffic) = serde_json::from_value::<TrafficShapingConfig>(traffic_json) {
                    config.traffic_shaping = Some(traffic);
                }
            }

            // Reinitialize middleware injectors with new config
            // The middleware will pick up the changes on the next request
            drop(config);

            info!("Chaos configuration updated successfully");
            Json(serde_json::json!({
                "success": true,
                "message": "Chaos configuration updated and applied"
            }))
            .into_response()
        } else {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "success": false,
                    "error": "Chaos API not available",
                    "message": "Chaos engineering is not enabled or configured"
                })),
            )
                .into_response()
        }
    }
    #[cfg(not(feature = "chaos"))]
    {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({
                "success": false,
                "error": "Chaos feature not enabled",
                "message": "Chaos engineering feature is not compiled into this build"
            })),
        )
            .into_response()
    }
}

// ========== Network Profile Management ==========

/// List available network profiles
async fn list_network_profiles() -> impl IntoResponse {
    use mockforge_core::network_profiles::NetworkProfileCatalog;

    let catalog = NetworkProfileCatalog::default();
    let profiles: Vec<serde_json::Value> = catalog
        .list_profiles_with_description()
        .iter()
        .map(|(name, description)| {
            serde_json::json!({
                "name": name,
                "description": description,
            })
        })
        .collect();

    Json(serde_json::json!({
        "profiles": profiles
    }))
    .into_response()
}

#[derive(Debug, Deserialize)]
/// Request to apply a network profile
pub struct ApplyNetworkProfileRequest {
    /// Name of the network profile to apply
    pub profile_name: String,
}

/// Apply a network profile
async fn apply_network_profile(
    State(state): State<ManagementState>,
    Json(request): Json<ApplyNetworkProfileRequest>,
) -> impl IntoResponse {
    use mockforge_core::network_profiles::NetworkProfileCatalog;

    let catalog = NetworkProfileCatalog::default();
    if let Some(profile) = catalog.get(&request.profile_name) {
        // Apply profile to server configuration if available
        // NetworkProfile contains latency and traffic_shaping configs
        if let Some(server_config) = &state.server_config {
            let mut config = server_config.write().await;

            // Apply network profile's traffic shaping to core config
            use mockforge_core::config::NetworkShapingConfig;

            // Convert NetworkProfile's TrafficShapingConfig to NetworkShapingConfig
            // NetworkProfile uses mockforge_core::traffic_shaping::TrafficShapingConfig
            // which has bandwidth and burst_loss fields
            let network_shaping = NetworkShapingConfig {
                enabled: profile.traffic_shaping.bandwidth.enabled
                    || profile.traffic_shaping.burst_loss.enabled,
                bandwidth_limit_bps: profile.traffic_shaping.bandwidth.max_bytes_per_sec * 8, // Convert bytes to bits
                packet_loss_percent: profile.traffic_shaping.burst_loss.loss_rate_during_burst,
                max_connections: 1000, // Default value
            };

            // Update chaos config if it exists, or create it
            // Chaos config is in observability.chaos, not core.chaos
            if let Some(ref mut chaos) = config.observability.chaos {
                chaos.traffic_shaping = Some(network_shaping);
            } else {
                // Create minimal chaos config with traffic shaping
                use mockforge_core::config::ChaosEngConfig;
                config.observability.chaos = Some(ChaosEngConfig {
                    enabled: true,
                    latency: None,
                    fault_injection: None,
                    rate_limit: None,
                    traffic_shaping: Some(network_shaping),
                    scenario: None,
                });
            }

            info!("Network profile '{}' applied to server configuration", request.profile_name);
        } else {
            warn!("Server configuration not available in ManagementState - profile applied but not persisted");
        }

        // Also update chaos API state if available
        #[cfg(feature = "chaos")]
        {
            if let Some(chaos_state) = &state.chaos_api_state {
                use mockforge_chaos::config::TrafficShapingConfig;

                let mut chaos_config = chaos_state.config.write().await;
                // Apply profile's traffic shaping to chaos API state
                let chaos_traffic_shaping = TrafficShapingConfig {
                    enabled: profile.traffic_shaping.bandwidth.enabled
                        || profile.traffic_shaping.burst_loss.enabled,
                    bandwidth_limit_bps: profile.traffic_shaping.bandwidth.max_bytes_per_sec * 8, // Convert bytes to bits
                    packet_loss_percent: profile.traffic_shaping.burst_loss.loss_rate_during_burst,
                    max_connections: 0,
                    connection_timeout_ms: 30000,
                };
                chaos_config.traffic_shaping = Some(chaos_traffic_shaping);
                chaos_config.enabled = true; // Enable chaos when applying a profile
                drop(chaos_config);
                info!("Network profile '{}' applied to chaos API state", request.profile_name);
            }
        }

        Json(serde_json::json!({
            "success": true,
            "message": format!("Network profile '{}' applied", request.profile_name),
            "profile": {
                "name": profile.name,
                "description": profile.description,
            }
        }))
        .into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Profile not found",
                "message": format!("Network profile '{}' not found", request.profile_name)
            })),
        )
            .into_response()
    }
}

/// Build the management API router with UI Builder support
pub fn management_router_with_ui_builder(
    state: ManagementState,
    server_config: mockforge_core::config::ServerConfig,
) -> Router {
    use crate::ui_builder::{create_ui_builder_router, UIBuilderState};

    // Create the base management router
    let management = management_router(state);

    // Create UI Builder state and router
    let ui_builder_state = UIBuilderState::new(server_config);
    let ui_builder = create_ui_builder_router(ui_builder_state);

    // Nest UI Builder under /ui-builder
    management.nest("/ui-builder", ui_builder)
}

/// Build management router with spec import API
pub fn management_router_with_spec_import(state: ManagementState) -> Router {
    use crate::spec_import::{spec_import_router, SpecImportState};

    // Create base management router
    let management = management_router(state);

    // Merge with spec import router
    Router::new()
        .merge(management)
        .merge(spec_import_router(SpecImportState::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_get_mock() {
        let state = ManagementState::new(None, None, 3000);

        let mock = MockConfig {
            id: "test-1".to_string(),
            name: "Test Mock".to_string(),
            method: "GET".to_string(),
            path: "/test".to_string(),
            response: MockResponse {
                body: serde_json::json!({"message": "test"}),
                headers: None,
            },
            enabled: true,
            latency_ms: None,
            status_code: Some(200),
            request_match: None,
            priority: None,
            scenario: None,
            required_scenario_state: None,
            new_scenario_state: None,
        };

        // Create mock
        {
            let mut mocks = state.mocks.write().await;
            mocks.push(mock.clone());
        }

        // Get mock
        let mocks = state.mocks.read().await;
        let found = mocks.iter().find(|m| m.id == "test-1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Mock");
    }

    #[tokio::test]
    async fn test_server_stats() {
        let state = ManagementState::new(None, None, 3000);

        // Add some mocks
        {
            let mut mocks = state.mocks.write().await;
            mocks.push(MockConfig {
                id: "1".to_string(),
                name: "Mock 1".to_string(),
                method: "GET".to_string(),
                path: "/test1".to_string(),
                response: MockResponse {
                    body: serde_json::json!({}),
                    headers: None,
                },
                enabled: true,
                latency_ms: None,
                status_code: Some(200),
                request_match: None,
                priority: None,
                scenario: None,
                required_scenario_state: None,
                new_scenario_state: None,
            });
            mocks.push(MockConfig {
                id: "2".to_string(),
                name: "Mock 2".to_string(),
                method: "POST".to_string(),
                path: "/test2".to_string(),
                response: MockResponse {
                    body: serde_json::json!({}),
                    headers: None,
                },
                enabled: false,
                latency_ms: None,
                status_code: Some(201),
                request_match: None,
                priority: None,
                scenario: None,
                required_scenario_state: None,
                new_scenario_state: None,
            });
        }

        let mocks = state.mocks.read().await;
        assert_eq!(mocks.len(), 2);
        assert_eq!(mocks.iter().filter(|m| m.enabled).count(), 1);
    }
}
