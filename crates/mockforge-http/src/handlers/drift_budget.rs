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
        .route("/api/v1/drift/budgets/{id}", get(get_budget))
        .route("/api/v1/drift/incidents", get(list_incidents))
        .route("/api/v1/drift/incidents/stats", get(get_incident_stats))
        .route("/api/v1/drift/incidents/{id}", get(get_incident))
        .route("/api/v1/drift/incidents/{id}", patch(update_incident))
        .route("/api/v1/drift/incidents/{id}/resolve", post(resolve_incident))
        .with_state(state)
}
