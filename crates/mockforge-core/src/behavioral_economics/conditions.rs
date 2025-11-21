//! Behavior condition definitions and evaluation
//!
//! Defines conditions that can be evaluated to determine if a behavior rule
//! should trigger. Conditions can be simple (latency threshold) or composite
//! (multiple conditions with logical operators).

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Logical operator for composite conditions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum LogicalOp {
    /// All conditions must be true (AND)
    And,
    /// At least one condition must be true (OR)
    Or,
    /// All conditions must be false (NOR)
    Nor,
}

/// Behavior condition
///
/// Conditions are evaluated to determine if a behavior rule should trigger.
/// Conditions can be simple (single check) or composite (multiple conditions).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BehaviorCondition {
    /// Always true (for testing or default behavior)
    Always,

    /// Latency threshold condition
    LatencyThreshold {
        /// Endpoint pattern to match
        endpoint: String,
        /// Threshold in milliseconds
        threshold_ms: u64,
    },

    /// Load pressure condition
    LoadPressure {
        /// Threshold in requests per second
        threshold_rps: f64,
    },

    /// Pricing change condition
    PricingChange {
        /// Product ID pattern
        product_id: String,
        /// Threshold change percentage
        threshold: f64,
    },

    /// Fraud suspicion condition
    FraudSuspicion {
        /// User ID pattern
        user_id: String,
        /// Risk score threshold (0.0 to 1.0)
        risk_score: f64,
    },

    /// Customer segment condition
    CustomerSegment {
        /// Segment name
        segment: String,
    },

    /// Error rate condition
    ErrorRate {
        /// Endpoint pattern
        endpoint: String,
        /// Error rate threshold (0.0 to 1.0)
        threshold: f64,
    },

    /// Composite condition (multiple conditions with logical operator)
    Composite {
        /// Logical operator
        operator: LogicalOp,
        /// List of conditions
        conditions: Vec<BehaviorCondition>,
    },
}

/// Condition evaluator
///
/// Evaluates behavior conditions based on current system state.
pub struct ConditionEvaluator {
    /// Current latency metrics (endpoint -> latency_ms)
    latency_metrics: HashMap<String, u64>,
    /// Current load metrics (requests per second)
    load_rps: f64,
    /// Current error rates (endpoint -> error_rate)
    error_rates: HashMap<String, f64>,
    /// Current pricing data (product_id -> price)
    pricing_data: HashMap<String, f64>,
    /// Current fraud scores (user_id -> risk_score)
    fraud_scores: HashMap<String, f64>,
    /// Current customer segments (user_id -> segment)
    customer_segments: HashMap<String, String>,
}

impl ConditionEvaluator {
    /// Create a new condition evaluator
    pub fn new() -> Self {
        Self {
            latency_metrics: HashMap::new(),
            load_rps: 0.0,
            error_rates: HashMap::new(),
            pricing_data: HashMap::new(),
            fraud_scores: HashMap::new(),
            customer_segments: HashMap::new(),
        }
    }

    /// Update latency metric for an endpoint
    pub fn update_latency(&mut self, endpoint: &str, latency_ms: u64) {
        self.latency_metrics.insert(endpoint.to_string(), latency_ms);
    }

    /// Update load metric
    pub fn update_load(&mut self, rps: f64) {
        self.load_rps = rps;
    }

    /// Update error rate for an endpoint
    pub fn update_error_rate(&mut self, endpoint: &str, error_rate: f64) {
        self.error_rates.insert(endpoint.to_string(), error_rate);
    }

    /// Update pricing data
    pub fn update_pricing(&mut self, product_id: &str, price: f64) {
        self.pricing_data.insert(product_id.to_string(), price);
    }

    /// Update fraud score
    pub fn update_fraud_score(&mut self, user_id: &str, risk_score: f64) {
        self.fraud_scores.insert(user_id.to_string(), risk_score);
    }

