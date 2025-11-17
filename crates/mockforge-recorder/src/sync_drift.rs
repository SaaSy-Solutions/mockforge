//! Drift budget integration for sync operations
//!
//! This module provides functionality to evaluate drift budgets when sync changes are detected
//! and create incidents with before/after samples.

use crate::{database::RecorderDatabase, sync::DetectedChange, Result};
use mockforge_core::{
    contract_drift::{DriftBudgetEngine, DriftResult},
    incidents::{IncidentManager, IncidentSeverity, IncidentType},
};
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Drift budget integration for sync operations
pub struct SyncDriftEvaluator {
    drift_engine: Arc<DriftBudgetEngine>,
    incident_manager: Arc<IncidentManager>,
    database: Arc<RecorderDatabase>,
}

impl SyncDriftEvaluator {
    /// Create a new sync drift evaluator
    pub fn new(
        drift_engine: Arc<DriftBudgetEngine>,
        incident_manager: Arc<IncidentManager>,
        database: Arc<RecorderDatabase>,
    ) -> Self {
        Self {
            drift_engine,
            incident_manager,
            database,
        }
    }

    /// Evaluate drift budget for sync changes and create incidents if needed
    ///
    /// This method processes detected changes from sync operations, evaluates them against
    /// drift budgets, and creates incidents with before/after samples when budgets are exceeded.
    pub async fn evaluate_sync_changes(
        &self,
        changes: &[DetectedChange],
        sync_cycle_id: &str,
        workspace_id: Option<&str>,
        service_name: Option<&str>,
        tags: Option<&[String]>,
    ) -> Result<Vec<String>> {
        let mut incident_ids = Vec::new();

        for change in changes {
            // Get before/after samples from database
            let (before_sample, after_sample) =
                self.get_before_after_samples(change).await.unwrap_or((None, None));

            // Evaluate drift budget for this change
            // Note: We need to convert DetectedChange to a format that can be evaluated
            // For now, we'll create a simplified evaluation based on the comparison result
            let drift_result = self
                .evaluate_change_against_budget(change, workspace_id, service_name, tags)
                .await;

            // Create incident if budget is exceeded
            if drift_result.should_create_incident {
                let incident_id = self
                    .create_incident_from_change(
                        change,
                        &drift_result,
                        sync_cycle_id,
                        workspace_id,
                        before_sample,
                        after_sample,
                    )
                    .await?;

                incident_ids.push(incident_id);
            }
        }

        if !incident_ids.is_empty() {
            info!(
                "Created {} drift incidents from sync cycle {}",
                incident_ids.len(),
                sync_cycle_id
            );
        }

        Ok(incident_ids)
    }

    /// Evaluate a single change against drift budget
    async fn evaluate_change_against_budget(
        &self,
        change: &DetectedChange,
        workspace_id: Option<&str>,
        service_name: Option<&str>,
        tags: Option<&[String]>,
    ) -> DriftResult {
        // For sync changes, we evaluate based on the comparison result
        // The comparison result contains information about differences
        let differences_count = change.comparison.differences.len() as u32;

        // Create a simplified drift result based on differences
        // In a full implementation, we'd convert the comparison result to a ContractDiffResult
        // For now, we'll use a heuristic based on the number of differences

        // Check if this looks like a breaking change based on difference types
        let breaking_changes = change
            .comparison
            .differences
            .iter()
            .filter(|diff| {
                // Heuristic: structural changes, missing fields, type changes are breaking
                diff.description.to_lowercase().contains("missing")
                    || diff.description.to_lowercase().contains("type")
                    || diff.description.to_lowercase().contains("removed")
            })
            .count() as u32;

        let non_breaking_changes = differences_count.saturating_sub(breaking_changes);

        // Get budget for this endpoint
        let budget = self.drift_engine.get_budget_for_endpoint(
            &change.path,
            &change.method,
            workspace_id,
            service_name,
            tags,
        );

        // Check if budget is exceeded
        let budget_exceeded = if let Some(max_churn_percent) = budget.max_field_churn_percent {
            // Simplified percentage calculation
            // In a full implementation, we'd track baseline field counts
            let baseline = 100.0; // Placeholder
            let churn_percent = (differences_count as f64 / baseline) * 100.0;
            churn_percent > max_churn_percent || breaking_changes > budget.max_breaking_changes
        } else {
            breaking_changes > budget.max_breaking_changes
                || non_breaking_changes > budget.max_non_breaking_changes
        };

        // Create a simplified DriftResult
        // Note: In a full implementation, we'd convert ComparisonResult to ContractDiffResult
        // and use the proper evaluation method
        DriftResult {
            budget_exceeded,
            breaking_changes,
            potentially_breaking_changes: 0,
            non_breaking_changes,
            breaking_mismatches: vec![],
            potentially_breaking_mismatches: vec![],
            non_breaking_mismatches: vec![],
            metrics: mockforge_core::contract_drift::types::DriftMetrics {
                endpoint: change.path.clone(),
                method: change.method.clone(),
                breaking_changes,
                non_breaking_changes,
                total_changes: differences_count,
                budget_exceeded,
                last_updated: chrono::Utc::now().timestamp(),
            },
            should_create_incident: budget_exceeded || breaking_changes > 0,
            fitness_test_results: vec![],
            consumer_impact: None,
        }
    }

