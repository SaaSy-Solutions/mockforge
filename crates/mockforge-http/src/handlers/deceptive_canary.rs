//! Deceptive Canary API handlers

use crate::middleware::DeceptiveCanaryState;
use axum::extract::State;
use axum::response::Json;
use mockforge_core::deceptive_canary::DeceptiveCanaryConfig;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Request to update canary configuration
#[derive(Debug, Deserialize)]
pub struct UpdateCanaryConfigRequest {
    /// New canary configuration
    pub config: DeceptiveCanaryConfig,
}

/// Response for canary statistics
#[derive(Debug, Serialize)]
pub struct CanaryStatsResponse {
    /// Success flag
    pub success: bool,
    /// Current statistics
    pub stats: Value,
    /// Canary routing percentage
    pub canary_percentage: f64,
}

/// Get current canary configuration
///
/// GET /api/v1/deceptive-canary/config
pub async fn get_canary_config(State(state): State<DeceptiveCanaryState>) -> Json<Value> {
    let router = state.router.read().await;
    let config = router.config();
    Json(json!({
        "success": true,
        "config": config,
    }))
}

/// Update canary configuration
///
/// POST /api/v1/deceptive-canary/config
pub async fn update_canary_config(
    State(state): State<DeceptiveCanaryState>,
    Json(request): Json<UpdateCanaryConfigRequest>,
) -> Json<Value> {
    let mut router = state.router.write().await;
    router.update_config(request.config.clone());
    Json(json!({
        "success": true,
        "message": "Configuration updated successfully",
        "config": request.config,
    }))
}

/// Get canary routing statistics
///
/// GET /api/v1/deceptive-canary/stats
pub async fn get_canary_stats(State(state): State<DeceptiveCanaryState>) -> Json<Value> {
    let router = state.router.read().await;
    let stats = router.stats();
    let canary_percentage = stats.canary_percentage();

    Json(json!({
        "success": true,
        "stats": stats,
        "canary_percentage": canary_percentage,
    }))
}
