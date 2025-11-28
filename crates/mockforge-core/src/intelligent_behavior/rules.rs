//! Consistency rules and state machines for intelligent behavior

use crate::intelligent_behavior::{sub_scenario::SubScenario, visual_layout::VisualLayout};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Consistency rule that enforces logical behavior patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ConsistencyRule {
    /// Rule name
    pub name: String,

    /// Description of what this rule does
    pub description: Option<String>,

    /// Condition for applying the rule (e.g., "path starts_with '/api/cart'")
    pub condition: String,

    /// Action to take when condition matches
    pub action: RuleAction,

    /// Priority (higher priority rules are evaluated first)
    #[serde(default)]
    pub priority: i32,
}

/// Action to take when a consistency rule matches
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RuleAction {
    /// Return an error response
    Error {
        /// HTTP status code
        status: u16,
        /// Error message
        message: String,
    },

    /// Transform the request before processing
    Transform {
        /// Description of the transformation
        description: String,
    },

    /// Execute a chain of requests
    ExecuteChain {
        /// Chain ID to execute
        chain_id: String,
    },

    /// Require authentication
    RequireAuth {
        /// Error message if not authenticated
        message: String,
    },

    /// Apply a state transition
    StateTransition {
        /// Resource type
        resource_type: String,
        /// Transition name
        transition: String,
    },
}

impl ConsistencyRule {
    /// Create a new consistency rule
    pub fn new(name: impl Into<String>, condition: impl Into<String>, action: RuleAction) -> Self {
        Self {
            name: name.into(),
            description: None,
            condition: condition.into(),
            action,
            priority: 0,
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Check if this rule's condition matches the given request
    ///
    /// This is a simplified implementation. In production, you'd want a more
    /// sophisticated condition evaluator (e.g., using a DSL or expression language).
    pub fn matches(&self, method: &str, path: &str) -> bool {
        // Simple condition parsing
        if self.condition.contains("path starts_with") {
            if let Some(prefix) = self.condition.split('\'').nth(1) {
                return path.starts_with(prefix);
            }
        }

        if self.condition.contains("method ==") {
            if let Some(expected_method) = self.condition.split('\'').nth(1) {
                return method.eq_ignore_ascii_case(expected_method);
            }
        }

        false
    }
}

/// State machine for resource lifecycle management
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StateMachine {
    /// Resource type this state machine applies to
    pub resource_type: String,

    /// All possible states
    pub states: Vec<String>,

    /// Initial state for new resources
    pub initial_state: String,

    /// Allowed transitions between states
    pub transitions: Vec<StateTransition>,

    /// Nested sub-scenarios that can be referenced from this state machine
    #[serde(default)]
    pub sub_scenarios: Vec<SubScenario>,

    /// Visual layout information for the editor (node positions, edges, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visual_layout: Option<VisualLayout>,

    /// Additional metadata for editor-specific data
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl StateMachine {
    /// Create a new state machine
    pub fn new(
        resource_type: impl Into<String>,
        states: Vec<String>,
        initial_state: impl Into<String>,
    ) -> Self {
        Self {
            resource_type: resource_type.into(),
            states,
            initial_state: initial_state.into(),
            transitions: Vec::new(),
            sub_scenarios: Vec::new(),
            visual_layout: None,
            metadata: HashMap::new(),
        }
    }

    /// Add a transition
    pub fn add_transition(mut self, transition: StateTransition) -> Self {
        self.transitions.push(transition);
        self
    }

    /// Add multiple transitions
    pub fn add_transitions(mut self, transitions: Vec<StateTransition>) -> Self {
        self.transitions.extend(transitions);
        self
    }

    /// Check if a transition is allowed
    pub fn can_transition(&self, from: &str, to: &str) -> bool {
        self.transitions.iter().any(|t| t.from_state == from && t.to_state == to)
    }

    /// Get next possible states from current state
    pub fn next_states(&self, current: &str) -> Vec<String> {
        self.transitions
            .iter()
            .filter(|t| t.from_state == current)
            .map(|t| t.to_state.clone())
            .collect()
    }

    /// Select next state based on probabilities
    pub fn select_next_state(&self, current: &str) -> Option<String> {
        let candidates: Vec<&StateTransition> =
            self.transitions.iter().filter(|t| t.from_state == current).collect();

        if candidates.is_empty() {
            return None;
        }

        // Calculate cumulative probabilities
        let total_probability: f64 = candidates.iter().map(|t| t.probability).sum();
        let mut cumulative = 0.0;
        let random = rand::random::<f64>() * total_probability;

        for transition in &candidates {
            cumulative += transition.probability;
            if random <= cumulative {
                return Some(transition.to_state.clone());
            }
        }

        // Fallback to first transition
        Some(candidates[0].to_state.clone())
    }

    /// Add a sub-scenario
    pub fn add_sub_scenario(mut self, sub_scenario: SubScenario) -> Self {
        self.sub_scenarios.push(sub_scenario);
        self
    }

    /// Set visual layout
    pub fn with_visual_layout(mut self, layout: VisualLayout) -> Self {
        self.visual_layout = Some(layout);
        self
    }

    /// Set metadata value
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Get a sub-scenario by ID
    pub fn get_sub_scenario(&self, id: &str) -> Option<&SubScenario> {
        self.sub_scenarios.iter().find(|s| s.id == id)
    }

    /// Get a sub-scenario by ID mutably
    pub fn get_sub_scenario_mut(&mut self, id: &str) -> Option<&mut SubScenario> {
        self.sub_scenarios.iter_mut().find(|s| s.id == id)
    }
}

/// State transition definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StateTransition {
    /// Source state
    #[serde(rename = "from")]
    pub from_state: String,

    /// Destination state
    #[serde(rename = "to")]
    pub to_state: String,

    /// Probability of this transition occurring (0.0 to 1.0)
    #[serde(default = "default_probability")]
    pub probability: f64,

    /// Optional condition for this transition (legacy string format)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,

    /// Optional side effects of this transition
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side_effects: Option<Vec<String>>,

    /// JavaScript/TypeScript expression for conditional transition
    ///
    /// This is the new preferred way to specify conditions. Supports full
    /// JavaScript expressions with variable access, comparison, and logical operators.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition_expression: Option<String>,

    /// Parsed condition AST for validation (not serialized, computed on demand)
    #[serde(skip)]
    pub condition_ast: Option<serde_json::Value>,

    /// Reference to a sub-scenario to execute during this transition
    ///
    /// If set, the sub-scenario will be executed when this transition is taken.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_scenario_ref: Option<String>,
}

impl StateTransition {
    /// Create a new state transition
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from_state: from.into(),
            to_state: to.into(),
            probability: default_probability(),
            condition: None,
            side_effects: None,
            condition_expression: None,
            condition_ast: None,
            sub_scenario_ref: None,
        }
    }

