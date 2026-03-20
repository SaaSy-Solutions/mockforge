//! Behavioral cloning type definitions
//!
//! This module defines the core data structures for behavioral cloning,
//! including sequences, probability models, and edge amplification.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A learned behavioral sequence representing a multi-step flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralSequence {
    /// Unique identifier for this sequence
    pub id: String,
    /// Human-readable name (e.g., "User Login Flow")
    pub name: String,
    /// Ordered list of steps in this sequence
    pub steps: Vec<SequenceStep>,
    /// How often this sequence occurs (0.0 to 1.0)
    pub frequency: f64,
    /// Confidence in sequence pattern (0.0 to 1.0)
    pub confidence: f64,
    /// Request IDs that contributed to learning this sequence
    pub learned_from: Vec<String>,
    /// Optional description
    pub description: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// A single step in a behavioral sequence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceStep {
    /// Endpoint path (e.g., "/api/users")
    pub endpoint: String,
    /// HTTP method (e.g., "GET", "POST")
    pub method: String,
    /// Expected delay in milliseconds before this step
    pub expected_delay_ms: Option<u64>,
    /// Conditions that must be met for this step (e.g., query params, headers)
    pub conditions: HashMap<String, String>,
    /// Probability of this step following the previous step (0.0 to 1.0)
    pub probability: f64,
    /// Optional step name/description
    pub name: Option<String>,
}

/// Probability model for an endpoint's behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointProbabilityModel {
    /// Endpoint path
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// Status code probability distribution (status_code -> probability)
    pub status_code_distribution: HashMap<u16, f64>,
    /// Latency distribution statistics
    pub latency_distribution: LatencyDistribution,
    /// Error patterns and their probabilities
    pub error_patterns: Vec<ErrorPattern>,
    /// Payload variations observed
    pub payload_variations: Vec<PayloadVariation>,
    /// Number of samples used to build this model
    pub sample_count: u64,
    /// Last update timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Original error pattern probabilities before amplification (for restoration)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_error_probabilities: Option<HashMap<String, f64>>,
}

/// Latency distribution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyDistribution {
    /// 50th percentile latency in milliseconds
    pub p50: u64,
    /// 95th percentile latency in milliseconds
    pub p95: u64,
    /// 99th percentile latency in milliseconds
    pub p99: u64,
    /// Mean latency in milliseconds
    pub mean: f64,
    /// Standard deviation in milliseconds
    pub std_dev: f64,
    /// Minimum observed latency
    pub min: u64,
    /// Maximum observed latency
    pub max: u64,
}

/// An error pattern with associated probability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPattern {
    /// Error type identifier (e.g., "timeout", "500", "400", "rate_limit")
    pub error_type: String,
    /// Probability of this error occurring (0.0 to 1.0)
    pub probability: f64,
    /// Conditions when this error occurs (e.g., specific query params, headers)
    pub conditions: Option<HashMap<String, String>>,
    /// Sample error responses observed
    pub sample_responses: Vec<serde_json::Value>,
    /// HTTP status code associated with this error (if applicable)
    pub status_code: Option<u16>,
}

/// A payload variation pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadVariation {
    /// Variation identifier
    pub id: String,
    /// Probability of this variation (0.0 to 1.0)
    pub probability: f64,
    /// Sample payload
    pub sample_payload: serde_json::Value,
    /// Conditions when this variation occurs
    pub conditions: Option<HashMap<String, String>>,
}

/// Configuration for edge case amplification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeAmplificationConfig {
    /// Whether amplification is enabled
    pub enabled: bool,
    /// Amplification factor (e.g., 0.5 = 50% frequency, was 1%)
    pub amplification_factor: f64,
    /// Scope of amplification
    pub scope: AmplificationScope,
    /// Specific patterns to amplify (if None, amplify all rare patterns)
    pub target_patterns: Option<Vec<String>>,
    /// Threshold for considering a pattern "rare" (default 0.01 = 1%)
    pub rare_threshold: f64,
}

/// Scope for edge amplification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AmplificationScope {
    /// Apply globally to all endpoints
    Global,
    /// Apply to a specific endpoint
    Endpoint {
        /// Endpoint path
        endpoint: String,
        /// HTTP method
        method: String,
    },
    /// Apply to a specific sequence
    Sequence {
        /// Sequence ID
        sequence_id: String,
    },
}

impl Default for EdgeAmplificationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            amplification_factor: 0.5,
            scope: AmplificationScope::Global,
            target_patterns: None,
            rare_threshold: 0.01, // 1%
        }
    }
}

impl BehavioralSequence {
    /// Create a new behavioral sequence
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            steps: Vec::new(),
            frequency: 0.0,
            confidence: 0.0,
            learned_from: Vec::new(),
            description: None,
            tags: Vec::new(),
        }
    }

    /// Add a step to the sequence
    pub fn add_step(mut self, step: SequenceStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Set frequency
    pub fn with_frequency(mut self, frequency: f64) -> Self {
        self.frequency = frequency.clamp(0.0, 1.0);
        self
    }

    /// Set confidence
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Add request IDs that contributed to learning
    pub fn with_learned_from(mut self, request_ids: Vec<String>) -> Self {
        self.learned_from = request_ids;
        self
    }
}

impl SequenceStep {
    /// Create a new sequence step
    pub fn new(endpoint: impl Into<String>, method: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            method: method.into(),
            expected_delay_ms: None,
            conditions: HashMap::new(),
            probability: 1.0,
            name: None,
        }
    }

    /// Set expected delay
    pub fn with_delay(mut self, delay_ms: u64) -> Self {
        self.expected_delay_ms = Some(delay_ms);
        self
    }

    /// Set probability
    pub fn with_probability(mut self, probability: f64) -> Self {
        self.probability = probability.clamp(0.0, 1.0);
        self
    }

    /// Add a condition
    pub fn with_condition(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.conditions.insert(key.into(), value.into());
        self
    }
}

impl LatencyDistribution {
    /// Create a new latency distribution
    pub fn new(p50: u64, p95: u64, p99: u64, mean: f64, std_dev: f64, min: u64, max: u64) -> Self {
        Self {
            p50,
            p95,
            p99,
            mean,
            std_dev,
            min,
            max,
        }
    }
}

impl ErrorPattern {
    /// Create a new error pattern
    pub fn new(error_type: impl Into<String>, probability: f64) -> Self {
        Self {
            error_type: error_type.into(),
            probability: probability.clamp(0.0, 1.0),
            conditions: None,
            sample_responses: Vec::new(),
            status_code: None,
        }
    }

    /// Add a sample response
    pub fn add_sample_response(mut self, response: serde_json::Value) -> Self {
        self.sample_responses.push(response);
        self
    }

    /// Set status code
    pub fn with_status_code(mut self, status_code: u16) -> Self {
        self.status_code = Some(status_code);
        self
    }
}
