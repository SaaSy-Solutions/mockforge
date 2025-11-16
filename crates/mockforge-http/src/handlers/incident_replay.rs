//! Incident Replay API handlers

use axum::extract::{Path, Query, State};
use axum::response::Json;
use mockforge_chaos::incident_replay::{
    IncidentFormatAdapter, IncidentReplayGenerator, IncidentTimeline,
};
use mockforge_chaos::OrchestratedScenario;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

/// Incident replay API state
#[derive(Clone)]
pub struct IncidentReplayState {
    /// Incident replay generator
    pub generator: Arc<IncidentReplayGenerator>,
}

impl IncidentReplayState {
    /// Create new incident replay state
    pub fn new() -> Self {
        Self {
            generator: Arc::new(IncidentReplayGenerator::new()),
        }
    }
}

impl Default for IncidentReplayState {
    fn default() -> Self {
        Self::new()
    }
}

/// Request to generate a chaos scenario from an incident timeline
#[derive(Debug, Deserialize)]
pub struct GenerateReplayRequest {
    /// Incident timeline data
    pub timeline: IncidentTimeline,
    /// Format of the timeline (if importing from external source)
    #[serde(default)]
    pub format: Option<String>,
}

/// Request to import an incident timeline from external format
#[derive(Debug, Deserialize)]
pub struct ImportIncidentRequest {
    /// Incident data in external format (JSON)
    pub data: Value,
    /// Format type: "pagerduty", "datadog", "custom"
    pub format: String,
}

/// Response for replay generation
#[derive(Debug, Serialize)]
pub struct ReplayGenerationResponse {
    /// Success flag
    pub success: bool,
    /// Generated orchestrated scenario
    pub scenario: OrchestratedScenario,
    /// Scenario in JSON format
    pub scenario_json: String,
    /// Scenario in YAML format
    pub scenario_yaml: String,
}

/// Generate a chaos scenario from an incident timeline
///
/// POST /api/v1/chaos/incident-replay/generate
pub async fn generate_replay(
    State(state): State<IncidentReplayState>,
    Json(request): Json<GenerateReplayRequest>,
) -> Result<Json<Value>, String> {
    let generator = &state.generator;
    let scenario = generator.generate_scenario(&request.timeline);

    // Export to JSON and YAML
    let scenario_json = generator
        .export_scenario_to_json(&scenario)
        .map_err(|e| format!("Failed to export scenario to JSON: {}", e))?;
    let scenario_yaml = generator
        .export_scenario_to_yaml(&scenario)
        .map_err(|e| format!("Failed to export scenario to YAML: {}", e))?;

    Ok(Json(json!({
        "success": true,
        "scenario": scenario,
        "scenario_json": scenario_json,
        "scenario_yaml": scenario_yaml,
    })))
}

/// Import incident timeline from external format
///
/// POST /api/v1/chaos/incident-replay/import
pub async fn import_incident(
    State(_state): State<IncidentReplayState>,
    Json(request): Json<ImportIncidentRequest>,
) -> Result<Json<Value>, String> {
    let timeline = match request.format.as_str() {
        "pagerduty" => IncidentFormatAdapter::from_pagerduty(&request.data)
            .map_err(|e| format!("Failed to parse PagerDuty format: {}", e))?,
        "datadog" => IncidentFormatAdapter::from_datadog(&request.data)
            .map_err(|e| format!("Failed to parse Datadog format: {}", e))?,
        "custom" => {
            // Try to parse as JSON directly
            serde_json::from_value::<IncidentTimeline>(request.data)
                .map_err(|e| format!("Failed to parse custom format: {}", e))?
        }
        _ => return Err(format!("Unsupported format: {}", request.format)),
    };

    Ok(Json(json!({
        "success": true,
        "timeline": timeline,
    })))
}

/// Import and generate scenario in one step
///
/// POST /api/v1/chaos/incident-replay/import-and-generate
pub async fn import_and_generate(
    State(state): State<IncidentReplayState>,
    Json(request): Json<ImportIncidentRequest>,
) -> Result<Json<Value>, String> {
    // First import the timeline
    let timeline = match request.format.as_str() {
        "pagerduty" => IncidentFormatAdapter::from_pagerduty(&request.data)
            .map_err(|e| format!("Failed to parse PagerDuty format: {}", e))?,
        "datadog" => IncidentFormatAdapter::from_datadog(&request.data)
            .map_err(|e| format!("Failed to parse Datadog format: {}", e))?,
        "custom" => {
            serde_json::from_value::<IncidentTimeline>(request.data)
                .map_err(|e| format!("Failed to parse custom format: {}", e))?
        }
        _ => return Err(format!("Unsupported format: {}", request.format)),
    };

    // Then generate the scenario
    let generator = &state.generator;
    let scenario = generator.generate_scenario(&timeline);

    // Export to JSON and YAML
    let scenario_json = generator
        .export_scenario_to_json(&scenario)
        .map_err(|e| format!("Failed to export scenario to JSON: {}", e))?;
    let scenario_yaml = generator
        .export_scenario_to_yaml(&scenario)
        .map_err(|e| format!("Failed to export scenario to YAML: {}", e))?;

    Ok(Json(json!({
        "success": true,
        "timeline": timeline,
        "scenario": scenario,
        "scenario_json": scenario_json,
        "scenario_yaml": scenario_yaml,
    })))
}
