//! Data drift simulation for evolving mock data
//!
//! This module provides data drift simulation capabilities, allowing mock data to
//! evolve naturally over time or across requests (e.g., order statuses progressing,
//! customer data changing).

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Drift strategy for data evolution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DriftStrategy {
    /// Linear drift - values change linearly over time
    Linear,
    /// Step-based drift - values change at discrete steps
    Stepped,
    /// State machine - values transition between defined states
    StateMachine,
    /// Random walk - values change randomly within bounds
    RandomWalk,
    /// Custom drift using a rule expression
    Custom(String),
}

/// Drift rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftRule {
    /// Field to apply drift to
    pub field: String,
    /// Drift strategy
    pub strategy: DriftStrategy,
    /// Parameters for the drift strategy
    pub params: HashMap<String, Value>,
    /// Rate of change (per request or per time unit)
    pub rate: f64,
    /// Minimum value (for numeric fields)
    pub min_value: Option<Value>,
    /// Maximum value (for numeric fields)
    pub max_value: Option<Value>,
    /// Possible states (for state machine)
    pub states: Option<Vec<String>>,
    /// Transition probabilities (for state machine)
    pub transitions: Option<HashMap<String, Vec<(String, f64)>>>,
}

impl DriftRule {
    /// Create a new drift rule
    pub fn new(field: String, strategy: DriftStrategy) -> Self {
        Self {
            field,
            strategy,
            params: HashMap::new(),
            rate: 1.0,
            min_value: None,
            max_value: None,
            states: None,
            transitions: None,
        }
    }

    /// Set rate of change
    pub fn with_rate(mut self, rate: f64) -> Self {
        self.rate = rate;
        self
    }

    /// Set value bounds
    pub fn with_bounds(mut self, min: Value, max: Value) -> Self {
        self.min_value = Some(min);
        self.max_value = Some(max);
        self
    }

    /// Set states for state machine
    pub fn with_states(mut self, states: Vec<String>) -> Self {
        self.states = Some(states);
        self
    }

    /// Set transitions for state machine
    pub fn with_transitions(mut self, transitions: HashMap<String, Vec<(String, f64)>>) -> Self {
        self.transitions = Some(transitions);
        self
    }

    /// Add a parameter
    pub fn with_param(mut self, key: String, value: Value) -> Self {
        self.params.insert(key, value);
        self
    }

    /// Validate the drift rule
    pub fn validate(&self) -> Result<()> {
        if self.field.is_empty() {
            return Err(Error::generic("Field name cannot be empty"));
        }

        if self.rate < 0.0 {
            return Err(Error::generic("Rate must be non-negative"));
        }

        if self.strategy == DriftStrategy::StateMachine
            && (self.states.is_none() || self.transitions.is_none())
        {
            return Err(Error::generic("State machine strategy requires states and transitions"));
        }

        Ok(())
    }
}

/// Data drift configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDriftConfig {
    /// Drift rules to apply
    pub rules: Vec<DriftRule>,
    /// Whether to enable time-based drift
    pub time_based: bool,
    /// Whether to enable request-based drift
    pub request_based: bool,
    /// Drift interval (seconds for time-based, requests for request-based)
    pub interval: u64,
    /// Random seed for reproducible drift
    pub seed: Option<u64>,
}

impl Default for DataDriftConfig {
    fn default() -> Self {
        Self {
            rules: Vec::new(),
            time_based: false,
            request_based: true,
            interval: 1,
            seed: None,
        }
    }
}

impl DataDriftConfig {
    /// Create a new data drift configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a drift rule
    pub fn with_rule(mut self, rule: DriftRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Enable time-based drift
    pub fn with_time_based(mut self, interval_secs: u64) -> Self {
        self.time_based = true;
        self.interval = interval_secs;
        self
    }

    /// Enable request-based drift
    pub fn with_request_based(mut self, interval_requests: u64) -> Self {
        self.request_based = true;
        self.interval = interval_requests;
        self
    }

    /// Set random seed
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        for rule in &self.rules {
            rule.validate()?;
        }

        if self.interval == 0 {
            return Err(Error::generic("Interval must be greater than 0"));
        }

        Ok(())
    }
}

/// Data drift engine state
#[derive(Debug)]
struct DriftState {
    /// Current values for drifting fields
    values: HashMap<String, Value>,
    /// Request counter
    request_count: u64,
    /// Start time
    start_time: std::time::Instant,
    /// Random number generator
    rng: rand::rngs::StdRng,
}

