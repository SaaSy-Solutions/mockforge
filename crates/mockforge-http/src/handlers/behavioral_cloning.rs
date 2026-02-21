//! Behavioral cloning API handlers
//!
//! This module provides HTTP endpoints for behavioral cloning features:
//! - Building probability models from recorded traffic
//! - Discovering sequences from traces
//! - Managing edge amplification
//! - Querying learned sequences and models

use axum::extract::{Path, Query, State};
use axum::response::Json;
use mockforge_core::behavioral_cloning::types::BehavioralSequence;
use mockforge_core::behavioral_cloning::{
    EdgeAmplificationConfig, EdgeAmplifier, EndpointProbabilityModel, ProbabilisticModel,
    SequenceLearner,
};
use mockforge_recorder::database::RecorderDatabase;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Behavioral cloning API state
#[derive(Clone)]
pub struct BehavioralCloningState {
    /// Edge amplifier instance
    pub edge_amplifier: Arc<EdgeAmplifier>,
    /// Optional recorder database path
    /// If provided, handlers will open the database as needed
    pub database_path: Option<PathBuf>,
}

impl BehavioralCloningState {
    /// Create new behavioral cloning state
    pub fn new() -> Self {
        Self {
            edge_amplifier: Arc::new(EdgeAmplifier::new()),
            database_path: None,
        }
    }

    /// Create new state with database path
    pub fn with_database_path(path: PathBuf) -> Self {
        Self {
            edge_amplifier: Arc::new(EdgeAmplifier::new()),
            database_path: Some(path),
        }
    }

    /// Open database connection
    async fn open_database(&self) -> Result<RecorderDatabase, String> {
        let db_path = self.database_path.as_ref().cloned().unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("recordings.db")
        });

        RecorderDatabase::new(&db_path)
            .await
            .map_err(|e| format!("Failed to open recorder database: {}", e))
    }
}

impl Default for BehavioralCloningState {
    fn default() -> Self {
        Self::new()
    }
}

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

/// Request to discover sequences from traces
#[derive(Debug, Deserialize)]
pub struct DiscoverSequencesRequest {
    /// Minimum number of requests per trace to consider
    #[serde(default)]
    pub min_requests_per_trace: Option<i32>,
    /// Minimum frequency threshold for sequences (0.0 to 1.0)
    #[serde(default = "default_min_frequency")]
    pub min_frequency: f64,
}

fn default_min_frequency() -> f64 {
    0.1 // 10% default
}

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

/// Response for probability model
#[derive(Debug, Serialize)]
pub struct ProbabilityModelResponse {
    /// Success flag
    pub success: bool,
    /// Probability model
    pub model: EndpointProbabilityModel,
}

/// Response for sequence discovery
#[derive(Debug, Serialize)]
pub struct SequenceDiscoveryResponse {
    /// Success flag
    pub success: bool,
    /// Number of sequences discovered
    pub count: usize,
    /// Discovered sequences
    pub sequences: Vec<BehavioralSequence>,
}

