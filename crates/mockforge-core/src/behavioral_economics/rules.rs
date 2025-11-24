//! Behavior rule definitions
//!
//! Defines the structure of behavior rules that can be either declarative
//! (simple YAML/JSON config) or scriptable (JavaScript/WASM) for complex logic.

use crate::behavioral_economics::actions::BehaviorAction;
use crate::behavioral_economics::conditions::BehaviorCondition;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Type of behavior rule
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum RuleType {
    /// Declarative rule - simple if-then logic defined in YAML/JSON
    Declarative,
    /// Scriptable rule - complex logic defined in JavaScript or WASM
    Scriptable,
}

/// Behavior rule definition
///
/// A rule consists of a condition and an action. When the condition evaluates
/// to true, the action is executed. Rules can be declarative (simple) or
/// scriptable (complex).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BehaviorRule {
    /// Rule name (unique identifier)
    pub name: String,

    /// Rule type (declarative or scriptable)
    pub rule_type: RuleType,

    /// Condition to evaluate
    pub condition: BehaviorCondition,

    /// Action to execute when condition is true
    pub action: BehaviorAction,

    /// Priority (higher = evaluated first)
    pub priority: u32,

    /// Optional script for scriptable rules (JavaScript or WASM)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,

    /// Optional script language (e.g., "javascript", "wasm")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_language: Option<String>,

    /// Additional metadata
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub metadata: std::collections::HashMap<String, Value>,
}

impl BehaviorRule {
    /// Create a new declarative rule
    pub fn declarative(
        name: String,
        condition: BehaviorCondition,
        action: BehaviorAction,
        priority: u32,
    ) -> Self {
        Self {
            name,
            rule_type: RuleType::Declarative,
            condition,
            action,
            priority,
            script: None,
            script_language: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create a new scriptable rule
    pub fn scriptable(
        name: String,
        condition: BehaviorCondition,
        action: BehaviorAction,
        priority: u32,
        script: String,
        script_language: String,
    ) -> Self {
        Self {
            name,
            rule_type: RuleType::Scriptable,
            condition,
            action,
            priority,
            script: Some(script),
            script_language: Some(script_language),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Validate the rule
    pub fn validate(&self) -> crate::Result<()> {
        if self.name.trim().is_empty() {
            return Err(crate::Error::generic("Rule name cannot be empty"));
        }

        if matches!(self.rule_type, RuleType::Scriptable) {
            if self.script.is_none() {
                return Err(crate::Error::generic("Scriptable rules must have a script"));
            }
            if self.script_language.is_none() {
                return Err(crate::Error::generic("Scriptable rules must have a script_language"));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behavioral_economics::actions::BehaviorAction;
    use crate::behavioral_economics::conditions::BehaviorCondition;

    #[test]
    fn test_declarative_rule_creation() {
        let rule = BehaviorRule::declarative(
            "test-rule".to_string(),
            BehaviorCondition::Always,
            BehaviorAction::NoOp,
            100,
        );
        assert_eq!(rule.rule_type, RuleType::Declarative);
        assert!(rule.script.is_none());
    }

    #[test]
    fn test_scriptable_rule_creation() {
        let rule = BehaviorRule::scriptable(
            "test-rule".to_string(),
            BehaviorCondition::Always,
            BehaviorAction::NoOp,
            100,
            "console.log('test')".to_string(),
            "javascript".to_string(),
        );
        assert_eq!(rule.rule_type, RuleType::Scriptable);
        assert!(rule.script.is_some());
    }

    #[test]
    fn test_rule_validation() {
        let mut rule = BehaviorRule::declarative(
            "test-rule".to_string(),
            BehaviorCondition::Always,
            BehaviorAction::NoOp,
            100,
        );
        assert!(rule.validate().is_ok());

        rule.name = "".to_string();
        assert!(rule.validate().is_err());
    }
}