/// Data drift engine
pub struct DataDriftEngine {
    /// Configuration
    config: DataDriftConfig,
    /// Current state
    state: Arc<RwLock<DriftState>>,
}

impl DataDriftEngine {
    /// Create a new data drift engine
    pub fn new(config: DataDriftConfig) -> Result<Self> {
        config.validate()?;

        use rand::SeedableRng;
        let rng = if let Some(seed) = config.seed {
            rand::rngs::StdRng::seed_from_u64(seed)
        } else {
            rand::rngs::StdRng::seed_from_u64(fastrand::u64(..))
        };

        let state = DriftState {
            values: HashMap::new(),
            request_count: 0,
            start_time: std::time::Instant::now(),
            rng,
        };

        Ok(Self {
            config,
            state: Arc::new(RwLock::new(state)),
        })
    }

    /// Apply drift to a value
    pub async fn apply_drift(&self, mut data: Value) -> Result<Value> {
        let mut state = self.state.write().await;
        state.request_count += 1;

        // Check if we should apply drift
        let should_drift = if self.config.time_based {
            let elapsed_secs = state.start_time.elapsed().as_secs();
            elapsed_secs % self.config.interval == 0
        } else if self.config.request_based {
            state.request_count % self.config.interval == 0
        } else {
            true // Always drift if no specific timing is configured
        };

        if !should_drift {
            return Ok(data);
        }

        // Apply each drift rule
        for rule in &self.config.rules {
            if let Some(obj) = data.as_object_mut() {
                if let Some(field_value) = obj.get(&rule.field) {
                    let new_value = self.apply_rule(rule, field_value.clone(), &mut state)?;
                    obj.insert(rule.field.clone(), new_value);
                }
            }
        }

        Ok(data)
    }

    /// Apply a single drift rule
    fn apply_rule(
        &self,
        rule: &DriftRule,
        current: Value,
        state: &mut DriftState,
    ) -> Result<Value> {
        use rand::Rng;

        match &rule.strategy {
            DriftStrategy::Linear => {
                // Linear drift for numeric values
                if let Some(num) = current.as_f64() {
                    let delta = rule.rate;
                    let mut new_val = num + delta;

                    // Apply bounds
                    if let Some(min) = &rule.min_value {
                        if let Some(min_num) = min.as_f64() {
                            new_val = new_val.max(min_num);
                        }
                    }
                    if let Some(max) = &rule.max_value {
                        if let Some(max_num) = max.as_f64() {
                            new_val = new_val.min(max_num);
                        }
                    }

                    Ok(Value::from(new_val))
                } else {
                    Ok(current)
                }
            }
            DriftStrategy::Stepped => {
                // Step-based drift
                if let Some(num) = current.as_i64() {
                    let step = rule.rate as i64;
                    let new_val = num + step;
                    Ok(Value::from(new_val))
                } else {
                    Ok(current)
                }
            }
            DriftStrategy::StateMachine => {
                // State machine transitions
                if let Some(current_state) = current.as_str() {
                    if let Some(transitions) = &rule.transitions {
                        if let Some(possible_transitions) = transitions.get(current_state) {
                            // Use weighted random selection
                            let random_val: f64 = state.rng.random();
                            let mut cumulative = 0.0;

                            for (next_state, probability) in possible_transitions {
                                cumulative += probability;
                                if random_val <= cumulative {
                                    return Ok(Value::String(next_state.clone()));
                                }
                            }
                        }
                    }
                }
                Ok(current)
            }
            DriftStrategy::RandomWalk => {
                // Random walk within bounds
                if let Some(num) = current.as_f64() {
                    let delta = state.rng.random_range(-rule.rate..=rule.rate);
                    let mut new_val = num + delta;

                    // Apply bounds
                    if let Some(min) = &rule.min_value {
                        if let Some(min_num) = min.as_f64() {
                            new_val = new_val.max(min_num);
                        }
                    }
                    if let Some(max) = &rule.max_value {
                        if let Some(max_num) = max.as_f64() {
                            new_val = new_val.min(max_num);
                        }
                    }

                    Ok(Value::from(new_val))
                } else {
                    Ok(current)
                }
            }
            DriftStrategy::Custom(_expr) => {
                // Custom drift rules (simplified - could use expression evaluation)
                Ok(current)
            }
        }
    }

    /// Reset the drift state
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        state.values.clear();
        state.request_count = 0;
        state.start_time = std::time::Instant::now();
    }

    /// Get current request count
    pub async fn request_count(&self) -> u64 {
        self.state.read().await.request_count
    }

    /// Get elapsed time since start
    pub async fn elapsed_secs(&self) -> u64 {
        self.state.read().await.start_time.elapsed().as_secs()
    }

    /// Update configuration
    pub fn update_config(&mut self, config: DataDriftConfig) -> Result<()> {
        config.validate()?;
        self.config = config;
        Ok(())
    }

    /// Get current configuration
    pub fn config(&self) -> &DataDriftConfig {
        &self.config
    }
}