/// Build a probability model for an endpoint
///
/// POST /api/v1/behavioral-cloning/probability-models
pub async fn build_probability_model(
    State(state): State<BehavioralCloningState>,
    Json(request): Json<BuildProbabilityModelRequest>,
) -> Result<Json<Value>, String> {
    // Open database connection
    let db = state.open_database().await?;

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
pub async fn get_probability_model(
    Path((endpoint, method)): Path<(String, String)>,
    State(state): State<BehavioralCloningState>,
) -> Result<Json<Value>, String> {
    let db = state.open_database().await?;

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
pub async fn list_probability_models(
    State(state): State<BehavioralCloningState>,
) -> Result<Json<Value>, String> {
    let db = state.open_database().await?;

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

/// Discover sequences from recorded traces
///
/// POST /api/v1/behavioral-cloning/sequences/discover
pub async fn discover_sequences(
    State(state): State<BehavioralCloningState>,
    Json(request): Json<DiscoverSequencesRequest>,
) -> Result<Json<Value>, String> {
    let db = state.open_database().await?;

    // Query database for requests grouped by trace_id
    let trace_groups = db
        .get_requests_by_trace(request.min_requests_per_trace)
        .await
        .map_err(|e| format!("Failed to query traces: {}", e))?;

    if trace_groups.is_empty() {
        return Ok(Json(json!({
            "success": true,
            "count": 0,
            "sequences": [],
            "message": "No traces found with sufficient requests"
        })));
    }

    // Convert to sequence format (endpoint, method, delay)
    let mut sequences: Vec<Vec<(String, String, Option<u64>)>> = Vec::new();

    for (_trace_id, requests) in trace_groups {
        let mut seq = Vec::new();
        let mut prev_timestamp = None;

        for req in requests {
            // Calculate delay from previous request
            let delay = if let Some(prev_ts) = prev_timestamp {
                let duration = req.timestamp.signed_duration_since(prev_ts);
                Some(duration.num_milliseconds().max(0) as u64)
            } else {
                None
            };

            seq.push((req.path.clone(), req.method.clone(), delay));
            prev_timestamp = Some(req.timestamp);
        }

        if !seq.is_empty() {
            sequences.push(seq);
        }
    }

    // Learn sequence patterns
    let learned_sequences =
        SequenceLearner::learn_sequence_pattern(&sequences, request.min_frequency)
            .map_err(|e| format!("Failed to learn sequences: {}", e))?;

    // Store sequences in database
    for sequence in &learned_sequences {
        db.insert_behavioral_sequence(sequence)
            .await
            .map_err(|e| format!("Failed to store sequence: {}", e))?;
    }

    Ok(Json(json!({
        "success": true,
        "count": learned_sequences.len(),
        "sequences": learned_sequences
    })))
}

/// List all learned sequences
///
/// GET /api/v1/behavioral-cloning/sequences
pub async fn list_sequences(
    State(state): State<BehavioralCloningState>,
) -> Result<Json<Value>, String> {
    let db = state.open_database().await?;

    let sequences = db
        .get_behavioral_sequences()
        .await
        .map_err(|e| format!("Failed to query sequences: {}", e))?;

    Ok(Json(json!({
        "success": true,
        "sequences": sequences,
        "count": sequences.len()
    })))
}

/// Get a specific sequence by ID
///
/// GET /api/v1/behavioral-cloning/sequences/{sequence_id}
pub async fn get_sequence(
    Path(sequence_id): Path<String>,
    State(state): State<BehavioralCloningState>,
) -> Result<Json<Value>, String> {
    let db = state.open_database().await?;

    let sequences = db
        .get_behavioral_sequences()
        .await
        .map_err(|e| format!("Failed to query sequences: {}", e))?;

    let sequence = sequences
        .into_iter()
        .find(|s| s.id == sequence_id)
        .ok_or_else(|| format!("Sequence {} not found", sequence_id))?;

    Ok(Json(json!({
        "success": true,
        "sequence": sequence
    })))
}

/// Apply edge amplification to a probability model
///
/// POST /api/v1/behavioral-cloning/amplification/apply
pub async fn apply_amplification(
    State(state): State<BehavioralCloningState>,
    Json(request): Json<ApplyAmplificationRequest>,
) -> Result<Json<Value>, String> {
    if !request.config.enabled {
        return Ok(Json(json!({
            "success": true,
            "message": "Amplification disabled"
        })));
    }

    let db = state.open_database().await?;

    // Determine which models to update based on scope
    let models_to_update = match &request.config.scope {
        mockforge_core::behavioral_cloning::AmplificationScope::Global => db
            .get_all_endpoint_probability_models()
            .await
            .map_err(|e| format!("Failed to query models: {}", e))?,
        mockforge_core::behavioral_cloning::AmplificationScope::Endpoint { endpoint, method } => {
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
        mockforge_core::behavioral_cloning::AmplificationScope::Sequence { sequence_id } => {
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
                return Err(format!(
                    "No probability models found for sequence {}",
                    sequence_id
                ));
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
pub async fn get_rare_edges(
    Path((endpoint, method)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<BehavioralCloningState>,
) -> Result<Json<Value>, String> {
    let db = state.open_database().await?;

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

/// Sample a status code from a probability model
///
/// POST /api/v1/behavioral-cloning/probability-models/{endpoint}/{method}/sample/status-code
pub async fn sample_status_code(
    Path((endpoint, method)): Path<(String, String)>,
    State(state): State<BehavioralCloningState>,
) -> Result<Json<Value>, String> {
    let db = state.open_database().await?;

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
pub async fn sample_latency(
    Path((endpoint, method)): Path<(String, String)>,
    State(state): State<BehavioralCloningState>,
) -> Result<Json<Value>, String> {
    let db = state.open_database().await?;

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

/// Generate a scenario from a learned sequence
///
/// POST /api/v1/behavioral-cloning/sequences/{sequence_id}/scenario
pub async fn generate_sequence_scenario(
    Path(sequence_id): Path<String>,
    State(state): State<BehavioralCloningState>,
) -> Result<Json<Value>, String> {
    let db = state.open_database().await?;

    let sequences = db
        .get_behavioral_sequences()
        .await
        .map_err(|e| format!("Failed to query sequences: {}", e))?;

    let sequence = sequences
        .into_iter()
        .find(|s| s.id == sequence_id)
        .ok_or_else(|| format!("Sequence {} not found", sequence_id))?;

    let scenario = SequenceLearner::generate_sequence_scenario(&sequence);

    Ok(Json(json!({
        "success": true,
        "sequence_id": sequence_id,
        "scenario": scenario
    })))
}

/// Create router for behavioral cloning endpoints
pub fn behavioral_cloning_router(state: BehavioralCloningState) -> axum::Router {
    use axum::routing::{get, post};
    use axum::Router;

    Router::new()
        // Probability model endpoints
        .route(
            "/api/v1/behavioral-cloning/probability-models",
            post(build_probability_model).get(list_probability_models),
        )
        .route(
            "/api/v1/behavioral-cloning/probability-models/{endpoint}/{method}",
            get(get_probability_model),
        )
        .route(
            "/api/v1/behavioral-cloning/probability-models/{endpoint}/{method}/sample/status-code",
            post(sample_status_code),
        )
        .route(
            "/api/v1/behavioral-cloning/probability-models/{endpoint}/{method}/sample/latency",
            post(sample_latency),
        )
        // Sequence endpoints
        .route(
            "/api/v1/behavioral-cloning/sequences",
            get(list_sequences),
        )
        .route(
            "/api/v1/behavioral-cloning/sequences/discover",
            post(discover_sequences),
        )
        .route(
            "/api/v1/behavioral-cloning/sequences/{sequence_id}",
            get(get_sequence),
        )
        .route(
            "/api/v1/behavioral-cloning/sequences/{sequence_id}/scenario",
            post(generate_sequence_scenario),
        )
        // Amplification endpoints
        .route(
            "/api/v1/behavioral-cloning/amplification/apply",
            post(apply_amplification),
        )
        .route(
            "/api/v1/behavioral-cloning/amplification/rare-edges/{endpoint}/{method}",
            get(get_rare_edges),
        )
        .with_state(state)
}
