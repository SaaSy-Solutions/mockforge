//! Drift budget evaluation engine
//!
//! This module provides the core logic for evaluating contract drift against configured budgets.

use crate::ai_contract_diff::ContractDiffResult;
use crate::contract_drift::consumer_mapping::ConsumerImpactAnalyzer;
use crate::contract_drift::fitness::FitnessFunctionRegistry;
use crate::openapi::OpenApiSpec;
use mockforge_contracts::contract_drift::field_tracking::FieldCountTracker;
use mockforge_contracts::contract_drift::types::{DriftBudget, DriftBudgetConfig, DriftResult};
use std::sync::Arc;

/// Engine for evaluating drift budgets
#[derive(Debug)]
pub struct DriftBudgetEngine {
    config: DriftBudgetConfig,
    /// Optional field count tracker for percentage-based budgets
    field_tracker: Option<Arc<tokio::sync::RwLock<FieldCountTracker>>>,
    /// Optional fitness function registry for running fitness tests
    fitness_registry: Option<Arc<tokio::sync::RwLock<FitnessFunctionRegistry>>>,
    /// Optional consumer impact analyzer for determining affected consumers
    consumer_analyzer: Option<Arc<tokio::sync::RwLock<ConsumerImpactAnalyzer>>>,
}

impl DriftBudgetEngine {
    /// Create a new drift budget engine
    pub fn new(config: DriftBudgetConfig) -> Self {
        Self {
            config,
            field_tracker: None,
            fitness_registry: None,
            consumer_analyzer: None,
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
            fitness_registry: None,
            consumer_analyzer: None,
        }
    }

    /// Set the fitness function registry
    pub fn set_fitness_registry(
        &mut self,
        fitness_registry: Arc<tokio::sync::RwLock<FitnessFunctionRegistry>>,
    ) {
        self.fitness_registry = Some(fitness_registry);
    }

    /// Set the consumer impact analyzer
    pub fn set_consumer_analyzer(
        &mut self,
        consumer_analyzer: Arc<tokio::sync::RwLock<ConsumerImpactAnalyzer>>,
    ) {
        self.consumer_analyzer = Some(consumer_analyzer);
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
    pub async fn evaluate(
        &self,
        diff_result: &ContractDiffResult,
        endpoint: &str,
        method: &str,
    ) -> DriftResult {
        self.evaluate_with_context(diff_result, endpoint, method, None, None, None)
            .await
    }

    /// Evaluate contract diff result against drift budget with context
    ///
    /// Returns a `DriftResult` indicating whether the budget is exceeded and
    /// which mismatches are considered breaking.
    pub async fn evaluate_with_context(
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
                metrics: mockforge_contracts::contract_drift::types::DriftMetrics {
                    endpoint: endpoint.to_string(),
                    method: method.to_string(),
                    breaking_changes: 0,
                    non_breaking_changes: diff_result.mismatches.len() as u32,
                    total_changes: diff_result.mismatches.len() as u32,
                    budget_exceeded: false,
                    last_updated: chrono::Utc::now().timestamp(),
                },
                should_create_incident: false,
                fitness_test_results: Vec::new(),
                consumer_impact: None,
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
                metrics: mockforge_contracts::contract_drift::types::DriftMetrics {
                    endpoint: endpoint.to_string(),
                    method: method.to_string(),
                    breaking_changes: 0,
                    non_breaking_changes: diff_result.mismatches.len() as u32,
                    total_changes: diff_result.mismatches.len() as u32,
                    budget_exceeded: false,
                    last_updated: chrono::Utc::now().timestamp(),
                },
                should_create_incident: false,
                fitness_test_results: Vec::new(),
                consumer_impact: None,
            };
        }

        // Calculate baseline field count if percentage-based budget is enabled and tracker is available.
        // Awaiting directly (instead of Handle::block_on) avoids panicking when this runs inside a
        // tokio runtime, and never blocks a worker thread (#759).
        let baseline_field_count = if budget.max_field_churn_percent.is_some() {
            if let (Some(tracker), Some(time_window)) =
                (self.field_tracker.as_ref(), budget.time_window_days)
            {
                let guard = tracker.read().await;
                guard.get_average_count(
                    None, // workspace_id - could be passed through if needed
                    endpoint,
                    method,
                    time_window,
                )
            } else {
                None
            }
        } else {
            None
        };

        // Evaluate against budget
        let mut result = mockforge_contracts::contract_drift::types::drift_result_from_diff(
            diff_result,
            endpoint.to_string(),
            method.to_string(),
            &budget,
            &self.config.breaking_change_rules,
            baseline_field_count,
        );

        // Note: Fitness tests are not run in this method because we don't have access to
        // the OpenAPI specs. Use evaluate_with_specs() instead if you need fitness test evaluation.

        // Analyze consumer impact if analyzer is available
        if let Some(ref analyzer) = self.consumer_analyzer {
            let guard = analyzer.read().await;
            if let Some(impact) = guard.analyze_impact(endpoint, method) {
                result.consumer_impact = Some(impact);
            }
        }

