//! Pillar Usage Analytics API handlers
//!
//! Provides endpoints for querying pillar usage metrics (Reality, Contracts, DevX, Cloud, AI)
//! at both workspace and organization levels.

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Json,
};
use mockforge_analytics::{AnalyticsDatabase, PillarUsageMetrics};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::{debug, error};

use crate::models::ApiResponse;

/// Pillar analytics state backed by a lazy-initialized `AnalyticsDatabase`.
///
/// Shares the same `OnceCell<AnalyticsDatabase>` with coverage metrics so the
/// admin UI only needs to initialize the database once.
#[derive(Clone)]
pub struct PillarAnalyticsState {
    pub db: Arc<OnceCell<AnalyticsDatabase>>,
}

impl PillarAnalyticsState {
    pub fn new(db: Arc<OnceCell<AnalyticsDatabase>>) -> Self {
        Self { db }
    }

    async fn get_db(&self) -> Result<&AnalyticsDatabase, StatusCode> {
        self.db.get().ok_or_else(|| {
            error!("Analytics database not initialized");
            StatusCode::SERVICE_UNAVAILABLE
        })
    }
}

/// Query parameters for pillar analytics
#[derive(Debug, Deserialize)]
pub struct PillarAnalyticsQuery {
    /// Duration in seconds (default: 3600 = 1 hour)
    #[serde(default = "default_duration")]
    pub duration: i64,
    /// Start time (Unix timestamp, optional)
    pub start_time: Option<i64>,
    /// End time (Unix timestamp, optional)
    pub end_time: Option<i64>,
}

fn default_duration() -> i64 {
    3600 // 1 hour
}

fn resolve_duration(query: &PillarAnalyticsQuery) -> i64 {
    if let (Some(start), Some(end)) = (query.start_time, query.end_time) {
        end - start
    } else {
        query.duration
    }
}

