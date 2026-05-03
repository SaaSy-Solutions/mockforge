//! Cloud dashboard handler — provides dashboard metrics for cloud-hosted users
//!
//! Returns data in the same shape as the local /__mockforge/dashboard endpoint
//! so the existing DashboardPage UI components work in cloud mode. Cloud-only
//! aggregates (per-status counts, egress, deployment list, audit activity) are
//! exposed under the extra `cloud_metrics` field — DashboardResponseSchema uses
//! `.passthrough()` so the UI sees them when running against this server.

use axum::{extract::State, http::HeaderMap, Json};
use chrono::Datelike;
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
    pub servers: Vec<ServerStatus>,
    pub recent_logs: Vec<serde_json::Value>,
    pub system: System,
    pub cloud_metrics: CloudMetrics,
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

/// Shape that matches the UI's `ServerStatus` type so ServerTable renders
/// hosted-mock deployments as if they were running mock servers.
#[derive(Debug, Serialize)]
pub struct ServerStatus {
    pub server_type: String,
    pub address: Option<String>,
    pub running: bool,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub uptime_seconds: Option<u64>,
    pub active_connections: u64,
    pub total_requests: u64,
}

/// Cloud-only aggregates that don't fit the local-mode dashboard shape.
/// Surfaced under `cloud_metrics` so the UI can replace local-only tiles
/// (CPU/memory/threads/uptime) with cloud-relevant ones.
#[derive(Debug, Serialize, Default)]
pub struct CloudMetrics {
    pub active_deployments: i64,
    pub total_deployments: i64,
    pub workspaces: i64,
    pub services: i64,
    pub fixtures: i64,
    pub federations: i64,
    pub requests_2xx: i64,
    pub requests_4xx: i64,
    pub requests_5xx: i64,
    pub egress_bytes: i64,
    pub period_start: Option<chrono::NaiveDate>,
}

#[derive(sqlx::FromRow)]
struct AggregatedMetrics {
    total_requests: Option<i64>,
    requests_2xx: Option<i64>,
    requests_4xx: Option<i64>,
    requests_5xx: Option<i64>,
    egress_bytes: Option<i64>,
    weighted_avg_response_time_ms: Option<f64>,
}

#[derive(sqlx::FromRow)]
struct ActiveDeployment {
    name: String,
    deployment_url: Option<String>,
    region: String,
    created_at: chrono::DateTime<chrono::Utc>,
    requests: Option<i64>,
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

    let workspace_count = count_table(pool, "workspaces", org_id).await;
    let service_count = count_table(pool, "services", org_id).await;
    let fixture_count = count_table(pool, "fixtures", org_id).await;
    let federation_count = count_table(pool, "federations", org_id).await;
    let total_deployments = count_table(pool, "hosted_mocks", org_id).await;
    let active_deployments = count_active_deployments(pool, org_id).await;

    let aggregated = aggregate_deployment_metrics(pool, org_id).await;
    let total_requests = aggregated.total_requests.unwrap_or(0).max(0) as u64;
    let requests_2xx = aggregated.requests_2xx.unwrap_or(0);
    let requests_4xx = aggregated.requests_4xx.unwrap_or(0);
    let requests_5xx = aggregated.requests_5xx.unwrap_or(0);
    let error_count = (requests_4xx + requests_5xx).max(0) as f64;
    let error_rate = if total_requests > 0 {
        (error_count / total_requests as f64) * 100.0
    } else {
        0.0
    };

    let now = chrono::Utc::now().date_naive();
    let period_start = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1);

    let servers = list_active_deployment_servers(pool, org_id).await;

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
            total_requests,
            active_requests: 0,
            average_response_time: aggregated.weighted_avg_response_time_ms.unwrap_or(0.0),
            error_rate,
        },
        servers,
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
        cloud_metrics: CloudMetrics {
            active_deployments,
            total_deployments,
            workspaces: workspace_count,
            services: service_count,
            fixtures: fixture_count,
            federations: federation_count,
            requests_2xx,
            requests_4xx,
            requests_5xx,
            egress_bytes: aggregated.egress_bytes.unwrap_or(0),
            period_start,
        },
    }))
}

/// Get cloud health status
pub async fn get_health(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let _org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

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

async fn count_active_deployments(pool: &sqlx::PgPool, org_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM hosted_mocks WHERE org_id = $1 AND status = 'active' AND deleted_at IS NULL",
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0)
}

/// SUM deployment_metrics across all of an org's hosted mocks for the current
/// period. Average response time is request-weighted so a busy mock outweighs
/// a quiet one. Unavailable until #232 lands the in-container log shipper, so
/// expect zeros for orgs without Fly Managed Prometheus configured.
async fn aggregate_deployment_metrics(pool: &sqlx::PgPool, org_id: Uuid) -> AggregatedMetrics {
    sqlx::query_as::<_, AggregatedMetrics>(
        r#"
        SELECT
            COALESCE(SUM(dm.requests), 0)::BIGINT AS total_requests,
            COALESCE(SUM(dm.requests_2xx), 0)::BIGINT AS requests_2xx,
            COALESCE(SUM(dm.requests_4xx), 0)::BIGINT AS requests_4xx,
            COALESCE(SUM(dm.requests_5xx), 0)::BIGINT AS requests_5xx,
            COALESCE(SUM(dm.egress_bytes), 0)::BIGINT AS egress_bytes,
            CASE WHEN COALESCE(SUM(dm.requests), 0) > 0
                THEN (SUM(dm.requests * dm.avg_response_time_ms)::FLOAT8
                      / NULLIF(SUM(dm.requests), 0)::FLOAT8)
                ELSE 0.0
            END AS weighted_avg_response_time_ms
        FROM deployment_metrics dm
        JOIN hosted_mocks hm ON hm.id = dm.hosted_mock_id
        WHERE hm.org_id = $1 AND hm.deleted_at IS NULL
        "#,
    )
    .bind(org_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(AggregatedMetrics {
        total_requests: Some(0),
        requests_2xx: Some(0),
        requests_4xx: Some(0),
        requests_5xx: Some(0),
        egress_bytes: Some(0),
        weighted_avg_response_time_ms: Some(0.0),
    })
}

/// Map active hosted-mock deployments into the `ServerStatus` shape the UI's
/// ServerTable expects. Each deployment becomes one "server" row with its
/// public URL as the address and the current period's request count.
async fn list_active_deployment_servers(pool: &sqlx::PgPool, org_id: Uuid) -> Vec<ServerStatus> {
    let rows = sqlx::query_as::<_, ActiveDeployment>(
        r#"
        SELECT
            hm.name,
            hm.deployment_url,
            hm.region,
            hm.created_at,
            (
                SELECT dm.requests
                FROM deployment_metrics dm
                WHERE dm.hosted_mock_id = hm.id
                ORDER BY dm.period_start DESC
                LIMIT 1
            ) AS requests
        FROM hosted_mocks hm
        WHERE hm.org_id = $1 AND hm.status = 'active' AND hm.deleted_at IS NULL
        ORDER BY hm.created_at DESC
        LIMIT 25
        "#,
    )
    .bind(org_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let now = chrono::Utc::now();
    rows.into_iter()
        .map(|r| {
            let uptime = (now - r.created_at).num_seconds().max(0) as u64;
            ServerStatus {
                server_type: format!("{} ({})", r.name, r.region),
                address: r.deployment_url,
                running: true,
                start_time: Some(r.created_at),
                uptime_seconds: Some(uptime),
                active_connections: 0,
                total_requests: r.requests.unwrap_or(0).max(0) as u64,
            }
        })
        .collect()
}
