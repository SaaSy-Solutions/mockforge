//! Stateful response handler for HTTP requests
//!
//! Integrates state machines with HTTP request handling to provide dynamic responses
//! based on request history and state transitions.

use crate::{Error, Result};
use axum::http::{HeaderMap, Method, Uri};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

/// Simple state instance for tracking resource state
#[derive(Debug, Clone)]
struct StateInstance {
    /// Resource identifier
    resource_id: String,
    /// Current state
    current_state: String,
    /// Resource type
    resource_type: String,
    /// State data (key-value pairs)
    state_data: HashMap<String, Value>,
}

impl StateInstance {
    fn new(resource_id: String, resource_type: String, initial_state: String) -> Self {
        Self {
            resource_id,
            current_state: initial_state,
            resource_type,
            state_data: HashMap::new(),
        }
    }

    fn transition_to(&mut self, new_state: String) {
        self.current_state = new_state;
    }
}

/// Simple state machine manager for stateful responses
struct StateMachineManager {
    /// State instances by resource ID
    instances: Arc<RwLock<HashMap<String, StateInstance>>>,
}

impl StateMachineManager {
    fn new() -> Self {
        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn get_or_create_instance(
        &self,
        resource_id: String,
        resource_type: String,
        initial_state: String,
    ) -> Result<StateInstance> {
        let mut instances = self.instances.write().await;
        if let Some(instance) = instances.get(&resource_id) {
            Ok(instance.clone())
        } else {
            let instance = StateInstance::new(resource_id.clone(), resource_type, initial_state);
            instances.insert(resource_id, instance.clone());
            Ok(instance)
        }
    }

    async fn update_instance(&self, resource_id: String, instance: StateInstance) -> Result<()> {
        let mut instances = self.instances.write().await;
        instances.insert(resource_id, instance);
        Ok(())
    }
}

/// Configuration for stateful response handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatefulConfig {
    /// Resource ID extraction configuration
    pub resource_id_extract: ResourceIdExtract,
    /// Resource type for this endpoint
    pub resource_type: String,
    /// State-based response configurations
    pub state_responses: HashMap<String, StateResponse>,
    /// Transition triggers (method + path combinations)
    pub transitions: Vec<TransitionTrigger>,
}

/// Resource ID extraction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResourceIdExtract {
    /// Extract from path parameter (e.g., "/orders/{order_id}" -> extract "order_id")
    PathParam {
        /// Path parameter name to extract
        param: String,
    },
    /// Extract from JSONPath in request body
    JsonPath {
        /// JSONPath expression to extract the resource ID
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
    /// Use a combination of values
    Composite {
        /// List of extractors to try in order
        extractors: Vec<ResourceIdExtract>,
    },
}

/// State-based response configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateResponse {
    /// HTTP status code for this state
    pub status_code: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body template
    pub body_template: String,
    /// Content type
    pub content_type: String,
}

/// Transition trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionTrigger {
    /// HTTP method that triggers this transition (as string)
    #[serde(with = "method_serde")]
    pub method: Method,
    /// Path pattern that triggers this transition
    pub path_pattern: String,
    /// Source state
    pub from_state: String,
    /// Target state
    pub to_state: String,
    /// Optional condition (JSONPath expression)
    pub condition: Option<String>,
}

mod method_serde {
    use axum::http::Method;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(method: &Method, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        method.as_str().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Method, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Method::from_bytes(s.as_bytes()).map_err(serde::de::Error::custom)
    }
}

/// Stateful response handler
pub struct StatefulResponseHandler {
    /// State machine manager
    state_manager: Arc<StateMachineManager>,
    /// Stateful configurations by path pattern
    configs: Arc<RwLock<HashMap<String, StatefulConfig>>>,
}

