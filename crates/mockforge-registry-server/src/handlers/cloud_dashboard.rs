//! Cloud dashboard handler — provides dashboard metrics for cloud-hosted users
//!
//! Returns data in the same shape as the local /__mockforge/dashboard endpoint
//! so the existing DashboardPage UI components work in cloud mode.

use axum::{extract::State, http::HeaderMap, Json};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    AppState,
};

#[derive(Debug, Serialize)]
pub struct CloudDashboardResponse {
    pub server_info: ServerInfo,
    pub system_info: SystemInfo,
    pub metrics: Metrics,
    pub servers: Vec<serde_json::Value>,
    pub recent_logs: Vec<serde_json::Value>,
    pub system: System,
}

#[derive(Debug, Serialize)]
pub struct ServerInfo {
    pub version: String,
    pub build_time: String,
    pub git_sha: String,
    pub api_enabled: bool,
    pub admin_port: u16,
}

#[derive(Debug, Serialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub uptime: u64,
    pub memory_usage: u64,
}

#[derive(Debug, Serialize)]
pub struct Metrics {
    pub total_requests: u64,
    pub active_requests: u64,
    pub average_response_time: f64,
    pub error_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct System {
    pub version: String,
    pub uptime_seconds: u64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub active_threads: u64,
    pub total_routes: i64,
    pub total_fixtures: i64,
}

/// Get cloud dashboard data with real counts from the database
pub async fn get_dashboard(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<CloudDashboardResponse>> {
    let pool = state.db.pool();

    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let org_id = org_ctx.org_id;

    // Get real counts from the database
    let workspace_count = count_table(pool, "workspaces", org_id).await;
    let service_count = count_table(pool, "services", org_id).await;
    let fixture_count = count_table(pool, "fixtures", org_id).await;
    let federation_count = count_table(pool, "federations", org_id).await;
    let hosted_mock_count = count_hosted_mocks(pool, org_id).await;
    let total_resources =
        (workspace_count + service_count + fixture_count + federation_count + hosted_mock_count)
            as u64;

    Ok(Json(CloudDashboardResponse {
        server_info: ServerInfo {
            version: "cloud".to_string(),
            build_time: String::new(),
            git_sha: String::new(),
            api_enabled: true,
            admin_port: 0,
        },
        system_info: SystemInfo {
            os: "cloud".to_string(),
            arch: "cloud".to_string(),
            uptime: 0,
            memory_usage: 0,
        },
        metrics: Metrics {
            total_requests: total_resources,
            active_requests: 0,
            average_response_time: 0.0,
            error_rate: 0.0,
        },
        servers: vec![],
        recent_logs: vec![],
        system: System {
            version: "cloud".to_string(),
            uptime_seconds: 0,
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            active_threads: 0,
            total_routes: service_count,
            total_fixtures: fixture_count,
        },
    }))
}

/// Get cloud health status
pub async fn get_health(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let _pool = state.db.pool();

    let _org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Simple health check — if we got here, the database is reachable
    Ok(Json(serde_json::json!({
        "status": "healthy",
        "services": {},
        "last_check": chrono::Utc::now().to_rfc3339(),
        "issues": []
    })))
}

/// Get cloud request logs (returns recent audit events as a proxy)
pub async fn get_logs(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let pool = state.db.pool();

    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Return recent audit events as "logs"
    let logs: Vec<serde_json::Value> = sqlx::query_scalar(
        r#"
        SELECT json_build_object(
            'id', id::text,
            'timestamp', created_at,
            'event_type', event_type::text,
            'description', description,
            'user_id', user_id::text,
            'ip_address', ip_address
        )
        FROM audit_logs
        WHERE org_id = $1
        ORDER BY created_at DESC
        LIMIT 50
        "#,
    )
    .bind(org_ctx.org_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    Ok(Json(logs))
}

async fn count_table(pool: &sqlx::PgPool, table: &str, org_id: Uuid) -> i64 {
    let query = format!("SELECT COUNT(*) FROM {} WHERE org_id = $1", table);
    sqlx::query_scalar::<_, i64>(&query)
        .bind(org_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0)
}

async fn count_hosted_mocks(pool: &sqlx::PgPool, org_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM hosted_mock_deployments WHERE org_id = $1")
        .bind(org_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0)
}
