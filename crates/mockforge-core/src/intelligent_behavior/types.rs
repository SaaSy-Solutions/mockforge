//! Core types for the Intelligent Mock Behavior system

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single interaction record (request + response pair)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionRecord {
    /// Timestamp of the interaction
    pub timestamp: DateTime<Utc>,

    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    pub method: String,

    /// Request path (e.g., /api/users/123)
    pub path: String,

    /// Query parameters
    #[serde(default)]
    pub query_params: HashMap<String, String>,

    /// Request headers
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Request body (if present)
    pub request: Option<serde_json::Value>,

    /// Response status code
    pub status: u16,

    /// Response body (if present)
    pub response: Option<serde_json::Value>,

    /// Vector embedding for semantic search (generated from interaction summary)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,

    /// Metadata about this interaction
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl InteractionRecord {
    /// Create a new interaction record
    pub fn new(
        method: impl Into<String>,
        path: impl Into<String>,
        request: Option<serde_json::Value>,
        status: u16,
        response: Option<serde_json::Value>,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            method: method.into(),
            path: path.into(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            request,
            status,
            response,
            embedding: None,
            metadata: HashMap::new(),
        }
    }

    /// Add query parameters
    pub fn with_query_params(mut self, params: HashMap<String, String>) -> Self {
        self.query_params = params;
        self
    }

    /// Add headers
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }

    /// Set embedding
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Generate a textual summary of this interaction for embedding
    pub fn summary(&self) -> String {
        let request_body = self
            .request
            .as_ref()
            .map(|r| serde_json::to_string(r).unwrap_or_default())
            .unwrap_or_default();

        let response_body = self
            .response
            .as_ref()
            .map(|r| serde_json::to_string(r).unwrap_or_default())
            .unwrap_or_default();

        format!(
            "{} {} | Request: {} | Status: {} | Response: {}",
            self.method, self.path, request_body, self.status, response_body
        )
    }
}

/// Behavior rules that define how the mock API should behave
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorRules {
    /// System prompt that describes the overall API behavior
    pub system_prompt: String,

    /// Resource schemas (e.g., User, Product, Order)
    /// Maps resource name to JSON Schema
    #[serde(default)]
    pub schemas: HashMap<String, serde_json::Value>,

    /// Consistency rules to enforce logical behavior
    #[serde(default)]
    pub consistency_rules: Vec<super::rules::ConsistencyRule>,

    /// State machines for resource lifecycle management
    #[serde(default)]
    pub state_transitions: HashMap<String, super::rules::StateMachine>,

    /// Maximum number of interactions to include in context
    #[serde(default = "default_max_context")]
    pub max_context_interactions: usize,

    /// Enable semantic search for relevant past interactions
    #[serde(default = "default_true")]
    pub enable_semantic_search: bool,
}

impl Default for BehaviorRules {
    fn default() -> Self {
        Self {
            system_prompt:
                "You are simulating a realistic REST API. Maintain consistency across requests."
                    .to_string(),
            schemas: HashMap::new(),
            consistency_rules: Vec::new(),
            state_transitions: HashMap::new(),
            max_context_interactions: 10,
            enable_semantic_search: true,
        }
    }
}

fn default_max_context() -> usize {
    10
}

fn default_true() -> bool {
    true
}

/// Session state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// Session ID
    pub session_id: String,

    /// Current state data (e.g., logged-in user, cart items, etc.)
    pub state: HashMap<String, serde_json::Value>,

    /// Interaction history for this session
    pub history: Vec<InteractionRecord>,

    /// Session creation time
    pub created_at: DateTime<Utc>,

    /// Last activity time
    pub last_activity: DateTime<Utc>,

    /// Session metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl SessionState {
    /// Create a new session state
    ///
    /// Automatically uses virtual clock if time travel is enabled,
    /// otherwise uses real time.
    pub fn new(session_id: impl Into<String>) -> Self {
        let now = crate::time_travel_now();
        Self {
            session_id: session_id.into(),
            state: HashMap::new(),
            history: Vec::new(),
            created_at: now,
            last_activity: now,
            metadata: HashMap::new(),
        }
    }

    /// Update last activity timestamp
    ///
    /// Automatically uses virtual clock if time travel is enabled,
    /// otherwise uses real time.
    pub fn touch(&mut self) {
        self.last_activity = crate::time_travel_now();
    }

    /// Add an interaction to history
    pub fn record_interaction(&mut self, interaction: InteractionRecord) {
        self.history.push(interaction);
        self.touch();
    }

    /// Get a value from state
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.state.get(key)
    }

    /// Set a value in state
    pub fn set(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.state.insert(key.into(), value);
        self.touch();
    }

    /// Remove a value from state
    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        let result = self.state.remove(key);
        self.touch();
        result
    }

    /// Check if session has been inactive for a duration
    ///
    /// Automatically uses virtual clock if time travel is enabled,
    /// otherwise uses real time.
    pub fn is_inactive(&self, duration: chrono::Duration) -> bool {
        crate::time_travel_now().signed_duration_since(self.last_activity) > duration
    }
}

