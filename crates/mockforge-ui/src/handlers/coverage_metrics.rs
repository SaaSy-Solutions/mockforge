//! Coverage Metrics API handlers (MockOps)
//!
//! Provides endpoints for scenario usage, persona CI hits, endpoint coverage,
//! reality level staleness, and drift percentage metrics.

use axum::{extract::Query, http::StatusCode, Json};
use mockforge_analytics::{
    AnalyticsDatabase, DriftPercentageMetrics, EndpointCoverage, PersonaCIHit,
    RealityLevelStaleness, ScenarioUsageMetrics,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, error};

use crate::models::ApiResponse;

/// Coverage metrics state
#[derive(Clone)]
pub struct CoverageMetricsState {
    pub db: Arc<tokio::sync::OnceCell<AnalyticsDatabase>>,
}

impl CoverageMetricsState {
    pub fn new(db: AnalyticsDatabase) -> Self {
        let cell = tokio::sync::OnceCell::new();
        let _ = cell.set(db);
        Self { db: Arc::new(cell) }
    }

    async fn get_db(&self) -> Result<&AnalyticsDatabase, StatusCode> {
        self.db.get().ok_or_else(|| {
            error!("Analytics database not initialized");
            StatusCode::SERVICE_UNAVAILABLE
        })
    }
}

/// Query parameters for coverage metrics endpoints
#[derive(Debug, Deserialize)]
pub struct CoverageQuery {
    /// Workspace ID filter
    pub workspace_id: Option<String>,
    /// Organization ID filter
    pub org_id: Option<String>,
    /// Limit results
    pub limit: Option<i64>,
    /// Minimum coverage percentage (for endpoint coverage)
    pub min_coverage: Option<f64>,
    /// Maximum staleness days (for reality level staleness)
    pub max_staleness_days: Option<i32>,
}

/// Get scenario usage metrics
///
/// GET /api/v2/analytics/scenarios/usage
pub async fn get_scenario_usage(
    axum::extract::Extension(state): axum::extract::Extension<CoverageMetricsState>,
    Query(params): Query<CoverageQuery>,
) -> Result<Json<ApiResponse<Vec<ScenarioUsageMetrics>>>, StatusCode> {
    debug!("Getting scenario usage metrics");

    let db = state.get_db().await?;
    match db
        .get_scenario_usage(params.workspace_id.as_deref(), params.org_id.as_deref(), params.limit)
        .await
    {
        Ok(metrics) => Ok(Json(ApiResponse::success(metrics))),
        Err(e) => {
            error!("Failed to get scenario usage metrics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get persona CI hits
///
/// GET /api/v2/analytics/personas/ci-hits
pub async fn get_persona_ci_hits(
    axum::extract::Extension(state): axum::extract::Extension<CoverageMetricsState>,
    Query(params): Query<CoverageQuery>,
) -> Result<Json<ApiResponse<Vec<PersonaCIHit>>>, StatusCode> {
    debug!("Getting persona CI hits");

    let db = state.get_db().await?;
    match db
        .get_persona_ci_hits(params.workspace_id.as_deref(), params.org_id.as_deref(), params.limit)
        .await
    {
        Ok(hits) => Ok(Json(ApiResponse::success(hits))),
        Err(e) => {
            error!("Failed to get persona CI hits: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get endpoint coverage
///
/// GET /api/v2/analytics/endpoints/coverage
pub async fn get_endpoint_coverage(
    axum::extract::Extension(state): axum::extract::Extension<CoverageMetricsState>,
    Query(params): Query<CoverageQuery>,
) -> Result<Json<ApiResponse<Vec<EndpointCoverage>>>, StatusCode> {
    debug!("Getting endpoint coverage");

    let db = state.get_db().await?;
    match db
        .get_endpoint_coverage(
            params.workspace_id.as_deref(),
            params.org_id.as_deref(),
            params.min_coverage,
        )
        .await
    {
        Ok(coverage) => Ok(Json(ApiResponse::success(coverage))),
        Err(e) => {
            error!("Failed to get endpoint coverage: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get reality level staleness
///
/// GET /api/v2/analytics/reality-levels/staleness
pub async fn get_reality_level_staleness(
    axum::extract::Extension(state): axum::extract::Extension<CoverageMetricsState>,
    Query(params): Query<CoverageQuery>,
) -> Result<Json<ApiResponse<Vec<RealityLevelStaleness>>>, StatusCode> {
    debug!("Getting reality level staleness");

    let db = state.get_db().await?;
    match db
        .get_reality_level_staleness(
            params.workspace_id.as_deref(),
            params.org_id.as_deref(),
            params.max_staleness_days,
        )
        .await
    {
        Ok(staleness) => Ok(Json(ApiResponse::success(staleness))),
        Err(e) => {
            error!("Failed to get reality level staleness: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get drift percentage metrics
///
/// GET /api/v2/analytics/drift/percentage
pub async fn get_drift_percentage(
    axum::extract::Extension(state): axum::extract::Extension<CoverageMetricsState>,
    Query(params): Query<CoverageQuery>,
) -> Result<Json<ApiResponse<Vec<DriftPercentageMetrics>>>, StatusCode> {
    debug!("Getting drift percentage metrics");

    let db = state.get_db().await?;
    match db
        .get_drift_percentage(
            params.workspace_id.as_deref(),
            params.org_id.as_deref(),
            params.limit,
        )
        .await
    {
        Ok(metrics) => Ok(Json(ApiResponse::success(metrics))),
        Err(e) => {
            error!("Failed to get drift percentage metrics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Create coverage metrics router
/// Note: Returns a router without state - handlers will get state from extensions
pub fn coverage_metrics_router() -> axum::Router {
    use axum::routing::get;

    axum::Router::new()
        .route("/scenarios/usage", get(get_scenario_usage))
        .route("/personas/ci-hits", get(get_persona_ci_hits))
        .route("/endpoints/coverage", get(get_endpoint_coverage))
        .route("/reality-levels/staleness", get(get_reality_level_staleness))
        .route("/drift/percentage", get(get_drift_percentage))
}
