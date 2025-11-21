//! Behavioral Economics configuration
//!
//! Defines the configuration structure for the behavioral economics engine,
//! including rule definitions and engine settings.

use crate::behavioral_economics::rules::BehaviorRule;
use serde::{Deserialize, Serialize};

/// Behavioral Economics configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BehavioralEconomicsConfig {
    /// Enable behavioral economics engine
    #[serde(default)]
    pub enabled: bool,

    /// List of behavior rules
    #[serde(default)]
    pub rules: Vec<BehaviorRule>,

    /// Evaluation interval in milliseconds (how often to re-evaluate conditions)
    #[serde(default = "default_evaluation_interval")]
    pub evaluation_interval_ms: u64,

    /// Enable metrics collection
    #[serde(default = "default_true")]
    pub enable_metrics: bool,
}

fn default_evaluation_interval() -> u64 {
    1000 // 1 second
}

fn default_true() -> bool {
    true
}

impl BehavioralEconomicsConfig {
    /// Create a new configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a rule
    pub fn with_rule(mut self, rule: BehaviorRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Enable the engine
    pub fn enable(mut self) -> Self {
        self.enabled = true;
        self
    }

    /// Disable the engine
    pub fn disable(mut self) -> Self {
        self.enabled = false;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behavioral_economics::actions::BehaviorAction;
    use crate::behavioral_economics::conditions::BehaviorCondition;
    use crate::behavioral_economics::rules::{BehaviorRule, RuleType};

    #[test]
    fn test_config_creation() {
        let config = BehavioralEconomicsConfig::new();
        assert!(!config.enabled);
        assert!(config.rules.is_empty());
    }

    #[test]
    fn test_config_with_rule() {
        let rule = BehaviorRule::declarative(
            "test-rule".to_string(),
            BehaviorCondition::Always,
            BehaviorAction::NoOp,
            100,
        );
        let config = BehavioralEconomicsConfig::new().with_rule(rule);
        assert_eq!(config.rules.len(), 1);
    }
}

