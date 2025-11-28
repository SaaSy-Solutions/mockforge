//! Reality Profile types for hyper-realistic mock behavior
//!
//! This module defines types for reality profiles that configure latency curves,
//! error distributions, data mutation behaviors, and protocol-specific behaviors.
//! These operate at a different level than domain packs - they define how mocks
//! behave under various conditions rather than what entities and schemas exist.

use crate::error::{Result, ScenarioError};
use mockforge_core::latency::LatencyDistribution;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Latency curve configuration for protocol-specific latency distributions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyCurve {
    /// Protocol name (e.g., "mqtt", "websocket", "rest", "grpc")
    pub protocol: String,
    /// Latency distribution type
    pub distribution: LatencyDistribution,
    /// Distribution parameters (mean, std_dev, shape, etc.)
    #[serde(default)]
    pub params: HashMap<String, f64>,
    /// Base latency in milliseconds
    pub base_ms: u64,
    /// Optional endpoint patterns to filter which endpoints this applies to
    #[serde(default)]
    pub endpoint_patterns: Vec<String>,
    /// Optional jitter in milliseconds (for fixed distribution)
    #[serde(default)]
    pub jitter_ms: u64,
    /// Minimum latency bound in milliseconds
    #[serde(default)]
    pub min_ms: u64,
    /// Maximum latency bound in milliseconds (None = no limit)
    #[serde(default)]
    pub max_ms: Option<u64>,
}

/// Error distribution configuration for endpoint-specific error patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDistribution {
    /// Endpoint pattern to match (e.g., "/api/checkout/*", "POST /api/users")
    pub endpoint_pattern: String,
    /// HTTP status codes to potentially return
    pub error_codes: Vec<u16>,
    /// Probability for each error code (must match length of error_codes)
    pub probabilities: Vec<f64>,
    /// Error injection pattern (burst, random, sequential) as JSON value
    /// Can be deserialized into mockforge_chaos::config::ErrorPattern
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<Value>,
    /// Optional conditions for when to apply this error distribution
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditions: Option<ErrorCondition>,
}

/// Condition for applying error distributions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorCondition {
    /// Load threshold (requests per second) above which errors increase
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub load_threshold_rps: Option<f64>,
    /// Latency threshold (milliseconds) above which errors increase
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_threshold_ms: Option<u64>,
    /// Time window (e.g., "peak_hours", "off_peak", "weekend")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_window: Option<String>,
    /// Customer segment filter
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer_segment: Option<String>,
}

/// Data mutation behavior configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataMutationBehavior {
    /// JSON path pattern to match fields (e.g., "body.quantity", "body.status")
    pub field_pattern: String,
    /// Type of mutation to apply
    pub mutation_type: MutationType,
    /// Rate of change (per request or per time unit)
    pub rate: f64,
    /// Optional conditions for when to apply this mutation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditions: Option<MutationCondition>,
    /// Mutation parameters (strategy-specific)
    #[serde(default)]
    pub params: HashMap<String, Value>,
}

/// Type of data mutation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MutationType {
    /// Increment numeric value
    Increment,
    /// Decrement numeric value
    Decrement,
    /// State transition (for enum-like fields)
    StateTransition,
    /// Random walk within bounds
    RandomWalk,
    /// Linear drift over time
    Linear,
    /// Custom mutation using expression
    Custom,
}

/// Condition for applying data mutations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationCondition {
    /// Minimum number of requests before mutation starts
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_requests: Option<usize>,
    /// Time window for mutation (e.g., "peak_hours")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_window: Option<String>,
    /// Persona trait filter
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persona_trait: Option<String>,
}

/// Protocol-specific behavior configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolBehavior {
    /// Protocol name (e.g., "mqtt", "websocket", "rest", "grpc")
    pub protocol: String,
    /// Protocol-specific behavior configuration (JSON)
    pub behaviors: HashMap<String, Value>,
    /// Optional description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl ErrorDistribution {
    /// Validate that error_codes and probabilities have matching lengths
    pub fn validate(&self) -> Result<()> {
        if self.error_codes.len() != self.probabilities.len() {
            return Err(ScenarioError::InvalidManifest(format!(
                "Error distribution '{}' has {} error codes but {} probabilities",
                self.endpoint_pattern,
                self.error_codes.len(),
                self.probabilities.len()
            )));
        }

        // Validate probabilities sum to <= 1.0
        let total_prob: f64 = self.probabilities.iter().sum();
        if total_prob > 1.0 {
            return Err(ScenarioError::InvalidManifest(format!(
                "Error distribution '{}' has total probability {} > 1.0",
                self.endpoint_pattern, total_prob
            )));
        }

        Ok(())
    }
}

impl DataMutationBehavior {
    /// Validate mutation behavior configuration
    pub fn validate(&self) -> Result<()> {
        if self.field_pattern.is_empty() {
            return Err(ScenarioError::InvalidManifest(
                "Data mutation behavior field_pattern cannot be empty".to_string(),
            ));
        }

        if self.rate <= 0.0 {
            return Err(ScenarioError::InvalidManifest(format!(
                "Data mutation behavior '{}' has invalid rate: {}",
                self.field_pattern, self.rate
            )));
        }

        Ok(())
    }
}

impl ProtocolBehavior {
    /// Validate protocol behavior configuration
    pub fn validate(&self) -> Result<()> {
        if self.protocol.is_empty() {
            return Err(ScenarioError::InvalidManifest(
                "Protocol behavior protocol name cannot be empty".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_distribution_validation() {
        let valid = ErrorDistribution {
            endpoint_pattern: "/api/test".to_string(),
            error_codes: vec![500, 502],
            probabilities: vec![0.1, 0.05],
            pattern: None,
            conditions: None,
        };
        assert!(valid.validate().is_ok());

        let invalid_length = ErrorDistribution {
            endpoint_pattern: "/api/test".to_string(),
            error_codes: vec![500, 502],
            probabilities: vec![0.1],
            pattern: None,
            conditions: None,
        };
        assert!(invalid_length.validate().is_err());

        let invalid_prob = ErrorDistribution {
            endpoint_pattern: "/api/test".to_string(),
            error_codes: vec![500],
            probabilities: vec![1.5],
            pattern: None,
            conditions: None,
        };
        assert!(invalid_prob.validate().is_err());
    }

    #[test]
    fn test_data_mutation_validation() {
        let valid = DataMutationBehavior {
            field_pattern: "body.quantity".to_string(),
            mutation_type: MutationType::Increment,
            rate: 1.0,
            conditions: None,
            params: HashMap::new(),
        };
        assert!(valid.validate().is_ok());

        let invalid_empty = DataMutationBehavior {
            field_pattern: "".to_string(),
            mutation_type: MutationType::Increment,
            rate: 1.0,
            conditions: None,
            params: HashMap::new(),
        };
        assert!(invalid_empty.validate().is_err());

        let invalid_rate = DataMutationBehavior {
            field_pattern: "body.quantity".to_string(),
            mutation_type: MutationType::Increment,
            rate: -1.0,
            conditions: None,
            params: HashMap::new(),
        };
        assert!(invalid_rate.validate().is_err());
    }
}