    /// Get before/after samples for a change
    async fn get_before_after_samples(
        &self,
        change: &DetectedChange,
    ) -> Result<(Option<Value>, Option<Value>)> {
        // Get the request and response from database
        let request = self.database.get_request(&change.request_id).await?.ok_or_else(|| {
            crate::RecorderError::NotFound(format!("Request {} not found", change.request_id))
        })?;

        let response = self.database.get_response(&change.request_id).await?.ok_or_else(|| {
            crate::RecorderError::NotFound(format!(
                "Response for request {} not found",
                change.request_id
            ))
        })?;

        // Create before sample (original state)
        let before_sample = serde_json::json!({
            "method": request.method,
            "path": request.path,
            "headers": request.headers,
            "body": request.body,
            "response": {
                "status_code": response.status_code,
                "headers": response.headers,
                "body": response.body,
            },
        });

        // Create after sample (new state from comparison)
        // Note: ComparisonResult doesn't store the new response directly,
        // so we'll include the differences to show what changed
        let after_sample = serde_json::json!({
            "method": request.method,
            "path": request.path,
            "headers": request.headers,
            "body": request.body,
            "response": {
                "status_code": response.status_code,
                "headers": response.headers,
                "body": response.body,
            },
            "differences": change.comparison.differences,
            "comparison_summary": change.comparison.summary,
        });

        Ok((Some(before_sample), Some(after_sample)))
    }

    /// Create an incident from a sync change
    async fn create_incident_from_change(
        &self,
        change: &DetectedChange,
        drift_result: &DriftResult,
        sync_cycle_id: &str,
        workspace_id: Option<&str>,
        before_sample: Option<Value>,
        after_sample: Option<Value>,
    ) -> Result<String> {
        let incident_type = if drift_result.breaking_changes > 0 {
            IncidentType::BreakingChange
        } else {
            IncidentType::ThresholdExceeded
        };

        let severity = self.determine_severity(drift_result);

        let details = serde_json::json!({
            "breaking_changes": drift_result.breaking_changes,
            "potentially_breaking_changes": drift_result.potentially_breaking_changes,
            "non_breaking_changes": drift_result.non_breaking_changes,
            "budget_exceeded": drift_result.budget_exceeded,
            "differences_count": change.comparison.differences.len(),
            "comparison_summary": change.comparison.differences.iter().map(|d| &d.description).collect::<Vec<_>>(),
        });

        let incident = self
            .incident_manager
            .create_incident_with_samples(
                change.path.clone(),
                change.method.clone(),
                incident_type,
                severity,
                details,
                None, // budget_id
                workspace_id.map(|s| s.to_string()),
                Some(sync_cycle_id.to_string()),
                None, // contract_diff_id
                before_sample,
                after_sample,
            )
            .await;

        info!(
            "Created drift incident {} for {} {} from sync cycle {}",
            incident.id, change.method, change.path, sync_cycle_id
        );

        Ok(incident.id)
    }

    /// Determine incident severity from drift result
    fn determine_severity(&self, drift_result: &DriftResult) -> IncidentSeverity {
        if drift_result.breaking_changes > 0 {
            if drift_result.breaking_changes > 5 {
                IncidentSeverity::Critical
            } else if drift_result.breaking_changes > 2 {
                IncidentSeverity::High
            } else {
                IncidentSeverity::Medium
            }
        } else if drift_result.non_breaking_changes > 10 {
            IncidentSeverity::Medium
        } else {
            IncidentSeverity::Low
        }
    }
}
