//! Observability saved-queries + dashboards handlers
//! (cloud-enablement task #2 / Phase 1).
//!
//! Cross-deployment query handlers themselves live in a follow-up
//! slice — this file owns the persistence layer for users' named
//! filters and dashboard layouts, which is enough for the UI to render
//! the "saved searches" sidebar and the dashboard list.
//!
//! Routes:
//!   GET    /api/v1/organizations/{org_id}/observability/saved-queries[?kind=]
//!   POST   /api/v1/organizations/{org_id}/observability/saved-queries
//!   PATCH  /api/v1/observability/saved-queries/{id}
//!   DELETE /api/v1/observability/saved-queries/{id}
//!
//!   GET    /api/v1/organizations/{org_id}/observability/dashboards
//!   POST   /api/v1/organizations/{org_id}/observability/dashboards
//!   PATCH  /api/v1/observability/dashboards/{id}
//!   DELETE /api/v1/observability/dashboards/{id}

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use mockforge_registry_core::models::observability_query::{CreateDashboard, CreateSavedQuery};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{ObservabilityDashboard, ObservabilitySavedQuery},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct ListQueriesQuery {
    #[serde(default)]
    pub kind: Option<String>,
}

/// `GET /api/v1/organizations/{org_id}/observability/saved-queries`
pub async fn list_saved_queries(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    Query(query): Query<ListQueriesQuery>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<ObservabilitySavedQuery>>> {
    authorize_org(&state, user_id, &headers, org_id).await?;
    let rows = ObservabilitySavedQuery::list_by_org(state.db.pool(), org_id, query.kind.as_deref())
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
pub struct CreateSavedQueryRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub kind: String,
    pub filters: serde_json::Value,
    #[serde(default)]
    pub workspace_id: Option<Uuid>,
}

/// `POST /api/v1/organizations/{org_id}/observability/saved-queries`
pub async fn create_saved_query(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreateSavedQueryRequest>,
) -> ApiResult<Json<ObservabilitySavedQuery>> {
    authorize_org(&state, user_id, &headers, org_id).await?;
    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("name must not be empty".into()));
    }
    if !ObservabilitySavedQuery::is_valid_kind(&request.kind) {
        return Err(ApiError::InvalidRequest(format!(
            "kind must be one of: {}",
            ObservabilitySavedQuery::VALID_KINDS.join(", ")
        )));
    }

    let row = ObservabilitySavedQuery::create(
        state.db.pool(),
        CreateSavedQuery {
            org_id,
            workspace_id: request.workspace_id,
            name: &request.name,
            description: request.description.as_deref(),
            kind: &request.kind,
            filters: &request.filters,
            created_by: Some(user_id),
        },
    )
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(row))
}

#[derive(Debug, Deserialize)]
pub struct UpdateSavedQueryRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub filters: Option<serde_json::Value>,
}

/// `PATCH /api/v1/observability/saved-queries/{id}`
pub async fn update_saved_query(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<UpdateSavedQueryRequest>,
) -> ApiResult<Json<ObservabilitySavedQuery>> {
    let existing = load_authorized_query(&state, user_id, &headers, id).await?;
    let _ = existing;
    let updated = ObservabilitySavedQuery::update(
        state.db.pool(),
        id,
        request.name.as_deref(),
        request.filters.as_ref(),
    )
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::InvalidRequest("Saved query not found".into()))?;
    Ok(Json(updated))
}

/// `DELETE /api/v1/observability/saved-queries/{id}`
pub async fn delete_saved_query(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    load_authorized_query(&state, user_id, &headers, id).await?;
    let deleted = ObservabilitySavedQuery::delete(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?;
    if !deleted {
        return Err(ApiError::InvalidRequest("Saved query not found".into()));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}

// --- dashboards ------------------------------------------------------------

/// `GET /api/v1/organizations/{org_id}/observability/dashboards`
pub async fn list_dashboards(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<ObservabilityDashboard>>> {
    authorize_org(&state, user_id, &headers, org_id).await?;
    let rows = ObservabilityDashboard::list_by_org(state.db.pool(), org_id)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
pub struct CreateDashboardRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub layout: serde_json::Value,
    pub queries: serde_json::Value,
    #[serde(default)]
    pub workspace_id: Option<Uuid>,
}

/// `POST /api/v1/organizations/{org_id}/observability/dashboards`
pub async fn create_dashboard(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreateDashboardRequest>,
) -> ApiResult<Json<ObservabilityDashboard>> {
    authorize_org(&state, user_id, &headers, org_id).await?;
    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("name must not be empty".into()));
    }
    let row = ObservabilityDashboard::create(
        state.db.pool(),
        CreateDashboard {
            org_id,
            workspace_id: request.workspace_id,
            name: &request.name,
            description: request.description.as_deref(),
            layout: &request.layout,
            queries: &request.queries,
            created_by: Some(user_id),
        },
    )
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(row))
}

