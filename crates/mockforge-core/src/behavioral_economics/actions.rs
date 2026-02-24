//! Behavior action definitions and execution
//!
//! Defines actions that can be executed when behavior conditions are met.
//! Actions modify mock behavior, responses, or trigger chaos rules.

use crate::Result;
use serde::{Deserialize, Serialize};

/// Behavior action
///
/// Actions are executed when behavior conditions evaluate to true.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BehaviorAction {
    /// No operation (for testing or disabled rules)
    NoOp,

    /// Modify conversion rate
    ModifyConversionRate {
        /// Multiplier (e.g., 0.8 = 80% of original rate)
        multiplier: f64,
    },

    /// Decline transaction
    DeclineTransaction {
        /// Decline reason
        reason: String,
    },

    /// Increase churn probability
    IncreaseChurnProbability {
        /// Factor to multiply churn probability by
        factor: f64,
    },

    /// Change response status code
    ChangeResponseStatus {
        /// HTTP status code
        status: u16,
    },

    /// Modify latency
    ModifyLatency {
        /// Adjustment in milliseconds (can be negative)
        adjustment_ms: i64,
    },

    /// Trigger chaos rule
    TriggerChaosRule {
        /// Name of chaos rule to trigger
        rule_name: String,
    },

    /// Modify response body
    ModifyResponseBody {
        /// JSON path to modify
        path: String,
        /// New value (as JSON string)
        value: String,
    },
}

/// The concrete effect produced by executing an action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionEffect {
    /// No effect
    None,
    /// Multiply a rate (conversion, churn, etc.)
    RateMultiplier {
        /// Rate being modified
        target: String,
        /// Multiplier value
        multiplier: f64,
    },
    /// Decline / reject a request
    Rejection {
        /// Reason for rejection
        reason: String,
    },
    /// Override the HTTP status code
    StatusOverride {
        /// New status code
        status: u16,
    },
    /// Adjust latency by a delta
    LatencyAdjustment {
        /// Millisecond adjustment (can be negative)
        delta_ms: i64,
    },
    /// Trigger an external chaos rule
    Chaostrigger {
        /// Name of the chaos rule
        rule_name: String,
    },
    /// Patch the response body at a JSON path
    BodyPatch {
        /// JSON path to modify
        path: String,
        /// New value (serialised JSON)
        value: String,
    },
}

/// Result of executing an action, containing both a human-readable
/// description and the structured effect.
#[derive(Debug, Clone)]
pub struct ActionResult {
    /// Human-readable description of what happened
    pub description: String,
    /// Structured effect that downstream code can act on
    pub effect: ActionEffect,
}

/// Action executor
///
/// Executes behavior actions and returns structured results describing
/// the effect that should be applied to the request/response pipeline.
pub struct ActionExecutor;

impl ActionExecutor {
    /// Create a new action executor
    pub fn new() -> Self {
        Self
    }