/// Pre-defined drift scenarios
pub mod scenarios {
    use super::*;

    /// Order status progression
    pub fn order_status_drift() -> DriftRule {
        let mut transitions = HashMap::new();
        transitions.insert(
            "pending".to_string(),
            vec![
                ("processing".to_string(), 0.7),
                ("cancelled".to_string(), 0.3),
            ],
        );
        transitions.insert(
            "processing".to_string(),
            vec![("shipped".to_string(), 0.9), ("cancelled".to_string(), 0.1)],
        );
        transitions.insert("shipped".to_string(), vec![("delivered".to_string(), 1.0)]);
        transitions.insert("delivered".to_string(), vec![]);
        transitions.insert("cancelled".to_string(), vec![]);

        DriftRule::new("status".to_string(), DriftStrategy::StateMachine)
            .with_states(vec![
                "pending".to_string(),
                "processing".to_string(),
                "shipped".to_string(),
                "delivered".to_string(),
                "cancelled".to_string(),
            ])
            .with_transitions(transitions)
    }

    /// Stock quantity depletion
    pub fn stock_depletion_drift() -> DriftRule {
        DriftRule::new("quantity".to_string(), DriftStrategy::Linear)
            .with_rate(-1.0)
            .with_bounds(Value::from(0), Value::from(1000))
    }

    /// Price fluctuation
    pub fn price_fluctuation_drift() -> DriftRule {
        DriftRule::new("price".to_string(), DriftStrategy::RandomWalk)
            .with_rate(0.5)
            .with_bounds(Value::from(0.0), Value::from(10000.0))
    }

    /// User activity score
    pub fn activity_score_drift() -> DriftRule {
        DriftRule::new("activity_score".to_string(), DriftStrategy::Linear)
            .with_rate(0.1)
            .with_bounds(Value::from(0.0), Value::from(100.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drift_strategy_serde() {
        let strategy = DriftStrategy::Linear;
        let serialized = serde_json::to_string(&strategy).unwrap();
        let deserialized: DriftStrategy = serde_json::from_str(&serialized).unwrap();
        assert_eq!(strategy, deserialized);
    }

    #[test]
    fn test_drift_rule_builder() {
        let rule = DriftRule::new("quantity".to_string(), DriftStrategy::Linear)
            .with_rate(1.5)
            .with_bounds(Value::from(0), Value::from(100));

        assert_eq!(rule.field, "quantity");
        assert_eq!(rule.strategy, DriftStrategy::Linear);
        assert_eq!(rule.rate, 1.5);
    }

    #[test]
    fn test_drift_rule_validate() {
        let rule = DriftRule::new("test".to_string(), DriftStrategy::Linear);
        assert!(rule.validate().is_ok());
    }

    #[test]
    fn test_drift_rule_validate_empty_field() {
        let rule = DriftRule::new("".to_string(), DriftStrategy::Linear);
        assert!(rule.validate().is_err());
    }

    #[test]
    fn test_drift_config_builder() {
        let rule = DriftRule::new("field".to_string(), DriftStrategy::Linear);
        let config = DataDriftConfig::new().with_rule(rule).with_request_based(10).with_seed(42);

        assert_eq!(config.rules.len(), 1);
        assert!(config.request_based);
        assert_eq!(config.interval, 10);
        assert_eq!(config.seed, Some(42));
    }

    #[tokio::test]
    async fn test_drift_engine_creation() {
        let config = DataDriftConfig::new();
        let result = DataDriftEngine::new(config);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_drift_engine_reset() {
        let config = DataDriftConfig::new();
        let engine = DataDriftEngine::new(config).unwrap();
        engine.reset().await;
        assert_eq!(engine.request_count().await, 0);
    }

    #[test]
    fn test_order_status_drift_scenario() {
        let rule = scenarios::order_status_drift();
        assert_eq!(rule.field, "status");
        assert_eq!(rule.strategy, DriftStrategy::StateMachine);
    }

    #[test]
    fn test_stock_depletion_drift_scenario() {
        let rule = scenarios::stock_depletion_drift();
        assert_eq!(rule.field, "quantity");
        assert_eq!(rule.strategy, DriftStrategy::Linear);
        assert_eq!(rule.rate, -1.0);
    }
}
