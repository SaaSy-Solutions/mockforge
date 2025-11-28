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

    #[test]
    fn test_path_matching() {
        let handler = StatefulResponseHandler::new().unwrap();

        assert!(handler.path_matches("/orders/{id}", "/orders/123"));
        assert!(handler.path_matches("/api/*", "/api/users"));
        assert!(!handler.path_matches("/orders/{id}", "/orders/123/items"));
    }
}
