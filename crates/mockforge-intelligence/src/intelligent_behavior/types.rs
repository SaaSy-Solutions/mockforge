//! Core types for the Intelligent Mock Behavior system
//!
//! Re-exported from `mockforge_foundation::intelligent_behavior` (Phase 6 / A2–A8).

// `InteractionRecord` is re-exported from `mockforge_foundation::intelligent_behavior::session_state`.
pub use mockforge_foundation::intelligent_behavior::session_state::InteractionRecord;

// `BehaviorRules` is re-exported from `mockforge_foundation::intelligent_behavior::types`.
pub use mockforge_foundation::intelligent_behavior::types::BehaviorRules;

// `SessionState` is re-exported from `mockforge_foundation::intelligent_behavior::session_state`.
pub use mockforge_foundation::intelligent_behavior::session_state::SessionState;

// `LlmGenerationRequest` is re-exported from `mockforge_foundation::intelligent_behavior`.
pub use mockforge_foundation::intelligent_behavior::LlmGenerationRequest;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_interaction_record_new() {
        let rec = InteractionRecord::new("GET", "/api/test", None, 200, Some(json!({"ok": true})));
        assert_eq!(rec.method, "GET");
        assert_eq!(rec.status, 200);
    }

    #[test]
    fn test_behavior_rules_defaults() {
        let rules = BehaviorRules::default();
        assert!(rules.consistency_rules.is_empty());
        assert!(rules.state_transitions.is_empty());
    }

    #[test]
    fn test_session_state_new() {
        let state = SessionState::new("sess-123");
        assert_eq!(state.session_id, "sess-123");
        assert!(state.history.is_empty());
    }

    #[test]
    fn test_session_state_set_get() {
        let mut state = SessionState::new("sess-123");
        state.set("key", json!("value"));
        assert_eq!(state.get("key"), Some(&json!("value")));
    }

    #[test]
    fn test_llm_request_builder() {
        let req = LlmGenerationRequest::new("sys", "user")
            .with_temperature(0.5)
            .with_max_tokens(100);
        assert_eq!(req.temperature, 0.5);
        assert_eq!(req.max_tokens, 100);
    }

    #[allow(unused)]
    fn _unused_hashmap_check() {
        let _: HashMap<String, serde_json::Value> = HashMap::new();
    }
}