impl StatefulResponseHandler {
    /// Create a new stateful response handler
    pub fn new() -> Result<Self> {
        Ok(Self {
            state_manager: Arc::new(StateMachineManager::new()),
            configs: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Add a stateful configuration for a path
    pub async fn add_config(&self, path_pattern: String, config: StatefulConfig) {
        let mut configs = self.configs.write().await;
        configs.insert(path_pattern, config);
    }

    /// Check if this handler can process the request
    pub async fn can_handle(&self, _method: &Method, path: &str) -> bool {
        let configs = self.configs.read().await;
        for (pattern, _) in configs.iter() {
            if self.path_matches(pattern, path) {
                return true;
            }
        }
        false
    }

    /// Process a request and return stateful response if applicable
    pub async fn process_request(
        &self,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
        body: Option<&[u8]>,
    ) -> Result<Option<StatefulResponse>> {
        let path = uri.path();

        // Find matching configuration
        let config = {
            let configs = self.configs.read().await;
            configs
                .iter()
                .find(|(pattern, _)| self.path_matches(pattern, path))
                .map(|(_, config)| config.clone())
        };

        let config = match config {
            Some(c) => c,
            None => return Ok(None),
        };

        // Extract resource ID
        let resource_id =
            self.extract_resource_id(&config.resource_id_extract, uri, headers, body)?;

        // Get or create state instance
        let state_instance = self
            .state_manager
            .get_or_create_instance(
                resource_id.clone(),
                config.resource_type.clone(),
                "initial".to_string(), // Default initial state
            )
            .await?;

        // Check for transition triggers
        let new_state = self
            .check_transitions(&config, method, path, &state_instance, headers, body)
            .await?;

        // Get current state (after potential transition)
        let current_state = if let Some(ref state) = new_state {
            state.clone()
        } else {
            state_instance.current_state.clone()
        };

        // Generate response based on current state
        let state_response = config.state_responses.get(&current_state).ok_or_else(|| {
            Error::generic(format!("No response configuration for state '{}'", current_state))
        })?;

        // Update state instance if transition occurred
        if let Some(ref new_state) = new_state {
            let mut updated_instance = state_instance.clone();
            updated_instance.transition_to(new_state.clone());
            self.state_manager
                .update_instance(resource_id.clone(), updated_instance)
                .await?;
        }

        Ok(Some(StatefulResponse {
            status_code: state_response.status_code,
            headers: state_response.headers.clone(),
            body: self.render_body_template(&state_response.body_template, &state_instance)?,
            content_type: state_response.content_type.clone(),
            state: current_state,
            resource_id: resource_id.clone(),
        }))
    }

    /// Extract resource ID from request
    fn extract_resource_id(
        &self,
        extract: &ResourceIdExtract,
        uri: &Uri,
        headers: &HeaderMap,
        body: Option<&[u8]>,
    ) -> Result<String> {
        let path = uri.path();
        match extract {
            ResourceIdExtract::PathParam { param } => {
                // Extract from path (e.g., "/orders/123" with pattern "/orders/{order_id}" -> "123")
                // Simple implementation: extract last segment or use regex
                let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
                if let Some(last) = segments.last() {
                    Ok(last.to_string())
                } else {
                    Err(Error::generic(format!(
                        "Could not extract path parameter '{}' from path '{}'",
                        param, path
                    )))
                }
            }
            ResourceIdExtract::Header { name } => headers
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
                .ok_or_else(|| Error::generic(format!("Header '{}' not found", name))),
            ResourceIdExtract::QueryParam { param } => {
                // Extract from query string
                uri.query()
                    .and_then(|q| {
                        url::form_urlencoded::parse(q.as_bytes())
                            .find(|(k, _)| k == param)
                            .map(|(_, v)| v.to_string())
                    })
                    .ok_or_else(|| Error::generic(format!("Query parameter '{}' not found", param)))
            }
            ResourceIdExtract::JsonPath { path: json_path } => {
                let body_str = body
                    .and_then(|b| std::str::from_utf8(b).ok())
                    .ok_or_else(|| Error::generic("Request body is not valid UTF-8".to_string()))?;

                let json: Value = serde_json::from_str(body_str)
                    .map_err(|e| Error::generic(format!("Invalid JSON body: {}", e)))?;

                // Simple JSONPath implementation (supports $.field notation)
                self.extract_json_path(&json, json_path)
            }
            ResourceIdExtract::Composite { extractors } => {
                // Try each extractor in order
                for extract in extractors {
                    if let Ok(id) = self.extract_resource_id(extract, uri, headers, body) {
                        return Ok(id);
                    }
                }
                Err(Error::generic("Could not extract resource ID from any source".to_string()))
            }
        }
    }

    /// Extract value from JSON using simple JSONPath
    fn extract_json_path(&self, json: &Value, path: &str) -> Result<String> {
        let path = path.trim_start_matches('$').trim_start_matches('.');
        let parts: Vec<&str> = path.split('.').collect();

        let mut current = json;
        for part in parts {
            match current {
                Value::Object(map) => {
                    current = map
                        .get(part)
                        .ok_or_else(|| Error::generic(format!("Path '{}' not found", path)))?;
                }
                Value::Array(arr) => {
                    let idx: usize = part
                        .parse()
                        .map_err(|_| Error::generic(format!("Invalid array index: {}", part)))?;
                    current = arr.get(idx).ok_or_else(|| {
                        Error::generic(format!("Array index {} out of bounds", idx))
                    })?;
                }
                _ => {
                    return Err(Error::generic(format!(
                        "Cannot traverse path '{}' at '{}'",
                        path, part
                    )));
                }
            }
        }

        match current {
            Value::String(s) => Ok(s.clone()),
            Value::Number(n) => Ok(n.to_string()),
            _ => {
                Err(Error::generic(format!("Path '{}' does not point to a string or number", path)))
            }
        }
    }

    /// Check for transition triggers
    async fn check_transitions(
        &self,
        config: &StatefulConfig,
        method: &Method,
        path: &str,
        instance: &StateInstance,
        headers: &HeaderMap,
        body: Option<&[u8]>,
    ) -> Result<Option<String>> {
        for transition in &config.transitions {
            // Check if method and path match
            if transition.method != *method {
                continue;
            }

            if !self.path_matches(&transition.path_pattern, path) {
                continue;
            }

            // Check if current state matches
            if instance.current_state != transition.from_state {
                continue;
            }

            // Check condition if present
            if let Some(ref condition) = transition.condition {
                if !self.evaluate_condition(condition, headers, body)? {
                    continue;
                }
            }

            // Transition matches!
            debug!(
                "State transition triggered: {} -> {} for resource {}",
                transition.from_state, transition.to_state, instance.resource_id
            );

            return Ok(Some(transition.to_state.clone()));
        }

        Ok(None)
    }

    /// Evaluate a condition expression
    fn evaluate_condition(
        &self,
        condition: &str,
        _headers: &HeaderMap,
        body: Option<&[u8]>,
    ) -> Result<bool> {
        // Simple condition evaluation (can be enhanced with Rhai later)
        // For now, support basic JSONPath expressions on body
        if condition.starts_with("$.") {
            let body_str = body
                .and_then(|b| std::str::from_utf8(b).ok())
                .ok_or_else(|| Error::generic("Request body is not valid UTF-8".to_string()))?;

            let json: Value = serde_json::from_str(body_str)
                .map_err(|e| Error::generic(format!("Invalid JSON body: {}", e)))?;

            // Extract value and check if it's truthy
            let value = self.extract_json_path(&json, condition)?;
            Ok(!value.is_empty() && value != "false" && value != "0")
        } else {
            // Default: condition is true if present
            Ok(true)
        }
    }

    /// Render body template with state data
    fn render_body_template(&self, template: &str, instance: &StateInstance) -> Result<String> {
        let mut result = template.to_string();

        // Replace {{state}} with current state
        result = result.replace("{{state}}", &instance.current_state);

        // Replace {{resource_id}} with resource ID
        result = result.replace("{{resource_id}}", &instance.resource_id);

        // Replace state data variables {{state_data.key}}
        for (key, value) in &instance.state_data {
            let placeholder = format!("{{{{state_data.{}}}}}", key);
            let value_str = match value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => serde_json::to_string(value).unwrap_or_default(),
            };
            result = result.replace(&placeholder, &value_str);
        }

        Ok(result)
    }

    /// Process a stub with state machine configuration
    ///
    /// This method extracts resource ID, manages state, and returns state information
    /// that can be used to select or modify stub responses based on current state.
    ///
    /// Returns:
    /// - `Ok(Some(StateInfo))` if state machine config exists and state was processed
    /// - `Ok(None)` if no state machine config or state processing not applicable
    /// - `Err` if there was an error processing state
    #[allow(clippy::too_many_arguments)]
    pub async fn process_stub_state(
        &self,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
        body: Option<&[u8]>,
        resource_type: &str,
        resource_id_extract: &ResourceIdExtract,
        initial_state: &str,
        transitions: Option<&[TransitionTrigger]>,
    ) -> Result<Option<StateInfo>> {
        // Extract resource ID
        let resource_id = self.extract_resource_id(resource_id_extract, uri, headers, body)?;

        // Get or create state instance
        let state_instance = self
            .state_manager
            .get_or_create_instance(
                resource_id.clone(),
                resource_type.to_string(),
                initial_state.to_string(),
            )
            .await?;

        // Check for transition triggers if provided
        let new_state = if let Some(transition_list) = transitions {
            let path = uri.path();
            // Create a temporary config-like structure for transition checking
            // We'll check transitions manually since we don't have a full StatefulConfig
            let mut transitioned_state = None;

            for transition in transition_list {
                // Check if method and path match
                if transition.method != *method {
                    continue;
                }

                if !self.path_matches(&transition.path_pattern, path) {
                    continue;
                }

                // Check if current state matches
                if state_instance.current_state != transition.from_state {
                    continue;
                }

                // Check condition if present
                if let Some(ref condition) = transition.condition {
                    if !self.evaluate_condition(condition, headers, body)? {
                        continue;
                    }
                }

                // Transition matches!
                debug!(
                    "State transition triggered in stub processing: {} -> {} for resource {}",
                    transition.from_state, transition.to_state, resource_id
                );

                transitioned_state = Some(transition.to_state.clone());
                break; // Use first matching transition
            }

            transitioned_state
        } else {
            None
        };

        // Update state if transition occurred
        let final_state = if let Some(ref new_state) = new_state {
            let mut updated_instance = state_instance.clone();
            updated_instance.transition_to(new_state.clone());
            self.state_manager
                .update_instance(resource_id.clone(), updated_instance)
                .await?;
            new_state.clone()
        } else {
            state_instance.current_state.clone()
        };

        Ok(Some(StateInfo {
            resource_id: resource_id.clone(),
            current_state: final_state,
            state_data: state_instance.state_data.clone(),
        }))
    }

    /// Update state for a resource (for use with stub transitions)
    pub async fn update_resource_state(
        &self,
        resource_id: &str,
        resource_type: &str,
        new_state: &str,
    ) -> Result<()> {
        let mut instances = self.state_manager.instances.write().await;
        if let Some(instance) = instances.get_mut(resource_id) {
            if instance.resource_type == resource_type {
                instance.transition_to(new_state.to_string());
                return Ok(());
            }
        }
        Err(Error::generic(format!(
            "Resource '{}' of type '{}' not found",
            resource_id, resource_type
        )))
    }

    /// Get current state for a resource
    pub async fn get_resource_state(
        &self,
        resource_id: &str,
        resource_type: &str,
    ) -> Result<Option<StateInfo>> {
        let instances = self.state_manager.instances.read().await;
        if let Some(instance) = instances.get(resource_id) {
            if instance.resource_type == resource_type {
                return Ok(Some(StateInfo {
                    resource_id: resource_id.to_string(),
                    current_state: instance.current_state.clone(),
                    state_data: instance.state_data.clone(),
                }));
            }
        }
        Ok(None)
    }

    /// Check if path matches pattern (simple wildcard matching)
    fn path_matches(&self, pattern: &str, path: &str) -> bool {
        // Simple pattern matching: support {param} and * wildcards
        let pattern_regex = pattern.replace("{", "(?P<").replace("}", ">[^/]+)").replace("*", ".*");
        let regex = regex::Regex::new(&format!("^{}$", pattern_regex));
        match regex {
            Ok(re) => re.is_match(path),
            Err(_) => pattern == path, // Fallback to exact match
        }
    }
}

/// State information for stub response selection
#[derive(Debug, Clone)]
pub struct StateInfo {
    /// Resource ID
    pub resource_id: String,
    /// Current state name
    pub current_state: String,
    /// State data (key-value pairs)
    pub state_data: HashMap<String, Value>,
}

/// Stateful response
#[derive(Debug, Clone)]
pub struct StatefulResponse {
    /// HTTP status code
    pub status_code: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: String,
    /// Content type
    pub content_type: String,
    /// Current state
    pub state: String,
    /// Resource ID
    pub resource_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // StateInstance tests
    // =========================================================================

    #[test]
    fn test_state_instance_new() {
        let instance =
            StateInstance::new("order-123".to_string(), "order".to_string(), "pending".to_string());
        assert_eq!(instance.resource_id, "order-123");
        assert_eq!(instance.resource_type, "order");
        assert_eq!(instance.current_state, "pending");
        assert!(instance.state_data.is_empty());
    }

    #[test]
    fn test_state_instance_transition_to() {
        let mut instance =
            StateInstance::new("order-123".to_string(), "order".to_string(), "pending".to_string());
        instance.transition_to("confirmed".to_string());
        assert_eq!(instance.current_state, "confirmed");

        instance.transition_to("shipped".to_string());
        assert_eq!(instance.current_state, "shipped");
    }

    #[test]
    fn test_state_instance_clone() {
        let instance =
            StateInstance::new("order-123".to_string(), "order".to_string(), "pending".to_string());
        let cloned = instance.clone();
        assert_eq!(cloned.resource_id, instance.resource_id);
        assert_eq!(cloned.current_state, instance.current_state);
    }

    #[test]
    fn test_state_instance_debug() {
        let instance =
            StateInstance::new("order-123".to_string(), "order".to_string(), "pending".to_string());
        let debug_str = format!("{:?}", instance);
        assert!(debug_str.contains("order-123"));
        assert!(debug_str.contains("pending"));
    }

    // =========================================================================
    // StateMachineManager tests
    // =========================================================================

    #[tokio::test]
    async fn test_state_machine_manager_new() {
        let manager = StateMachineManager::new();
        let instances = manager.instances.read().await;
        assert!(instances.is_empty());
    }

    #[tokio::test]
    async fn test_state_machine_manager_get_or_create_new() {
        let manager = StateMachineManager::new();
        let instance = manager
            .get_or_create_instance(
                "order-123".to_string(),
                "order".to_string(),
                "pending".to_string(),
            )
            .await
            .unwrap();
        assert_eq!(instance.resource_id, "order-123");
        assert_eq!(instance.current_state, "pending");
    }

    #[tokio::test]
    async fn test_state_machine_manager_get_or_create_existing() {
        let manager = StateMachineManager::new();

        // Create initial instance
        let instance1 = manager
            .get_or_create_instance(
                "order-123".to_string(),
                "order".to_string(),
                "pending".to_string(),
            )
            .await
            .unwrap();
        assert_eq!(instance1.current_state, "pending");

        // Get the same instance - should return existing with same state
        let instance2 = manager
            .get_or_create_instance(
                "order-123".to_string(),
                "order".to_string(),
                "confirmed".to_string(), // Different initial state - should be ignored
            )
            .await
            .unwrap();
        assert_eq!(instance2.current_state, "pending"); // Still pending
    }

    #[tokio::test]
    async fn test_state_machine_manager_update_instance() {
        let manager = StateMachineManager::new();

        // Create initial instance
        let mut instance = manager
            .get_or_create_instance(
                "order-123".to_string(),
                "order".to_string(),
                "pending".to_string(),
            )
            .await
            .unwrap();

        // Update state
        instance.transition_to("confirmed".to_string());
        manager.update_instance("order-123".to_string(), instance).await.unwrap();

        // Verify update
        let updated = manager
            .get_or_create_instance(
                "order-123".to_string(),
                "order".to_string(),
                "pending".to_string(),
            )
            .await
            .unwrap();
        assert_eq!(updated.current_state, "confirmed");
    }

    // =========================================================================
    // StatefulConfig tests
    // =========================================================================

    #[test]
    fn test_stateful_config_serialize_deserialize() {
        let config = StatefulConfig {
            resource_id_extract: ResourceIdExtract::PathParam {
                param: "order_id".to_string(),
            },
            resource_type: "order".to_string(),
            state_responses: {
                let mut map = HashMap::new();
                map.insert(
                    "pending".to_string(),
                    StateResponse {
                        status_code: 200,
                        headers: HashMap::new(),
                        body_template: "{\"status\": \"pending\"}".to_string(),
                        content_type: "application/json".to_string(),
                    },
                );
                map
            },
            transitions: vec![],
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: StatefulConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.resource_type, "order");
    }

    #[test]
    fn test_stateful_config_debug() {
        let config = StatefulConfig {
            resource_id_extract: ResourceIdExtract::PathParam {
                param: "order_id".to_string(),
            },
            resource_type: "order".to_string(),
            state_responses: HashMap::new(),
            transitions: vec![],
        };
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("order"));
    }

    #[test]
    fn test_stateful_config_clone() {
        let config = StatefulConfig {
            resource_id_extract: ResourceIdExtract::PathParam {
                param: "id".to_string(),
            },
            resource_type: "user".to_string(),
            state_responses: HashMap::new(),
            transitions: vec![],
        };
        let cloned = config.clone();
        assert_eq!(cloned.resource_type, "user");
    }

    // =========================================================================
    // ResourceIdExtract tests
    // =========================================================================

    #[test]
    fn test_resource_id_extract_path_param() {
        let extract = ResourceIdExtract::PathParam {
            param: "order_id".to_string(),
        };
        let json = serde_json::to_string(&extract).unwrap();
        assert!(json.contains("path_param"));
    }

    #[test]
    fn test_resource_id_extract_json_path() {
        let extract = ResourceIdExtract::JsonPath {
            path: "$.order.id".to_string(),
        };
        let json = serde_json::to_string(&extract).unwrap();
        assert!(json.contains("json_path"));
    }

    #[test]
    fn test_resource_id_extract_header() {
        let extract = ResourceIdExtract::Header {
            name: "X-Order-ID".to_string(),
        };
        let json = serde_json::to_string(&extract).unwrap();
        assert!(json.contains("header"));
    }

    #[test]
    fn test_resource_id_extract_query_param() {
        let extract = ResourceIdExtract::QueryParam {
            param: "order_id".to_string(),
        };
        let json = serde_json::to_string(&extract).unwrap();
        assert!(json.contains("query_param"));
    }

    #[test]
    fn test_resource_id_extract_composite() {
        let extract = ResourceIdExtract::Composite {
            extractors: vec![
                ResourceIdExtract::PathParam {
                    param: "id".to_string(),
                },
                ResourceIdExtract::Header {
                    name: "X-ID".to_string(),
                },
            ],
        };
        let json = serde_json::to_string(&extract).unwrap();
        assert!(json.contains("composite"));
    }

    // =========================================================================
    // StateResponse tests
    // =========================================================================

    #[test]
    fn test_state_response_serialize_deserialize() {
        let response = StateResponse {
            status_code: 200,
            headers: {
                let mut h = HashMap::new();
                h.insert("X-State".to_string(), "pending".to_string());
                h
            },
            body_template: "{\"state\": \"{{state}}\"}".to_string(),
            content_type: "application/json".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        let deserialized: StateResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.status_code, 200);
        assert_eq!(deserialized.content_type, "application/json");
    }

    #[test]
    fn test_state_response_clone() {
        let response = StateResponse {
            status_code: 201,
            headers: HashMap::new(),
            body_template: "{}".to_string(),
            content_type: "text/plain".to_string(),
        };
        let cloned = response.clone();
        assert_eq!(cloned.status_code, 201);
    }

    #[test]
    fn test_state_response_debug() {
        let response = StateResponse {
            status_code: 404,
            headers: HashMap::new(),
            body_template: "Not found".to_string(),
            content_type: "text/plain".to_string(),
        };
        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("404"));
    }

