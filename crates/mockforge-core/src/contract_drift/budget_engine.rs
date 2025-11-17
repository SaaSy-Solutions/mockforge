//! Drift budget evaluation engine
//!
//! This module provides the core logic for evaluating contract drift against configured budgets.

use crate::ai_contract_diff::ContractDiffResult;
use crate::contract_drift::field_tracking::FieldCountTracker;
use crate::contract_drift::types::{DriftBudget, DriftBudgetConfig, DriftResult};
use std::sync::Arc;

/// Engine for evaluating drift budgets
#[derive(Debug, Clone)]
pub struct DriftBudgetEngine {
    config: DriftBudgetConfig,
    /// Optional field count tracker for percentage-based budgets
    field_tracker: Option<Arc<tokio::sync::RwLock<FieldCountTracker>>>,
}

impl DriftBudgetEngine {
    /// Create a new drift budget engine
    pub fn new(config: DriftBudgetConfig) -> Self {
        Self {
            config,
            field_tracker: None,
        }
    }

    /// Create a new drift budget engine with field count tracking
    pub fn new_with_tracker(
        config: DriftBudgetConfig,
        field_tracker: Arc<tokio::sync::RwLock<FieldCountTracker>>,
    ) -> Self {
        Self {
            config,
            field_tracker: Some(field_tracker),
        }
    }

    /// Set the field count tracker
    pub fn set_field_tracker(
        &mut self,
        field_tracker: Arc<tokio::sync::RwLock<FieldCountTracker>>,
    ) {
        self.field_tracker = Some(field_tracker);
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
        self.evaluate_with_context(diff_result, endpoint, method, None, None, None)
    }

    /// Evaluate contract diff result against drift budget with context
    ///
    /// Returns a `DriftResult` indicating whether the budget is exceeded and
    /// which mismatches are considered breaking.
    pub fn evaluate_with_context(
        &self,
        diff_result: &ContractDiffResult,
        endpoint: &str,
        method: &str,
        workspace_id: Option<&str>,
        service_name: Option<&str>,
        tags: Option<&[String]>,
    ) -> DriftResult {
        if !self.config.enabled {
            // If drift tracking is disabled, return a result indicating no issues
            return DriftResult {
                budget_exceeded: false,
                breaking_changes: 0,
                potentially_breaking_changes: 0,
                non_breaking_changes: 0,
                breaking_mismatches: vec![],
                potentially_breaking_mismatches: vec![],
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

        // Get budget for this endpoint using priority hierarchy
        let budget =
            self.get_budget_for_endpoint(endpoint, method, workspace_id, service_name, tags);

        if !budget.enabled {
            // Budget disabled for this endpoint
            return DriftResult {
                budget_exceeded: false,
                breaking_changes: 0,
                potentially_breaking_changes: 0,
                non_breaking_changes: 0,
                breaking_mismatches: vec![],
                potentially_breaking_mismatches: vec![],
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

        // Calculate baseline field count if percentage-based budget is enabled and tracker is available
        let baseline_field_count = if budget.max_field_churn_percent.is_some() {
            if let Some(ref tracker) = self.field_tracker {
                // Use blocking call to get baseline (evaluate_with_context is sync)
                // This is safe because we're in a tokio runtime context
                let rt = tokio::runtime::Handle::try_current();
                if let Ok(handle) = rt {
                    if let Some(time_window) = budget.time_window_days {
                        // Use blocking call to get baseline (average over time window)
                        handle.block_on(async {
                            let guard = tracker.read().await;
                            guard.get_average_count(
                                None, // workspace_id - could be passed through if needed
                                endpoint,
                                method,
                                time_window,
                            )
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Evaluate against budget
        DriftResult::from_diff_result(
            diff_result,
            endpoint.to_string(),
            method.to_string(),
            &budget,
            &self.config.breaking_change_rules,
            baseline_field_count,
        )
    }

    /// Get budget for a specific endpoint
    ///
    /// Priority order: workspace > service/tag > endpoint > default
    pub fn get_budget_for_endpoint(
        &self,
        endpoint: &str,
        method: &str,
        workspace_id: Option<&str>,
        service_name: Option<&str>,
        tags: Option<&[String]>,
    ) -> DriftBudget {
        // Priority 1: Per-workspace budget
        if let Some(workspace_id) = workspace_id {
            if let Some(budget) = self.config.per_workspace_budgets.get(workspace_id) {
                return budget.clone();
            }
        }

        // Priority 2: Per-service budget (explicit service name)
        if let Some(service_name) = service_name {
            if let Some(budget) = self.config.per_service_budgets.get(service_name) {
                return budget.clone();
            }
        }

        // Priority 3: Per-tag budget (from OpenAPI tags)
        if let Some(tags) = tags {
            for tag in tags {
                if let Some(budget) = self.config.per_tag_budgets.get(tag) {
                    return budget.clone();
                }
                // Also check per_service_budgets for tag matches
                if let Some(budget) = self.config.per_service_budgets.get(tag) {
                    return budget.clone();
                }
            }
        }

        // Priority 4: Per-endpoint budget
        let key = format!("{} {}", method, endpoint);
        if let Some(budget) = self.config.per_endpoint_budgets.get(&key) {
            return budget.clone();
        }

        // Priority 5: Default budget
        self.config.default_budget.clone().unwrap_or_else(DriftBudget::default)
    }

    /// Get budget for a specific endpoint (backward compatibility)
    fn get_budget_for_endpoint_legacy(&self, endpoint: &str, method: &str) -> DriftBudget {
        self.get_budget_for_endpoint(endpoint, method, None, None, None)
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