#[derive(Debug, Deserialize)]
pub struct UpdateDashboardRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub layout: Option<serde_json::Value>,
    #[serde(default)]
    pub queries: Option<serde_json::Value>,
}

/// `PATCH /api/v1/observability/dashboards/{id}`
pub async fn update_dashboard(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<UpdateDashboardRequest>,
) -> ApiResult<Json<ObservabilityDashboard>> {
    load_authorized_dashboard(&state, user_id, &headers, id).await?;
    let updated = ObservabilityDashboard::update(
        state.db.pool(),
        id,
        request.name.as_deref(),
        request.layout.as_ref(),
        request.queries.as_ref(),
    )
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::InvalidRequest("Dashboard not found".into()))?;
    Ok(Json(updated))
}

/// `DELETE /api/v1/observability/dashboards/{id}`
pub async fn delete_dashboard(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    load_authorized_dashboard(&state, user_id, &headers, id).await?;
    let deleted = ObservabilityDashboard::delete(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?;
    if !deleted {
        return Err(ApiError::InvalidRequest("Dashboard not found".into()));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn authorize_org(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    org_id: Uuid,
) -> ApiResult<()> {
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != org_id {
        return Err(ApiError::InvalidRequest(
            "Cannot access observability for a different org".into(),
        ));
    }
    Ok(())
}

async fn load_authorized_query(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    id: Uuid,
) -> ApiResult<ObservabilitySavedQuery> {
    let row = ObservabilitySavedQuery::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Saved query not found".into()))?;
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != row.org_id {
        return Err(ApiError::InvalidRequest("Saved query not found".into()));
    }
    Ok(row)
}

async fn load_authorized_dashboard(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    id: Uuid,
) -> ApiResult<ObservabilityDashboard> {
    let row = ObservabilityDashboard::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Dashboard not found".into()))?;
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != row.org_id {
        return Err(ApiError::InvalidRequest("Dashboard not found".into()));
    }
    Ok(row)
}

// --- cross-deployment trace query ------------------------------------------

/// One trace span row. Mirrors the runtime_traces table minus the
/// internal `id` / `received_at` plumbing the UI doesn't need.
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct TraceSpanRow {
    pub deployment_id: Uuid,
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub service_name: Option<String>,
    pub name: String,
    pub kind: Option<i16>,
    pub start_unix_nano: i64,
    pub end_unix_nano: i64,
    pub occurred_at: chrono::DateTime<chrono::Utc>,
    pub status_code: Option<i16>,
    pub status_message: Option<String>,
    pub attributes: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct TraceQueryRequest {
    /// Restrict to one deployment. None = all deployments in the org.
    #[serde(default)]
    pub deployment_id: Option<Uuid>,
    /// Filter by service_name (exact match — the OTel resource attr).
    #[serde(default)]
    pub service_name: Option<String>,
    /// Free-text name filter (LIKE %query%).
    #[serde(default)]
    pub name_contains: Option<String>,
    /// Status filter: "ok" | "error" | "any" (default).
    #[serde(default)]
    pub status: Option<String>,
    /// ISO-8601; defaults to 1h ago.
    #[serde(default)]
    pub since: Option<chrono::DateTime<chrono::Utc>>,
    /// ISO-8601; defaults to now.
    #[serde(default)]
    pub until: Option<chrono::DateTime<chrono::Utc>>,
    /// Page size, capped at 500.
    #[serde(default)]
    pub limit: Option<i64>,
}

/// `POST /api/v1/organizations/{org_id}/observability/traces/query`
///
/// Cross-deployment trace search scoped to the caller's org. Joins
/// `runtime_traces` against `hosted_mocks` so the org_id check is one
/// authoritative WHERE clause, not a per-row filter the caller could
/// fool. Runs as POST (not GET) because the filter set is too wide for
/// a sane query string.
pub async fn query_traces(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<TraceQueryRequest>,
) -> ApiResult<Json<Vec<TraceSpanRow>>> {
    authorize_org(&state, user_id, &headers, org_id).await?;

    let limit = req.limit.unwrap_or(200).clamp(1, 500);
    let until = req.until.unwrap_or_else(chrono::Utc::now);
    let since = req.since.unwrap_or_else(|| chrono::Utc::now() - chrono::Duration::hours(1));
    if until < since {
        return Err(ApiError::InvalidRequest("until must be >= since".into()));
    }

    let status_filter: Option<i16> = match req.status.as_deref() {
        Some("ok") => Some(1),
        Some("error") => Some(2),
        Some("any") | None => None,
        Some(other) => {
            return Err(ApiError::InvalidRequest(format!(
                "status must be 'ok', 'error', or 'any' — got '{other}'"
            )));
        }
    };

    let name_pattern: Option<String> = req
        .name_contains
        .as_ref()
        .map(|s| format!("%{}%", s.replace('%', r"\%").replace('_', r"\_")));

    let rows = sqlx::query_as::<_, TraceSpanRow>(
        r#"
        SELECT t.deployment_id, t.trace_id, t.span_id, t.parent_span_id,
               t.service_name, t.name, t.kind,
               t.start_unix_nano, t.end_unix_nano, t.occurred_at,
               t.status_code, t.status_message, t.attributes
          FROM runtime_traces t
          JOIN hosted_mocks d ON d.id = t.deployment_id
         WHERE d.org_id = $1
           AND t.occurred_at >= $2
           AND t.occurred_at <= $3
           AND ($4::uuid IS NULL OR t.deployment_id = $4)
           AND ($5::text IS NULL OR t.service_name = $5)
           AND ($6::text IS NULL OR t.name ILIKE $6)
           AND ($7::int2 IS NULL OR t.status_code = $7)
         ORDER BY t.occurred_at DESC
         LIMIT $8
        "#,
    )
    .bind(org_id)
    .bind(since)
    .bind(until)
    .bind(req.deployment_id)
    .bind(req.service_name)
    .bind(name_pattern)
    .bind(status_filter)
    .bind(limit)
    .fetch_all(state.db.pool())
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(rows))
}

// --- saved-query execution (#465) ------------------------------------------

/// One result bucket. `label` is the group key for grouped queries
/// (e.g. status code "200", "404") or `"all"` for ungrouped totals.
#[derive(Debug, serde::Serialize)]
pub struct QueryBucket {
    pub label: String,
    pub count: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct ExecuteQueryResponse {
    pub metric: String,
    pub total: i64,
    pub window_minutes: i64,
    pub series: Vec<QueryBucket>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ExecuteQueryRequest {
    /// Override the saved query's workspace scope. None falls back to
    /// `SavedQuery.workspace_id`. None on both means org-wide.
    #[serde(default)]
    pub workspace_id: Option<Uuid>,
    /// Override the window encoded in `filters.window_minutes` for ad-hoc
    /// time-range tweaks from the UI.
    #[serde(default)]
    pub window_minutes: Option<i64>,
}

/// `POST /api/v1/observability/saved-queries/{id}/execute`
///
/// Runs the saved query's `filters` payload against runtime data and
/// returns a flat `{ metric, total, window_minutes, series }` shape the
/// UI's tile components consume directly. Phase 1 supports three
/// `filters.kind` shapes — `request_count`, `request_count_by_status`,
/// `incident_count` — each with an optional `window_minutes`.
pub async fn execute_saved_query(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    body: Option<Json<ExecuteQueryRequest>>,
) -> ApiResult<Json<ExecuteQueryResponse>> {
    let req = body.map(|Json(b)| b).unwrap_or_default();
    let query = load_authorized_query(&state, user_id, &headers, id).await?;

    let kind = query
        .filters
        .get("kind")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ApiError::InvalidRequest("Saved query filters missing required `kind`".into())
        })?
        .to_string();
    let window_minutes = req
        .window_minutes
        .or_else(|| query.filters.get("window_minutes").and_then(|v| v.as_i64()))
        .unwrap_or(15)
        .clamp(1, 24 * 60);
    let workspace_filter = req.workspace_id.or(query.workspace_id);

    let pool = state.db.pool();
    let response = match kind.as_str() {
        "request_count" => run_request_count(pool, query.org_id, workspace_filter, window_minutes)
            .await
            .map_err(ApiError::Database)?,
        "request_count_by_status" => {
            run_request_count_by_status(pool, query.org_id, workspace_filter, window_minutes)
                .await
                .map_err(ApiError::Database)?
        }
        "incident_count" => {
            run_incident_count(pool, query.org_id, workspace_filter, window_minutes)
                .await
                .map_err(ApiError::Database)?
        }
        other => {
            return Err(ApiError::InvalidRequest(format!(
                "Unsupported saved-query kind '{other}'. Supported: request_count, request_count_by_status, incident_count"
            )));
        }
    };

    Ok(Json(response))
}

/// Total request count over the window. Workspace-scoped when
/// `workspace_id` is set; otherwise the `runtime_captures.workspace_id`
/// filter is dropped (org-wide).
///
/// Note (#462): captures shipped from in-container hosted-mocks land
/// with `workspace_id IS NULL` today, so workspace-scoped counts only
/// reflect `--cloud-ship` traffic until the shipper backfill ships.
async fn run_request_count(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    workspace_id: Option<Uuid>,
    window_minutes: i64,
) -> sqlx::Result<ExecuteQueryResponse> {
    let total: i64 = if let Some(ws) = workspace_id {
        sqlx::query_scalar(
            r#"
            SELECT COUNT(*)::BIGINT
            FROM runtime_captures
            WHERE workspace_id = $1
              AND occurred_at >= NOW() - make_interval(mins => $2::int)
            "#,
        )
        .bind(ws)
        .bind(window_minutes as i32)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar(
            r#"
            SELECT COUNT(*)::BIGINT
            FROM runtime_captures rc
            JOIN hosted_mocks hm ON hm.id = rc.deployment_id
            WHERE hm.org_id = $1
              AND rc.occurred_at >= NOW() - make_interval(mins => $2::int)
            "#,
        )
        .bind(org_id)
        .bind(window_minutes as i32)
        .fetch_one(pool)
        .await?
    };

    Ok(ExecuteQueryResponse {
        metric: "request_count".into(),
        total,
        window_minutes,
        series: vec![QueryBucket {
            label: "all".into(),
            count: total,
        }],
    })
}

async fn run_request_count_by_status(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    workspace_id: Option<Uuid>,
    window_minutes: i64,
) -> sqlx::Result<ExecuteQueryResponse> {
    // `COALESCE(response_status_code, status_code)` mirrors what the
    // request-log endpoint surfaces — a "request that never got a
    // response" still has a status_code on the request side.
    let rows: Vec<(Option<i32>, i64)> = if let Some(ws) = workspace_id {
        sqlx::query_as(
            r#"
            SELECT COALESCE(response_status_code, status_code) AS status,
                   COUNT(*)::BIGINT
            FROM runtime_captures
            WHERE workspace_id = $1
              AND occurred_at >= NOW() - make_interval(mins => $2::int)
            GROUP BY status
            ORDER BY status NULLS LAST
            "#,
        )
        .bind(ws)
        .bind(window_minutes as i32)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as(
            r#"
            SELECT COALESCE(rc.response_status_code, rc.status_code) AS status,
                   COUNT(*)::BIGINT
            FROM runtime_captures rc
            JOIN hosted_mocks hm ON hm.id = rc.deployment_id
            WHERE hm.org_id = $1
              AND rc.occurred_at >= NOW() - make_interval(mins => $2::int)
            GROUP BY status
            ORDER BY status NULLS LAST
            "#,
        )
        .bind(org_id)
        .bind(window_minutes as i32)
        .fetch_all(pool)
        .await?
    };

    let total = rows.iter().map(|(_, c)| *c).sum();
    let series = rows
        .into_iter()
        .map(|(status, count)| QueryBucket {
            label: status.map(|s| s.to_string()).unwrap_or_else(|| "unknown".into()),
            count,
        })
        .collect();

    Ok(ExecuteQueryResponse {
        metric: "request_count_by_status".into(),
        total,
        window_minutes,
        series,
    })
}

async fn run_incident_count(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    workspace_id: Option<Uuid>,
    window_minutes: i64,
) -> sqlx::Result<ExecuteQueryResponse> {
    let rows: Vec<(String, i64)> = if let Some(ws) = workspace_id {
        sqlx::query_as(
            r#"
            SELECT severity, COUNT(*)::BIGINT
            FROM incidents
            WHERE workspace_id = $1
              AND created_at >= NOW() - make_interval(mins => $2::int)
              AND status != 'resolved'
            GROUP BY severity
            ORDER BY CASE severity
                       WHEN 'critical' THEN 0
                       WHEN 'high' THEN 1
                       WHEN 'medium' THEN 2
                       WHEN 'low' THEN 3
                       ELSE 4
                     END
            "#,
        )
        .bind(ws)
        .bind(window_minutes as i32)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as(
            r#"
            SELECT severity, COUNT(*)::BIGINT
            FROM incidents
            WHERE org_id = $1
              AND created_at >= NOW() - make_interval(mins => $2::int)
              AND status != 'resolved'
            GROUP BY severity
            ORDER BY CASE severity
                       WHEN 'critical' THEN 0
                       WHEN 'high' THEN 1
                       WHEN 'medium' THEN 2
                       WHEN 'low' THEN 3
                       ELSE 4
                     END
            "#,
        )
        .bind(org_id)
        .bind(window_minutes as i32)
        .fetch_all(pool)
        .await?
    };

    let total = rows.iter().map(|(_, c)| *c).sum();
    let series = rows
        .into_iter()
        .map(|(severity, count)| QueryBucket {
            label: severity,
            count,
        })
        .collect();

    Ok(ExecuteQueryResponse {
        metric: "incident_count".into(),
        total,
        window_minutes,
        series,
    })
}
