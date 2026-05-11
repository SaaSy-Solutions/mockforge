//! Workspace request-log handler (#462) — queries `runtime_captures` for the
//! workspace and returns them in the UI-friendly `RequestLog` shape used by
//! the local `/__mockforge/logs` endpoint. Lets the same `LogsPage` UI work
//! against the registry.
//!
//! Today this reaches captures shipped with `workspace_id` populated — i.e.
//! `--cloud-ship` (local mockforge sending to cloud). Hosted-mock captures
//! still land with `workspace_id IS NULL` because the in-container shipper
//! doesn't have the workspace context at ingest time; backfilling that is
//! tracked separately. The endpoint returns `[]` for those today and will
//! light up automatically once the shipper fix lands.
//!
//! Route: `GET /api/v1/workspaces/{workspace_id}/request-logs`

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::CloudWorkspace,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct ListLogsQuery {
    /// Exact HTTP method filter (case-insensitive).
    #[serde(default)]
    pub method: Option<String>,
    /// Substring match against `path`.
    #[serde(default)]
    pub path: Option<String>,
    /// Status-class filter: `2xx`, `4xx`, `5xx`, or an exact code like `404`.
    #[serde(default)]
    pub status: Option<String>,
    /// Max rows. Capped at 1000.
    #[serde(default)]
    pub limit: Option<i64>,
}

/// Response shape mirrors the UI's `RequestLog` interface in
/// `crates/mockforge-ui/ui/src/types/index.ts` so the same `LogsPage` can
/// render either source unchanged.
#[derive(Debug, Serialize)]
pub struct RequestLogEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub method: String,
    pub path: String,
    pub status_code: i32,
    pub response_time_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    pub headers: serde_json::Value,
    pub response_size_bytes: i64,
}

pub async fn list_workspace_request_logs(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    Query(query): Query<ListLogsQuery>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<RequestLogEntry>>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;

    let limit = query.limit.unwrap_or(100).clamp(1, 1000);
    let method_filter = query.method.as_ref().map(|s| s.to_uppercase());
    let path_filter = query.path.as_deref().filter(|s| !s.is_empty());
    let (status_min, status_max) = parse_status_filter(query.status.as_deref());

    let rows: Vec<RuntimeCaptureRow> = sqlx::query_as::<_, RuntimeCaptureRow>(
        r#"
        SELECT id, occurred_at, method, path,
               COALESCE(response_status_code, status_code, 0) AS effective_status,
               COALESCE(duration_ms, 0) AS duration_ms,
               client_ip,
               request_headers,
               COALESCE(response_size_bytes, 0) AS response_size_bytes
        FROM runtime_captures
        WHERE workspace_id = $1
          AND ($2::text IS NULL OR UPPER(method) = $2)
          AND ($3::text IS NULL OR position($3 IN path) > 0)
          AND ($4::int IS NULL OR COALESCE(response_status_code, status_code, 0) BETWEEN $4 AND $5)
        ORDER BY occurred_at DESC
        LIMIT $6
        "#,
    )
    .bind(workspace_id)
    .bind(method_filter)
    .bind(path_filter)
    .bind(status_min)
    .bind(status_max)
    .bind(limit)
    .fetch_all(state.db.pool())
    .await
    .map_err(ApiError::Database)?;

    let entries = rows.into_iter().map(row_to_entry).collect();
    Ok(Json(entries))
}

#[derive(sqlx::FromRow)]
struct RuntimeCaptureRow {
    id: i64,
    occurred_at: DateTime<Utc>,
    method: String,
    path: String,
    effective_status: i32,
    duration_ms: i64,
    client_ip: Option<String>,
    request_headers: String,
    response_size_bytes: i64,
}

