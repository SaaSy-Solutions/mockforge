//! Incident replay HTTP handlers with a typed downstream Chaos provider.
use axum::{extract::State, response::Json};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};

/// Replay and format operations supplied by the application's existing engine.
pub trait IncidentReplayProvider: Clone + Send + Sync + 'static {
    /// Original typed timeline used for custom JSON and imports.
    type Timeline: DeserializeOwned + Serialize + Send + Sync + 'static;
    /// Original typed orchestrated scenario.
    type Scenario: Serialize;
    /// Generate using the existing replay generator.
    fn generate_scenario(&self, timeline: &Self::Timeline) -> Self::Scenario;
    /// Export the existing JSON representation.
    fn export_scenario_to_json(&self, scenario: &Self::Scenario) -> Result<String, String>;
    /// Export the existing YAML representation.
    fn export_scenario_to_yaml(&self, scenario: &Self::Scenario) -> Result<String, String>;
    /// Import with the existing PagerDuty format adapter.
    fn import_pagerduty(&self, data: &Value) -> Result<Self::Timeline, String>;
    /// Import with the existing Datadog format adapter.
    fn import_datadog(&self, data: &Value) -> Result<Self::Timeline, String>;
}

/// Request to generate a chaos scenario from an incident timeline
#[derive(Debug, Deserialize)]
pub struct GenerateReplayRequest<T> {
    /// Incident timeline data
    pub timeline: T,
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
pub struct ReplayGenerationResponse<C> {
    /// Success flag
    pub success: bool,
    /// Generated orchestrated scenario
    pub scenario: C,
    /// Scenario in JSON format
    pub scenario_json: String,
    /// Scenario in YAML format
    pub scenario_yaml: String,
}

/// Generate a chaos scenario from an incident timeline
///
/// POST /api/v1/chaos/incident-replay/generate
pub async fn generate_replay<S: IncidentReplayProvider>(
    State(state): State<S>,
    Json(request): Json<GenerateReplayRequest<S::Timeline>>,
) -> Result<Json<Value>, String> {
    let generator = &state;
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
pub async fn import_incident<S: IncidentReplayProvider>(
    State(state): State<S>,
    Json(request): Json<ImportIncidentRequest>,
) -> Result<Json<Value>, String> {
    let timeline = match request.format.as_str() {
        "pagerduty" => state
            .import_pagerduty(&request.data)
            .map_err(|e| format!("Failed to parse PagerDuty format: {}", e))?,
        "datadog" => state
            .import_datadog(&request.data)
            .map_err(|e| format!("Failed to parse Datadog format: {}", e))?,
        "custom" => {
            // Try to parse as JSON directly
            serde_json::from_value::<S::Timeline>(request.data)
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
pub async fn import_and_generate<S: IncidentReplayProvider>(
    State(state): State<S>,
    Json(request): Json<ImportIncidentRequest>,
) -> Result<Json<Value>, String> {
    // First import the timeline
    let timeline = match request.format.as_str() {
        "pagerduty" => state
            .import_pagerduty(&request.data)
            .map_err(|e| format!("Failed to parse PagerDuty format: {}", e))?,
        "datadog" => state
            .import_datadog(&request.data)
            .map_err(|e| format!("Failed to parse Datadog format: {}", e))?,
        "custom" => serde_json::from_value::<S::Timeline>(request.data)
            .map_err(|e| format!("Failed to parse custom format: {}", e))?,
        _ => return Err(format!("Unsupported format: {}", request.format)),
    };

    // Then generate the scenario
    let generator = &state;
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
