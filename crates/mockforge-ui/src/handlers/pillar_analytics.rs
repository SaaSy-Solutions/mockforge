//! Pillar Usage Analytics API handlers
//!
//! Provides endpoints for querying pillar usage metrics (Reality, Contracts, DevX, Cloud, AI)
//! at both workspace and organization levels.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use mockforge_analytics::{AnalyticsDatabase, PillarUsageMetrics};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error};

use crate::models::ApiResponse;

/// Pillar analytics state
#[derive(Clone)]
pub struct PillarAnalyticsState {
    pub db: Arc<AnalyticsDatabase>,
}

impl PillarAnalyticsState {
    pub fn new(db: AnalyticsDatabase) -> Self {
        Self { db: Arc::new(db) }
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

/// Get pillar usage metrics for a workspace
///
/// Returns comprehensive pillar usage metrics including:
/// - Reality pillar: blended reality usage, personas, chaos
/// - Contracts pillar: validation modes, drift budgets
/// - DevX pillar: SDK usage, client generations
/// - Cloud pillar: shared scenarios, templates
/// - AI pillar: AI-generated mocks, contract diffs
pub async fn get_workspace_pillar_metrics(
    State(state): State<PillarAnalyticsState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<PillarAnalyticsQuery>,
) -> Result<Json<ApiResponse<PillarUsageMetrics>>, StatusCode> {
    debug!("Fetching pillar metrics for workspace: {}", workspace_id);

    let duration = if let (Some(start), Some(end)) = (query.start_time, query.end_time) {
        end - start
    } else {
        query.duration
    };

    match state.db.get_workspace_pillar_metrics(&workspace_id, duration).await {
        Ok(metrics) => Ok(Json(ApiResponse::success(metrics))),
        Err(e) => {
            error!("Failed to get workspace pillar metrics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get pillar usage metrics for an organization
///
/// Returns aggregated pillar metrics across all workspaces in the organization.
pub async fn get_org_pillar_metrics(
    State(state): State<PillarAnalyticsState>,
    Path(org_id): Path<String>,
    Query(query): Query<PillarAnalyticsQuery>,
) -> Result<Json<ApiResponse<PillarUsageMetrics>>, StatusCode> {
    debug!("Fetching pillar metrics for org: {}", org_id);

    let duration = if let (Some(start), Some(end)) = (query.start_time, query.end_time) {
        end - start
    } else {
        query.duration
    };

    match state.db.get_org_pillar_metrics(&org_id, duration).await {
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

pub async fn get_reality_pillar_details(
    State(state): State<PillarAnalyticsState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<PillarAnalyticsQuery>,
) -> Result<Json<ApiResponse<RealityPillarDetails>>, StatusCode> {
    debug!("Fetching Reality pillar details for workspace: {}", workspace_id);

    let duration = if let (Some(start), Some(end)) = (query.start_time, query.end_time) {
        end - start
    } else {
        query.duration
    };

    match state.db.get_workspace_pillar_metrics(&workspace_id, duration).await {
        Ok(metrics) => {
            let details = RealityPillarDetails {
                metrics: metrics.reality,
                time_range: metrics.time_range,
            };
            Ok(Json(ApiResponse::success(details)))
        }
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

pub async fn get_contracts_pillar_details(
    State(state): State<PillarAnalyticsState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<PillarAnalyticsQuery>,
) -> Result<Json<ApiResponse<ContractsPillarDetails>>, StatusCode> {
    debug!("Fetching Contracts pillar details for workspace: {}", workspace_id);

    let duration = if let (Some(start), Some(end)) = (query.start_time, query.end_time) {
        end - start
    } else {
        query.duration
    };

    match state.db.get_workspace_pillar_metrics(&workspace_id, duration).await {
        Ok(metrics) => {
            let details = ContractsPillarDetails {
                metrics: metrics.contracts,
                time_range: metrics.time_range,
            };
            Ok(Json(ApiResponse::success(details)))
        }
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

pub async fn get_ai_pillar_details(
    State(state): State<PillarAnalyticsState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<PillarAnalyticsQuery>,
) -> Result<Json<ApiResponse<AiPillarDetails>>, StatusCode> {
    debug!("Fetching AI pillar details for workspace: {}", workspace_id);

    let duration = if let (Some(start), Some(end)) = (query.start_time, query.end_time) {
        end - start
    } else {
        query.duration
    };

    match state.db.get_workspace_pillar_metrics(&workspace_id, duration).await {
        Ok(metrics) => {
            let details = AiPillarDetails {
                metrics: metrics.ai,
                time_range: metrics.time_range,
            };
            Ok(Json(ApiResponse::success(details)))
        }
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

/// Get pillar usage summary showing most/least used pillars
///
/// This endpoint provides a high-level view of pillar usage, ranking pillars
/// by their usage metrics to help identify where investment and usage are concentrated.
pub async fn get_pillar_usage_summary(
    State(state): State<PillarAnalyticsState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<PillarAnalyticsQuery>,
) -> Result<Json<ApiResponse<PillarUsageSummary>>, StatusCode> {
    debug!("Fetching pillar usage summary for workspace: {}", workspace_id);

    let duration = if let (Some(start), Some(end)) = (query.start_time, query.end_time) {
        end - start
    } else {
        query.duration
    };

    match state.db.get_workspace_pillar_metrics(&workspace_id, duration).await {
        Ok(metrics) => {
            let mut rankings = Vec::new();
            let mut total_usage = 0u64;

            // Calculate usage scores for each pillar
            // Reality: blended_reality_percent + smart_personas_percent + chaos_enabled_count
            if let Some(ref reality) = metrics.reality {
                let usage = (reality.blended_reality_percent + reality.smart_personas_percent)
                    as u64
                    + reality.chaos_enabled_count;
                rankings.push(PillarRanking {
                    pillar: "Reality".to_string(),
                    usage,
                    percentage: 0.0, // Will calculate after total
                    is_most_used: false,
                    is_least_used: false,
                });
                total_usage += usage;
            }

            // Contracts: validation_enforce_percent + drift_budget_configured_count + drift_incidents_count
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

            // DevX: sdk_installations + client_generations + playground_sessions
            if let Some(ref devx) = metrics.devx {
                let usage =
                    devx.sdk_installations + devx.client_generations + devx.playground_sessions;
                rankings.push(PillarRanking {
                    pillar: "DevX".to_string(),
                    usage,
                    percentage: 0.0,
                    is_most_used: false,
                    is_least_used: false,
                });
                total_usage += usage;
            }

            // Cloud: shared_scenarios_count + marketplace_downloads + collaborative_workspaces
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

            // AI: ai_generated_mocks + ai_contract_diffs + llm_assisted_operations
            if let Some(ref ai) = metrics.ai {
                let usage =
                    ai.ai_generated_mocks + ai.ai_contract_diffs + ai.llm_assisted_operations;
                rankings.push(PillarRanking {
                    pillar: "AI".to_string(),
                    usage,
                    percentage: 0.0,
                    is_most_used: false,
                    is_least_used: false,
                });
                total_usage += usage;
            }

            // Calculate percentages and sort by usage (highest first)
            for ranking in &mut rankings {
                if total_usage > 0 {
                    ranking.percentage = (ranking.usage as f64 / total_usage as f64) * 100.0;
                }
            }

            rankings.sort_by(|a, b| b.usage.cmp(&a.usage));

            // Mark most/least used
            let rankings_len = rankings.len();
            if let Some(first) = rankings.first_mut() {
                first.is_most_used = true;
            }
            if rankings_len > 1 {
                if let Some(last) = rankings.last_mut() {
                    last.is_least_used = true;
                }
            }

            let summary = PillarUsageSummary {
                time_range: metrics.time_range,
                rankings,
                total_usage,
            };

            Ok(Json(ApiResponse::success(summary)))
        }
        Err(e) => {
            error!("Failed to get pillar usage summary: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