    /// Update customer segment
    pub fn update_customer_segment(&mut self, user_id: &str, segment: String) {
        self.customer_segments.insert(user_id.to_string(), segment);
    }

    /// Evaluate a condition
    pub fn evaluate(&self, condition: &BehaviorCondition) -> Result<bool> {
        match condition {
            BehaviorCondition::Always => Ok(true),

            BehaviorCondition::LatencyThreshold { endpoint, threshold_ms } => {
                // Simple pattern matching (supports wildcards)
                let matches = self
                    .latency_metrics
                    .iter()
                    .any(|(ep, latency)| {
                        self.matches_pattern(ep, endpoint) && *latency > *threshold_ms
                    });
                Ok(matches)
            }

            BehaviorCondition::LoadPressure { threshold_rps } => {
                Ok(self.load_rps > *threshold_rps)
            }

            BehaviorCondition::PricingChange { product_id, threshold: _ } => {
                // Check if price change exceeds threshold
                // This is simplified - in practice, you'd track price history
                Ok(self.pricing_data.contains_key(product_id))
            }

            BehaviorCondition::FraudSuspicion { user_id, risk_score } => {
                let score = self.fraud_scores.get(user_id).copied().unwrap_or(0.0);
                Ok(score > *risk_score)
            }

            BehaviorCondition::CustomerSegment { segment } => {
                Ok(self
                    .customer_segments
                    .values()
                    .any(|s| s == segment))
            }

            BehaviorCondition::ErrorRate { endpoint, threshold } => {
                let matches = self
                    .error_rates
                    .iter()
                    .any(|(ep, rate)| {
                        self.matches_pattern(ep, endpoint) && *rate > *threshold
                    });
                Ok(matches)
            }

            BehaviorCondition::Composite { operator, conditions } => {
                let results: Result<Vec<bool>> = conditions
                    .iter()
                    .map(|c| self.evaluate(c))
                    .collect();
                let results = results?;

                match operator {
                    LogicalOp::And => Ok(results.iter().all(|&r| r)),
                    LogicalOp::Or => Ok(results.iter().any(|&r| r)),
                    LogicalOp::Nor => Ok(!results.iter().any(|&r| r)),
                }
            }
        }
    }

    /// Simple pattern matching (supports * wildcard)
    fn matches_pattern(&self, text: &str, pattern: &str) -> bool {
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                text.starts_with(parts[0]) && text.ends_with(parts[1])
            } else {
                text == pattern
            }
        } else {
            text == pattern
        }
    }
}

impl Default for ConditionEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_always_condition() {
        let evaluator = ConditionEvaluator::new();
        assert!(evaluator
            .evaluate(&BehaviorCondition::Always)
            .unwrap());
    }

    #[test]
    fn test_latency_threshold_condition() {
        let mut evaluator = ConditionEvaluator::new();
        evaluator.update_latency("/api/checkout", 500);
        assert!(evaluator
            .evaluate(&BehaviorCondition::LatencyThreshold {
                endpoint: "/api/checkout".to_string(),
                threshold_ms: 400,
            })
            .unwrap());
    }

    #[test]
    fn test_load_pressure_condition() {
        let mut evaluator = ConditionEvaluator::new();
        evaluator.update_load(150.0);
        assert!(evaluator
            .evaluate(&BehaviorCondition::LoadPressure { threshold_rps: 100.0 })
            .unwrap());
    }

    #[test]
    fn test_composite_and_condition() {
        let mut evaluator = ConditionEvaluator::new();
        evaluator.update_latency("/api/checkout", 500);
        evaluator.update_load(150.0);

        let condition = BehaviorCondition::Composite {
            operator: LogicalOp::And,
            conditions: vec![
                BehaviorCondition::LatencyThreshold {
                    endpoint: "/api/checkout".to_string(),
                    threshold_ms: 400,
                },
                BehaviorCondition::LoadPressure { threshold_rps: 100.0 },
            ],
        };

        assert!(evaluator.evaluate(&condition).unwrap());
    }
}

