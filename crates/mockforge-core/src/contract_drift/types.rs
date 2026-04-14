//! Core types for contract drift and budget management
//!
//! `DriftBudget`, `DriftBudgetConfig`, `DriftMetrics`, `BreakingChangeRule`,
//! `BreakingChangeRuleType`, `BreakingChangeRuleConfig`, `DriftResult`,
//! `FitnessTestResult`, and the consumer-mapping types are re-exported
//! from `mockforge_foundation::contract_drift_types` (Phase 6 / A6 / A13).
//!
//! The `drift_result_from_diff` helper lives in this file because it depends on
//! core-only types (`BreakingChangeDetector`, `ContractDiffResult`).

use crate::ai_contract_diff::ContractDiffResult;
pub use mockforge_foundation::contract_drift_types::{
    BreakingChangeRule, BreakingChangeRuleConfig, BreakingChangeRuleType, DriftBudget,
    DriftBudgetConfig, DriftMetrics, DriftResult,
};

/// Create a new drift result from a contract diff result.
///
/// `baseline_field_count` is optional and used for percentage-based budget
/// calculations. If provided, it represents the historical baseline field count
/// for the endpoint.
///
/// Lives here (not in foundation) because it depends on `BreakingChangeDetector`
/// and `ContractDiffResult`, which are only in core.
pub fn drift_result_from_diff(
    diff_result: &ContractDiffResult,
    endpoint: String,
    method: String,
    budget: &DriftBudget,
    breaking_rules: &[BreakingChangeRule],
    baseline_field_count: Option<f64>,
) -> DriftResult {
    use crate::contract_drift::BreakingChangeDetector;
    let detector = BreakingChangeDetector::new(breaking_rules.to_vec());
    let (non_breaking_mismatches, potentially_breaking_mismatches, breaking_mismatches) =
        detector.classify_three_way(&diff_result.mismatches);

    let breaking_count = breaking_mismatches.len() as u32;
    let potentially_breaking_count = potentially_breaking_mismatches.len() as u32;
    let non_breaking_count = non_breaking_mismatches.len() as u32;
    let total_changes = breaking_count + potentially_breaking_count + non_breaking_count;

    let budget_exceeded = if let Some(percent) = budget.max_field_churn_percent {
        if let Some(baseline) = baseline_field_count {
            let churn_percent = (total_changes as f64 / baseline) * 100.0;
            churn_percent > percent
        } else {
            breaking_count > budget.max_breaking_changes
                || non_breaking_count > budget.max_non_breaking_changes
        }
    } else {
        breaking_count > budget.max_breaking_changes
            || non_breaking_count > budget.max_non_breaking_changes
    };

    let metrics = DriftMetrics {
        endpoint,
        method,
        breaking_changes: breaking_count,
        non_breaking_changes: non_breaking_count,
        total_changes,
        budget_exceeded,
        last_updated: chrono::Utc::now().timestamp(),
    };

    DriftResult {
        budget_exceeded,
        breaking_changes: breaking_count,
        potentially_breaking_changes: potentially_breaking_count,
        non_breaking_changes: non_breaking_count,
        breaking_mismatches,
        potentially_breaking_mismatches,
        non_breaking_mismatches,
        metrics,
        should_create_incident: budget_exceeded && budget.enabled,
        fitness_test_results: Vec::new(),
        consumer_impact: None,
    }
}
