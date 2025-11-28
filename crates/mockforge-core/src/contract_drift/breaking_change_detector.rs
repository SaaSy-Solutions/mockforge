//! Breaking change detection logic
//!
//! This module provides utilities for detecting breaking changes based on configurable rules.

use crate::ai_contract_diff::Mismatch;
use crate::contract_drift::types::{
    BreakingChangeRule, BreakingChangeRuleConfig, BreakingChangeRuleType,
};

/// Detector for breaking changes
#[derive(Debug, Clone)]
pub struct BreakingChangeDetector {
    rules: Vec<BreakingChangeRule>,
}

impl BreakingChangeDetector {
    /// Create a new breaking change detector with rules
    pub fn new(rules: Vec<BreakingChangeRule>) -> Self {
        Self { rules }
    }

    /// Check if a mismatch represents a breaking change
    pub fn is_breaking(&self, mismatch: &Mismatch) -> bool {
        self.rules.iter().filter(|rule| rule.enabled).any(|rule| rule.matches(mismatch))
    }

    /// Classify mismatches into breaking and non-breaking
    pub fn classify(&self, mismatches: &[Mismatch]) -> (Vec<Mismatch>, Vec<Mismatch>) {
        let mut breaking = Vec::new();
        let mut non_breaking = Vec::new();

        for mismatch in mismatches {
            if self.is_breaking(mismatch) {
                breaking.push(mismatch.clone());
            } else {
                non_breaking.push(mismatch.clone());
            }
        }

        (breaking, non_breaking)
    }

    /// Classify mismatches into three categories: non-breaking, potentially breaking, and definitely breaking
    ///
    /// - **Non-breaking**: Additive changes, documentation-only, unexpected fields (additive)
    /// - **Potentially breaking**: Medium severity changes, format mismatches, constraint violations
    /// - **Definitely breaking**: Critical/High severity, missing required fields, type changes, removals
    pub fn classify_three_way(
        &self,
        mismatches: &[Mismatch],
    ) -> (Vec<Mismatch>, Vec<Mismatch>, Vec<Mismatch>) {
        let mut non_breaking = Vec::new();
        let mut potentially_breaking = Vec::new();
        let mut definitely_breaking = Vec::new();

        for mismatch in mismatches {
            // Definitely breaking: explicitly matches breaking rules
            if self.is_breaking(mismatch) {
                definitely_breaking.push(mismatch.clone());
            }
            // Potentially breaking: medium severity or certain mismatch types that need review
            else if mismatch.severity == crate::ai_contract_diff::MismatchSeverity::Medium
                || matches!(
                    mismatch.mismatch_type,
                    crate::ai_contract_diff::MismatchType::FormatMismatch
                        | crate::ai_contract_diff::MismatchType::ConstraintViolation
                        | crate::ai_contract_diff::MismatchType::TypeMismatch
                )
            {
                potentially_breaking.push(mismatch.clone());
            }
            // Non-breaking: additive changes, documentation, unexpected fields
            else {
                non_breaking.push(mismatch.clone());
            }
        }

        (non_breaking, potentially_breaking, definitely_breaking)
    }

    /// Get the rules used by this detector
    pub fn rules(&self) -> &[BreakingChangeRule] {
        &self.rules
    }

    /// Add a new rule
    pub fn add_rule(&mut self, rule: BreakingChangeRule) {
        self.rules.push(rule);
    }

    /// Remove a rule by index
    pub fn remove_rule(&mut self, index: usize) {
        if index < self.rules.len() {
            self.rules.remove(index);
        }
    }
}

impl Default for BreakingChangeDetector {
    fn default() -> Self {
        Self::new(vec![
            // Default: Critical and High severity are breaking
            BreakingChangeRule {
                rule_type: BreakingChangeRuleType::Severity,
                config: BreakingChangeRuleConfig::Severity {
                    severity: crate::ai_contract_diff::MismatchSeverity::High,
                    include_higher: true,
                },
                enabled: true,
            },
            // Missing required fields are always breaking
            BreakingChangeRule {
                rule_type: BreakingChangeRuleType::MismatchType,
                config: BreakingChangeRuleConfig::MismatchType {
                    mismatch_type: crate::ai_contract_diff::MismatchType::MissingRequiredField,
                },
                enabled: true,
            },
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_contract_diff::{MismatchSeverity, MismatchType};

    fn create_test_mismatch(mismatch_type: MismatchType, severity: MismatchSeverity) -> Mismatch {
        Mismatch {
            mismatch_type,
            path: "body.field".to_string(),
            method: Some("POST".to_string()),
            expected: Some("string".to_string()),
            actual: None,
            description: "Test mismatch".to_string(),
            severity,
            confidence: 1.0,
            context: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_severity_based_detection() {
        let detector = BreakingChangeDetector::default();

        let critical_mismatch =
            create_test_mismatch(MismatchType::TypeMismatch, MismatchSeverity::Critical);
        assert!(detector.is_breaking(&critical_mismatch));

        let high_mismatch =
            create_test_mismatch(MismatchType::TypeMismatch, MismatchSeverity::High);
        assert!(detector.is_breaking(&high_mismatch));

        let medium_mismatch =
            create_test_mismatch(MismatchType::TypeMismatch, MismatchSeverity::Medium);
        assert!(!detector.is_breaking(&medium_mismatch));
    }

    #[test]
    fn test_mismatch_type_based_detection() {
        let detector = BreakingChangeDetector::default();

        let missing_field =
            create_test_mismatch(MismatchType::MissingRequiredField, MismatchSeverity::Medium);
        assert!(detector.is_breaking(&missing_field));

        let unexpected_field =
            create_test_mismatch(MismatchType::UnexpectedField, MismatchSeverity::Medium);
        assert!(!detector.is_breaking(&unexpected_field));
    }

    #[test]
    fn test_classify() {
        let detector = BreakingChangeDetector::default();

        let mismatches = vec![
            create_test_mismatch(MismatchType::MissingRequiredField, MismatchSeverity::Critical),
            create_test_mismatch(MismatchType::UnexpectedField, MismatchSeverity::Low),
            create_test_mismatch(MismatchType::TypeMismatch, MismatchSeverity::High),
        ];

        let (breaking, non_breaking) = detector.classify(&mismatches);

        assert_eq!(breaking.len(), 2); // MissingRequiredField and High severity
        assert_eq!(non_breaking.len(), 1); // UnexpectedField with Low severity
    }
}
