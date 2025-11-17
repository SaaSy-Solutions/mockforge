//! Drift budget and incident management handlers
//!
//! This module provides HTTP handlers for managing drift budgets and incidents.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use mockforge_core::contract_drift::{
    DriftBudget, DriftBudgetConfig, DriftBudgetEngine, DriftResult,
};
use mockforge_core::incidents::{
    IncidentManager, IncidentQuery, IncidentSeverity, IncidentStatus, IncidentType,
};
use mockforge_core::incidents::types::DriftIncident;

/// State for drift budget handlers
#[derive(Clone)]
pub struct DriftBudgetState {
    /// Drift budget engine
    pub engine: Arc<DriftBudgetEngine>,
    /// Incident manager
    pub incident_manager: Arc<IncidentManager>,
    /// GitOps handler (optional)
    pub gitops_handler: Option<Arc<mockforge_core::drift_gitops::DriftGitOpsHandler>>,
}

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

/// Request to query incidents
#[derive(Debug, Deserialize)]
pub struct ListIncidentsRequest {
    /// Filter by status
    pub status: Option<String>,
    /// Filter by severity
    pub severity: Option<String>,
    /// Filter by endpoint
    pub endpoint: Option<String>,
    /// Filter by method
    pub method: Option<String>,
    /// Filter by incident type
    pub incident_type: Option<String>,
    /// Filter by workspace ID
    pub workspace_id: Option<String>,
    /// Limit results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Response for listing incidents
#[derive(Debug, Serialize)]
pub struct ListIncidentsResponse {
    /// List of incidents
    pub incidents: Vec<DriftIncident>,
    /// Total count
    pub total: usize,
}

/// Request to update incident status
#[derive(Debug, Deserialize)]
pub struct UpdateIncidentRequest {
    /// New status
    pub status: Option<String>,
    /// External ticket ID
    pub external_ticket_id: Option<String>,
    /// External ticket URL
    pub external_ticket_url: Option<String>,
}

/// Request to resolve incident
#[derive(Debug, Deserialize)]
pub struct ResolveIncidentRequest {
    /// Optional resolution note
    pub note: Option<String>,
}

/// Create or update a drift budget
///
/// POST /api/v1/drift/budgets
pub async fn create_budget(
    State(state): State<DriftBudgetState>,
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
                "critical" => Some(mockforge_core::ai_contract_diff::MismatchSeverity::Critical),
                "high" => Some(mockforge_core::ai_contract_diff::MismatchSeverity::High),
                "medium" => Some(mockforge_core::ai_contract_diff::MismatchSeverity::Medium),
                "low" => Some(mockforge_core::ai_contract_diff::MismatchSeverity::Low),
                _ => None,
            })
            .unwrap_or(mockforge_core::ai_contract_diff::MismatchSeverity::High),
        enabled: request.enabled.unwrap_or(true),
    };

    // Generate budget ID
    let budget_id = format!("{}:{}:{}", request.method, request.endpoint, uuid::Uuid::new_v4());

    // Update engine config with new budget
    let mut config = state.engine.config().clone();
    let key = format!("{} {}", request.method, request.endpoint);
    config.per_endpoint_budgets.insert(key, budget.clone());

    // Note: In a full implementation, this would persist to database
    // For now, we just update the engine config
    // state.engine.update_config(config);

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
pub async fn list_budgets(
    State(_state): State<DriftBudgetState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // In a full implementation, this would query from database
    // For now, return empty list
    Ok(Json(serde_json::json!({
        "budgets": []
    })))
}

