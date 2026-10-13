//! Structural incident filtering, lifecycle and external ticket attachment.

use super::DriftBudgetStateProvider;
use crate::handlers::structural_incidents::StructuralIncidents;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use mockforge_foundation::incidents_types::{
    DriftIncident, IncidentQuery, IncidentSeverity, IncidentStatus, IncidentType,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// List incidents
///
/// GET /api/v1/drift/incidents
pub async fn list_incidents<S: DriftBudgetStateProvider>(
    State(state): State<S>,
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

    let incidents = state.incident_manager().query_incidents(query).await;
    let total = incidents.len();

    Ok(Json(ListIncidentsResponse { incidents, total }))
}

/// Get a specific incident
///
/// GET /api/v1/drift/incidents/{id}
pub async fn get_incident<S: DriftBudgetStateProvider>(
    State(state): State<S>,
    Path(id): Path<String>,
) -> Result<Json<DriftIncident>, StatusCode> {
    state
        .incident_manager()
        .get_incident(&id)
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Update an incident
///
/// PATCH /api/v1/drift/incidents/{id}
pub async fn update_incident<S: DriftBudgetStateProvider>(
    State(state): State<S>,
    Path(id): Path<String>,
    Json(request): Json<UpdateIncidentRequest>,
) -> Result<Json<DriftIncident>, StatusCode> {
    let mut incident =
        state.incident_manager().get_incident(&id).await.ok_or(StatusCode::NOT_FOUND)?;

    if let Some(status_str) = request.status {
        match status_str.as_str() {
            "acknowledged" => {
                incident = state
                    .incident_manager()
                    .acknowledge_incident(&id)
                    .await
                    .ok_or(StatusCode::NOT_FOUND)?;
            }
            "resolved" => {
                incident = state
                    .incident_manager()
                    .resolve_incident(&id)
                    .await
                    .ok_or(StatusCode::NOT_FOUND)?;
            }
            "closed" => {
                incident = state
                    .incident_manager()
                    .close_incident(&id)
                    .await
                    .ok_or(StatusCode::NOT_FOUND)?;
            }
            other => {
                tracing::warn!(
                    "Invalid incident status '{}': expected acknowledged, resolved, or closed",
                    other
                );
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }

    if let Some(ticket_id) = request.external_ticket_id {
        incident = state
            .incident_manager()
            .link_external_ticket(&id, ticket_id, request.external_ticket_url)
            .await
            .ok_or(StatusCode::NOT_FOUND)?;
    }

    Ok(Json(incident))
}

/// Resolve an incident
///
/// POST /api/v1/drift/incidents/{id}/resolve
pub async fn resolve_incident<S: DriftBudgetStateProvider>(
    State(state): State<S>,
    Path(id): Path<String>,
    Json(_request): Json<ResolveIncidentRequest>,
) -> Result<Json<DriftIncident>, StatusCode> {
    state
        .incident_manager()
        .resolve_incident(&id)
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Get incident statistics
///
/// GET /api/v1/drift/incidents/stats
pub async fn get_incident_stats<S: DriftBudgetStateProvider>(
    State(state): State<S>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let stats = state.incident_manager().get_statistics().await;
    Ok(Json(serde_json::json!({
        "stats": stats
    })))
}
