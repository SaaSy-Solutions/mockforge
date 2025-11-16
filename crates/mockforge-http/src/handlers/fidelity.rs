//! Fidelity score API handlers

use axum::extract::{Path, Query, State};
use axum::response::Json;
use mockforge_core::fidelity::{FidelityCalculator, FidelityScore};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

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

/// Calculate fidelity score
///
/// POST /api/v1/workspace/{workspace_id}/fidelity
pub async fn calculate_fidelity(
    Path(workspace_id): Path<String>,
    State(_state): State<Arc<()>>, // TODO: Replace with actual state
    Json(request): Json<CalculateFidelityRequest>,
) -> Json<Value> {
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

    Json(json!({
        "success": true,
        "workspace_id": workspace_id,
        "score": score
    }))
}

/// Get fidelity score for a workspace
///
/// GET /api/v1/workspace/{workspace_id}/fidelity
pub async fn get_fidelity(
    Path(workspace_id): Path<String>,
    State(_state): State<Arc<()>>, // TODO: Replace with actual state
) -> Json<Value> {
    // TODO: Retrieve stored fidelity score from database/workspace
    // For now, return a placeholder response
    Json(json!({
        "success": true,
        "workspace_id": workspace_id,
        "message": "Fidelity score retrieval not yet implemented"
    }))
}
