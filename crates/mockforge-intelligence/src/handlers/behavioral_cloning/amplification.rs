//! Edge amplification and rare-pattern queries for recorded models.

use crate::behavioral_cloning::storage::{BehavioralCloningStateProvider, BehavioralCloningStore};
use crate::behavioral_cloning::{EdgeAmplificationConfig, EdgeAmplifier};
use axum::extract::Query;
use axum::extract::{Path, State};
use axum::response::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Request to apply edge amplification
#[derive(Debug, Deserialize)]
pub struct ApplyAmplificationRequest {
    /// Amplification configuration
    pub config: EdgeAmplificationConfig,
    /// Optional endpoint to apply to (if scope is Endpoint)
    #[serde(default)]
    pub endpoint: Option<String>,
    /// Optional method to apply to (if scope is Endpoint)
    #[serde(default)]
    pub method: Option<String>,
}

/// Apply edge amplification to a probability model
///
/// POST /api/v1/behavioral-cloning/amplification/apply
pub async fn apply_amplification<S: BehavioralCloningStateProvider>(
    State(state): State<S>,
    Json(request): Json<ApplyAmplificationRequest>,
) -> Result<Json<Value>, String> {
    if !request.config.enabled {
        return Ok(Json(json!({
            "success": true,
            "message": "Amplification disabled"
        })));
    }

    let db = state.open_store().await?;

    // Determine which models to update based on scope
    let models_to_update = match &request.config.scope {
        crate::behavioral_cloning::AmplificationScope::Global => db
            .get_all_endpoint_probability_models()
            .await
            .map_err(|e| format!("Failed to query models: {}", e))?,
        crate::behavioral_cloning::AmplificationScope::Endpoint { endpoint, method } => {
            if let Some(model) = db
                .get_endpoint_probability_model(endpoint, method)
                .await
                .map_err(|e| format!("Failed to query model: {}", e))?
            {
                vec![model]
            } else {
                return Err(format!("No probability model found for {} {}", method, endpoint));
            }
        }
        crate::behavioral_cloning::AmplificationScope::Sequence { sequence_id } => {
            let sequences = db
                .get_behavioral_sequences()
                .await
                .map_err(|e| format!("Failed to query sequences: {}", e))?;
            let sequence = sequences
                .into_iter()
                .find(|s| s.id == *sequence_id)
                .ok_or_else(|| format!("Sequence {} not found", sequence_id))?;

            let mut models = Vec::new();
            for step in sequence.steps {
                if let Some(model) = db
                    .get_endpoint_probability_model(&step.endpoint, &step.method)
                    .await
                    .map_err(|e| format!("Failed to query model: {}", e))?
                {
                    models.push(model);
                }
            }

            if models.is_empty() {
                return Err(format!("No probability models found for sequence {}", sequence_id));
            }

            models
        }
    };

    // Apply amplification to each model
    let mut updated_count = 0;
    for mut model in models_to_update {
        EdgeAmplifier::apply_amplification(&mut model, &request.config)
            .map_err(|e| format!("Failed to apply amplification: {}", e))?;

        // Store updated model
        db.insert_endpoint_probability_model(&model)
            .await
            .map_err(|e| format!("Failed to store updated model: {}", e))?;

        updated_count += 1;
    }

    Ok(Json(json!({
        "success": true,
        "updated_models": updated_count,
        "config": request.config
    })))
}

/// Get rare edge patterns for an endpoint
///
/// GET /api/v1/behavioral-cloning/amplification/rare-edges/{endpoint}/{method}
pub async fn get_rare_edges<S: BehavioralCloningStateProvider>(
    Path((endpoint, method)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<S>,
) -> Result<Json<Value>, String> {
    let db = state.open_store().await?;

    let model = db
        .get_endpoint_probability_model(&endpoint, &method)
        .await
        .map_err(|e| format!("Failed to query model: {}", e))?
        .ok_or_else(|| format!("No probability model found for {} {}", method, endpoint))?;

    let threshold: f64 = params.get("threshold").and_then(|s| s.parse().ok()).unwrap_or(0.01); // Default 1%

    let rare_patterns = EdgeAmplifier::identify_rare_edges(&model, threshold);

    Ok(Json(json!({
        "success": true,
        "endpoint": endpoint,
        "method": method,
        "threshold": threshold,
        "rare_patterns": rare_patterns
    })))
}
