//! Test schedule CRUD (cloud-enablement task #4 / Phase 3).
//!
//! Routes:
//!   GET    /api/v1/test-suites/{suite_id}/schedules
//!   POST   /api/v1/test-suites/{suite_id}/schedules
//!   PATCH  /api/v1/test-schedules/{id}      (toggle enabled)
//!   DELETE /api/v1/test-schedules/{id}
//!
//! Cron evaluation lives in workers/test_schedule_runner; this file just
//! validates inputs and persists rows. Cron parse-failures here are
//! rejected at write time so the worker doesn't have to handle bad rows.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use cron::Schedule;
use mockforge_registry_core::models::{CloudWorkspace, TestSchedule, TestSuite};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    store::with_org_context,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct CreateScheduleRequest {
    pub cron: String,
    #[serde(default = "default_tz")]
    pub timezone: String,
}

fn default_tz() -> String {
    "UTC".to_string()
}

/// Schedule + computed next-fire timestamp. Wraps the bare `TestSchedule`
/// row so the UI can render "next run at ..." without parsing cron
/// client-side.
#[derive(Debug, Serialize)]
pub struct ScheduleWithNextFire {
    #[serde(flatten)]
    pub schedule: TestSchedule,
    /// Next fire time in UTC, or null if the cron expression has no
    /// future fire (cron::Schedule::upcoming exhausted) or the row
    /// can't be evaluated (parse error — should never happen since
    /// create() validates).
    pub next_fire_at: Option<DateTime<Utc>>,
}

fn compute_next_fire(schedule: &TestSchedule) -> Option<DateTime<Utc>> {
    if !schedule.enabled {
        return None;
    }
    let tz: Tz = schedule.timezone.parse().ok()?;
    let cron = Schedule::from_str(&schedule.cron).ok()?;
    cron.upcoming(tz).next().map(|dt| dt.with_timezone(&Utc))
}

/// `GET /api/v1/test-suites/{suite_id}/schedules`
pub async fn list_for_suite(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(suite_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<ScheduleWithNextFire>>> {
    let (_suite, org_id) = load_authorized_suite(&state, user_id, &headers, suite_id).await?;
    let rows = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(TestSchedule::list_by_suite(&mut **tx, suite_id).await?) })
    })
    .await?;
    let with_next = rows
        .into_iter()
        .map(|s| ScheduleWithNextFire {
            next_fire_at: compute_next_fire(&s),
            schedule: s,
        })
        .collect();
    Ok(Json(with_next))
}

/// `POST /api/v1/test-suites/{suite_id}/schedules`
pub async fn create(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(suite_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<CreateScheduleRequest>,
) -> ApiResult<Json<TestSchedule>> {
    let (_suite, org_id) = load_authorized_suite(&state, user_id, &headers, suite_id).await?;

    if body.cron.trim().is_empty() {
        return Err(ApiError::InvalidRequest("cron must not be empty".into()));
    }
    if Schedule::from_str(&body.cron).is_err() {
        return Err(ApiError::InvalidRequest("cron expression is invalid".into()));
    }
    if body.timezone.parse::<Tz>().is_err() {
        return Err(ApiError::InvalidRequest("timezone is not an IANA name".into()));
    }

    let row = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(TestSchedule::create(&mut **tx, suite_id, &body.cron, &body.timezone).await?)
        })
    })
    .await?;
    Ok(Json(row))
}

#[derive(Debug, Deserialize)]
pub struct UpdateScheduleRequest {
    pub enabled: bool,
}

/// `PATCH /api/v1/test-schedules/{id}`
///
/// Phase 3 only supports enabled-toggling. Cron / tz changes are
/// delete-and-recreate so the worker's last_triggered_at cursor doesn't
/// quietly carry over to a fundamentally different schedule.
pub async fn set_enabled(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<UpdateScheduleRequest>,
) -> ApiResult<Json<TestSchedule>> {
    // Authorize via the parent suite.
    let (existing, _org_id) = load_authorized_schedule(&state, user_id, &headers, id).await?;
    let (_suite, org_id) =
        load_authorized_suite(&state, user_id, &headers, existing.suite_id).await?;

    let enabled = body.enabled;
    let updated = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(TestSchedule::set_enabled(&mut **tx, id, enabled).await?) })
    })
    .await?
    .ok_or_else(|| ApiError::InvalidRequest("Test schedule not found".into()))?;
    Ok(Json(updated))
}

/// `DELETE /api/v1/test-schedules/{id}`
pub async fn delete(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let (existing, _org_id) = load_authorized_schedule(&state, user_id, &headers, id).await?;
    let (_suite, org_id) =
        load_authorized_suite(&state, user_id, &headers, existing.suite_id).await?;

    let deleted = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(TestSchedule::delete(&mut **tx, id).await?) })
    })
    .await?;
    if !deleted {
        return Err(ApiError::InvalidRequest("Test schedule not found".into()));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// Load a schedule row bound to the caller's org. A schedule whose suite lives
/// in another org reads as absent (RLS joins test_schedules -> test_suites ->
/// workspaces) and yields the same "Test schedule not found" as a missing row.
/// The parent suite is then authorized explicitly by `load_authorized_suite`.
async fn load_authorized_schedule(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    id: Uuid,
) -> ApiResult<(TestSchedule, Uuid)> {
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    let existing = with_org_context(state.db.runtime_pool(), ctx.org_id, |tx| {
        Box::pin(async move {
            Ok(sqlx::query_as::<_, TestSchedule>("SELECT * FROM test_schedules WHERE id = $1")
                .bind(id)
                .fetch_optional(&mut **tx)
                .await?)
        })
    })
    .await?
    .ok_or_else(|| ApiError::InvalidRequest("Test schedule not found".into()))?;
    Ok((existing, ctx.org_id))
}

/// Load a test suite only if it belongs to the caller's org; returns the suite
/// and that org. Cross-org suites read as "Test suite not found".
async fn load_authorized_suite(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    suite_id: Uuid,
) -> ApiResult<(TestSuite, Uuid)> {
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    // Bound to the caller's org: a suite or workspace in another org reads as
    // absent and yields the same "Test suite not found" as a missing row.
    let found = with_org_context(state.db.runtime_pool(), ctx.org_id, |tx| {
        Box::pin(async move {
            let Some(suite) = TestSuite::find_by_id(&mut **tx, suite_id).await? else {
                return Ok(None);
            };
            let workspace = CloudWorkspace::find_by_id(&mut **tx, suite.workspace_id).await?;
            Ok(workspace.map(|ws| (suite, ws)))
        })
    })
    .await?;
    let (suite, workspace) =
        found.ok_or_else(|| ApiError::InvalidRequest("Test suite not found".into()))?;
    if ctx.org_id != workspace.org_id {
        return Err(ApiError::InvalidRequest("Test suite not found".into()));
    }
    Ok((suite, ctx.org_id))
}