    /// Execute an action and return a structured result
    pub fn execute_action(&self, action: &BehaviorAction) -> Result<ActionResult> {
        match action {
            BehaviorAction::NoOp => Ok(ActionResult {
                description: "No operation".to_string(),
                effect: ActionEffect::None,
            }),

            BehaviorAction::ModifyConversionRate { multiplier } => Ok(ActionResult {
                description: format!("Modified conversion rate by factor {}", multiplier),
                effect: ActionEffect::RateMultiplier {
                    target: "conversion".to_string(),
                    multiplier: *multiplier,
                },
            }),

            BehaviorAction::DeclineTransaction { reason } => Ok(ActionResult {
                description: format!("Declined transaction: {}", reason),
                effect: ActionEffect::Rejection {
                    reason: reason.clone(),
                },
            }),

            BehaviorAction::IncreaseChurnProbability { factor } => Ok(ActionResult {
                description: format!("Increased churn probability by factor {}", factor),
                effect: ActionEffect::RateMultiplier {
                    target: "churn".to_string(),
                    multiplier: *factor,
                },
            }),

            BehaviorAction::ChangeResponseStatus { status } => Ok(ActionResult {
                description: format!("Changed response status to {}", status),
                effect: ActionEffect::StatusOverride { status: *status },
            }),

            BehaviorAction::ModifyLatency { adjustment_ms } => Ok(ActionResult {
                description: format!("Modified latency by {}ms", adjustment_ms),
                effect: ActionEffect::LatencyAdjustment {
                    delta_ms: *adjustment_ms,
                },
            }),

            BehaviorAction::TriggerChaosRule { rule_name } => Ok(ActionResult {
                description: format!("Triggered chaos rule: {}", rule_name),
                effect: ActionEffect::Chaostrigger {
                    rule_name: rule_name.clone(),
                },
            }),

            BehaviorAction::ModifyResponseBody { path, value } => Ok(ActionResult {
                description: format!("Modified response body at {} to {}", path, value),
                effect: ActionEffect::BodyPatch {
                    path: path.clone(),
                    value: value.clone(),
                },
            }),
        }
    }

    /// Execute an action and return a description string
    ///
    /// This is a convenience wrapper around [`execute_action`](Self::execute_action)
    /// for callers that only need a log-friendly description.
    pub fn execute(&self, action: &BehaviorAction) -> Result<String> {
        self.execute_action(action).map(|r| r.description)
    }
}

impl Default for ActionExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_action() {
        let executor = ActionExecutor::new();
        let result = executor.execute(&BehaviorAction::NoOp).unwrap();
        assert_eq!(result, "No operation");
    }

    #[test]
    fn test_modify_conversion_rate() {
        let executor = ActionExecutor::new();
        let result = executor
            .execute(&BehaviorAction::ModifyConversionRate { multiplier: 0.8 })
            .unwrap();
        assert!(result.contains("0.8"));
    }

    #[test]
    fn test_decline_transaction() {
        let executor = ActionExecutor::new();
        let result = executor
            .execute(&BehaviorAction::DeclineTransaction {
                reason: "fraud_detected".to_string(),
            })
            .unwrap();
        assert!(result.contains("fraud_detected"));
    }

    #[test]
    fn test_execute_action_status_override() {
        let executor = ActionExecutor::new();
        let result = executor
            .execute_action(&BehaviorAction::ChangeResponseStatus { status: 503 })
            .unwrap();
        assert_eq!(result.effect, ActionEffect::StatusOverride { status: 503 });
    }

    #[test]
    fn test_execute_action_latency_adjustment() {
        let executor = ActionExecutor::new();
        let result = executor
            .execute_action(&BehaviorAction::ModifyLatency { adjustment_ms: -50 })
            .unwrap();
        assert_eq!(result.effect, ActionEffect::LatencyAdjustment { delta_ms: -50 });
    }

    #[test]
    fn test_execute_action_body_patch() {
        let executor = ActionExecutor::new();
        let result = executor
            .execute_action(&BehaviorAction::ModifyResponseBody {
                path: "$.price".to_string(),
                value: "99.99".to_string(),
            })
            .unwrap();
        assert_eq!(
            result.effect,
            ActionEffect::BodyPatch {
                path: "$.price".to_string(),
                value: "99.99".to_string(),
            }
        );
    }

    #[test]
    fn test_execute_action_churn_multiplier() {
        let executor = ActionExecutor::new();
        let result = executor
            .execute_action(&BehaviorAction::IncreaseChurnProbability { factor: 2.0 })
            .unwrap();
        assert_eq!(
            result.effect,
            ActionEffect::RateMultiplier {
                target: "churn".to_string(),
                multiplier: 2.0,
            }
        );
    }

    #[test]
    fn test_execute_action_noop() {
        let executor = ActionExecutor::new();
        let result = executor.execute_action(&BehaviorAction::NoOp).unwrap();
        assert_eq!(result.effect, ActionEffect::None);
    }
}
