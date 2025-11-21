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

/// Action executor
///
/// Executes behavior actions. This is a trait-like structure that can be
/// extended with actual execution logic in the engine.
pub struct ActionExecutor;

impl ActionExecutor {
    /// Create a new action executor
    pub fn new() -> Self {
        Self
    }

    /// Execute an action
    ///
    /// Returns a description of what was executed (for logging/debugging)
    pub fn execute(&self, action: &BehaviorAction) -> Result<String> {
        match action {
            BehaviorAction::NoOp => Ok("No operation".to_string()),

            BehaviorAction::ModifyConversionRate { multiplier } => {
                Ok(format!("Modified conversion rate by factor {}", multiplier))
            }

            BehaviorAction::DeclineTransaction { reason } => {
                Ok(format!("Declined transaction: {}", reason))
            }

            BehaviorAction::IncreaseChurnProbability { factor } => {
                Ok(format!("Increased churn probability by factor {}", factor))
            }

            BehaviorAction::ChangeResponseStatus { status } => {
                Ok(format!("Changed response status to {}", status))
            }

            BehaviorAction::ModifyLatency { adjustment_ms } => {
                Ok(format!("Modified latency by {}ms", adjustment_ms))
            }

            BehaviorAction::TriggerChaosRule { rule_name } => {
                Ok(format!("Triggered chaos rule: {}", rule_name))
            }

            BehaviorAction::ModifyResponseBody { path, value } => {
                Ok(format!("Modified response body at {} to {}", path, value))
            }
        }
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
}

