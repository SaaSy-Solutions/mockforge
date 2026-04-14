//! Core types for the Intelligent Mock Behavior system

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// `InteractionRecord` is re-exported from `mockforge_foundation::intelligent_behavior::session`.
pub use mockforge_foundation::intelligent_behavior::session_state::InteractionRecord;

/// Behavior rules that define how the mock API should behave
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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

// `SessionState` is re-exported from `mockforge_foundation::intelligent_behavior::session`.
// The foundation version uses the registered clock (see `mockforge_core::time_travel::register_with_foundation`
// which is called during core initialization).
pub use mockforge_foundation::intelligent_behavior::session_state::SessionState;

// `LlmGenerationRequest` is re-exported from `mockforge_foundation::intelligent_behavior`.
pub use mockforge_foundation::intelligent_behavior::LlmGenerationRequest;

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
