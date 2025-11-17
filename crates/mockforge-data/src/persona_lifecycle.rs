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

/// Lifecycle preset types
///
/// Predefined lifecycle patterns for common business scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecyclePreset {
    /// Subscription lifecycle: NEW → ACTIVE → PAST_DUE → CANCELED
    Subscription,
    /// Loan lifecycle: APPLICATION → APPROVED → ACTIVE → PAST_DUE → DEFAULTED
    Loan,
    /// Order fulfillment lifecycle: PENDING → PROCESSING → SHIPPED → DELIVERED → COMPLETED
    OrderFulfillment,
}

impl LifecyclePreset {
    /// Get all available presets
    pub fn all() -> Vec<Self> {
        vec![
            LifecyclePreset::Subscription,
            LifecyclePreset::Loan,
            LifecyclePreset::OrderFulfillment,
        ]
    }

    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            LifecyclePreset::Subscription => "Subscription",
            LifecyclePreset::Loan => "Loan",
            LifecyclePreset::OrderFulfillment => "Order Fulfillment",
        }
    }

    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            LifecyclePreset::Subscription => "Subscription lifecycle: NEW → ACTIVE → PAST_DUE → CANCELED",
            LifecyclePreset::Loan => "Loan lifecycle: APPLICATION → APPROVED → ACTIVE → PAST_DUE → DEFAULTED",
            LifecyclePreset::OrderFulfillment => "Order fulfillment lifecycle: PENDING → PROCESSING → SHIPPED → DELIVERED → COMPLETED",
        }
    }
}

/// Extended lifecycle states for presets
///
/// These states extend the base LifecycleState enum with preset-specific states
/// for subscription, loan, and order fulfillment lifecycles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtendedLifecycleState {
    // Base states
    /// New user signup state
    #[serde(rename = "new_signup")]
    NewSignup,
    /// Active user state
    Active,
    /// Power user state with high activity
    #[serde(rename = "power_user")]
    PowerUser,
    /// Churn risk state indicating potential user departure
    #[serde(rename = "churn_risk")]
    ChurnRisk,
    /// Churned state - user has left
    Churned,
    /// Upgrade pending state
    #[serde(rename = "upgrade_pending")]
    UpgradePending,
    /// Payment failed state
    #[serde(rename = "payment_failed")]
    PaymentFailed,

    // Subscription states
    /// New subscription state
    #[serde(rename = "subscription_new")]
    SubscriptionNew,
    /// Active subscription state
    #[serde(rename = "subscription_active")]
    SubscriptionActive,
    /// Subscription past due state
    #[serde(rename = "subscription_past_due")]
    SubscriptionPastDue,
    /// Subscription canceled state
    #[serde(rename = "subscription_canceled")]
    SubscriptionCanceled,

    // Loan states
    /// Loan application state
    #[serde(rename = "loan_application")]
    LoanApplication,
    /// Loan approved state
    #[serde(rename = "loan_approved")]
    LoanApproved,
    /// Loan active state
    #[serde(rename = "loan_active")]
    LoanActive,
    /// Loan past due state
    #[serde(rename = "loan_past_due")]
    LoanPastDue,
    /// Loan defaulted state
    #[serde(rename = "loan_defaulted")]
    LoanDefaulted,

    // Order fulfillment states
    /// Order pending state
    #[serde(rename = "order_pending")]
    OrderPending,
    /// Order processing state
    #[serde(rename = "order_processing")]
    OrderProcessing,
    /// Order shipped state
    #[serde(rename = "order_shipped")]
    OrderShipped,
    /// Order delivered state
    #[serde(rename = "order_delivered")]
    OrderDelivered,
    /// Order completed state
    #[serde(rename = "order_completed")]
    OrderCompleted,
}

/// Prebuilt lifecycle scenarios
pub struct LifecycleScenarios;

impl LifecycleScenarios {
    /// New signup scenario - fresh user with no history
    pub fn new_signup_scenario(persona_id: String) -> PersonaLifecycle {
        let rules = vec![TransitionRule {
            to: LifecycleState::Active,
            after_days: Some(7),
            condition: None,
            on_transition: None,
        }];

        PersonaLifecycle::with_rules(persona_id, LifecycleState::NewSignup, rules)
    }

