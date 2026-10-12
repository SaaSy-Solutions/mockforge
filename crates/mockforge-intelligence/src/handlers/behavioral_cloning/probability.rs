//! Probability models and sampling from recorded exchanges.

use crate::behavioral_cloning::storage::{BehavioralCloningStateProvider, BehavioralCloningStore};
use crate::behavioral_cloning::{EndpointProbabilityModel, ProbabilisticModel};
use axum::extract::{Path, State};
use axum::response::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Request to build a probability model for an endpoint
#[derive(Debug, Deserialize)]
pub struct BuildProbabilityModelRequest {
    /// Endpoint path
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// Optional limit on number of samples to analyze
    #[serde(default)]
    pub sample_limit: Option<u32>,
}

/// Response for probability model
#[derive(Debug, Serialize)]
pub struct ProbabilityModelResponse {
    /// Success flag
    pub success: bool,
    /// Probability model
    pub model: EndpointProbabilityModel,
}

/// Build a probability model for an endpoint
///
/// POST /api/v1/behavioral-cloning/probability-models
pub async fn build_probability_model<S: BehavioralCloningStateProvider>(
    State(state): State<S>,
    Json(request): Json<BuildProbabilityModelRequest>,
) -> Result<Json<Value>, String> {
    // Open database connection
    let db = state.open_store().await?;

    // Get exchanges for the endpoint
    let limit = request.sample_limit.map(|l| l as i32);
    let exchanges = db
        .get_exchanges_for_endpoint(&request.endpoint, &request.method, limit)
        .await
        .map_err(|e| format!("Failed to query exchanges: {}", e))?;

    if exchanges.is_empty() {
        return Err(format!(
            "No recorded traffic found for {} {}",
            request.method, request.endpoint
        ));
    }

    // Extract data for model building
    let mut status_codes = Vec::new();
    let mut latencies_ms = Vec::new();
    let mut error_responses = Vec::new();

    for (req, resp_opt) in &exchanges {
        // Get status code from request or response
        let status_code = if let Some(resp) = resp_opt {
            resp.status_code as u16
        } else if let Some(code) = req.status_code {
            code as u16
        } else {
            continue; // Skip if no status code
        };

        status_codes.push(status_code);

        // Get latency
        if let Some(duration) = req.duration_ms {
            latencies_ms.push(duration as u64);
        }

        // Extract error response body if status >= 400
        if status_code >= 400 {
            if let Some(resp) = resp_opt {
                if let Some(ref body) = resp.body {
                    // Try to parse as JSON
                    if let Ok(json_body) = serde_json::from_str::<Value>(body) {
                        error_responses.push((status_code, json_body));
                    } else {
                        // If not JSON, create a simple error object
                        error_responses.push((
                            status_code,
                            json!({
                                "error": body.clone()
                            }),
                        ));
                    }
                }
            }
        }
    }

    // Extract request and response payloads
    let mut request_payloads = Vec::new();
    let mut response_payloads = Vec::new();

    for (req, resp_opt) in &exchanges {
        // Parse request body if available
        if let Some(ref body) = req.body {
            if let Ok(json) = serde_json::from_str::<Value>(body) {
                request_payloads.push(json);
            }
        }

        // Parse response body if available
        if let Some(ref resp) = resp_opt {
            if let Some(ref body) = resp.body {
                if let Ok(json) = serde_json::from_str::<Value>(body) {
                    response_payloads.push(json);
                }
            }
        }
    }

    // Build probability model
    let model = ProbabilisticModel::build_probability_model_from_data(
        &request.endpoint,
        &request.method,
        &status_codes,
        &latencies_ms,
        &error_responses,
        &request_payloads,
        &response_payloads,
    );

    // Store model in database
    db.insert_endpoint_probability_model(&model)
        .await
        .map_err(|e| format!("Failed to store probability model: {}", e))?;

    Ok(Json(json!({
        "success": true,
        "model": model
    })))
}

/// Get a probability model for an endpoint
///
/// GET /api/v1/behavioral-cloning/probability-models/{endpoint}/{method}
pub async fn get_probability_model<S: BehavioralCloningStateProvider>(
    Path((endpoint, method)): Path<(String, String)>,
    State(state): State<S>,
) -> Result<Json<Value>, String> {
    let db = state.open_store().await?;

    let model = db
        .get_endpoint_probability_model(&endpoint, &method)
        .await
        .map_err(|e| format!("Failed to query probability model: {}", e))?
        .ok_or_else(|| format!("No probability model found for {} {}", method, endpoint))?;

    Ok(Json(json!({
        "success": true,
        "model": model
    })))
}

/// List all probability models
///
/// GET /api/v1/behavioral-cloning/probability-models
pub async fn list_probability_models<S: BehavioralCloningStateProvider>(
    State(state): State<S>,
) -> Result<Json<Value>, String> {
    let db = state.open_store().await?;

    let models = db
        .get_all_endpoint_probability_models()
        .await
        .map_err(|e| format!("Failed to query probability models: {}", e))?;

    Ok(Json(json!({
        "success": true,
        "models": models,
        "count": models.len()
    })))
}

/// Sample a status code from a probability model
///
/// POST /api/v1/behavioral-cloning/probability-models/{endpoint}/{method}/sample/status-code
pub async fn sample_status_code<S: BehavioralCloningStateProvider>(
    Path((endpoint, method)): Path<(String, String)>,
    State(state): State<S>,
) -> Result<Json<Value>, String> {
    let db = state.open_store().await?;

    let model = db
        .get_endpoint_probability_model(&endpoint, &method)
        .await
        .map_err(|e| format!("Failed to query model: {}", e))?
        .ok_or_else(|| format!("No probability model found for {} {}", method, endpoint))?;

    let sampled_code = ProbabilisticModel::sample_status_code(&model);

    Ok(Json(json!({
        "success": true,
        "endpoint": endpoint,
        "method": method,
        "status_code": sampled_code
    })))
}

/// Sample latency from a probability model
///
/// POST /api/v1/behavioral-cloning/probability-models/{endpoint}/{method}/sample/latency
pub async fn sample_latency<S: BehavioralCloningStateProvider>(
    Path((endpoint, method)): Path<(String, String)>,
    State(state): State<S>,
) -> Result<Json<Value>, String> {
    let db = state.open_store().await?;

    let model = db
        .get_endpoint_probability_model(&endpoint, &method)
        .await
        .map_err(|e| format!("Failed to query model: {}", e))?
        .ok_or_else(|| format!("No probability model found for {} {}", method, endpoint))?;

    let sampled_latency = ProbabilisticModel::sample_latency(&model);

    Ok(Json(json!({
        "success": true,
        "endpoint": endpoint,
        "method": method,
        "latency_ms": sampled_latency
    })))
}