/// Get a specific drift budget
///
/// GET /api/v1/drift/budgets/{id}
pub async fn get_budget(
    State(_state): State<DriftBudgetState>,
    Path(_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// Get budget for a specific endpoint/workspace/service
///
/// GET /api/v1/drift/budgets/lookup?endpoint=/api/users&method=GET&workspace_id=...
#[derive(Debug, Deserialize)]
pub struct GetBudgetQuery {
    pub endpoint: String,
    pub method: String,
    pub workspace_id: Option<String>,
    pub service_name: Option<String>,
    pub tags: Option<String>, // Comma-separated tags
}

/// Get budget for endpoint
///
/// GET /api/v1/drift/budgets/lookup
pub async fn get_budget_for_endpoint(
    State(state): State<DriftBudgetState>,
    Query(params): Query<GetBudgetQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let tags = params.tags.as_ref().map(|t| {
        t.split(',')
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>()
    });

    let budget = state.engine.get_budget_for_endpoint(
        &params.endpoint,
        &params.method,
        params.workspace_id.as_deref(),
        params.service_name.as_deref(),
        tags.as_deref().map(|t| t.as_slice()),
    );

    Ok(Json(serde_json::json!({
        "endpoint": params.endpoint,
        "method": params.method,
        "workspace_id": params.workspace_id,
        "service_name": params.service_name,
        "budget": budget,
    })))
}

/// Request to create workspace/service/tag budget
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateWorkspaceBudgetRequest {
    pub workspace_id: String,
    pub max_breaking_changes: Option<u32>,
    pub max_non_breaking_changes: Option<u32>,
    pub max_field_churn_percent: Option<f64>,
    pub time_window_days: Option<u32>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateServiceBudgetRequest {
    pub service_name: String,
    pub max_breaking_changes: Option<u32>,
    pub max_non_breaking_changes: Option<u32>,
    pub max_field_churn_percent: Option<f64>,
    pub time_window_days: Option<u32>,
    pub enabled: Option<bool>,
}

/// Create or update workspace budget
///
/// POST /api/v1/drift/budgets/workspace
pub async fn create_workspace_budget(
    State(state): State<DriftBudgetState>,
    Json(request): Json<CreateWorkspaceBudgetRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let budget = DriftBudget {
        max_breaking_changes: request.max_breaking_changes.unwrap_or(0),
        max_non_breaking_changes: request.max_non_breaking_changes.unwrap_or(10),
        max_field_churn_percent: request.max_field_churn_percent,
        time_window_days: request.time_window_days,
        severity_threshold: mockforge_core::ai_contract_diff::MismatchSeverity::High,
        enabled: request.enabled.unwrap_or(true),
    };

    let mut config = state.engine.config().clone();
    config.per_workspace_budgets.insert(request.workspace_id.clone(), budget.clone());

    // Note: In a full implementation, this would persist to database
    // state.engine.update_config(config);

    Ok(Json(serde_json::json!({
        "workspace_id": request.workspace_id,
        "budget": budget,
    })))
}

/// Create or update service budget
///
/// POST /api/v1/drift/budgets/service
pub async fn create_service_budget(
    State(state): State<DriftBudgetState>,
    Json(request): Json<CreateServiceBudgetRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let budget = DriftBudget {
        max_breaking_changes: request.max_breaking_changes.unwrap_or(0),
        max_non_breaking_changes: request.max_non_breaking_changes.unwrap_or(10),
        max_field_churn_percent: request.max_field_churn_percent,
        time_window_days: request.time_window_days,
        severity_threshold: mockforge_core::ai_contract_diff::MismatchSeverity::High,
        enabled: request.enabled.unwrap_or(true),
    };

    let mut config = state.engine.config().clone();
    config.per_service_budgets.insert(request.service_name.clone(), budget.clone());

    // Note: In a full implementation, this would persist to database
    // state.engine.update_config(config);

    Ok(Json(serde_json::json!({
        "service_name": request.service_name,
        "budget": budget,
    })))
}

/// Request to generate GitOps PR from incidents
#[derive(Debug, Deserialize)]
pub struct GeneratePRRequest {
    pub incident_ids: Option<Vec<String>>,
    pub workspace_id: Option<String>,
    pub status: Option<String>, // Filter by status (e.g., "open")
}

/// Generate GitOps PR from drift incidents
///
/// POST /api/v1/drift/gitops/generate-pr
pub async fn generate_gitops_pr(
    State(state): State<DriftBudgetState>,
    Json(request): Json<GeneratePRRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let handler = state
        .gitops_handler
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    // Get incidents to include in PR
    let mut query = IncidentQuery::default();
    
    if let Some(incident_ids) = &request.incident_ids {
        // Filter by specific incident IDs
        // Note: IncidentQuery doesn't support ID filtering yet, so we'll get all and filter
        let all_incidents = state.incident_manager.query_incidents(query).await;
        let incidents: Vec<_> = all_incidents
            .into_iter()
            .filter(|inc| incident_ids.contains(&inc.id))
            .collect();
        
        match handler.generate_pr_from_incidents(&incidents).await {
            Ok(Some(pr_result)) => Ok(Json(serde_json::json!({
                "success": true,
                "pr": pr_result,
            }))),
            Ok(None) => Ok(Json(serde_json::json!({
                "success": false,
                "message": "No PR generated (no file changes or incidents)",
            }))),
            Err(e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    } else {
        // Filter by workspace and/or status
        query.workspace_id = request.workspace_id;
        if let Some(status_str) = &request.status {
            query.status = match status_str.as_str() {
                "open" => Some(IncidentStatus::Open),
                "acknowledged" => Some(IncidentStatus::Acknowledged),
                _ => None,
            };
        }

        let incidents = state.incident_manager.query_incidents(query).await;
        
        match handler.generate_pr_from_incidents(&incidents).await {
            Ok(Some(pr_result)) => Ok(Json(serde_json::json!({
                "success": true,
                "pr": pr_result,
                "incidents_included": incidents.len(),
            }))),
            Ok(None) => Ok(Json(serde_json::json!({
                "success": false,
                "message": "No PR generated (no file changes or incidents)",
            }))),
            Err(e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}

/// Get drift metrics over time
///
/// GET /api/v1/drift/metrics?endpoint=/api/users&method=GET&days=30
#[derive(Debug, Deserialize)]
pub struct GetMetricsQuery {
    pub endpoint: Option<String>,
    pub method: Option<String>,
    pub workspace_id: Option<String>,
    pub days: Option<u32>, // Lookback window in days
}

/// Get drift metrics
///
/// GET /api/v1/drift/metrics
pub async fn get_drift_metrics(
    State(state): State<DriftBudgetState>,
    Query(params): Query<GetMetricsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Query incidents for metrics
    let mut query = IncidentQuery::default();
    query.endpoint = params.endpoint;
    query.method = params.method;
    query.workspace_id = params.workspace_id;
    
    // Filter by date range if days specified
    if let Some(days) = params.days {
        let start_date = chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::days(days as i64))
            .map(|dt| dt.timestamp())
            .unwrap_or(0);
        query.start_date = Some(start_date);
    }

    let incidents = state.incident_manager.query_incidents(query).await;

    // Calculate metrics
    let total_incidents = incidents.len();
    let breaking_changes = incidents
        .iter()
        .filter(|i| matches!(i.incident_type, IncidentType::BreakingChange))
        .count();
    let threshold_exceeded = total_incidents - breaking_changes;

    let by_severity: std::collections::HashMap<String, usize> = incidents
        .iter()
        .fold(std::collections::HashMap::new(), |mut acc, inc| {
            let key = format!("{:?}", inc.severity).to_lowercase();
            *acc.entry(key).or_insert(0) += 1;
            acc
        });

    Ok(Json(serde_json::json!({
        "total_incidents": total_incidents,
        "breaking_changes": breaking_changes,
        "threshold_exceeded": threshold_exceeded,
        "by_severity": by_severity,
        "incidents": incidents.iter().take(100).collect::<Vec<_>>(), // Limit to first 100
    })))
}

/// List incidents
///
/// GET /api/v1/drift/incidents
pub async fn list_incidents(
    State(state): State<DriftBudgetState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ListIncidentsResponse>, StatusCode> {
    let mut query = IncidentQuery::default();

    if let Some(status_str) = params.get("status") {
        query.status = match status_str.as_str() {
            "open" => Some(IncidentStatus::Open),
            "acknowledged" => Some(IncidentStatus::Acknowledged),
            "resolved" => Some(IncidentStatus::Resolved),
            "closed" => Some(IncidentStatus::Closed),
            _ => None,
        };
    }

    if let Some(severity_str) = params.get("severity") {
        query.severity = match severity_str.as_str() {
            "critical" => Some(IncidentSeverity::Critical),
            "high" => Some(IncidentSeverity::High),
            "medium" => Some(IncidentSeverity::Medium),
            "low" => Some(IncidentSeverity::Low),
            _ => None,
        };
    }

    if let Some(endpoint) = params.get("endpoint") {
        query.endpoint = Some(endpoint.clone());
    }

    if let Some(method) = params.get("method") {
        query.method = Some(method.clone());
    }

    if let Some(incident_type_str) = params.get("incident_type") {
        query.incident_type = match incident_type_str.as_str() {
            "breaking_change" => Some(IncidentType::BreakingChange),
            "threshold_exceeded" => Some(IncidentType::ThresholdExceeded),
            _ => None,
        };
    }

    if let Some(workspace_id) = params.get("workspace_id") {
        query.workspace_id = Some(workspace_id.clone());
    }

    if let Some(limit_str) = params.get("limit") {
        if let Ok(limit) = limit_str.parse() {
            query.limit = Some(limit);
        }
    }

    if let Some(offset_str) = params.get("offset") {
        if let Ok(offset) = offset_str.parse() {
            query.offset = Some(offset);
        }
    }

    let incidents = state.incident_manager.query_incidents(query).await;
    let total = incidents.len();

    Ok(Json(ListIncidentsResponse { incidents, total }))
}

/// Get a specific incident
///
/// GET /api/v1/drift/incidents/{id}
pub async fn get_incident(
    State(state): State<DriftBudgetState>,
    Path(id): Path<String>,
) -> Result<Json<DriftIncident>, StatusCode> {
    state
        .incident_manager
        .get_incident(&id)
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Update an incident
///
/// PATCH /api/v1/drift/incidents/{id}
pub async fn update_incident(
    State(state): State<DriftBudgetState>,
    Path(id): Path<String>,
    Json(request): Json<UpdateIncidentRequest>,
) -> Result<Json<DriftIncident>, StatusCode> {
    let mut incident = state
        .incident_manager
        .get_incident(&id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(status_str) = request.status {
        match status_str.as_str() {
            "acknowledged" => {
                incident = state.incident_manager.acknowledge_incident(&id).await.ok_or(StatusCode::NOT_FOUND)?;
            }
            "resolved" => {
                incident = state.incident_manager.resolve_incident(&id).await.ok_or(StatusCode::NOT_FOUND)?;
            }
            "closed" => {
                incident = state.incident_manager.close_incident(&id).await.ok_or(StatusCode::NOT_FOUND)?;
            }
            _ => {}
        }
    }

    if let Some(ticket_id) = request.external_ticket_id {
        incident = state
            .incident_manager
            .link_external_ticket(&id, ticket_id, request.external_ticket_url)
            .await
            .ok_or(StatusCode::NOT_FOUND)?;
    }

    Ok(Json(incident))
}

/// Resolve an incident
///
/// POST /api/v1/drift/incidents/{id}/resolve
pub async fn resolve_incident(
    State(state): State<DriftBudgetState>,
    Path(id): Path<String>,
    Json(_request): Json<ResolveIncidentRequest>,
) -> Result<Json<DriftIncident>, StatusCode> {
    state
        .incident_manager
        .resolve_incident(&id)
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Get incident statistics
///
/// GET /api/v1/drift/incidents/stats
pub async fn get_incident_stats(
    State(state): State<DriftBudgetState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let stats = state.incident_manager.get_statistics().await;
    Ok(Json(serde_json::json!({
        "stats": stats
    })))
}

/// Create drift budget router
pub fn drift_budget_router(state: DriftBudgetState) -> axum::Router {
    use axum::routing::{get, patch, post};

    axum::Router::new()
        .route("/api/v1/drift/budgets", post(create_budget))
        .route("/api/v1/drift/budgets", get(list_budgets))
        .route("/api/v1/drift/budgets/lookup", get(get_budget_for_endpoint))
        .route("/api/v1/drift/budgets/workspace", post(create_workspace_budget))
        .route("/api/v1/drift/budgets/service", post(create_service_budget))
        .route("/api/v1/drift/budgets/{id}", get(get_budget))
        .route("/api/v1/drift/incidents", get(list_incidents))
        .route("/api/v1/drift/incidents/stats", get(get_incident_stats))
        .route("/api/v1/drift/incidents/{id}", get(get_incident))
        .route("/api/v1/drift/incidents/{id}", patch(update_incident))
        .route("/api/v1/drift/incidents/{id}/resolve", post(resolve_incident))
        .route("/api/v1/drift/gitops/generate-pr", post(generate_gitops_pr))
        .route("/api/v1/drift/metrics", get(get_drift_metrics))
        .with_state(state)
}