/// LLM generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmGenerationRequest {
    /// System prompt
    pub system_prompt: String,

    /// User prompt (constructed from request context)
    pub user_prompt: String,

    /// Temperature for generation (0.0 to 2.0)
    #[serde(default = "default_temperature")]
    pub temperature: f64,

    /// Maximum tokens to generate
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,

    /// Expected response schema (JSON Schema)
    pub schema: Option<serde_json::Value>,
}

impl LlmGenerationRequest {
    /// Create a new LLM generation request
    pub fn new(system_prompt: impl Into<String>, user_prompt: impl Into<String>) -> Self {
        Self {
            system_prompt: system_prompt.into(),
            user_prompt: user_prompt.into(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            schema: None,
        }
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = temperature;
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Set expected schema
    pub fn with_schema(mut self, schema: serde_json::Value) -> Self {
        self.schema = Some(schema);
        self
    }
}

fn default_temperature() -> f64 {
    0.7
}

fn default_max_tokens() -> usize {
    1024
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_interaction_record_creation() {
        let record = InteractionRecord::new(
            "POST",
            "/api/users",
            Some(json!({"name": "Alice"})),
            201,
            Some(json!({"id": "user_1", "name": "Alice"})),
        );

        assert_eq!(record.method, "POST");
        assert_eq!(record.path, "/api/users");
        assert_eq!(record.status, 201);
        assert!(record.request.is_some());
        assert!(record.response.is_some());
    }

    #[test]
    fn test_interaction_record_summary() {
        let record = InteractionRecord::new(
            "GET",
            "/api/users/123",
            None,
            200,
            Some(json!({"id": "123", "name": "Bob"})),
        );

        let summary = record.summary();
        assert!(summary.contains("GET"));
        assert!(summary.contains("/api/users/123"));
        assert!(summary.contains("200"));
    }

    #[test]
    fn test_session_state() {
        let mut state = SessionState::new("session_123");

        // Set values
        state.set("user_id", json!("user_1"));
        state.set("logged_in", json!(true));

        // Get values
        assert_eq!(state.get("user_id"), Some(&json!("user_1")));
        assert_eq!(state.get("logged_in"), Some(&json!(true)));

        // Remove value
        let removed = state.remove("logged_in");
        assert_eq!(removed, Some(json!(true)));
        assert_eq!(state.get("logged_in"), None);
    }

    #[test]
    fn test_session_state_interaction_history() {
        let mut state = SessionState::new("session_123");

        let interaction = InteractionRecord::new(
            "POST",
            "/api/login",
            Some(json!({"email": "alice@example.com"})),
            200,
            Some(json!({"token": "abc123"})),
        );

        state.record_interaction(interaction.clone());

        assert_eq!(state.history.len(), 1);
        assert_eq!(state.history[0].method, "POST");
        assert_eq!(state.history[0].path, "/api/login");
    }

    #[test]
    fn test_behavior_rules_default() {
        let rules = BehaviorRules::default();

        assert!(!rules.system_prompt.is_empty());
        assert_eq!(rules.max_context_interactions, 10);
        assert!(rules.enable_semantic_search);
    }

    #[test]
    fn test_llm_generation_request() {
        let request = LlmGenerationRequest::new("You are a helpful API", "Generate user data")
            .with_temperature(0.8)
            .with_max_tokens(512)
            .with_schema(json!({
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "name": {"type": "string"}
                }
            }));

        assert_eq!(request.temperature, 0.8);
        assert_eq!(request.max_tokens, 512);
        assert!(request.schema.is_some());
    }
}