        result
    }

    /// Evaluate contract diff result with OpenAPI specs for fitness function support
    ///
    /// This method is similar to `evaluate_with_context` but accepts old and new OpenAPI specs
    /// to enable fitness function evaluation.
    #[allow(clippy::too_many_arguments)]
    pub async fn evaluate_with_specs(
        &self,
        old_spec: Option<&OpenApiSpec>,
        new_spec: &OpenApiSpec,
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
                metrics: mockforge_contracts::contract_drift::types::DriftMetrics {
                    endpoint: endpoint.to_string(),
                    method: method.to_string(),
                    breaking_changes: 0,
                    non_breaking_changes: diff_result.mismatches.len() as u32,
                    total_changes: diff_result.mismatches.len() as u32,
                    budget_exceeded: false,
                    last_updated: chrono::Utc::now().timestamp(),
                },
                should_create_incident: false,
                fitness_test_results: Vec::new(),
                consumer_impact: None,
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
                metrics: mockforge_contracts::contract_drift::types::DriftMetrics {
                    endpoint: endpoint.to_string(),
                    method: method.to_string(),
                    breaking_changes: 0,
                    non_breaking_changes: diff_result.mismatches.len() as u32,
                    total_changes: diff_result.mismatches.len() as u32,
                    budget_exceeded: false,
                    last_updated: chrono::Utc::now().timestamp(),
                },
                should_create_incident: false,
                fitness_test_results: Vec::new(),
                consumer_impact: None,
            };
        }

        // Calculate baseline field count if percentage-based budget is enabled and tracker is available.
        // Awaiting directly avoids the Handle::block_on panic inside a runtime (#759).
        let baseline_field_count = if budget.max_field_churn_percent.is_some() {
            if let (Some(tracker), Some(time_window)) =
                (self.field_tracker.as_ref(), budget.time_window_days)
            {
                let guard = tracker.read().await;
                guard.get_average_count(
                    None, // workspace_id - could be passed through if needed
                    endpoint,
                    method,
                    time_window,
                )
            } else {
                None
            }
        } else {
            None
        };

        // Evaluate against budget
        let mut result = mockforge_contracts::contract_drift::types::drift_result_from_diff(
            diff_result,
            endpoint.to_string(),
            method.to_string(),
            &budget,
            &self.config.breaking_change_rules,
            baseline_field_count,
        );

        // Run fitness tests if registry is available
        if let Some(ref registry) = self.fitness_registry {
            let guard = registry.read().await;
            if let Ok(results) = guard.evaluate_all(
                old_spec,
                new_spec,
                diff_result,
                endpoint,
                method,
                workspace_id,
                service_name,
            ) {
                result.fitness_test_results = results;
                // If any fitness test fails, we should consider creating an incident
                if result.fitness_test_results.iter().any(|r| !r.passed) {
                    result.should_create_incident = true;
                }
            }
        }

        // Analyze consumer impact if analyzer is available
        if let Some(ref analyzer) = self.consumer_analyzer {
            let guard = analyzer.read().await;
            if let Some(impact) = guard.analyze_impact(endpoint, method) {
                result.consumer_impact = Some(impact);
            }
        }

        result
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
        self.config.default_budget.clone().unwrap_or_default()
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
                analyzed_at: chrono::Utc::now(),
                request_source: "budget_engine".to_string(),
                contract_version: Some("3.0.0".to_string()),
                contract_format: "openapi".to_string(),
                endpoint_path: String::new(),
                http_method: String::new(),
                request_count: 0,
                llm_provider: None,
                llm_model: None,
            },
        }
    }

    #[tokio::test]
    async fn test_budget_evaluation_no_mismatches() {
        let config = DriftBudgetConfig::default();
        let engine = DriftBudgetEngine::new(config);
        let diff_result = create_test_diff_result(vec![]);

        let result = engine.evaluate(&diff_result, "/api/users", "GET").await;

        assert!(!result.budget_exceeded);
        assert_eq!(result.breaking_changes, 0);
        assert_eq!(result.non_breaking_changes, 0);
        assert!(!result.should_create_incident);
    }

    /// #759: evaluating from inside a multi-threaded tokio runtime must not panic.
    /// The old `Handle::try_current()` + `handle.block_on(...)` pattern panicked
    /// ("Cannot start a runtime from within a runtime") whenever a percentage-based
    /// budget with a configured tracker drove the baseline lookup.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_evaluate_from_within_runtime_does_not_panic() {
        use mockforge_contracts::contract_drift::field_tracking::FieldCountTracker;
        use std::sync::Arc;

        let budget = DriftBudget {
            enabled: true,
            max_field_churn_percent: Some(10.0),
            time_window_days: Some(7),
            ..Default::default()
        };

        let config = DriftBudgetConfig {
            enabled: true,
            default_budget: Some(budget),
            ..Default::default()
        };

        let tracker = Arc::new(tokio::sync::RwLock::new(FieldCountTracker::default()));
        let engine = DriftBudgetEngine::new_with_tracker(config, tracker);

        let diff_result = create_test_diff_result(vec![]);
        // Would panic under the old block_on path; with `.await` it completes cleanly.
        let result = engine
            .evaluate_with_context(&diff_result, "/api/users", "GET", None, None, None)
            .await;
        assert!(!result.budget_exceeded);
    }

    #[tokio::test]
    async fn test_budget_evaluation_breaking_change() {
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
        let result = engine.evaluate(&diff_result, "/api/users", "POST").await;

        assert!(result.breaking_changes > 0);
        assert!(result.should_create_incident);
    }

    #[tokio::test]
    async fn test_budget_evaluation_non_breaking_change() {
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
        let result = engine.evaluate(&diff_result, "/api/users", "POST").await;

        assert_eq!(result.breaking_changes, 0);
        assert!(result.non_breaking_changes > 0);
        // Non-breaking changes within budget shouldn't create incidents
        assert!(!result.should_create_incident);
    }
}
