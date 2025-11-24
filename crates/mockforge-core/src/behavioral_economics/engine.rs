//! Behavioral Economics Engine
//!
//! Main engine that evaluates behavior rules and executes actions based on
//! current system state (latency, load, pricing, fraud, etc.).

use crate::behavioral_economics::actions::ActionExecutor;
use crate::behavioral_economics::conditions::ConditionEvaluator;
use crate::behavioral_economics::config::BehavioralEconomicsConfig;
use crate::behavioral_economics::rules::BehaviorRule;
use crate::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Behavioral Economics Engine
///
/// Evaluates behavior rules and executes actions based on system state.
/// Rules are evaluated in priority order, with declarative rules evaluated
/// before scriptable rules.
pub struct BehavioralEconomicsEngine {
    /// Engine configuration
    config: BehavioralEconomicsConfig,
    /// Condition evaluator
    condition_evaluator: Arc<RwLock<ConditionEvaluator>>,
    /// Action executor
    action_executor: ActionExecutor,
    /// Rules sorted by priority (highest first)
    rules: Vec<BehaviorRule>,
}

impl BehavioralEconomicsEngine {
    /// Create a new behavioral economics engine
    pub fn new(config: BehavioralEconomicsConfig) -> Result<Self> {
        // Validate all rules
        for rule in &config.rules {
            rule.validate()?;
        }

        // Sort rules by priority (highest first)
        let mut rules = config.rules.clone();
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        Ok(Self {
            config,
            condition_evaluator: Arc::new(RwLock::new(ConditionEvaluator::new())),
            action_executor: ActionExecutor::new(),
            rules,
        })
    }

    /// Create engine with default config
    pub fn default() -> Self {
        Self::new(BehavioralEconomicsConfig::default()).expect("Failed to create default engine")
    }

    /// Get condition evaluator (for updating metrics)
    pub fn condition_evaluator(&self) -> Arc<RwLock<ConditionEvaluator>> {
        Arc::clone(&self.condition_evaluator)
    }

    /// Evaluate all rules and execute matching actions
    ///
    /// Returns a list of executed actions (for logging/debugging)
    pub async fn evaluate(&self) -> Result<Vec<String>> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }

        let evaluator = self.condition_evaluator.read().await;
        let mut executed_actions = Vec::new();

        // Evaluate rules in priority order
        for rule in &self.rules {
            match evaluator.evaluate(&rule.condition) {
                Ok(true) => {
                    debug!("Rule '{}' condition met, executing action", rule.name);
                    match self.action_executor.execute(&rule.action) {
                        Ok(action_desc) => {
                            info!("Executed action for rule '{}': {}", rule.name, action_desc);
                            executed_actions.push(format!("{}: {}", rule.name, action_desc));
                        }
                        Err(e) => {
                            warn!("Failed to execute action for rule '{}': {}", rule.name, e);
                        }
                    }
                }
                Ok(false) => {
                    debug!("Rule '{}' condition not met", rule.name);
                }
                Err(e) => {
                    warn!("Failed to evaluate condition for rule '{}': {}", rule.name, e);
                }
            }
        }

        Ok(executed_actions)
    }

    /// Add a rule at runtime
    pub fn add_rule(&mut self, rule: BehaviorRule) -> Result<()> {
        rule.validate()?;
        self.rules.push(rule);
        // Re-sort by priority
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        Ok(())
    }

    /// Remove a rule by name
    pub fn remove_rule(&mut self, name: &str) -> bool {
        let initial_len = self.rules.len();
        self.rules.retain(|r| r.name != name);
        self.rules.len() < initial_len
    }

    /// Get all rules
    pub fn rules(&self) -> &[BehaviorRule] {
        &self.rules
    }

    /// Update configuration
    pub fn update_config(&mut self, config: BehavioralEconomicsConfig) -> Result<()> {
        // Validate all rules
        for rule in &config.rules {
            rule.validate()?;
        }

        // Sort rules by priority
        let mut rules = config.rules.clone();
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        self.config = config;
        self.rules = rules;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behavioral_economics::actions::BehaviorAction;
    use crate::behavioral_economics::conditions::BehaviorCondition;
    use crate::behavioral_economics::rules::{BehaviorRule, RuleType};

    #[tokio::test]
    async fn test_engine_creation() {
        let config = BehavioralEconomicsConfig::new();
        let engine = BehavioralEconomicsEngine::new(config).unwrap();
        assert!(engine.rules().is_empty());
    }

    #[tokio::test]
    async fn test_engine_evaluation() {
        let rule = BehaviorRule::declarative(
            "test-rule".to_string(),
            BehaviorCondition::Always,
            BehaviorAction::NoOp,
            100,
        );
        let config = BehavioralEconomicsConfig::new().enable().with_rule(rule);
        let engine = BehavioralEconomicsEngine::new(config).unwrap();
        let results = engine.evaluate().await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_engine_disabled() {
        let rule = BehaviorRule::declarative(
            "test-rule".to_string(),
            BehaviorCondition::Always,
            BehaviorAction::NoOp,
            100,
        );
        let config = BehavioralEconomicsConfig::new().disable().with_rule(rule);
        let engine = BehavioralEconomicsEngine::new(config).unwrap();
        let results = engine.evaluate().await.unwrap();
        assert!(results.is_empty());
    }
}
