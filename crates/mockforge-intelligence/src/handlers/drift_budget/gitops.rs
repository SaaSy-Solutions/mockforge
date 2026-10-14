//! GitOps incident selection and downstream pipeline event metadata.

use super::DriftBudgetStateProvider;
use super::DriftGitOps;
use crate::handlers::structural_incidents::StructuralIncidents;
use axum::{extract::State, http::StatusCode, response::Json};
use mockforge_foundation::incidents_types::{
    DriftIncident, IncidentQuery, IncidentStatus, IncidentType,
};
use serde::Deserialize;

/// Request to generate GitOps PR from incidents
#[derive(Debug, Deserialize)]
pub struct GeneratePRRequest {
    /// Optional list of specific incident IDs
    pub incident_ids: Option<Vec<String>>,
    /// Optional workspace ID filter
    pub workspace_id: Option<String>,
    /// Optional status filter (e.g., "open")
    pub status: Option<String>,
}

fn emit_threshold_event<S: DriftBudgetStateProvider>(
    state: &S,
    incidents: &[DriftIncident],
    workspace_override: Option<&str>,
) {
    let exceeded = incidents
        .iter()
        .filter(|incident| matches!(incident.incident_type, IncidentType::ThresholdExceeded))
        .count();
    if exceeded == 0 {
        return;
    }
    let workspace = workspace_override
        .and_then(|id| uuid::Uuid::parse_str(id).ok())
        .or_else(|| {
            incidents
                .first()
                .and_then(|incident| incident.workspace_id.as_deref())
                .and_then(|id| uuid::Uuid::parse_str(id).ok())
        })
        .unwrap_or_else(uuid::Uuid::new_v4);
    let endpoint = incidents
        .first()
        .map(|incident| format!("{} {}", incident.method, incident.endpoint))
        .unwrap_or_else(|| "unknown".to_string());
    state.publish_drift_threshold(workspace, endpoint, exceeded as i32, incidents.len() as i32);
}

/// Generate GitOps PR from drift incidents
///
/// POST /api/v1/drift/gitops/generate-pr
pub async fn generate_gitops_pr<S: DriftBudgetStateProvider>(
    State(state): State<S>,
    Json(request): Json<GeneratePRRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let handler = state.gitops_handler().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    // Get incidents to include in PR
    let mut query = IncidentQuery::default();

    if let Some(incident_ids) = &request.incident_ids {
        // Filter by specific incident IDs
        // Note: IncidentQuery doesn't support ID filtering yet, so we'll get all and filter
        let all_incidents = state.incident_manager().query_incidents(query).await;
        let incidents: Vec<_> =
            all_incidents.into_iter().filter(|inc| incident_ids.contains(&inc.id)).collect();

        match handler.generate_pr_from_incidents(&incidents).await {
            Ok(Some(pr_result)) => {
                emit_threshold_event(&state, &incidents, None);

                Ok(Json(serde_json::json!({
                    "success": true,
                    "pr": pr_result,
                })))
            }
            Ok(None) => Ok(Json(serde_json::json!({
                "success": false,
                "message": "No PR generated (no file changes or incidents)",
            }))),
            Err(_e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    } else {
        // Filter by workspace and/or status
        let workspace_id_str = request.workspace_id.clone();
        query.workspace_id = request.workspace_id;
        if let Some(status_str) = &request.status {
            query.status = match status_str.as_str() {
                "open" => Some(IncidentStatus::Open),
                "acknowledged" => Some(IncidentStatus::Acknowledged),
                _ => None,
            };
        }

        let incidents = state.incident_manager().query_incidents(query).await;

        match handler.generate_pr_from_incidents(&incidents).await {
            Ok(Some(pr_result)) => {
                emit_threshold_event(&state, &incidents, workspace_id_str.as_deref());

                Ok(Json(serde_json::json!({
                    "success": true,
                    "pr": pr_result,
                    "incidents_included": incidents.len(),
                })))
            }
            Ok(None) => Ok(Json(serde_json::json!({
                "success": false,
                "message": "No PR generated (no file changes or incidents)",
            }))),
            Err(_e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}
