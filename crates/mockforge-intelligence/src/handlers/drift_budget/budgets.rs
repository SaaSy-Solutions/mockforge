//! Endpoint budgets and effective precedence lookup.

use super::DriftBudgetSource;
use super::DriftBudgetStateProvider;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use mockforge_contracts::contract_drift::types::DriftBudget;
use serde::{Deserialize, Serialize};

/// Request to create or update a drift budget
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateDriftBudgetRequest {
    /// Endpoint path
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// Maximum breaking changes allowed
    pub max_breaking_changes: Option<u32>,
    /// Maximum non-breaking changes allowed
    pub max_non_breaking_changes: Option<u32>,
    /// Severity threshold
    pub severity_threshold: Option<String>,
    /// Whether enabled
    pub enabled: Option<bool>,
    /// Workspace ID (optional)
    pub workspace_id: Option<String>,
}

/// Response for drift budget operations
#[derive(Debug, Serialize)]
pub struct DriftBudgetResponse {
    /// Budget ID
    pub id: String,
    /// Endpoint
    pub endpoint: String,
    /// Method
    pub method: String,
    /// Budget configuration
    pub budget: DriftBudget,
    /// Workspace ID
    pub workspace_id: Option<String>,
}

/// Get budget for a specific endpoint/workspace/service
///
/// GET /api/v1/drift/budgets/lookup?endpoint=/api/users&method=GET&workspace_id=...
#[derive(Debug, Deserialize)]
pub struct GetBudgetQuery {
    /// Endpoint path
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// Optional workspace ID
    pub workspace_id: Option<String>,
    /// Optional service name
    pub service_name: Option<String>,
    /// Optional comma-separated tags
    pub tags: Option<String>,
}

/// Create or update a drift budget
///
/// POST /api/v1/drift/budgets
pub async fn create_budget<S: DriftBudgetStateProvider>(
    State(_state): State<S>,
    Json(request): Json<CreateDriftBudgetRequest>,
) -> Result<Json<DriftBudgetResponse>, StatusCode> {
    let budget = DriftBudget {
        max_breaking_changes: request.max_breaking_changes.unwrap_or(0),
        max_non_breaking_changes: request.max_non_breaking_changes.unwrap_or(10),
        max_field_churn_percent: None,
        time_window_days: None,
        severity_threshold: request
            .severity_threshold
            .as_deref()
            .and_then(|s| match s.to_lowercase().as_str() {
                "critical" => Some(crate::ai_contract_diff::MismatchSeverity::Critical),
                "high" => Some(crate::ai_contract_diff::MismatchSeverity::High),
                "medium" => Some(crate::ai_contract_diff::MismatchSeverity::Medium),
                "low" => Some(crate::ai_contract_diff::MismatchSeverity::Low),
                _ => None,
            })
            .unwrap_or(crate::ai_contract_diff::MismatchSeverity::High),
        enabled: request.enabled.unwrap_or(true),
    };

    // Generate budget ID
    let budget_id = format!("{}:{}:{}", request.method, request.endpoint, uuid::Uuid::new_v4());

    // Build the key for this budget (matches the format used by list_budgets/get_budget)
    let key = format!("{} {}", request.method, request.endpoint);

    // Note: DriftBudgetEngine.config is not behind interior mutability (RwLock),
    // so budget creation is returned but not persisted in the running engine.
    // A future refactor should wrap config in RwLock for runtime mutation.
    let _ = key;

    Ok(Json(DriftBudgetResponse {
        id: budget_id,
        endpoint: request.endpoint,
        method: request.method,
        budget,
        workspace_id: request.workspace_id,
    }))
}

/// List drift budgets
///
/// GET /api/v1/drift/budgets
pub async fn list_budgets<S: DriftBudgetStateProvider>(
    State(state): State<S>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let config = state.engine().config();
    let budgets: Vec<serde_json::Value> = config
        .per_endpoint_budgets
        .iter()
        .map(|(key, budget)| {
            // Key format is "METHOD /path"
            let parts: Vec<&str> = key.splitn(2, ' ').collect();
            let (method, endpoint) = if parts.len() == 2 {
                (parts[0].to_string(), parts[1].to_string())
            } else {
                ("GET".to_string(), key.clone())
            };
            serde_json::json!({
                "id": key,
                "method": method,
                "endpoint": endpoint,
                "budget": {
                    "max_breaking_changes": budget.max_breaking_changes,
                    "max_non_breaking_changes": budget.max_non_breaking_changes,
                    "enabled": budget.enabled,
                }
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "budgets": budgets,
        "total": budgets.len(),
    })))
}

/// Get a specific drift budget
///
/// GET /api/v1/drift/budgets/{id}
pub async fn get_budget<S: DriftBudgetStateProvider>(
    State(state): State<S>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let config = state.engine().config();
    if let Some(budget) = config.per_endpoint_budgets.get(&id) {
        let parts: Vec<&str> = id.splitn(2, ' ').collect();
        let (method, endpoint) = if parts.len() == 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            ("GET".to_string(), id.clone())
        };
        Ok(Json(serde_json::json!({
            "id": id,
            "method": method,
            "endpoint": endpoint,
            "budget": {
                "max_breaking_changes": budget.max_breaking_changes,
                "max_non_breaking_changes": budget.max_non_breaking_changes,
                "enabled": budget.enabled,
            }
        })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Get budget for endpoint
///
/// GET /api/v1/drift/budgets/lookup
pub async fn get_budget_for_endpoint<S: DriftBudgetStateProvider>(
    State(state): State<S>,
    Query(params): Query<GetBudgetQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let tags = params
        .tags
        .as_ref()
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect::<Vec<_>>());

    let budget = state.engine().get_budget_for_endpoint(
        &params.endpoint,
        &params.method,
        params.workspace_id.as_deref(),
        params.service_name.as_deref(),
        tags.as_deref(),
    );

    Ok(Json(serde_json::json!({
        "endpoint": params.endpoint,
        "method": params.method,
        "workspace_id": params.workspace_id,
        "service_name": params.service_name,
        "budget": budget,
    })))
}
