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

/// `GET /api/v1/organizations/{org_id}/incidents/stats`
///
/// Counts + rolling-30-day MTTR for the dashboard. One query per
/// dimension (status × severity) to keep the SQL straightforward and
/// the response shape stable as we add severities later.
#[derive(Debug, serde::Serialize)]
pub struct IncidentStats {
    pub open: SeverityBreakdown,
    pub resolved_30d: SeverityBreakdown,
    /// Mean-time-to-resolve in seconds, computed over the last 30
    /// days of resolved incidents. Null when there are no resolved
    /// incidents in that window.
    pub mttr_seconds_30d: Option<i64>,
    /// Number of dispatch attempts in the last 24h (success + failure
    /// — see incident_events.event_type = 'notification_sent').
    pub notification_attempts_24h: i64,
}

#[derive(Debug, Default, serde::Serialize)]
pub struct SeverityBreakdown {
    pub total: i64,
    pub critical: i64,
    pub high: i64,
    pub medium: i64,
    pub low: i64,
}

pub async fn get_stats(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<IncidentStats>> {
    let ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != org_id {
        return Err(ApiError::InvalidRequest("Cannot read stats for a different org".into()));
    }
    let pool = state.db.pool();

    let open_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT severity, COUNT(*) FROM incidents \
         WHERE org_id = $1 AND status = 'open' GROUP BY severity",
    )
    .bind(org_id)
    .fetch_all(pool)
    .await
    .map_err(ApiError::Database)?;
    let open = breakdown_from_rows(&open_rows);

    let resolved_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT severity, COUNT(*) FROM incidents \
         WHERE org_id = $1 AND status = 'resolved' \
           AND resolved_at >= NOW() - INTERVAL '30 days' GROUP BY severity",
    )
    .bind(org_id)
    .fetch_all(pool)
    .await
    .map_err(ApiError::Database)?;
    let resolved_30d = breakdown_from_rows(&resolved_rows);

    let mttr: Option<f64> = sqlx::query_scalar(
        "SELECT AVG(EXTRACT(EPOCH FROM (resolved_at - created_at))) \
         FROM incidents \
         WHERE org_id = $1 AND status = 'resolved' \
           AND resolved_at IS NOT NULL \
           AND resolved_at >= NOW() - INTERVAL '30 days'",
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .map_err(ApiError::Database)?;
    let mttr_seconds_30d = mttr.map(|s| s as i64);

    let attempts: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM incident_events e \
            JOIN incidents i ON i.id = e.incident_id \
          WHERE i.org_id = $1 \
            AND e.event_type = 'notification_sent' \
            AND e.created_at >= NOW() - INTERVAL '24 hours'",
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(IncidentStats {
        open,
        resolved_30d,
        mttr_seconds_30d,
        notification_attempts_24h: attempts,
    }))
}

fn breakdown_from_rows(rows: &[(String, i64)]) -> SeverityBreakdown {
    let mut b = SeverityBreakdown::default();
    for (sev, n) in rows {
        b.total += n;
        match sev.as_str() {
            "critical" => b.critical = *n,
            "high" => b.high = *n,
            "medium" => b.medium = *n,
            "low" => b.low = *n,
            _ => {}
        }
    }
    b
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