    /// Set probability
    pub fn with_probability(mut self, probability: f64) -> Self {
        self.probability = probability.clamp(0.0, 1.0);
        self
    }

    /// Set condition
    pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
        self.condition = Some(condition.into());
        self
    }

    /// Add side effect
    pub fn with_side_effect(mut self, effect: impl Into<String>) -> Self {
        let mut effects = self.side_effects.unwrap_or_default();
        effects.push(effect.into());
        self.side_effects = Some(effects);
        self
    }

    /// Set condition expression (JavaScript/TypeScript)
    pub fn with_condition_expression(mut self, expression: impl Into<String>) -> Self {
        self.condition_expression = Some(expression.into());
        self
    }

    /// Set sub-scenario reference
    pub fn with_sub_scenario_ref(mut self, sub_scenario_id: impl Into<String>) -> Self {
        self.sub_scenario_ref = Some(sub_scenario_id.into());
        self
    }
}

fn default_probability() -> f64 {
    1.0
}

/// Evaluation context for rules and conditions
#[derive(Debug, Clone)]
pub struct EvaluationContext {
    /// HTTP method
    pub method: String,

    /// Request path
    pub path: String,

    /// Request headers
    pub headers: HashMap<String, String>,

    /// Session state
    pub session_state: HashMap<String, serde_json::Value>,
}

impl EvaluationContext {
    /// Create a new evaluation context
    pub fn new(method: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            headers: HashMap::new(),
            session_state: HashMap::new(),
        }
    }

    /// Add headers
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }

    /// Add session state
    pub fn with_session_state(mut self, state: HashMap<String, serde_json::Value>) -> Self {
        self.session_state = state;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consistency_rule_matches() {
        let rule = ConsistencyRule::new(
            "require_auth",
            "path starts_with '/api/cart'",
            RuleAction::RequireAuth {
                message: "Authentication required".to_string(),
            },
        );

        assert!(rule.matches("GET", "/api/cart"));
        assert!(rule.matches("POST", "/api/cart/items"));
        assert!(!rule.matches("GET", "/api/products"));
    }

    #[test]
    fn test_state_machine_transitions() {
        let machine = StateMachine::new(
            "order",
            vec![
                "pending".to_string(),
                "processing".to_string(),
                "shipped".to_string(),
                "delivered".to_string(),
            ],
            "pending",
        )
        .add_transition(StateTransition::new("pending", "processing").with_probability(0.8))
        .add_transition(StateTransition::new("processing", "shipped").with_probability(0.9))
        .add_transition(StateTransition::new("shipped", "delivered").with_probability(1.0));

        assert!(machine.can_transition("pending", "processing"));
        assert!(machine.can_transition("processing", "shipped"));
        assert!(!machine.can_transition("pending", "shipped")); // No direct transition
    }

    #[test]
    fn test_state_machine_next_states() {
        let machine = StateMachine::new(
            "order",
            vec![
                "pending".to_string(),
                "processing".to_string(),
                "cancelled".to_string(),
            ],
            "pending",
        )
        .add_transition(StateTransition::new("pending", "processing"))
        .add_transition(StateTransition::new("pending", "cancelled"));

        let next = machine.next_states("pending");
        assert_eq!(next.len(), 2);
        assert!(next.contains(&"processing".to_string()));
        assert!(next.contains(&"cancelled".to_string()));
    }

    #[test]
    fn test_rule_action_serialization() {
        let action = RuleAction::Error {
            status: 401,
            message: "Unauthorized".to_string(),
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"error\""));
        assert!(json.contains("401"));

        let deserialized: RuleAction = serde_json::from_str(&json).unwrap();
        match deserialized {
            RuleAction::Error { status, message } => {
                assert_eq!(status, 401);
                assert_eq!(message, "Unauthorized");
            }
            _ => panic!("Unexpected action type"),
        }
    }

    #[test]
    fn test_state_transition_probability() {
        let transition = StateTransition::new("pending", "processing").with_probability(0.75);

        assert_eq!(transition.probability, 0.75);

        // Test clamping
        let transition_clamped = StateTransition::new("a", "b").with_probability(1.5);
        assert_eq!(transition_clamped.probability, 1.0);
    }
}
