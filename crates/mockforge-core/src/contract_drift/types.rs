//! Core types for contract drift and budget management
//!
//! `DriftBudget`, `DriftBudgetConfig`, `DriftMetrics`, `BreakingChangeRule`,
//! `BreakingChangeRuleType`, and `BreakingChangeRuleConfig` are re-exported
//! from `mockforge_foundation::contract_drift_types` (Phase 6 / A6).
//!
//! `DriftResult` remains here because of cross-module dependencies on
//! `crate::contract_drift::fitness::FitnessTestResult` and
//! `crate::contract_drift::consumer_mapping::ConsumerImpact`.

use crate::ai_contract_diff::{ContractDiffResult, Mismatch};
pub use mockforge_foundation::contract_drift_types::{
    BreakingChangeRule, BreakingChangeRuleConfig, BreakingChangeRuleType, DriftBudget,
    DriftBudgetConfig, DriftMetrics,
};
use serde::{Deserialize, Serialize};

/// Result of drift budget evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftResult {
    /// Whether budget is exceeded
    pub budget_exceeded: bool,
    /// Number of breaking changes (definitely breaking)
    pub breaking_changes: u32,
    /// Number of potentially breaking changes (requires review)
    #[serde(default)]
    pub potentially_breaking_changes: u32,
    /// Number of non-breaking changes
    pub non_breaking_changes: u32,
    /// Mismatches that are considered breaking (definitely breaking)
    pub breaking_mismatches: Vec<Mismatch>,
    /// Mismatches that are potentially breaking (requires review)
    #[serde(default)]
    pub potentially_breaking_mismatches: Vec<Mismatch>,
    /// Mismatches that are non-breaking
    pub non_breaking_mismatches: Vec<Mismatch>,
    /// Current drift metrics
    pub metrics: DriftMetrics,
    /// Whether an incident should be created
    pub should_create_incident: bool,
    /// Results from fitness function tests
    #[serde(default)]
    pub fitness_test_results: Vec<crate::contract_drift::fitness::FitnessTestResult>,
    /// Consumer impact analysis (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consumer_impact: Option<crate::contract_drift::consumer_mapping::ConsumerImpact>,
}

impl DriftResult {
    /// Create a new drift result from contract diff result.
    ///
    /// `baseline_field_count` is optional and used for percentage-based budget calculations.
    /// If provided, it represents the historical baseline field count for the endpoint.
    pub fn from_diff_result(
        diff_result: &ContractDiffResult,
        endpoint: String,
        method: String,
        budget: &DriftBudget,
        breaking_rules: &[BreakingChangeRule],
        baseline_field_count: Option<f64>,
    ) -> Self {
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

        Self {
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
}