/// GET /api/v2/analytics/pillars/workspace/{workspace_id}
pub async fn get_workspace_pillar_metrics(
    axum::extract::Extension(state): axum::extract::Extension<PillarAnalyticsState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<PillarAnalyticsQuery>,
) -> Result<Json<ApiResponse<PillarUsageMetrics>>, StatusCode> {
    debug!("Fetching pillar metrics for workspace: {}", workspace_id);

    let db = state.get_db().await?;
    let duration = resolve_duration(&query);

    match db.get_workspace_pillar_metrics(&workspace_id, duration).await {
        Ok(metrics) => Ok(Json(ApiResponse::success(metrics))),
        Err(e) => {
            error!("Failed to get workspace pillar metrics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/v2/analytics/pillars/org/{org_id}
pub async fn get_org_pillar_metrics(
    axum::extract::Extension(state): axum::extract::Extension<PillarAnalyticsState>,
    Path(org_id): Path<String>,
    Query(query): Query<PillarAnalyticsQuery>,
) -> Result<Json<ApiResponse<PillarUsageMetrics>>, StatusCode> {
    debug!("Fetching pillar metrics for org: {}", org_id);

    let db = state.get_db().await?;
    let duration = resolve_duration(&query);

    match db.get_org_pillar_metrics(&org_id, duration).await {
        Ok(metrics) => Ok(Json(ApiResponse::success(metrics))),
        Err(e) => {
            error!("Failed to get org pillar metrics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get Reality pillar detailed metrics
#[derive(Debug, Serialize)]
pub struct RealityPillarDetails {
    pub metrics: Option<mockforge_analytics::RealityPillarMetrics>,
    pub time_range: String,
}

/// GET /api/v2/analytics/pillars/workspace/{workspace_id}/reality
pub async fn get_reality_pillar_details(
    axum::extract::Extension(state): axum::extract::Extension<PillarAnalyticsState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<PillarAnalyticsQuery>,
) -> Result<Json<ApiResponse<RealityPillarDetails>>, StatusCode> {
    debug!("Fetching Reality pillar details for workspace: {}", workspace_id);

    let db = state.get_db().await?;
    let duration = resolve_duration(&query);

    match db.get_workspace_pillar_metrics(&workspace_id, duration).await {
        Ok(metrics) => Ok(Json(ApiResponse::success(RealityPillarDetails {
            metrics: metrics.reality,
            time_range: metrics.time_range,
        }))),
        Err(e) => {
            error!("Failed to get Reality pillar details: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get Contracts pillar detailed metrics
#[derive(Debug, Serialize)]
pub struct ContractsPillarDetails {
    pub metrics: Option<mockforge_analytics::ContractsPillarMetrics>,
    pub time_range: String,
}

/// GET /api/v2/analytics/pillars/workspace/{workspace_id}/contracts
pub async fn get_contracts_pillar_details(
    axum::extract::Extension(state): axum::extract::Extension<PillarAnalyticsState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<PillarAnalyticsQuery>,
) -> Result<Json<ApiResponse<ContractsPillarDetails>>, StatusCode> {
    debug!("Fetching Contracts pillar details for workspace: {}", workspace_id);

    let db = state.get_db().await?;
    let duration = resolve_duration(&query);

    match db.get_workspace_pillar_metrics(&workspace_id, duration).await {
        Ok(metrics) => Ok(Json(ApiResponse::success(ContractsPillarDetails {
            metrics: metrics.contracts,
            time_range: metrics.time_range,
        }))),
        Err(e) => {
            error!("Failed to get Contracts pillar details: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get AI pillar detailed metrics
#[derive(Debug, Serialize)]
pub struct AiPillarDetails {
    pub metrics: Option<mockforge_analytics::AiPillarMetrics>,
    pub time_range: String,
}

/// GET /api/v2/analytics/pillars/workspace/{workspace_id}/ai
pub async fn get_ai_pillar_details(
    axum::extract::Extension(state): axum::extract::Extension<PillarAnalyticsState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<PillarAnalyticsQuery>,
) -> Result<Json<ApiResponse<AiPillarDetails>>, StatusCode> {
    debug!("Fetching AI pillar details for workspace: {}", workspace_id);

    let db = state.get_db().await?;
    let duration = resolve_duration(&query);

    match db.get_workspace_pillar_metrics(&workspace_id, duration).await {
        Ok(metrics) => Ok(Json(ApiResponse::success(AiPillarDetails {
            metrics: metrics.ai,
            time_range: metrics.time_range,
        }))),
        Err(e) => {
            error!("Failed to get AI pillar details: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Pillar usage summary showing most/least used pillars
#[derive(Debug, Serialize)]
pub struct PillarUsageSummary {
    /// Time range for the summary
    pub time_range: String,
    /// Pillar usage rankings (sorted by usage, highest first)
    pub rankings: Vec<PillarRanking>,
    /// Total usage across all pillars
    pub total_usage: u64,
}

/// Individual pillar ranking
#[derive(Debug, Serialize)]
pub struct PillarRanking {
    /// Pillar name
    pub pillar: String,
    /// Usage count/score for this pillar
    pub usage: u64,
    /// Percentage of total usage
    pub percentage: f64,
    /// Whether this is the most used pillar
    pub is_most_used: bool,
    /// Whether this is the least used pillar
    pub is_least_used: bool,
}

fn build_summary(metrics: PillarUsageMetrics) -> PillarUsageSummary {
    let mut rankings = Vec::new();
    let mut total_usage = 0u64;

    if let Some(ref reality) = metrics.reality {
        let usage = (reality.blended_reality_percent + reality.smart_personas_percent) as u64
            + reality.chaos_enabled_count;
        rankings.push(PillarRanking {
            pillar: "Reality".to_string(),
            usage,
            percentage: 0.0,
            is_most_used: false,
            is_least_used: false,
        });
        total_usage += usage;
    }

    if let Some(ref contracts) = metrics.contracts {
        let usage = contracts.validation_enforce_percent as u64
            + contracts.drift_budget_configured_count
            + contracts.drift_incidents_count;
        rankings.push(PillarRanking {
            pillar: "Contracts".to_string(),
            usage,
            percentage: 0.0,
            is_most_used: false,
            is_least_used: false,
        });
        total_usage += usage;
    }

    if let Some(ref devx) = metrics.devx {
        let usage = devx.sdk_installations + devx.client_generations + devx.playground_sessions;
        rankings.push(PillarRanking {
            pillar: "DevX".to_string(),
            usage,
            percentage: 0.0,
            is_most_used: false,
            is_least_used: false,
        });
        total_usage += usage;
    }

    if let Some(ref cloud) = metrics.cloud {
        let usage = cloud.shared_scenarios_count
            + cloud.marketplace_downloads
            + cloud.collaborative_workspaces;
        rankings.push(PillarRanking {
            pillar: "Cloud".to_string(),
            usage,
            percentage: 0.0,
            is_most_used: false,
            is_least_used: false,
        });
        total_usage += usage;
    }

    if let Some(ref ai) = metrics.ai {
        let usage = ai.ai_generated_mocks + ai.ai_contract_diffs + ai.llm_assisted_operations;
        rankings.push(PillarRanking {
            pillar: "AI".to_string(),
            usage,
            percentage: 0.0,
            is_most_used: false,
            is_least_used: false,
        });
        total_usage += usage;
    }

    for ranking in &mut rankings {
        if total_usage > 0 {
            ranking.percentage = (ranking.usage as f64 / total_usage as f64) * 100.0;
        }
    }

    rankings.sort_by(|a, b| b.usage.cmp(&a.usage));

    let rankings_len = rankings.len();
    if let Some(first) = rankings.first_mut() {
        first.is_most_used = true;
    }
    if rankings_len > 1 {
        if let Some(last) = rankings.last_mut() {
            last.is_least_used = true;
        }
    }

    PillarUsageSummary {
        time_range: metrics.time_range,
        rankings,
        total_usage,
    }
}

/// GET /api/v2/analytics/pillars/workspace/{workspace_id}/summary
pub async fn get_workspace_pillar_usage_summary(
    axum::extract::Extension(state): axum::extract::Extension<PillarAnalyticsState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<PillarAnalyticsQuery>,
) -> Result<Json<ApiResponse<PillarUsageSummary>>, StatusCode> {
    debug!("Fetching pillar usage summary for workspace: {}", workspace_id);

    let db = state.get_db().await?;
    let duration = resolve_duration(&query);

    match db.get_workspace_pillar_metrics(&workspace_id, duration).await {
        Ok(metrics) => Ok(Json(ApiResponse::success(build_summary(metrics)))),
        Err(e) => {
            error!("Failed to get pillar usage summary: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/v2/analytics/pillars/org/{org_id}/summary
pub async fn get_org_pillar_usage_summary(
    axum::extract::Extension(state): axum::extract::Extension<PillarAnalyticsState>,
    Path(org_id): Path<String>,
    Query(query): Query<PillarAnalyticsQuery>,
) -> Result<Json<ApiResponse<PillarUsageSummary>>, StatusCode> {
    debug!("Fetching pillar usage summary for org: {}", org_id);

    let db = state.get_db().await?;
    let duration = resolve_duration(&query);

    match db.get_org_pillar_metrics(&org_id, duration).await {
        Ok(metrics) => Ok(Json(ApiResponse::success(build_summary(metrics)))),
        Err(e) => {
            error!("Failed to get pillar usage summary: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
