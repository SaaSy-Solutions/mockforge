//! Incident management handlers (cloud-enablement task #3 / Phase 1).
//!
//! Public CRUD over the incidents table. Internal raises from other
//! subsystems (#2 Observability alerts, #8 Contract drift,
//! hosted-mock health) call `Incident::raise` directly; an
//! `IncidentBus` trait abstraction comes in a follow-up slice when
//! those subsystems land.
//!
//! Routes:
//!   GET    /api/v1/organizations/{org_id}/incidents       (?status=&limit=)
//!   POST   /api/v1/organizations/{org_id}/incidents       (external/webhook raise)
//!   GET    /api/v1/incidents/{id}
//!   GET    /api/v1/incidents/{id}/events
//!   POST   /api/v1/incidents/{id}/acknowledge
//!   POST   /api/v1/incidents/{id}/resolve

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use mockforge_registry_core::models::incident::RaiseIncidentInput;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{Incident, IncidentEvent},
    AppState,
};

const DEFAULT_LIMIT: i64 = 100;
const MAX_LIMIT: i64 = 500;

#[derive(Debug, Deserialize)]
pub struct ListIncidentsQuery {
    /// Filter by `open` | `acknowledged` | `resolved`. Omit for all statuses.
    #[serde(default)]
    pub status: Option<String>,
    /// Max rows returned (1..=500, default 100).
    #[serde(default)]
    pub limit: Option<i64>,
}

/// `GET /api/v1/organizations/{org_id}/incidents`
pub async fn list_incidents(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    Query(query): Query<ListIncidentsQuery>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<Incident>>> {
    // Verify caller has access to this org.
    let ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != org_id {
        return Err(ApiError::InvalidRequest("Cannot read incidents for a different org".into()));
    }

    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let incidents = Incident::list_by_org(state.db.pool(), org_id, query.status.as_deref(), limit)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(incidents))
}

#[derive(Debug, Deserialize)]
pub struct ExternalRaiseRequest {
    pub source: String,
    #[serde(default)]
    pub source_ref: Option<String>,
    pub dedupe_key: String,
    pub severity: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<Uuid>,
}

/// `POST /api/v1/organizations/{org_id}/incidents`
///
/// External raise endpoint — used by CI / cron / external monitors to
/// push their own incidents. Subsystems inside the registry use
/// `Incident::raise` directly without going through this HTTP path.
pub async fn raise_incident_external(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ExternalRaiseRequest>,
) -> ApiResult<Json<Incident>> {
    let ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != org_id {
        return Err(ApiError::InvalidRequest("Cannot raise incidents for a different org".into()));
    }

    if request.source.trim().is_empty() {
        return Err(ApiError::InvalidRequest("source must not be empty".into()));
    }
    if request.dedupe_key.trim().is_empty() {
        return Err(ApiError::InvalidRequest("dedupe_key must not be empty".into()));
    }
    if request.title.trim().is_empty() {
        return Err(ApiError::InvalidRequest("title must not be empty".into()));
    }
    if !is_valid_severity(&request.severity) {
        return Err(ApiError::InvalidRequest(
            "severity must be 'critical', 'high', 'medium', or 'low'".into(),
        ));
    }

    let incident = Incident::raise(
        state.db.pool(),
        RaiseIncidentInput {
            org_id,
            workspace_id: request.workspace_id,
            source: &request.source,
            source_ref: request.source_ref.as_deref(),
            dedupe_key: &request.dedupe_key,
            severity: &request.severity,
            title: &request.title,
            description: request.description.as_deref(),
        },
    )
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(incident))
}

/// `GET /api/v1/incidents/{id}`
pub async fn get_incident(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Incident>> {
    let incident = load_authorized_incident(&state, user_id, &headers, id).await?;
    Ok(Json(incident))
}

/// `GET /api/v1/incidents/{id}/events`
pub async fn list_incident_events(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<IncidentEvent>>> {
    let incident = load_authorized_incident(&state, user_id, &headers, id).await?;
    let events = Incident::list_events(state.db.pool(), incident.id)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(events))
}

/// `POST /api/v1/incidents/{id}/acknowledge`
pub async fn acknowledge_incident(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Incident>> {
    // Authorize first so we don't leak existence to non-members.
    load_authorized_incident(&state, user_id, &headers, id).await?;
    let updated = Incident::acknowledge(state.db.pool(), id, user_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Incident not found".into()))?;
    Ok(Json(updated))
}

/// `POST /api/v1/incidents/{id}/resolve`
pub async fn resolve_incident(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Incident>> {
    load_authorized_incident(&state, user_id, &headers, id).await?;
    let updated = Incident::resolve(state.db.pool(), id, user_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Incident not found".into()))?;
    Ok(Json(updated))
}

/// Fetch an incident and verify the caller belongs to its org.
/// Returns `InvalidRequest("Incident not found")` for both "no row" and
/// "row exists but caller is in another org" — never confirm existence
/// to outsiders.
async fn load_authorized_incident(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    id: Uuid,
) -> ApiResult<Incident> {
    let incident = Incident::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Incident not found".into()))?;

    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != incident.org_id {
        return Err(ApiError::InvalidRequest("Incident not found".into()));
    }
    Ok(incident)
}

fn is_valid_severity(s: &str) -> bool {
    matches!(s, "critical" | "high" | "medium" | "low")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_validator_accepts_canonical_values() {
        assert!(is_valid_severity("critical"));
        assert!(is_valid_severity("high"));
        assert!(is_valid_severity("medium"));
        assert!(is_valid_severity("low"));
    }

    #[test]
    fn severity_validator_rejects_other_values() {
        assert!(!is_valid_severity("urgent"));
        assert!(!is_valid_severity("CRITICAL"));
        assert!(!is_valid_severity(""));
        assert!(!is_valid_severity("warning"));
    }
}
