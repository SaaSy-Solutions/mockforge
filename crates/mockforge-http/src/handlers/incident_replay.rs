//! Compatibility state and typed Chaos adapter for Intelligence handlers.
use axum::{extract::State, response::Json};
use mockforge_chaos::{
    incident_replay::{IncidentFormatAdapter, IncidentReplayGenerator, IncidentTimeline},
    OrchestratedScenario,
};
use mockforge_intelligence::handlers::incident_replay::{self as handlers, IncidentReplayProvider};
use serde_json::Value;
use std::sync::Arc;

pub use handlers::ImportIncidentRequest;
/// Original typed replay request.
pub type GenerateReplayRequest = handlers::GenerateReplayRequest<IncidentTimeline>;
/// Original typed replay response.
pub type ReplayGenerationResponse = handlers::ReplayGenerationResponse<OrchestratedScenario>;

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

impl IncidentReplayProvider for IncidentReplayState {
    type Timeline = IncidentTimeline;
    type Scenario = OrchestratedScenario;
    fn generate_scenario(&self, timeline: &Self::Timeline) -> Self::Scenario {
        self.generator.generate_scenario(timeline)
    }
    fn export_scenario_to_json(&self, scenario: &Self::Scenario) -> Result<String, String> {
        self.generator.export_scenario_to_json(scenario).map_err(|e| e.to_string())
    }
    fn export_scenario_to_yaml(&self, scenario: &Self::Scenario) -> Result<String, String> {
        self.generator.export_scenario_to_yaml(scenario).map_err(|e| e.to_string())
    }
    fn import_pagerduty(&self, data: &Value) -> Result<Self::Timeline, String> {
        IncidentFormatAdapter::from_pagerduty(data)
    }
    fn import_datadog(&self, data: &Value) -> Result<Self::Timeline, String> {
        IncidentFormatAdapter::from_datadog(data)
    }
}
/// Generate through the original state and request paths.
pub async fn generate_replay(
    state: State<IncidentReplayState>,
    request: Json<GenerateReplayRequest>,
) -> Result<Json<Value>, String> {
    handlers::generate_replay(state, request).await
}
/// Import through the original state and request paths.
pub async fn import_incident(
    state: State<IncidentReplayState>,
    request: Json<ImportIncidentRequest>,
) -> Result<Json<Value>, String> {
    handlers::import_incident(state, request).await
}
/// Import and generate through the original state and request paths.
pub async fn import_and_generate(
    state: State<IncidentReplayState>,
    request: Json<ImportIncidentRequest>,
) -> Result<Json<Value>, String> {
    handlers::import_and_generate(state, request).await
}
