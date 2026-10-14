//! Trace sequence learning and shared-foundation scenario conversion.

use crate::behavioral_cloning::storage::{BehavioralCloningStateProvider, BehavioralCloningStore};
use crate::behavioral_cloning::{BehavioralSequence, SequenceLearner};
use axum::extract::{Path, State};
use axum::response::Json;
use mockforge_foundation::scenario_types::{ScenarioDefinition, ScenarioStep};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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

/// Discover sequences from recorded traces
///
/// POST /api/v1/behavioral-cloning/sequences/discover
pub async fn discover_sequences<S: BehavioralCloningStateProvider>(
    State(state): State<S>,
    Json(request): Json<DiscoverSequencesRequest>,
) -> Result<Json<Value>, String> {
    let db = state.open_store().await?;

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
pub async fn list_sequences<S: BehavioralCloningStateProvider>(
    State(state): State<S>,
) -> Result<Json<Value>, String> {
    let db = state.open_store().await?;

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
pub async fn get_sequence<S: BehavioralCloningStateProvider>(
    Path(sequence_id): Path<String>,
    State(state): State<S>,
) -> Result<Json<Value>, String> {
    let db = state.open_store().await?;

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

/// Generate a scenario from a learned sequence
///
/// POST /api/v1/behavioral-cloning/sequences/{sequence_id}/scenario
pub async fn generate_sequence_scenario<S: BehavioralCloningStateProvider>(
    Path(sequence_id): Path<String>,
    State(state): State<S>,
) -> Result<Json<Value>, String> {
    let db = state.open_store().await?;

    let sequences = db
        .get_behavioral_sequences()
        .await
        .map_err(|e| format!("Failed to query sequences: {}", e))?;

    let sequence = sequences
        .into_iter()
        .find(|s| s.id == sequence_id)
        .ok_or_else(|| format!("Sequence {} not found", sequence_id))?;

    let scenario = sequence_to_scenario_definition(&sequence);

    Ok(Json(json!({
        "success": true,
        "sequence_id": sequence_id,
        "scenario": scenario
    })))
}

/// Convert a learned `BehavioralSequence` into a `ScenarioDefinition`.
///
/// Scenario types live in foundation, so this conversion has no dependency
/// on core or the concrete recorder database.
fn sequence_to_scenario_definition(sequence: &BehavioralSequence) -> ScenarioDefinition {
    let mut scenario =
        ScenarioDefinition::new(&sequence.id, &sequence.name).with_tags(sequence.tags.clone());

    if let Some(description) = &sequence.description {
        scenario.description = Some(description.clone());
    }

    for (idx, step) in sequence.steps.iter().enumerate() {
        let mut scenario_step = ScenarioStep::new(
            format!("step_{}", idx),
            step.name.as_deref().unwrap_or(&step.endpoint),
            &step.method,
            &step.endpoint,
        )
        .expect_status(200);

        if let Some(delay) = step.expected_delay_ms {
            scenario_step.delay_ms = Some(delay);
        }

        for (key, value) in &step.conditions {
            scenario_step.query_params.insert(key.clone(), value.clone());
        }

        scenario = scenario.add_step(scenario_step);
    }

    scenario
}
