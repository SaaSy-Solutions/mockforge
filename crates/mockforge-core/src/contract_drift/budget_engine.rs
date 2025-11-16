//! Drift budget evaluation engine
//!
//! This module provides the core logic for evaluating contract drift against configured budgets.

use crate::ai_contract_diff::ContractDiffResult;
use crate::contract_drift::types::{
    BreakingChangeRule, DriftBudget, DriftBudgetConfig, DriftResult,
};

/// Engine for evaluating drift budgets
#[derive(Debug, Clone)]
pub struct DriftBudgetEngine {
    config: DriftBudgetConfig,
}

impl DriftBudgetEngine {
    /// Create a new drift budget engine
    pub fn new(config: DriftBudgetConfig) -> Self {
        Self { config }
    }

    /// Evaluate contract diff result against drift budget
    ///
    /// Returns a `DriftResult` indicating whether the budget is exceeded and
    /// which mismatches are considered breaking.
    pub fn evaluate(
        &self,
        diff_result: &ContractDiffResult,
        endpoint: &str,
        method: &str,
    ) -> DriftResult {
        if !self.config.enabled {
            // If drift tracking is disabled, return a result indicating no issues
            return DriftResult {
                budget_exceeded: false,
                breaking_changes: 0,
                non_breaking_changes: 0,
                breaking_mismatches: vec![],
                non_breaking_mismatches: diff_result.mismatches.clone(),
                metrics: crate::contract_drift::types::DriftMetrics {
                    endpoint: endpoint.to_string(),
                    method: method.to_string(),
                    breaking_changes: 0,
                    non_breaking_changes: diff_result.mismatches.len() as u32,
                    total_changes: diff_result.mismatches.len() as u32,
                    budget_exceeded: false,
                    last_updated: chrono::Utc::now().timestamp(),
                },
                should_create_incident: false,
            };
        }

        // Get budget for this endpoint
        let budget = self.get_budget_for_endpoint(endpoint, method);

        if !budget.enabled {
            // Budget disabled for this endpoint
            return DriftResult {
                budget_exceeded: false,
                breaking_changes: 0,
                non_breaking_changes: 0,
                breaking_mismatches: vec![],
                non_breaking_mismatches: diff_result.mismatches.clone(),
                metrics: crate::contract_drift::types::DriftMetrics {
                    endpoint: endpoint.to_string(),
                    method: method.to_string(),
                    breaking_changes: 0,
                    non_breaking_changes: diff_result.mismatches.len() as u32,
                    total_changes: diff_result.mismatches.len() as u32,
                    budget_exceeded: false,
                    last_updated: chrono::Utc::now().timestamp(),
                },
                should_create_incident: false,
            };
        }

        // Evaluate against budget
        DriftResult::from_diff_result(
            diff_result,
            endpoint.to_string(),
            method.to_string(),
            &budget,
            &self.config.breaking_change_rules,
        )
    }

    /// Get budget for a specific endpoint
    fn get_budget_for_endpoint(&self, endpoint: &str, method: &str) -> DriftBudget {
        let key = format!("{} {}", method, endpoint);

        // Check per-endpoint budgets first
        if let Some(budget) = self.config.per_endpoint_budgets.get(&key) {
            return budget.clone();
        }

        // Fall back to default budget
        self.config
            .default_budget
            .clone()
            .unwrap_or_else(DriftBudget::default)
    }

    /// Get the drift budget configuration
    pub fn config(&self) -> &DriftBudgetConfig {
        &self.config
    }

    /// Update the drift budget configuration
    pub fn update_config(&mut self, config: DriftBudgetConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_contract_diff::{
        ContractDiffResult, DiffMetadata, Mismatch, MismatchSeverity, MismatchType,
    };

    fn create_test_diff_result(mismatches: Vec<Mismatch>) -> ContractDiffResult {
        ContractDiffResult {
            matches: mismatches.is_empty(),
            confidence: 1.0,
            mismatches,
            recommendations: vec![],
            corrections: vec![],
            metadata: DiffMetadata {
                contract_format: "openapi".to_string(),
                contract_version: "3.0.0".to_string(),
                analyzed_at: chrono::Utc::now(),
            },
        }
    }

    #[test]
    fn test_budget_evaluation_no_mismatches() {
        let config = DriftBudgetConfig::default();
        let engine = DriftBudgetEngine::new(config);
        let diff_result = create_test_diff_result(vec![]);

        let result = engine.evaluate(&diff_result, "/api/users", "GET");

        assert!(!result.budget_exceeded);
        assert_eq!(result.breaking_changes, 0);
        assert_eq!(result.non_breaking_changes, 0);
        assert!(!result.should_create_incident);
    }

    #[test]
    fn test_budget_evaluation_breaking_change() {
        let config = DriftBudgetConfig::default();
        let engine = DriftBudgetEngine::new(config);

        let mismatch = Mismatch {
            mismatch_type: MismatchType::MissingRequiredField,
            path: "body.email".to_string(),
            method: Some("POST".to_string()),
            expected: Some("string".to_string()),
            actual: None,
            description: "Missing required field: email".to_string(),
            severity: MismatchSeverity::Critical,
            confidence: 1.0,
            context: std::collections::HashMap::new(),
        };

        let diff_result = create_test_diff_result(vec![mismatch]);
        let result = engine.evaluate(&diff_result, "/api/users", "POST");

        assert!(result.breaking_changes > 0);
        assert!(result.should_create_incident);
    }

    #[test]
    fn test_budget_evaluation_non_breaking_change() {
        let config = DriftBudgetConfig::default();
        let engine = DriftBudgetEngine::new(config);

        let mismatch = Mismatch {
            mismatch_type: MismatchType::UnexpectedField,
            path: "body.extra_field".to_string(),
            method: Some("POST".to_string()),
            expected: None,
            actual: Some("value".to_string()),
            description: "Unexpected field: extra_field".to_string(),
            severity: MismatchSeverity::Low,
            confidence: 1.0,
            context: std::collections::HashMap::new(),
        };

        let diff_result = create_test_diff_result(vec![mismatch]);
        let result = engine.evaluate(&diff_result, "/api/users", "POST");

        assert_eq!(result.breaking_changes, 0);
        assert!(result.non_breaking_changes > 0);
        // Non-breaking changes within budget shouldn't create incidents
        assert!(!result.should_create_incident);
    }
}
