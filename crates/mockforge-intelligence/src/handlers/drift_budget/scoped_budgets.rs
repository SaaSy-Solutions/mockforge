//! Workspace and service budget creation responses.

use super::DriftBudgetSource;
use super::DriftBudgetStateProvider;
use axum::{extract::State, http::StatusCode, response::Json};
use mockforge_contracts::contract_drift::types::DriftBudget;
use serde::{Deserialize, Serialize};

/// Request to create workspace/service/tag budget
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateWorkspaceBudgetRequest {
    /// Workspace ID
    pub workspace_id: String,
    /// Maximum allowed breaking changes
    pub max_breaking_changes: Option<u32>,
    /// Maximum allowed non-breaking changes
    pub max_non_breaking_changes: Option<u32>,
    /// Maximum field churn percentage
    pub max_field_churn_percent: Option<f64>,
    /// Time window in days
    pub time_window_days: Option<u32>,
    /// Whether the budget is enabled
    pub enabled: Option<bool>,
}

/// Request to create a service-level drift budget
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateServiceBudgetRequest {
    /// Service name
    pub service_name: String,
    /// Maximum allowed breaking changes
    pub max_breaking_changes: Option<u32>,
    /// Maximum allowed non-breaking changes
    pub max_non_breaking_changes: Option<u32>,
    /// Maximum field churn percentage
    pub max_field_churn_percent: Option<f64>,
    /// Time window in days
    pub time_window_days: Option<u32>,
    /// Whether the budget is enabled
    pub enabled: Option<bool>,
}

/// Create or update workspace budget
///
/// POST /api/v1/drift/budgets/workspace
pub async fn create_workspace_budget<S: DriftBudgetStateProvider>(
    State(state): State<S>,
    Json(request): Json<CreateWorkspaceBudgetRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let budget = DriftBudget {
        max_breaking_changes: request.max_breaking_changes.unwrap_or(0),
        max_non_breaking_changes: request.max_non_breaking_changes.unwrap_or(10),
        max_field_churn_percent: request.max_field_churn_percent,
        time_window_days: request.time_window_days,
        severity_threshold: crate::ai_contract_diff::MismatchSeverity::High,
        enabled: request.enabled.unwrap_or(true),
    };

    let mut config = state.engine().config().clone();
    config
        .per_workspace_budgets
        .insert(request.workspace_id.clone(), budget.clone());

    // Note: In a full implementation, this would persist to database
    // state.engine().update_config(config);

    Ok(Json(serde_json::json!({
        "workspace_id": request.workspace_id,
        "budget": budget,
    })))
}

/// Create or update service budget
///
/// POST /api/v1/drift/budgets/service
pub async fn create_service_budget<S: DriftBudgetStateProvider>(
    State(state): State<S>,
    Json(request): Json<CreateServiceBudgetRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let budget = DriftBudget {
        max_breaking_changes: request.max_breaking_changes.unwrap_or(0),
        max_non_breaking_changes: request.max_non_breaking_changes.unwrap_or(10),
        max_field_churn_percent: request.max_field_churn_percent,
        time_window_days: request.time_window_days,
        severity_threshold: crate::ai_contract_diff::MismatchSeverity::High,
        enabled: request.enabled.unwrap_or(true),
    };

    let mut config = state.engine().config().clone();
    config.per_service_budgets.insert(request.service_name.clone(), budget.clone());

    // Note: In a full implementation, this would persist to database
    // state.engine().update_config(config);

    Ok(Json(serde_json::json!({
        "service_name": request.service_name,
        "budget": budget,
    })))
}