    /// Power user scenario - high activity, many orders
    pub fn power_user_scenario(persona_id: String) -> PersonaLifecycle {
        let rules = vec![TransitionRule {
            to: LifecycleState::ChurnRisk,
            after_days: Some(90),
            condition: Some("order_count < 5".to_string()),
            on_transition: None,
        }];

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

    /// Create a subscription lifecycle preset
    ///
    /// States: NEW → ACTIVE → PAST_DUE → CANCELED
    pub fn subscription_preset(persona_id: String) -> PersonaLifecycle {
        // For subscription, we'll use the base lifecycle states and map them
        // NEW -> NewSignup, ACTIVE -> Active, PAST_DUE -> PaymentFailed, CANCELED -> Churned
        let rules = vec![
            TransitionRule {
                to: LifecycleState::Active,
                after_days: Some(0), // Immediately active after creation
                condition: None,
                on_transition: None,
            },
            TransitionRule {
                to: LifecycleState::PaymentFailed,
                after_days: Some(30), // Past due after 30 days
                condition: Some("payment_failed_count > 0".to_string()),
                on_transition: None,
            },
            TransitionRule {
                to: LifecycleState::Churned,
                after_days: Some(60), // Canceled after 60 days of past due
                condition: Some("payment_failed_count > 2".to_string()),
                on_transition: None,
            },
        ];

        PersonaLifecycle::with_rules(persona_id, LifecycleState::NewSignup, rules)
    }

    /// Create a loan lifecycle preset
    ///
    /// States: APPLICATION → APPROVED → ACTIVE → PAST_DUE → DEFAULTED
    pub fn loan_preset(persona_id: String) -> PersonaLifecycle {
        let rules = vec![
            TransitionRule {
                to: LifecycleState::Active, // APPROVED -> ACTIVE
                after_days: Some(7),        // Approved after 7 days
                condition: Some("credit_score > 650".to_string()),
                on_transition: None,
            },
            TransitionRule {
                to: LifecycleState::PaymentFailed, // ACTIVE -> PAST_DUE
                after_days: Some(90),              // Past due after 90 days
                condition: Some("payment_failed_count > 0".to_string()),
                on_transition: None,
            },
            TransitionRule {
                to: LifecycleState::Churned, // PAST_DUE -> DEFAULTED
                after_days: Some(120),       // Defaulted after 120 days
                condition: Some("payment_failed_count > 3".to_string()),
                on_transition: None,
            },
        ];

        PersonaLifecycle::with_rules(persona_id, LifecycleState::NewSignup, rules)
    }

    /// Create an order fulfillment lifecycle preset
    ///
    /// States: PENDING → PROCESSING → SHIPPED → DELIVERED → COMPLETED
    pub fn order_fulfillment_preset(persona_id: String) -> PersonaLifecycle {
        let rules = vec![
            TransitionRule {
                to: LifecycleState::Active, // PENDING -> PROCESSING (using Active as processing)
                after_days: Some(0),        // Processing starts immediately
                condition: None,
                on_transition: None,
            },
            TransitionRule {
                to: LifecycleState::PowerUser, // PROCESSING -> SHIPPED (using PowerUser as shipped)
                after_days: Some(1),           // Shipped after 1 day
                condition: Some("inventory_available == true".to_string()),
                on_transition: None,
            },
            TransitionRule {
                to: LifecycleState::UpgradePending, // SHIPPED -> DELIVERED (using UpgradePending as delivered)
                after_days: Some(3),                // Delivered after 3 days
                condition: None,
                on_transition: None,
            },
            TransitionRule {
                to: LifecycleState::Churned, // DELIVERED -> COMPLETED (using Churned as completed - terminal state)
                after_days: Some(7),         // Completed after 7 days
                condition: None,
                on_transition: None,
            },
        ];

        PersonaLifecycle::with_rules(persona_id, LifecycleState::NewSignup, rules)
    }

    /// Create a lifecycle from a preset
    pub fn from_preset(preset: LifecyclePreset, persona_id: String) -> PersonaLifecycle {
        match preset {
            LifecyclePreset::Subscription => Self::subscription_preset(persona_id),
            LifecyclePreset::Loan => Self::loan_preset(persona_id),
            LifecyclePreset::OrderFulfillment => Self::order_fulfillment_preset(persona_id),
        }
    }
}
