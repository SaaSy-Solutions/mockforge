//! Fidelity score API handlers

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use mockforge_core::fidelity::{FidelityCalculator, FidelityScore};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

/// State for fidelity handlers
#[derive(Clone)]
pub struct FidelityState {
    /// Stored fidelity scores per workspace
    scores: Arc<RwLock<HashMap<String, FidelityScore>>>,
}

/// Request to calculate fidelity score
#[derive(Debug, Deserialize)]
pub struct CalculateFidelityRequest {
    /// Mock schema
    pub mock_schema: Value,
    /// Real schema
    pub real_schema: Value,
    /// Mock sample responses
    #[serde(default)]
    pub mock_samples: Vec<Value>,
    /// Real sample responses
    #[serde(default)]
    pub real_samples: Vec<Value>,
    /// Mock response times (optional)
    #[serde(default)]
    pub mock_response_times: Option<Vec<u64>>,
    /// Real response times (optional)
    #[serde(default)]
    pub real_response_times: Option<Vec<u64>>,
    /// Mock error patterns (optional)
    #[serde(default)]
    pub mock_error_patterns: Option<HashMap<String, usize>>,
    /// Real error patterns (optional)
    #[serde(default)]
    pub real_error_patterns: Option<HashMap<String, usize>>,
}

/// Response for fidelity score calculation
#[derive(Debug, Serialize)]
pub struct FidelityResponse {
    /// Success flag
    pub success: bool,
    /// Fidelity score
    pub score: FidelityScore,
}

impl FidelityState {
    /// Create a new fidelity state
    pub fn new() -> Self {
        Self {
            scores: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for FidelityState {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate fidelity score
///
/// POST /api/v1/workspace/{workspace_id}/fidelity
pub async fn calculate_fidelity(
    Path(workspace_id): Path<String>,
    State(state): State<FidelityState>,
    Json(request): Json<CalculateFidelityRequest>,
) -> Result<Json<Value>, StatusCode> {
    let calculator = FidelityCalculator::new();

    let score = calculator.calculate(
        &request.mock_schema,
        &request.real_schema,
        &request.mock_samples,
        &request.real_samples,
        request.mock_response_times.as_deref(),
        request.real_response_times.as_deref(),
        request.mock_error_patterns.as_ref(),
        request.real_error_patterns.as_ref(),
    );

    // Store the score
    {
        let mut scores = state.scores.write().await;
        scores.insert(workspace_id.clone(), score.clone());
    }

    info!("Calculated fidelity score for workspace: {} - Overall: {:.2}%",
          workspace_id, score.overall * 100.0);

    Ok(Json(json!({
        "success": true,
        "workspace_id": workspace_id,
        "score": score
    })))
}

/// Get fidelity score for a workspace
///
/// GET /api/v1/workspace/{workspace_id}/fidelity
pub async fn get_fidelity(
    Path(workspace_id): Path<String>,
    State(state): State<FidelityState>,
) -> Result<Json<Value>, StatusCode> {
    let scores = state.scores.read().await;

    if let Some(score) = scores.get(&workspace_id) {
        // Return score with driver metrics breakdown
        Ok(Json(json!({
            "success": true,
            "workspace_id": workspace_id,
            "score": {
                "overall": score.overall,
                "overall_percentage": (score.overall * 100.0).round() as u8,
                "driver_metrics": {
                    "schema_similarity": {
                        "value": score.schema_similarity,
                        "percentage": (score.schema_similarity * 100.0).round() as u8,
                        "label": "Schema Match"
                    },
                    "sample_similarity": {
                        "value": score.sample_similarity,
                        "percentage": (score.sample_similarity * 100.0).round() as u8,
                        "label": "Sample Similarity"
                    },
                    "response_time_similarity": {
                        "value": score.response_time_similarity,
                        "percentage": (score.response_time_similarity * 100.0).round() as u8,
                        "label": "Response Time Match"
                    },
                    "error_pattern_similarity": {
                        "value": score.error_pattern_similarity,
                        "percentage": (score.error_pattern_similarity * 100.0).round() as u8,
                        "label": "Error Pattern Match"
                    }
                },
                "computed_at": score.computed_at,
                "metadata": score.metadata
            }
        })))
    } else {
        error!("Fidelity score not found for workspace: {}", workspace_id);
        Err(StatusCode::NOT_FOUND)
    }
}

/// Create fidelity router
pub fn fidelity_router(state: FidelityState) -> axum::Router {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/api/v1/workspace/:workspace_id/fidelity", get(get_fidelity).post(calculate_fidelity))
        .with_state(state)
}