fn row_to_entry(row: RuntimeCaptureRow) -> RequestLogEntry {
    let (headers, user_agent) = parse_request_headers(&row.request_headers);
    RequestLogEntry {
        id: row.id.to_string(),
        timestamp: row.occurred_at,
        method: row.method,
        path: row.path,
        status_code: row.effective_status,
        response_time_ms: row.duration_ms,
        client_ip: row.client_ip,
        user_agent,
        headers,
        response_size_bytes: row.response_size_bytes,
    }
}

/// `request_headers` is stored as a JSON-encoded TEXT (per the shipper's
/// `RecordedRequest::headers` serialization). Parse to an object; lift
/// User-Agent out so the UI can show it without re-parsing headers per row.
/// Malformed input falls back to an empty object — never fail the whole
/// listing because one row's headers are unparsable.
fn parse_request_headers(raw: &str) -> (serde_json::Value, Option<String>) {
    let parsed: serde_json::Value =
        serde_json::from_str(raw).unwrap_or(serde_json::Value::Object(Default::default()));
    let user_agent = parsed
        .as_object()
        .and_then(|m| {
            m.iter().find_map(|(k, v)| {
                if k.eq_ignore_ascii_case("user-agent") {
                    v.as_str().map(str::to_string)
                } else {
                    None
                }
            })
        })
        .filter(|s| !s.is_empty());
    (parsed, user_agent)
}

/// Accepts `2xx`, `4xx`, `5xx`, or an exact `404`-style integer. Returns
/// the inclusive (min, max) status range, or `(None, None)` when unset /
/// unparsable so the SQL WHERE clause skips the filter.
fn parse_status_filter(raw: Option<&str>) -> (Option<i32>, Option<i32>) {
    let Some(s) = raw else { return (None, None) };
    let trimmed = s.trim().to_lowercase();
    match trimmed.as_str() {
        "2xx" => (Some(200), Some(299)),
        "3xx" => (Some(300), Some(399)),
        "4xx" => (Some(400), Some(499)),
        "5xx" => (Some(500), Some(599)),
        other => match other.parse::<i32>() {
            Ok(n) if (100..=599).contains(&n) => (Some(n), Some(n)),
            _ => (None, None),
        },
    }
}

async fn authorize_workspace(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    workspace_id: Uuid,
) -> ApiResult<()> {
    let workspace = CloudWorkspace::find_by_id(state.db.pool(), workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".into()))?;
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != workspace.org_id {
        return Err(ApiError::InvalidRequest("Workspace not found".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_filter_parses_classes() {
        assert_eq!(parse_status_filter(Some("2xx")), (Some(200), Some(299)));
        assert_eq!(parse_status_filter(Some("4XX")), (Some(400), Some(499)));
        assert_eq!(parse_status_filter(Some("5xx")), (Some(500), Some(599)));
    }

    #[test]
    fn status_filter_parses_exact_code() {
        assert_eq!(parse_status_filter(Some("404")), (Some(404), Some(404)));
        assert_eq!(parse_status_filter(Some("200")), (Some(200), Some(200)));
    }

    #[test]
    fn status_filter_rejects_garbage() {
        assert_eq!(parse_status_filter(Some("9xx")), (None, None));
        assert_eq!(parse_status_filter(Some("abc")), (None, None));
        assert_eq!(parse_status_filter(Some("99")), (None, None));
        assert_eq!(parse_status_filter(None), (None, None));
    }

    #[test]
    fn headers_parse_extracts_user_agent_case_insensitive() {
        let (h, ua) = parse_request_headers(r#"{"User-Agent":"curl/8.4"}"#);
        assert_eq!(ua.as_deref(), Some("curl/8.4"));
        assert!(h.is_object());

        let (_, ua) = parse_request_headers(r#"{"user-agent":"foo"}"#);
        assert_eq!(ua.as_deref(), Some("foo"));
    }

    #[test]
    fn headers_parse_handles_malformed_input() {
        let (h, ua) = parse_request_headers("not json");
        assert!(h.is_object() && h.as_object().unwrap().is_empty());
        assert!(ua.is_none());
    }
}
