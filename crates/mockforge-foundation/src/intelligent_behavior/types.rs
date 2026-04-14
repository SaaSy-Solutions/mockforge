//! Core data types for intelligent behavior
//!
//! Moved from `mockforge-core::intelligent_behavior::types` (Phase 6 / A8).
//! All dependencies (StateMachine, ConsistencyRule) are already in foundation.

use crate::state_machine::rules::{ConsistencyRule, StateMachine};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Behavior rules that define how the mock API should behave
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BehaviorRules {
    /// System prompt that describes the overall API behavior
    pub system_prompt: String,
    /// Resource schemas (e.g., User, Product, Order). Maps resource name to JSON Schema.
    #[serde(default)]
    pub schemas: HashMap<String, serde_json::Value>,
    /// Consistency rules to enforce logical behavior
    #[serde(default)]
    pub consistency_rules: Vec<ConsistencyRule>,
    /// State machines for resource lifecycle management
    #[serde(default)]
    pub state_transitions: HashMap<String, StateMachine>,
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
