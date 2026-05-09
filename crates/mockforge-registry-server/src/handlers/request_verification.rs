//! Cloud-mode request verification — WireMock-style assertions against the
//! workspace's `runtime_captures` table.
//!
//! Mirrors the local `/__mockforge/verification/*` surface from
//! `mockforge-core`, but sources its log entries from the per-workspace
//! recorder pipeline instead of the in-process ring buffer. The matcher
//! itself is the same code (`mockforge_core::verification`) so cloud and
//! local results stay bit-for-bit consistent.
//!
//! Note on data availability: only deployments that have the recorder
//! enabled write to `runtime_captures`. The UI surfaces this so users
//! aren't surprised by a "0 matches" result against a deployment that
//! simply isn't capturing.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use chrono::{DateTime, Duration, Utc};
use mockforge_core::{
    request_logger::RequestLogEntry,
    verification::{
        verify_entries, verify_sequence_entries, VerificationCount, VerificationRequest,
        VerificationResult,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::CloudWorkspace,
    AppState,
};

/// Hard cap on how many capture rows a single verification call will
/// pull. Picked to be generous enough for typical staging windows while
/// keeping a runaway request from materialising the entire workspace
/// retention into memory.
const MAX_CAPTURE_ROWS: i64 = 5000;

/// Default lookback if the caller omits `since`. One hour matches what
/// the cloud UI defaults to and stays well inside even the Free-tier
/// 24h retention.
const DEFAULT_LOOKBACK: Duration = Duration::hours(1);

/// Hard ceiling on lookback regardless of caller input. Free retention
/// is 24h, so anything beyond that would be empty for the worst-case
/// tier anyway, and we don't want a misconfigured client to scan the
/// full Pro/Team retention by accident.
const MAX_LOOKBACK: Duration = Duration::hours(24);

#[derive(Debug, Deserialize)]
pub struct TimeWindow {
    /// RFC3339 timestamp; only captures with `occurred_at >= since` are
    /// considered. Defaults to `now() - 1h`.
    #[serde(default)]
    pub since: Option<DateTime<Utc>>,
    /// RFC3339 timestamp; only captures with `occurred_at <= until` are
    /// considered. Defaults to `now()`.
    #[serde(default)]
    pub until: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct VerifyBody {
    pub pattern: VerificationRequest,
    pub expected: VerificationCount,
    #[serde(default, flatten)]
    pub window: TimeWindow,
}

#[derive(Debug, Deserialize)]
pub struct CountBody {
    pub pattern: VerificationRequest,
    #[serde(default, flatten)]
    pub window: TimeWindow,
}

#[derive(Debug, Serialize)]
pub struct CountResponse {
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct SequenceBody {
    pub patterns: Vec<VerificationRequest>,
    #[serde(default, flatten)]
    pub window: TimeWindow,
}

#[derive(Debug, Deserialize)]
pub struct NeverBody {
    pub pattern: VerificationRequest,
    #[serde(default, flatten)]
    pub window: TimeWindow,
}

#[derive(Debug, Deserialize)]
pub struct AtLeastBody {
    pub pattern: VerificationRequest,
    pub min: usize,
    #[serde(default, flatten)]
    pub window: TimeWindow,
}

async fn require_workspace(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    workspace_id: Uuid,
) -> ApiResult<CloudWorkspace> {
    let org_ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let workspace = CloudWorkspace::find_by_id(state.db.pool(), workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".to_string()))?;

    if workspace.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Workspace does not belong to this organization".to_string(),
        ));
    }

    Ok(workspace)
}

/// Resolve the time window, clamping to `MAX_LOOKBACK` and rejecting
/// inverted ranges.
fn resolve_window(window: &TimeWindow) -> ApiResult<(DateTime<Utc>, DateTime<Utc>)> {
    let now = Utc::now();
    let until = window.until.unwrap_or(now);
    let since = window.since.unwrap_or(until - DEFAULT_LOOKBACK);

    if since > until {
        return Err(ApiError::InvalidRequest("`since` must be earlier than `until`".to_string()));
    }

    let max_since = until - MAX_LOOKBACK;
    if since < max_since {
        return Err(ApiError::InvalidRequest(format!(
            "Window too large: max lookback is {} hours",
            MAX_LOOKBACK.num_hours()
        )));
    }

    Ok((since, until))
}

/// Pulled column subset from `runtime_captures`. Mirrors the shape we
/// need to materialise a `RequestLogEntry` — anything outside this
/// columns list (response_body, tags, etc.) is intentionally dropped
/// because the matcher doesn't consult it.
#[derive(sqlx::FromRow)]
struct CaptureRow {
    occurred_at: DateTime<Utc>,
    method: String,
    path: String,
    query_params: Option<String>,
    request_headers: String,
    request_body: Option<String>,
    duration_ms: Option<i64>,
    status_code: Option<i32>,
    client_ip: Option<String>,
    response_size_bytes: Option<i64>,
}

async fn load_captures(
    state: &AppState,
    workspace_id: Uuid,
    since: DateTime<Utc>,
    until: DateTime<Utc>,
) -> ApiResult<Vec<CaptureRow>> {
    sqlx::query_as::<_, CaptureRow>(
        r#"
        SELECT occurred_at,
               method,
               path,
               query_params,
               request_headers,
               request_body,
               duration_ms,
               status_code,
               client_ip,
               response_size_bytes
          FROM runtime_captures
         WHERE workspace_id = $1
           AND occurred_at >= $2
           AND occurred_at <= $3
         ORDER BY occurred_at DESC
         LIMIT $4
        "#,
    )
    .bind(workspace_id)
    .bind(since)
    .bind(until)
    .bind(MAX_CAPTURE_ROWS)
    .fetch_all(state.db.pool())
    .await
    .map_err(ApiError::Database)
}

/// Convert a capture row into the in-memory shape the local matcher
/// expects. The body (if any) is stashed in `metadata["request_body"]`
/// because that's where `mockforge_core::verification::matches_body_pattern`
/// already looks.
fn row_to_entry(row: CaptureRow) -> RequestLogEntry {
    let headers: HashMap<String, String> =
        serde_json::from_str(&row.request_headers).unwrap_or_default();
    let query_params: HashMap<String, String> = row
        .query_params
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    let mut metadata = HashMap::new();
    if let Some(body) = row.request_body {
        metadata.insert("request_body".to_string(), body);
    }

    RequestLogEntry {
        id: format!("capture-{}", row.occurred_at.timestamp_nanos_opt().unwrap_or(0)),
        timestamp: row.occurred_at,
        server_type: "HTTP".to_string(),
        method: row.method,
        path: row.path,
        status_code: row.status_code.unwrap_or(0).max(0) as u16,
        response_time_ms: row.duration_ms.unwrap_or(0).max(0) as u64,
        client_ip: row.client_ip,
        user_agent: None,
        headers,
        query_params,
        response_size_bytes: row.response_size_bytes.unwrap_or(0).max(0) as u64,
        error_message: None,
        metadata,
        reality_metadata: None,
    }
}

/// `POST /api/v1/workspaces/{workspace_id}/request-log/verify`
pub async fn verify(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(body): Json<VerifyBody>,
) -> ApiResult<Json<VerificationResult>> {
    require_workspace(&state, user_id, &headers, workspace_id).await?;
    let (since, until) = resolve_window(&body.window)?;
    let rows = load_captures(&state, workspace_id, since, until).await?;
    let entries: Vec<RequestLogEntry> = rows.into_iter().map(row_to_entry).collect();
    Ok(Json(verify_entries(&entries, &body.pattern, body.expected)))
}

/// `POST /api/v1/workspaces/{workspace_id}/request-log/count`
pub async fn count(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(body): Json<CountBody>,
) -> ApiResult<Json<CountResponse>> {
    require_workspace(&state, user_id, &headers, workspace_id).await?;
    let (since, until) = resolve_window(&body.window)?;
    let rows = load_captures(&state, workspace_id, since, until).await?;
    let entries: Vec<RequestLogEntry> = rows.into_iter().map(row_to_entry).collect();
    let result = verify_entries(&entries, &body.pattern, VerificationCount::AtLeast(0));
    Ok(Json(CountResponse {
        count: result.count,
    }))
}

/// `POST /api/v1/workspaces/{workspace_id}/request-log/sequence`
pub async fn sequence(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(body): Json<SequenceBody>,
) -> ApiResult<Json<VerificationResult>> {
    require_workspace(&state, user_id, &headers, workspace_id).await?;
    let (since, until) = resolve_window(&body.window)?;
    let rows = load_captures(&state, workspace_id, since, until).await?;
    // Captures come back DESC; sequence verification expects chronological order.
    let entries: Vec<RequestLogEntry> = rows.into_iter().rev().map(row_to_entry).collect();
    Ok(Json(verify_sequence_entries(&entries, &body.patterns)))
}

/// `POST /api/v1/workspaces/{workspace_id}/request-log/never`
pub async fn never(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(body): Json<NeverBody>,
) -> ApiResult<Json<VerificationResult>> {
    require_workspace(&state, user_id, &headers, workspace_id).await?;
    let (since, until) = resolve_window(&body.window)?;
    let rows = load_captures(&state, workspace_id, since, until).await?;
    let entries: Vec<RequestLogEntry> = rows.into_iter().map(row_to_entry).collect();
    Ok(Json(verify_entries(&entries, &body.pattern, VerificationCount::Never)))
}

/// `POST /api/v1/workspaces/{workspace_id}/request-log/at-least`
pub async fn at_least(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(body): Json<AtLeastBody>,
) -> ApiResult<Json<VerificationResult>> {
    require_workspace(&state, user_id, &headers, workspace_id).await?;
    let (since, until) = resolve_window(&body.window)?;
    let rows = load_captures(&state, workspace_id, since, until).await?;
    let entries: Vec<RequestLogEntry> = rows.into_iter().map(row_to_entry).collect();
    Ok(Json(verify_entries(
        &entries,
        &body.pattern,
        VerificationCount::AtLeast(body.min),
    )))
}

#[derive(Debug, Serialize)]
pub struct WorkspaceCaptureStatus {
    /// Whether at least one deployment in the workspace currently has
    /// captured rows. The UI uses this to surface the "enable recording"
    /// hint when the verification feature would silently return zeros.
    pub has_captures: bool,
    /// Total capture rows in the workspace within the default lookback
    /// window. Useful as a sanity check in the UI.
    pub recent_capture_count: i64,
}

/// `GET /api/v1/workspaces/{workspace_id}/request-log/status`
///
/// Lightweight surface used by the cloud Verification page to decide
/// whether to show the "no recordings yet" empty state.
pub async fn status(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> ApiResult<Json<WorkspaceCaptureStatus>> {
    require_workspace(&state, user_id, &headers, workspace_id).await?;

    let recent_capture_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
          FROM runtime_captures
         WHERE workspace_id = $1
           AND occurred_at >= NOW() - INTERVAL '1 hour'
        "#,
    )
    .bind(workspace_id)
    .fetch_one(state.db.pool())
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(WorkspaceCaptureStatus {
        has_captures: recent_capture_count > 0,
        recent_capture_count,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_window_uses_defaults() {
        let w = TimeWindow {
            since: None,
            until: None,
        };
        let (since, until) = resolve_window(&w).unwrap();
        let span = until - since;
        // Should default to roughly DEFAULT_LOOKBACK (allow a few seconds of jitter).
        let drift = (span - DEFAULT_LOOKBACK).num_seconds().abs();
        assert!(drift < 5, "expected ~1h window, got {}s", span.num_seconds());
    }

    #[test]
    fn resolve_window_rejects_inverted_range() {
        let now = Utc::now();
        let w = TimeWindow {
            since: Some(now),
            until: Some(now - Duration::minutes(5)),
        };
        assert!(resolve_window(&w).is_err());
    }

    #[test]
    fn resolve_window_rejects_too_large() {
        let now = Utc::now();
        let w = TimeWindow {
            since: Some(now - Duration::hours(48)),
            until: Some(now),
        };
        assert!(resolve_window(&w).is_err());
    }

    #[test]
    fn row_to_entry_extracts_headers_and_body() {
        let row = CaptureRow {
            occurred_at: Utc::now(),
            method: "POST".to_string(),
            path: "/api/checkout".to_string(),
            query_params: Some(r#"{"ref":"abc"}"#.to_string()),
            request_headers: r#"{"content-type":"application/json"}"#.to_string(),
            request_body: Some(r#"{"item":"widget"}"#.to_string()),
            duration_ms: Some(42),
            status_code: Some(201),
            client_ip: Some("10.0.0.1".to_string()),
            response_size_bytes: Some(128),
        };
        let entry = row_to_entry(row);
        assert_eq!(entry.method, "POST");
        assert_eq!(entry.headers.get("content-type").map(String::as_str), Some("application/json"));
        assert_eq!(entry.query_params.get("ref").map(String::as_str), Some("abc"));
        assert_eq!(
            entry.metadata.get("request_body").map(String::as_str),
            Some(r#"{"item":"widget"}"#)
        );
        assert_eq!(entry.status_code, 201);
        assert_eq!(entry.response_time_ms, 42);
    }

    #[test]
    fn row_to_entry_handles_invalid_header_json() {
        let row = CaptureRow {
            occurred_at: Utc::now(),
            method: "GET".to_string(),
            path: "/".to_string(),
            query_params: None,
            request_headers: "not valid json".to_string(),
            request_body: None,
            duration_ms: None,
            status_code: None,
            client_ip: None,
            response_size_bytes: None,
        };
        let entry = row_to_entry(row);
        assert!(entry.headers.is_empty());
        assert!(entry.query_params.is_empty());
        assert!(!entry.metadata.contains_key("request_body"));
    }
}