    // =========================================================================
    // TransitionTrigger tests
    // =========================================================================

    #[test]
    fn test_transition_trigger_serialize_deserialize() {
        let trigger = TransitionTrigger {
            method: Method::POST,
            path_pattern: "/orders/{id}/confirm".to_string(),
            from_state: "pending".to_string(),
            to_state: "confirmed".to_string(),
            condition: None,
        };
        let json = serde_json::to_string(&trigger).unwrap();
        let deserialized: TransitionTrigger = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.from_state, "pending");
        assert_eq!(deserialized.to_state, "confirmed");
    }

    #[test]
    fn test_transition_trigger_with_condition() {
        let trigger = TransitionTrigger {
            method: Method::POST,
            path_pattern: "/orders/{id}/ship".to_string(),
            from_state: "confirmed".to_string(),
            to_state: "shipped".to_string(),
            condition: Some("$.payment.verified".to_string()),
        };
        let json = serde_json::to_string(&trigger).unwrap();
        assert!(json.contains("payment.verified"));
    }

    #[test]
    fn test_transition_trigger_clone() {
        let trigger = TransitionTrigger {
            method: Method::DELETE,
            path_pattern: "/orders/{id}".to_string(),
            from_state: "pending".to_string(),
            to_state: "cancelled".to_string(),
            condition: None,
        };
        let cloned = trigger.clone();
        assert_eq!(cloned.method, Method::DELETE);
    }

    // =========================================================================
    // StatefulResponseHandler tests
    // =========================================================================

    #[tokio::test]
    async fn test_stateful_response_handler_new() {
        let handler = StatefulResponseHandler::new().unwrap();
        let configs = handler.configs.read().await;
        assert!(configs.is_empty());
    }

    #[tokio::test]
    async fn test_stateful_response_handler_add_config() {
        let handler = StatefulResponseHandler::new().unwrap();
        let config = StatefulConfig {
            resource_id_extract: ResourceIdExtract::PathParam {
                param: "id".to_string(),
            },
            resource_type: "order".to_string(),
            state_responses: HashMap::new(),
            transitions: vec![],
        };

        handler.add_config("/orders/{id}".to_string(), config).await;

        let configs = handler.configs.read().await;
        assert!(configs.contains_key("/orders/{id}"));
    }

    #[tokio::test]
    async fn test_stateful_response_handler_can_handle_true() {
        let handler = StatefulResponseHandler::new().unwrap();
        let config = StatefulConfig {
            resource_id_extract: ResourceIdExtract::PathParam {
                param: "id".to_string(),
            },
            resource_type: "order".to_string(),
            state_responses: HashMap::new(),
            transitions: vec![],
        };

        handler.add_config("/orders/{id}".to_string(), config).await;

        assert!(handler.can_handle(&Method::GET, "/orders/123").await);
    }

    #[tokio::test]
    async fn test_stateful_response_handler_can_handle_false() {
        let handler = StatefulResponseHandler::new().unwrap();
        assert!(!handler.can_handle(&Method::GET, "/orders/123").await);
    }

    // =========================================================================
    // Path matching tests
    // =========================================================================

    #[test]
    fn test_path_matching() {
        let handler = StatefulResponseHandler::new().unwrap();

        assert!(handler.path_matches("/orders/{id}", "/orders/123"));
        assert!(handler.path_matches("/api/*", "/api/users"));
        assert!(!handler.path_matches("/orders/{id}", "/orders/123/items"));
    }

    #[test]
    fn test_path_matching_exact() {
        let handler = StatefulResponseHandler::new().unwrap();
        assert!(handler.path_matches("/api/health", "/api/health"));
        assert!(!handler.path_matches("/api/health", "/api/health/check"));
    }

    #[test]
    fn test_path_matching_multiple_params() {
        let handler = StatefulResponseHandler::new().unwrap();
        assert!(handler.path_matches("/users/{user_id}/orders/{order_id}", "/users/1/orders/2"));
    }

    #[test]
    fn test_path_matching_wildcard() {
        let handler = StatefulResponseHandler::new().unwrap();
        assert!(handler.path_matches("/api/*", "/api/anything"));
        assert!(handler.path_matches("/api/*", "/api/users/123"));
    }

    // =========================================================================
    // Resource ID extraction tests
    // =========================================================================

    #[tokio::test]
    async fn test_extract_resource_id_from_path() {
        let handler = StatefulResponseHandler::new().unwrap();
        let extract = ResourceIdExtract::PathParam {
            param: "order_id".to_string(),
        };
        let uri: Uri = "/orders/12345".parse().unwrap();
        let headers = HeaderMap::new();

        let id = handler.extract_resource_id(&extract, &uri, &headers, None).unwrap();
        assert_eq!(id, "12345");
    }

    #[tokio::test]
    async fn test_extract_resource_id_from_header() {
        let handler = StatefulResponseHandler::new().unwrap();
        let extract = ResourceIdExtract::Header {
            name: "x-order-id".to_string(),
        };
        let uri: Uri = "/orders".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("x-order-id", "order-abc".parse().unwrap());

        let id = handler.extract_resource_id(&extract, &uri, &headers, None).unwrap();
        assert_eq!(id, "order-abc");
    }

    #[tokio::test]
    async fn test_extract_resource_id_from_query_param() {
        let handler = StatefulResponseHandler::new().unwrap();
        let extract = ResourceIdExtract::QueryParam {
            param: "id".to_string(),
        };
        let uri: Uri = "/orders?id=query-123".parse().unwrap();
        let headers = HeaderMap::new();

        let id = handler.extract_resource_id(&extract, &uri, &headers, None).unwrap();
        assert_eq!(id, "query-123");
    }

    #[tokio::test]
    async fn test_extract_resource_id_from_json_body() {
        let handler = StatefulResponseHandler::new().unwrap();
        let extract = ResourceIdExtract::JsonPath {
            path: "$.order.id".to_string(),
        };
        let uri: Uri = "/orders".parse().unwrap();
        let headers = HeaderMap::new();
        let body = br#"{"order": {"id": "json-456"}}"#;

        let id = handler.extract_resource_id(&extract, &uri, &headers, Some(body)).unwrap();
        assert_eq!(id, "json-456");
    }

    #[tokio::test]
    async fn test_extract_resource_id_composite() {
        let handler = StatefulResponseHandler::new().unwrap();
        let extract = ResourceIdExtract::Composite {
            extractors: vec![
                ResourceIdExtract::Header {
                    name: "x-id".to_string(),
                },
                ResourceIdExtract::PathParam {
                    param: "id".to_string(),
                },
            ],
        };
        let uri: Uri = "/orders/fallback-123".parse().unwrap();
        let headers = HeaderMap::new(); // No header, should fall back to path

        let id = handler.extract_resource_id(&extract, &uri, &headers, None).unwrap();
        assert_eq!(id, "fallback-123");
    }

    #[tokio::test]
    async fn test_extract_resource_id_header_not_found() {
        let handler = StatefulResponseHandler::new().unwrap();
        let extract = ResourceIdExtract::Header {
            name: "x-missing".to_string(),
        };
        let uri: Uri = "/orders".parse().unwrap();
        let headers = HeaderMap::new();

        let result = handler.extract_resource_id(&extract, &uri, &headers, None);
        assert!(result.is_err());
    }

    // =========================================================================
    // JSON path extraction tests
    // =========================================================================

    #[test]
    fn test_extract_json_path_simple() {
        let handler = StatefulResponseHandler::new().unwrap();
        let json: Value = serde_json::json!({"id": "123"});
        let result = handler.extract_json_path(&json, "$.id").unwrap();
        assert_eq!(result, "123");
    }

    #[test]
    fn test_extract_json_path_nested() {
        let handler = StatefulResponseHandler::new().unwrap();
        let json: Value = serde_json::json!({"order": {"details": {"id": "456"}}});
        let result = handler.extract_json_path(&json, "$.order.details.id").unwrap();
        assert_eq!(result, "456");
    }

    #[test]
    fn test_extract_json_path_number() {
        let handler = StatefulResponseHandler::new().unwrap();
        let json: Value = serde_json::json!({"count": 42});
        let result = handler.extract_json_path(&json, "$.count").unwrap();
        assert_eq!(result, "42");
    }

    #[test]
    fn test_extract_json_path_array_index() {
        let handler = StatefulResponseHandler::new().unwrap();
        let json: Value = serde_json::json!({"items": ["a", "b", "c"]});
        let result = handler.extract_json_path(&json, "$.items.1").unwrap();
        assert_eq!(result, "b");
    }

    #[test]
    fn test_extract_json_path_not_found() {
        let handler = StatefulResponseHandler::new().unwrap();
        let json: Value = serde_json::json!({"other": "value"});
        let result = handler.extract_json_path(&json, "$.missing");
        assert!(result.is_err());
    }

    // =========================================================================
    // Body template rendering tests
    // =========================================================================

    #[test]
    fn test_render_body_template_state() {
        let handler = StatefulResponseHandler::new().unwrap();
        let instance =
            StateInstance::new("order-123".to_string(), "order".to_string(), "pending".to_string());
        let template = r#"{"status": "{{state}}"}"#;
        let result = handler.render_body_template(template, &instance).unwrap();
        assert_eq!(result, r#"{"status": "pending"}"#);
    }

    #[test]
    fn test_render_body_template_resource_id() {
        let handler = StatefulResponseHandler::new().unwrap();
        let instance =
            StateInstance::new("order-456".to_string(), "order".to_string(), "shipped".to_string());
        let template = r#"{"id": "{{resource_id}}"}"#;
        let result = handler.render_body_template(template, &instance).unwrap();
        assert_eq!(result, r#"{"id": "order-456"}"#);
    }

    #[test]
    fn test_render_body_template_state_data() {
        let handler = StatefulResponseHandler::new().unwrap();
        let mut instance =
            StateInstance::new("order-789".to_string(), "order".to_string(), "shipped".to_string());
        instance
            .state_data
            .insert("carrier".to_string(), Value::String("FedEx".to_string()));
        let template = r#"{"carrier": "{{state_data.carrier}}"}"#;
        let result = handler.render_body_template(template, &instance).unwrap();
        assert_eq!(result, r#"{"carrier": "FedEx"}"#);
    }

    #[test]
    fn test_render_body_template_multiple_placeholders() {
        let handler = StatefulResponseHandler::new().unwrap();
        let instance = StateInstance::new(
            "order-abc".to_string(),
            "order".to_string(),
            "confirmed".to_string(),
        );
        let template = r#"{"id": "{{resource_id}}", "status": "{{state}}"}"#;
        let result = handler.render_body_template(template, &instance).unwrap();
        assert_eq!(result, r#"{"id": "order-abc", "status": "confirmed"}"#);
    }

    // =========================================================================
    // Process request tests
    // =========================================================================

    #[tokio::test]
    async fn test_process_request_no_config() {
        let handler = StatefulResponseHandler::new().unwrap();
        let uri: Uri = "/orders/123".parse().unwrap();
        let headers = HeaderMap::new();

        let result = handler.process_request(&Method::GET, &uri, &headers, None).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_process_request_with_config() {
        let handler = StatefulResponseHandler::new().unwrap();
        let mut state_responses = HashMap::new();
        state_responses.insert(
            "initial".to_string(),
            StateResponse {
                status_code: 200,
                headers: HashMap::new(),
                body_template: r#"{"state": "{{state}}", "id": "{{resource_id}}"}"#.to_string(),
                content_type: "application/json".to_string(),
            },
        );

        let config = StatefulConfig {
            resource_id_extract: ResourceIdExtract::PathParam {
                param: "id".to_string(),
            },
            resource_type: "order".to_string(),
            state_responses,
            transitions: vec![],
        };

        handler.add_config("/orders/{id}".to_string(), config).await;

        let uri: Uri = "/orders/test-123".parse().unwrap();
        let headers = HeaderMap::new();

        let result = handler.process_request(&Method::GET, &uri, &headers, None).await.unwrap();
        assert!(result.is_some());

        let response = result.unwrap();
        assert_eq!(response.status_code, 200);
        assert_eq!(response.state, "initial");
        assert_eq!(response.resource_id, "test-123");
        assert!(response.body.contains("test-123"));
    }

    #[tokio::test]
    async fn test_process_request_with_transition() {
        let handler = StatefulResponseHandler::new().unwrap();
        let mut state_responses = HashMap::new();
        state_responses.insert(
            "initial".to_string(),
            StateResponse {
                status_code: 200,
                headers: HashMap::new(),
                body_template: r#"{"state": "{{state}}"}"#.to_string(),
                content_type: "application/json".to_string(),
            },
        );
        state_responses.insert(
            "confirmed".to_string(),
            StateResponse {
                status_code: 200,
                headers: HashMap::new(),
                body_template: r#"{"state": "{{state}}"}"#.to_string(),
                content_type: "application/json".to_string(),
            },
        );

        let config = StatefulConfig {
            resource_id_extract: ResourceIdExtract::PathParam {
                param: "id".to_string(),
            },
            resource_type: "order".to_string(),
            state_responses,
            transitions: vec![TransitionTrigger {
                method: Method::POST,
                path_pattern: "/orders/{id}".to_string(),
                from_state: "initial".to_string(),
                to_state: "confirmed".to_string(),
                condition: None,
            }],
        };

        handler.add_config("/orders/{id}".to_string(), config).await;

        // First request - should be in initial state
        let uri: Uri = "/orders/order-1".parse().unwrap();
        let headers = HeaderMap::new();

        let result = handler.process_request(&Method::GET, &uri, &headers, None).await.unwrap();
        assert_eq!(result.unwrap().state, "initial");

        // POST to trigger transition
        let result = handler.process_request(&Method::POST, &uri, &headers, None).await.unwrap();
        assert_eq!(result.unwrap().state, "confirmed");

        // Subsequent GET should show confirmed state
        let result = handler.process_request(&Method::GET, &uri, &headers, None).await.unwrap();
        assert_eq!(result.unwrap().state, "confirmed");
    }

    // =========================================================================
    // Stub state processing tests
    // =========================================================================

    #[tokio::test]
    async fn test_process_stub_state() {
        let handler = StatefulResponseHandler::new().unwrap();
        let extract = ResourceIdExtract::PathParam {
            param: "id".to_string(),
        };
        let uri: Uri = "/users/user-123".parse().unwrap();
        let headers = HeaderMap::new();

        let result = handler
            .process_stub_state(
                &Method::GET,
                &uri,
                &headers,
                None,
                "user",
                &extract,
                "active",
                None,
            )
            .await
            .unwrap();

        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.resource_id, "user-123");
        assert_eq!(info.current_state, "active");
    }

    #[tokio::test]
    async fn test_process_stub_state_with_transition() {
        let handler = StatefulResponseHandler::new().unwrap();
        let extract = ResourceIdExtract::PathParam {
            param: "id".to_string(),
        };
        let uri: Uri = "/users/user-456".parse().unwrap();
        let headers = HeaderMap::new();

        // Create initial state
        let _ = handler
            .process_stub_state(
                &Method::GET,
                &uri,
                &headers,
                None,
                "user",
                &extract,
                "active",
                None,
            )
            .await
            .unwrap();

        // Now process with transition
        let transitions = vec![TransitionTrigger {
            method: Method::DELETE,
            path_pattern: "/users/{id}".to_string(),
            from_state: "active".to_string(),
            to_state: "deleted".to_string(),
            condition: None,
        }];

        let result = handler
            .process_stub_state(
                &Method::DELETE,
                &uri,
                &headers,
                None,
                "user",
                &extract,
                "active",
                Some(&transitions),
            )
            .await
            .unwrap();

        let info = result.unwrap();
        assert_eq!(info.current_state, "deleted");
    }

    // =========================================================================
    // Get/Update resource state tests
    // =========================================================================

    #[tokio::test]
    async fn test_get_resource_state_not_found() {
        let handler = StatefulResponseHandler::new().unwrap();
        let result = handler.get_resource_state("nonexistent", "order").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_resource_state_exists() {
        let handler = StatefulResponseHandler::new().unwrap();

        // Create a resource via stub processing
        let extract = ResourceIdExtract::PathParam {
            param: "id".to_string(),
        };
        let uri: Uri = "/orders/order-999".parse().unwrap();
        let headers = HeaderMap::new();

        handler
            .process_stub_state(
                &Method::GET,
                &uri,
                &headers,
                None,
                "order",
                &extract,
                "pending",
                None,
            )
            .await
            .unwrap();

        // Now get the state
        let result = handler.get_resource_state("order-999", "order").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().current_state, "pending");
    }

    #[tokio::test]
    async fn test_update_resource_state() {
        let handler = StatefulResponseHandler::new().unwrap();

        // Create a resource via stub processing
        let extract = ResourceIdExtract::PathParam {
            param: "id".to_string(),
        };
        let uri: Uri = "/orders/order-update".parse().unwrap();
        let headers = HeaderMap::new();

        handler
            .process_stub_state(
                &Method::GET,
                &uri,
                &headers,
                None,
                "order",
                &extract,
                "pending",
                None,
            )
            .await
            .unwrap();

        // Update the state
        handler.update_resource_state("order-update", "order", "shipped").await.unwrap();

        // Verify update
        let result = handler.get_resource_state("order-update", "order").await.unwrap();
        assert_eq!(result.unwrap().current_state, "shipped");
    }

    #[tokio::test]
    async fn test_update_resource_state_not_found() {
        let handler = StatefulResponseHandler::new().unwrap();
        let result = handler.update_resource_state("nonexistent", "order", "shipped").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_resource_state_wrong_type() {
        let handler = StatefulResponseHandler::new().unwrap();

        // Create a resource with type "order"
        let extract = ResourceIdExtract::PathParam {
            param: "id".to_string(),
        };
        let uri: Uri = "/orders/order-type-test".parse().unwrap();
        let headers = HeaderMap::new();

        handler
            .process_stub_state(
                &Method::GET,
                &uri,
                &headers,
                None,
                "order",
                &extract,
                "pending",
                None,
            )
            .await
            .unwrap();

        // Try to update with wrong type
        let result = handler.update_resource_state("order-type-test", "user", "active").await;
        assert!(result.is_err());
    }

    // =========================================================================
    // Condition evaluation tests
    // =========================================================================

    #[test]
    fn test_evaluate_condition_json_path_truthy() {
        let handler = StatefulResponseHandler::new().unwrap();
        let headers = HeaderMap::new();
        let body = br#"{"verified": "true"}"#;
        let result = handler.evaluate_condition("$.verified", &headers, Some(body)).unwrap();
        assert!(result);
    }

    #[test]
    fn test_evaluate_condition_json_path_falsy() {
        let handler = StatefulResponseHandler::new().unwrap();
        let headers = HeaderMap::new();
        let body = br#"{"verified": "false"}"#;
        let result = handler.evaluate_condition("$.verified", &headers, Some(body)).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_evaluate_condition_non_jsonpath() {
        let handler = StatefulResponseHandler::new().unwrap();
        let headers = HeaderMap::new();
        let result = handler.evaluate_condition("some_condition", &headers, None).unwrap();
        assert!(result); // Non-JSONPath conditions default to true
    }

    // =========================================================================
    // StateInfo tests
    // =========================================================================

    #[test]
    fn test_state_info_clone() {
        let info = StateInfo {
            resource_id: "res-1".to_string(),
            current_state: "active".to_string(),
            state_data: HashMap::new(),
        };
        let cloned = info.clone();
        assert_eq!(cloned.resource_id, "res-1");
    }

    #[test]
    fn test_state_info_debug() {
        let info = StateInfo {
            resource_id: "res-2".to_string(),
            current_state: "inactive".to_string(),
            state_data: HashMap::new(),
        };
        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("res-2"));
        assert!(debug_str.contains("inactive"));
    }

    // =========================================================================
    // StatefulResponse tests
    // =========================================================================

    #[test]
    fn test_stateful_response_clone() {
        let response = StatefulResponse {
            status_code: 200,
            headers: HashMap::new(),
            body: "{}".to_string(),
            content_type: "application/json".to_string(),
            state: "active".to_string(),
            resource_id: "res-3".to_string(),
        };
        let cloned = response.clone();
        assert_eq!(cloned.status_code, 200);
        assert_eq!(cloned.state, "active");
    }

    #[test]
    fn test_stateful_response_debug() {
        let response = StatefulResponse {
            status_code: 404,
            headers: HashMap::new(),
            body: "Not found".to_string(),
            content_type: "text/plain".to_string(),
            state: "deleted".to_string(),
            resource_id: "res-4".to_string(),
        };
        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("404"));
        assert!(debug_str.contains("deleted"));
    }
}
