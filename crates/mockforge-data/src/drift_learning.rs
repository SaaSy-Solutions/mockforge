//! Drift Learning System
//!
//! This module extends the DataDriftEngine with learning capabilities that allow
//! mocks to gradually learn from recorded traffic and adapt their behavior.
//!
//! Features:
//! - Traffic pattern learning from recorded requests
//! - Persona behavior adaptation based on request patterns
//! - Configurable learning rate and sensitivity
//! - Opt-in per persona/endpoint learning

use crate::drift::{DataDriftConfig, DataDriftEngine};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Learning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    /// Enable drift learning
    #[serde(default)]
    pub enabled: bool,

    /// Learning mode
    #[serde(default)]
    pub mode: LearningMode,

    /// Learning rate (0.0 to 1.0) - how quickly mocks learn from patterns
    #[serde(default = "default_learning_rate")]
    pub sensitivity: f64,

    /// Decay rate (0.0 to 1.0) - drift resets if upstream patterns reverse
    #[serde(default = "default_decay_rate")]
    pub decay: f64,

    /// Minimum number of samples before learning starts
    #[serde(default = "default_min_samples")]
    pub min_samples: usize,

    /// Update interval for learning
    #[serde(default = "default_update_interval")]
    pub update_interval: Duration,

    /// Enable persona adaptation
    #[serde(default = "default_true")]
    pub persona_adaptation: bool,

    /// Enable traffic pattern mirroring
    #[serde(default = "default_true")]
    pub traffic_mirroring: bool,

    /// Per-endpoint opt-in learning (endpoint pattern -> enabled)
    #[serde(default)]
    pub endpoint_learning: HashMap<String, bool>,

    /// Per-persona opt-in learning (persona_id -> enabled)
    #[serde(default)]
    pub persona_learning: HashMap<String, bool>,
}

fn default_learning_rate() -> f64 {
    0.2 // 20% learning rate - conservative default
}

fn default_decay_rate() -> f64 {
    0.05 // 5% decay rate
}

fn default_min_samples() -> usize {
    10 // Need at least 10 samples before learning
}

fn default_update_interval() -> Duration {
    Duration::from_secs(60) // Update every minute
}

fn default_true() -> bool {
    true
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Opt-in by default
            mode: LearningMode::Behavioral,
            sensitivity: default_learning_rate(),
            decay: default_decay_rate(),
            min_samples: default_min_samples(),
            update_interval: default_update_interval(),
            persona_adaptation: true,
            traffic_mirroring: true,
            endpoint_learning: HashMap::new(),
            persona_learning: HashMap::new(),
        }
    }
}

/// Learning mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum LearningMode {
    /// Behavioral learning - adapts to behavior patterns
    #[default]
    Behavioral,
    /// Statistical learning - adapts to statistical patterns
    Statistical,
    /// Hybrid - combines behavioral and statistical
    Hybrid,
}

/// Drift Learning Engine
///
/// Extends DataDriftEngine with learning capabilities from recorded traffic.
pub struct DriftLearningEngine {
    /// Base drift engine
    drift_engine: DataDriftEngine,
    /// Learning configuration
    learning_config: LearningConfig,
    /// Traffic pattern learner
    traffic_learner: Option<Arc<RwLock<TrafficPatternLearner>>>,
    /// Persona behavior learner
    persona_learner: Option<Arc<RwLock<PersonaBehaviorLearner>>>,
    /// Learned patterns cache
    learned_patterns: Arc<RwLock<HashMap<String, LearnedPattern>>>,
}

