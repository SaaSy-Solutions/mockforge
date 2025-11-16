//! Time-Aware Personas ("Life Events")
//!
//! This module provides lifecycle state management for personas that evolve over pseudo-time.
//! Supports prebuilt lifecycle scenarios (new signup, power user, churn risk) and time-based
//! state transitions.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Lifecycle state for a persona
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    /// New user signup - fresh account, no history
    NewSignup,
    /// Active user - regular usage
    Active,
    /// Power user - high activity, many orders
    PowerUser,
    /// Churn risk - declining activity, potential to leave
    ChurnRisk,
    /// Churned - user has left
    Churned,
    /// Upgrade pending - user has requested upgrade
    UpgradePending,
    /// Payment failed - payment issue detected
    PaymentFailed,
}

impl LifecycleState {
    /// Get a human-readable name for the state
    pub fn name(&self) -> &'static str {
        match self {
            LifecycleState::NewSignup => "New Signup",
            LifecycleState::Active => "Active",
            LifecycleState::PowerUser => "Power User",
            LifecycleState::ChurnRisk => "Churn Risk",
            LifecycleState::Churned => "Churned",
            LifecycleState::UpgradePending => "Upgrade Pending",
            LifecycleState::PaymentFailed => "Payment Failed",
        }
    }

    /// Check if this state is a terminal state (no further transitions)
    pub fn is_terminal(&self) -> bool {
        matches!(self, LifecycleState::Churned)
    }
}

/// Rule for transitioning between lifecycle states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionRule {
    /// Target state to transition to
    pub to: LifecycleState,
    /// Time threshold in days before transition can occur
    pub after_days: Option<u64>,
    /// Optional condition that must be met (e.g., "payment_failed_count > 2")
    pub condition: Option<String>,
    /// Optional callback to apply when transitioning
    pub on_transition: Option<String>,
}

