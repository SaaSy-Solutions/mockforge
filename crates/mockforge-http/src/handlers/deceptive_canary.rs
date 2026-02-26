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
    let config = state.router.config();
    Json(json!({
        "success": true,
        "config": config,
    }))
}

/// Update canary configuration
///
/// POST /api/v1/deceptive-canary/config
pub async fn update_canary_config(
    State(_state): State<DeceptiveCanaryState>,
    Json(request): Json<UpdateCanaryConfigRequest>,
) -> Json<Value> {
    // Update router configuration
    // Note: This requires mutable access, which would need Arc<RwLock<DeceptiveCanaryRouter>>
    // For now, we'll return the config that should be applied
    Json(json!({
        "success": true,
        "message": "Configuration update requires router state management",
        "config": request.config,
    }))
}

/// Get canary routing statistics
///
/// GET /api/v1/deceptive-canary/stats
pub async fn get_canary_stats(State(state): State<DeceptiveCanaryState>) -> Json<Value> {
    let stats = state.router.stats();
    let canary_percentage = stats.canary_percentage();

    Json(json!({
        "success": true,
        "stats": stats,
        "canary_percentage": canary_percentage,
    }))
}