/// Learned pattern from traffic analysis
#[derive(Debug, Clone)]
pub struct LearnedPattern {
    /// Pattern identifier
    pub pattern_id: String,
    /// Pattern type
    pub pattern_type: PatternType,
    /// Learned parameters
    pub parameters: HashMap<String, Value>,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Sample count used for learning
    pub sample_count: usize,
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Pattern type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternType {
    /// Latency pattern
    Latency,
    /// Error rate pattern
    ErrorRate,
    /// Request sequence pattern
    RequestSequence,
    /// Persona behavior pattern
    PersonaBehavior,
}

impl DriftLearningEngine {
    /// Create a new drift learning engine
    pub fn new(
        drift_config: DataDriftConfig,
        learning_config: LearningConfig,
    ) -> Result<Self> {
        let drift_engine = DataDriftEngine::new(drift_config)?;

        let traffic_learner = if learning_config.traffic_mirroring {
            Some(Arc::new(RwLock::new(TrafficPatternLearner::new(
                learning_config.clone(),
            )?)))
        } else {
            None
        };

        let persona_learner = if learning_config.persona_adaptation {
            Some(Arc::new(RwLock::new(PersonaBehaviorLearner::new(
                learning_config.clone(),
            )?)))
        } else {
            None
        };

        Ok(Self {
            drift_engine,
            learning_config,
            traffic_learner,
            persona_learner,
            learned_patterns: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get the base drift engine
    pub fn drift_engine(&self) -> &DataDriftEngine {
        &self.drift_engine
    }

    /// Get learning configuration
    pub fn learning_config(&self) -> &LearningConfig {
        &self.learning_config
    }

    /// Update learning configuration
    pub fn update_learning_config(&mut self, config: LearningConfig) -> Result<()> {
        self.learning_config = config;
        Ok(())
    }

    /// Get learned patterns
    pub async fn get_learned_patterns(&self) -> HashMap<String, LearnedPattern> {
        self.learned_patterns.read().await.clone()
    }

    /// Apply drift with learning
    pub async fn apply_drift_with_learning(&self, data: Value) -> Result<Value> {
        // First apply base drift
        let mut data = self.drift_engine.apply_drift(data).await?;

        // Then apply learned patterns if learning is enabled
        if !self.learning_config.enabled {
            return Ok(data);
        }

        // Apply learned patterns
        let patterns = self.learned_patterns.read().await;
        for (pattern_id, pattern) in patterns.iter() {
            // Check if pattern should be applied based on confidence and decay
            if pattern.confidence < 0.5 {
                continue; // Low confidence, skip
            }

            // Apply pattern based on type
            match pattern.pattern_type {
                PatternType::Latency => {
                    // Latency patterns are handled separately
                }
                PatternType::ErrorRate => {
                    // Error rate patterns are handled separately
                }
                PatternType::RequestSequence => {
                    // Request sequence patterns affect persona behavior
                }
                PatternType::PersonaBehavior => {
                    // Persona behavior patterns affect data generation
                    if let Some(obj) = data.as_object_mut() {
                        for (key, value) in &pattern.parameters {
                            if let Some(existing) = obj.get(key) {
                                // Blend learned value with existing value
                                let blended = self.blend_values(existing, value, pattern.confidence)?;
                                obj.insert(key.clone(), blended);
                            }
                        }
                    }
                }
            }
        }

        Ok(data)
    }

    /// Blend two values based on confidence
    fn blend_values(&self, existing: &Value, learned: &Value, confidence: f64) -> Result<Value> {
        // Simple blending: existing * (1 - confidence * sensitivity) + learned * (confidence * sensitivity)
        let weight = confidence * self.learning_config.sensitivity;

        match (existing, learned) {
            (Value::Number(existing_num), Value::Number(learned_num)) => {
                if let (Some(existing_f64), Some(learned_f64)) =
                    (existing_num.as_f64(), learned_num.as_f64())
                {
                    let blended = existing_f64 * (1.0 - weight) + learned_f64 * weight;
                    Ok(Value::from(blended))
                } else {
                    Ok(existing.clone())
                }
            }
            _ => Ok(existing.clone()), // For non-numeric, keep existing
        }
    }
}

/// Traffic Pattern Learner
///
/// Analyzes recorded traffic to detect patterns and trends.
pub struct TrafficPatternLearner {
    /// Learning configuration
    config: LearningConfig,
    /// Pattern window for analysis
    pattern_window: Duration,
    /// Detected patterns
    patterns: HashMap<String, TrafficPattern>,
}

/// Traffic pattern detected from analysis
#[derive(Debug, Clone)]
struct TrafficPattern {
    /// Pattern identifier
    pattern_id: String,
    /// Pattern type
    pattern_type: PatternType,
    /// Pattern parameters
    parameters: HashMap<String, Value>,
    /// Sample count
    sample_count: usize,
    /// First seen timestamp
    first_seen: chrono::DateTime<chrono::Utc>,
    /// Last seen timestamp
    last_seen: chrono::DateTime<chrono::Utc>,
}

impl TrafficPatternLearner {
    /// Create a new traffic pattern learner
    pub fn new(config: LearningConfig) -> Result<Self> {
        Ok(Self {
            config,
            pattern_window: Duration::from_secs(3600), // 1 hour window
            patterns: HashMap::new(),
        })
    }

    /// Analyze traffic patterns from recorded requests
    ///
    /// NOTE: This method is disabled to break circular dependencies.
    /// The recorder integration has been moved to a higher-level crate.
    pub async fn analyze_traffic_patterns(
        &mut self,
        _database: &dyn std::any::Any, // Use Any to avoid dependency on mockforge-recorder
    ) -> Result<Vec<LearnedPattern>> {
        // Disabled to break circular dependency
        Ok(Vec::new())
    }

    /// Internal method to detect latency patterns from requests
    ///
    /// NOTE: This method is disabled to break circular dependency.
    /// The recorder integration has been moved to a higher-level crate.
    #[allow(dead_code)]
    pub async fn detect_latency_patterns_from_requests(
        &self,
        _requests: &[serde_json::Value],
    ) -> Result<Vec<LearnedPattern>> {
        // Disabled to break circular dependency
        Ok(Vec::new())
    }

    // NOTE: The following code is disabled to break circular dependency
    // Original implementation would process requests here
    /*
    fn _detect_latency_patterns_original(
        &self,
        requests: &[serde_json::Value],
    ) -> Result<Vec<LearnedPattern>> {
        use std::collections::HashMap;
        use chrono::Utc;

        // Group requests by endpoint/method
        let mut endpoint_latencies: HashMap<String, Vec<i64>> = HashMap::new();

        for request in requests {
            if let Some(duration) = request.duration_ms {
                let key = format!("{} {}", request.method, request.path);
                endpoint_latencies.entry(key).or_insert_with(Vec::new).push(duration);
            }
        }

        let mut patterns = Vec::new();

        for (endpoint_key, latencies) in endpoint_latencies {
            if latencies.len() < 10 {
                // Need at least 10 samples for meaningful analysis
                continue;
            }

            // Calculate statistics
            let sum: i64 = latencies.iter().sum();
            let count = latencies.len();
            let avg_latency = sum as f64 / count as f64;

            // Calculate percentiles
            let mut sorted = latencies.clone();
            sorted.sort();
            let p50 = sorted[sorted.len() / 2];
            let p95 = sorted[(sorted.len() * 95) / 100];
            let p99 = sorted[(sorted.len() * 99) / 100];

            // Detect if latency is increasing (trend analysis)
            let recent_avg = if latencies.len() >= 20 {
                let recent: Vec<i64> = latencies.iter().rev().take(10).copied().collect();
                let recent_sum: i64 = recent.iter().sum();
                recent_sum as f64 / recent.len() as f64
            } else {
                avg_latency
            };

            let latency_trend = if recent_avg > avg_latency * 1.2 {
                "increasing"
            } else if recent_avg < avg_latency * 0.8 {
                "decreasing"
            } else {
                "stable"
            };

            // Create pattern if there's significant variation or trend
            if p99 > p50 * 2.0 || latency_trend != "stable" {
                let mut parameters = HashMap::new();
                parameters.insert("endpoint".to_string(), serde_json::json!(endpoint_key));
                parameters.insert("avg_latency_ms".to_string(), serde_json::json!(avg_latency));
                parameters.insert("p50_ms".to_string(), serde_json::json!(p50));
                parameters.insert("p95_ms".to_string(), serde_json::json!(p95));
                parameters.insert("p99_ms".to_string(), serde_json::json!(p99));
                parameters.insert("sample_count".to_string(), serde_json::json!(count));
                parameters.insert("trend".to_string(), serde_json::json!(latency_trend));

                // Confidence based on sample size
                let confidence = (count as f64 / 100.0).min(1.0);

                patterns.push(LearnedPattern {
                    pattern_id: format!("latency_{}", endpoint_key.replace('/', "_").replace(' ', "_")),
                    pattern_type: PatternType::Latency,
                    parameters,
                    confidence,
                    sample_count: count,
                    last_updated: Utc::now(),
                });
            }
        }

        Ok(patterns)
    }
    */

    /// Internal method to detect error rate patterns from requests
    /// NOTE: Disabled to break circular dependency
    #[allow(dead_code)]
    async fn detect_error_patterns_internal(
        &self,
        _requests: &[serde_json::Value],
    ) -> Result<Vec<LearnedPattern>> {
        use std::collections::HashMap;
        use chrono::Utc;

        // Disabled to break circular dependency
        let _requests = _requests;
        let mut endpoint_errors: HashMap<String, (usize, usize)> = HashMap::new(); // (total, errors)

        // Disabled - would iterate over requests here
        /*
        for request in requests {
            let key = format!("{} {}", request.method, request.path);
            let entry = endpoint_errors.entry(key).or_insert((0, 0));
            entry.0 += 1;

            // Consider 4xx and 5xx as errors
            if let Some(status) = request.status_code {
                if status >= 400 {
                    entry.1 += 1;
                }
            }
        }
        */

        let mut patterns = Vec::new();

        for (endpoint_key, (total, errors)) in endpoint_errors {
            if total < 20 {
                // Need at least 20 samples for meaningful analysis
                continue;
            }

            let error_rate = errors as f64 / total as f64;

            // Create pattern if error rate is significant (>5%) or increasing
            if error_rate > 0.05 {
                let mut parameters = HashMap::new();
                parameters.insert("endpoint".to_string(), serde_json::json!(endpoint_key));
                parameters.insert("error_rate".to_string(), serde_json::json!(error_rate));
                parameters.insert("total_requests".to_string(), serde_json::json!(total));
                parameters.insert("error_count".to_string(), serde_json::json!(errors));

                // Confidence based on sample size and error rate
                let confidence = ((total as f64 / 100.0).min(1.0) * error_rate * 10.0).min(1.0);

                patterns.push(LearnedPattern {
                    pattern_id: format!("error_rate_{}", endpoint_key.replace('/', "_").replace(' ', "_")),
                    pattern_type: PatternType::ErrorRate,
                    parameters,
                    confidence,
                    sample_count: total,
                    last_updated: Utc::now(),
                });
            }
        }

        Ok(patterns)
    }

    /// Internal method to detect request sequence patterns
    /// NOTE: Disabled to break circular dependency
    #[allow(dead_code)]
    async fn detect_sequence_patterns_internal(
        &self,
        _requests: &[serde_json::Value],
    ) -> Result<Vec<LearnedPattern>> {
        use std::collections::HashMap;
        use chrono::Utc;

        // Disabled to break circular dependency
        let _requests = _requests;
        if _requests.len() < 50 {
            // Need sufficient data for sequence detection
            return Ok(Vec::new());
        }

        // Disabled - would process requests here
        let mut trace_sequences: HashMap<Option<String>, Vec<String>> = HashMap::new();

        /*
        for request in requests {
            let trace_id = request.trace_id.clone();
            let endpoint_key = format!("{} {}", request.method, request.path);
            trace_sequences
                .entry(trace_id)
                .or_insert_with(Vec::new)
                .push(endpoint_key);
        }
        */

        // Find common sequences (patterns that appear multiple times)
        let mut sequence_counts: HashMap<String, usize> = HashMap::new();

        for sequence in trace_sequences.values() {
            if sequence.len() >= 2 {
                // Create sequence signature (first 3 endpoints)
                let signature: Vec<String> = sequence.iter().take(3).cloned().collect();
                let signature_str = signature.join(" -> ");
                *sequence_counts.entry(signature_str).or_insert(0) += 1;
            }
        }

        let mut patterns = Vec::new();

        for (sequence_str, count) in sequence_counts {
            if count >= 5 {
                // Pattern appears at least 5 times
                let mut parameters = HashMap::new();
                parameters.insert("sequence".to_string(), serde_json::json!(sequence_str));
                parameters.insert("occurrence_count".to_string(), serde_json::json!(count));

                // Confidence based on occurrence frequency
                let confidence = (count as f64 / 20.0).min(1.0);

                patterns.push(LearnedPattern {
                    pattern_id: format!("sequence_{}", sequence_str.replace('/', "_").replace(' ', "_").replace("->", "_")),
                    pattern_type: PatternType::RequestSequence,
                    parameters,
                    confidence,
                    sample_count: count,
                    last_updated: Utc::now(),
                });
            }
        }

        Ok(patterns)
    }

    /// Detect latency patterns
    ///
    /// This method is a convenience wrapper that requires a database.
    /// Use `analyze_traffic_patterns` with a RecorderDatabase for full analysis.
    pub async fn detect_latency_patterns(&mut self) -> Result<Vec<LearnedPattern>> {
        // This method now requires database access - use analyze_traffic_patterns instead
        Ok(Vec::new())
    }

    /// Detect error rate patterns
    ///
    /// This method is a convenience wrapper that requires a database.
    /// Use `analyze_traffic_patterns` with a RecorderDatabase for full analysis.
    pub async fn detect_error_patterns(&mut self) -> Result<Vec<LearnedPattern>> {
        // This method now requires database access - use analyze_traffic_patterns instead
        Ok(Vec::new())
    }
}

/// Persona Behavior Learner
///
/// Adapts persona profiles based on request patterns.
pub struct PersonaBehaviorLearner {
    /// Learning configuration
    config: LearningConfig,
    /// Behavior history (persona_id -> behavior events)
    behavior_history: HashMap<String, Vec<BehaviorEvent>>,
}

/// Behavior event for a persona
#[derive(Debug, Clone)]
pub struct BehaviorEvent {
    /// Event timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Event type
    pub event_type: BehaviorEventType,
    /// Event data
    pub data: HashMap<String, Value>,
}

/// Behavior event type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BehaviorEventType {
    /// Request made to an endpoint
    Request {
        /// Endpoint path
        endpoint: String,
        /// HTTP method
        method: String,
    },
    /// Request failed
    RequestFailed {
        /// Endpoint path
        endpoint: String,
        /// HTTP status code
        status_code: u16,
    },
    /// Request succeeded after failure
    RequestSucceededAfterFailure {
        /// Endpoint path
        endpoint: String,
    },
    /// Pattern detected
    PatternDetected {
        /// Pattern identifier
        pattern: String,
    },
}

impl PersonaBehaviorLearner {
    /// Create a new persona behavior learner
    pub fn new(config: LearningConfig) -> Result<Self> {
        Ok(Self {
            config,
            behavior_history: HashMap::new(),
        })
    }

    /// Record a behavior event for a persona
    pub fn record_event(&mut self, persona_id: String, event: BehaviorEvent) {
        if !self.config.enabled {
            return;
        }

        // Check if persona learning is enabled for this persona
        if let Some(&enabled) = self.config.persona_learning.get(&persona_id) {
            if !enabled {
                return; // Learning disabled for this persona
            }
        }

        let events = self.behavior_history.entry(persona_id).or_insert_with(Vec::new);
        events.push(event);

        // Keep only recent events (last 1000)
        if events.len() > 1000 {
            events.remove(0);
        }
    }

    /// Analyze behavior patterns for a persona
    pub async fn analyze_persona_behavior(
        &self,
        persona_id: &str,
    ) -> Result<Option<LearnedPattern>> {
        if !self.config.enabled {
            return Ok(None);
        }

        let events = match self.behavior_history.get(persona_id) {
            Some(events) => events,
            None => return Ok(None),
        };

        if events.len() < self.config.min_samples {
            return Ok(None); // Not enough samples
        }

        // Analyze patterns
        // Example: If persona repeatedly requests /checkout after failure, learn this pattern
        let mut checkout_after_failure_count = 0;
        let mut total_failures = 0;

        for i in 1..events.len() {
            if let BehaviorEventType::RequestFailed { .. } = events[i - 1].event_type {
                total_failures += 1;
                if let BehaviorEventType::Request { endpoint, .. } = &events[i].event_type {
                    if endpoint.contains("/checkout") {
                        checkout_after_failure_count += 1;
                    }
                }
            }
        }

        if total_failures > 0 && checkout_after_failure_count as f64 / total_failures as f64 > 0.5 {
            // Pattern detected: persona requests /checkout after failure > 50% of the time
            let mut parameters = HashMap::new();
            parameters.insert(
                "retry_checkout_after_failure".to_string(),
                Value::from(true),
            );
            parameters.insert(
                "retry_probability".to_string(),
                Value::from(checkout_after_failure_count as f64 / total_failures as f64),
            );

            return Ok(Some(LearnedPattern {
                pattern_id: format!("persona_{}_checkout_retry", persona_id),
                pattern_type: PatternType::PersonaBehavior,
                parameters,
                confidence: (checkout_after_failure_count as f64 / total_failures as f64).min(1.0),
                sample_count: total_failures,
                last_updated: chrono::Utc::now(),
            }));
        }

        Ok(None)
    }

    /// Get behavior history for a persona
    pub fn get_behavior_history(&self, persona_id: &str) -> Option<&Vec<BehaviorEvent>> {
        self.behavior_history.get(persona_id)
    }

    /// Apply learned patterns to a persona in PersonaRegistry
    ///
    /// This method should be called periodically to update persona profiles
    /// based on learned behavior patterns.
    pub async fn apply_learned_patterns_to_persona(
        &self,
        persona_id: &str,
        persona_registry: &crate::PersonaRegistry,
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Analyze behavior for this persona
        if let Some(pattern) = self.analyze_persona_behavior(persona_id).await? {
            // Convert learned pattern parameters to traits
            let mut learned_traits = std::collections::HashMap::new();
            for (key, value) in &pattern.parameters {
                let trait_key = format!("learned_{}", key);
                let trait_value = if let Some(s) = value.as_str() {
                    s.to_string()
                } else if let Some(n) = value.as_f64() {
                    n.to_string()
                } else if let Some(b) = value.as_bool() {
                    b.to_string()
                } else {
                    value.to_string()
                };
                learned_traits.insert(trait_key, trait_value);
            }
            
            // Update persona traits in registry
            if !learned_traits.is_empty() {
                persona_registry.update_persona(persona_id, learned_traits)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_learning_config_default() {
        let config = LearningConfig::default();
        assert!(!config.enabled); // Opt-in by default
        assert_eq!(config.sensitivity, 0.2);
        assert_eq!(config.min_samples, 10);
    }

    #[test]
    fn test_drift_learning_engine_creation() {
        let drift_config = DataDriftConfig::new();
        let learning_config = LearningConfig::default();
        let engine = DriftLearningEngine::new(drift_config, learning_config);
        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_persona_behavior_learner() {
        let config = LearningConfig {
            enabled: true,
            persona_adaptation: true,
            ..Default::default()
        };
        let mut learner = PersonaBehaviorLearner::new(config).unwrap();

        // Record failure
        learner.record_event(
            "persona-1".to_string(),
            BehaviorEvent {
                timestamp: chrono::Utc::now(),
                event_type: BehaviorEventType::RequestFailed {
                    endpoint: "/api/checkout".to_string(),
                    status_code: 500,
                },
                data: HashMap::new(),
            },
        );

        // Record checkout request after failure
        learner.record_event(
            "persona-1".to_string(),
            BehaviorEvent {
                timestamp: chrono::Utc::now(),
                event_type: BehaviorEventType::Request {
                    endpoint: "/api/checkout".to_string(),
                    method: "POST".to_string(),
                },
                data: HashMap::new(),
            },
        );

        // Analyze (won't find pattern with only 2 samples, need min_samples)
        let pattern = learner.analyze_persona_behavior("persona-1").await.unwrap();
        assert!(pattern.is_none()); // Not enough samples
    }
}