/// Persona lifecycle manager
///
/// Manages the lifecycle state of a persona, including state transitions
/// based on pseudo-time and conditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaLifecycle {
    /// Persona ID
    pub persona_id: String,
    /// Current lifecycle state
    pub current_state: LifecycleState,
    /// History of state transitions
    pub state_history: Vec<(DateTime<Utc>, LifecycleState)>,
    /// Transition rules for this persona
    pub transition_rules: Vec<TransitionRule>,
    /// State entered at time
    pub state_entered_at: DateTime<Utc>,
    /// Additional metadata for lifecycle tracking
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl PersonaLifecycle {
    /// Create a new persona lifecycle with initial state
    pub fn new(persona_id: String, initial_state: LifecycleState) -> Self {
        let now = Utc::now();
        Self {
            persona_id,
            current_state: initial_state,
            state_history: vec![(now, initial_state)],
            transition_rules: Vec::new(),
            state_entered_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Create a new persona lifecycle with transition rules
    pub fn with_rules(
        persona_id: String,
        initial_state: LifecycleState,
        transition_rules: Vec<TransitionRule>,
    ) -> Self {
        let mut lifecycle = Self::new(persona_id, initial_state);
        lifecycle.transition_rules = transition_rules;
        lifecycle
    }

    /// Check if a transition should occur based on elapsed time
    ///
    /// Returns the target state if a transition should occur, None otherwise.
    pub fn transition_if_elapsed(
        &self,
        current_time: DateTime<Utc>,
    ) -> Option<(LifecycleState, &TransitionRule)> {
        let elapsed_days = (current_time - self.state_entered_at).num_days() as u64;

        for rule in &self.transition_rules {
            // Check if time threshold is met
            if let Some(after_days) = rule.after_days {
                if elapsed_days >= after_days {
                    // Check if condition is met (if specified)
                    if let Some(ref condition) = rule.condition {
                        if !self.evaluate_condition(condition) {
                            continue;
                        }
                    }
                    return Some((rule.to, rule));
                }
            }
        }

        None
    }

    /// Evaluate a condition string against the persona's metadata
    ///
    /// Simple condition evaluation (e.g., "payment_failed_count > 2")
    fn evaluate_condition(&self, condition: &str) -> bool {
        // Simple condition parser - supports basic comparisons
        // Format: "field operator value"
        // Operators: >, <, >=, <=, ==, !=

        let parts: Vec<&str> = condition.split_whitespace().collect();
        if parts.len() != 3 {
            return false;
        }

        let field = parts[0];
        let operator = parts[1];
        let value_str = parts[2];

        // Get field value from metadata
        let field_value = self.metadata.get(field).and_then(|v| {
            if let Some(num) = v.as_u64() {
                Some(num as i64)
            } else if let Some(num) = v.as_i64() {
                Some(num)
            } else {
                None
            }
        });

        let value = value_str.parse::<i64>().ok();

        match (field_value, value) {
            (Some(fv), Some(v)) => match operator {
                ">" => fv > v,
                "<" => fv < v,
                ">=" => fv >= v,
                "<=" => fv <= v,
                "==" => fv == v,
                "!=" => fv != v,
                _ => false,
            },
            _ => false,
        }
    }

    /// Apply lifecycle effects to persona traits
    ///
    /// Updates persona traits based on the current lifecycle state.
    pub fn apply_lifecycle_effects(&self) -> HashMap<String, String> {
        let mut traits = HashMap::new();

        match self.current_state {
            LifecycleState::NewSignup => {
                traits.insert("account_age".to_string(), "0".to_string());
                traits.insert("order_count".to_string(), "0".to_string());
                traits.insert("loyalty_level".to_string(), "bronze".to_string());
            }
            LifecycleState::Active => {
                traits.insert("loyalty_level".to_string(), "silver".to_string());
                traits.insert("engagement_level".to_string(), "medium".to_string());
            }
            LifecycleState::PowerUser => {
                traits.insert("loyalty_level".to_string(), "gold".to_string());
                traits.insert("engagement_level".to_string(), "high".to_string());
                traits.insert("order_frequency".to_string(), "high".to_string());
            }
            LifecycleState::ChurnRisk => {
                traits.insert("engagement_level".to_string(), "low".to_string());
                traits.insert("last_active_days".to_string(), "30+".to_string());
            }
            LifecycleState::Churned => {
                traits.insert("status".to_string(), "inactive".to_string());
                traits.insert("engagement_level".to_string(), "none".to_string());
            }
            LifecycleState::UpgradePending => {
                traits.insert("upgrade_status".to_string(), "pending".to_string());
            }
            LifecycleState::PaymentFailed => {
                traits.insert("payment_status".to_string(), "failed".to_string());
                traits.insert("account_status".to_string(), "restricted".to_string());
            }
        }

        traits
    }

    /// Transition to a new state
    pub fn transition_to(&mut self, new_state: LifecycleState, transition_time: DateTime<Utc>) {
        if self.current_state == new_state {
            return;
        }

        self.state_history.push((transition_time, new_state));
        self.current_state = new_state;
        self.state_entered_at = transition_time;
    }

    /// Get the duration in the current state
    pub fn current_state_duration(&self, current_time: DateTime<Utc>) -> Duration {
        current_time - self.state_entered_at
    }

    /// Add metadata for lifecycle tracking
    pub fn set_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.get(key)
    }
}

/// Prebuilt lifecycle scenarios
pub struct LifecycleScenarios;

impl LifecycleScenarios {
    /// New signup scenario - fresh user with no history
    pub fn new_signup_scenario(persona_id: String) -> PersonaLifecycle {
        let rules = vec![
            TransitionRule {
                to: LifecycleState::Active,
                after_days: Some(7),
                condition: None,
                on_transition: None,
            },
        ];

        PersonaLifecycle::with_rules(persona_id, LifecycleState::NewSignup, rules)
    }

    /// Power user scenario - high activity, many orders
    pub fn power_user_scenario(persona_id: String) -> PersonaLifecycle {
        let rules = vec![
            TransitionRule {
                to: LifecycleState::ChurnRisk,
                after_days: Some(90),
                condition: Some("order_count < 5".to_string()),
                on_transition: None,
            },
        ];

        PersonaLifecycle::with_rules(persona_id, LifecycleState::PowerUser, rules)
    }

    /// Churn risk scenario - declining activity, failed payments
    pub fn churn_risk_scenario(persona_id: String) -> PersonaLifecycle {
        let rules = vec![
            TransitionRule {
                to: LifecycleState::Churned,
                after_days: Some(30),
                condition: Some("payment_failed_count > 2".to_string()),
                on_transition: None,
            },
            TransitionRule {
                to: LifecycleState::Active,
                after_days: Some(7),
                condition: Some("payment_failed_count == 0".to_string()),
                on_transition: None,
            },
        ];

        PersonaLifecycle::with_rules(persona_id, LifecycleState::ChurnRisk, rules)
    }

    /// Active user scenario - regular usage
    pub fn active_scenario(persona_id: String) -> PersonaLifecycle {
        let rules = vec![
            TransitionRule {
                to: LifecycleState::PowerUser,
                after_days: Some(30),
                condition: Some("order_count > 10".to_string()),
                on_transition: None,
            },
            TransitionRule {
                to: LifecycleState::ChurnRisk,
                after_days: Some(60),
                condition: Some("last_active_days > 30".to_string()),
                on_transition: None,
            },
        ];

        PersonaLifecycle::with_rules(persona_id, LifecycleState::Active, rules)
    }
}
