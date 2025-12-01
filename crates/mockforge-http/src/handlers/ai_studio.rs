//! AI Studio HTTP handlers
//!
//! This module provides HTTP endpoints for AI Studio features:
//! - API Architecture Critique
//! - System Generation
//! - Behavioral Simulation

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::post,
    Router,
};
use mockforge_core::{
    ai_studio::{
        api_critique::{ApiCritiqueEngine, CritiqueRequest},
        artifact_freezer::{ArtifactFreezer, FreezeMetadata, FreezeRequest},
        behavioral_simulator::{BehavioralSimulator, CreateAgentRequest, SimulateBehaviorRequest},
        config::DeterministicModeConfig,
        system_generator::{SystemGenerationRequest, SystemGenerator},
    },
    intelligent_behavior::IntelligentBehaviorConfig,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, warn};

/// State for AI Studio handlers
#[derive(Clone)]
pub struct AiStudioState {
    /// API critique engine
    pub critique_engine: Arc<ApiCritiqueEngine>,
    /// System generator engine
    pub system_generator: Arc<SystemGenerator>,
    /// Behavioral simulator engine (wrapped in Mutex for mutability)
    pub behavioral_simulator: Arc<Mutex<BehavioralSimulator>>,
    /// Artifact freezer for storing critiques and systems
    pub artifact_freezer: Arc<ArtifactFreezer>,
    /// AI configuration (for metadata)
    pub config: IntelligentBehaviorConfig,
    /// Deterministic mode configuration (optional, from workspace)
    pub deterministic_config: Option<DeterministicModeConfig>,
    /// Workspace ID (optional, can be set per request)
    pub workspace_id: Option<String>,
    /// In-memory storage for generated systems (system_id -> GeneratedSystem)
    pub system_storage:
        Arc<RwLock<HashMap<String, mockforge_core::ai_studio::system_generator::GeneratedSystem>>>,
}

impl AiStudioState {
    /// Create new AI Studio state
    pub fn new(config: IntelligentBehaviorConfig) -> Self {
        let critique_engine = Arc::new(ApiCritiqueEngine::new(config.clone()));
        let system_generator = Arc::new(SystemGenerator::new(config.clone()));
        let behavioral_simulator = Arc::new(Mutex::new(BehavioralSimulator::new(config.clone())));
        let artifact_freezer = Arc::new(ArtifactFreezer::new());
        Self {
            critique_engine,
            system_generator,
            behavioral_simulator,
            artifact_freezer,
            config,
            deterministic_config: None,
            workspace_id: None,
            system_storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with custom workspace ID and deterministic config
    pub fn with_workspace(
        mut self,
        workspace_id: String,
        deterministic_config: Option<DeterministicModeConfig>,
    ) -> Self {
        self.workspace_id = Some(workspace_id);
        self.deterministic_config = deterministic_config;
        self
    }
}

/// Request body for API critique endpoint
#[derive(Debug, Deserialize, Serialize)]
pub struct ApiCritiqueRequest {
    /// API schema (OpenAPI JSON, GraphQL SDL, or Protobuf)
    pub schema: serde_json::Value,

    /// Schema type: "openapi", "graphql", or "protobuf"
    pub schema_type: String,

    /// Optional focus areas for analysis
    /// Valid values: "anti-patterns", "redundancy", "naming", "tone", "restructuring"
    #[serde(default)]
    pub focus_areas: Vec<String>,

    /// Optional workspace ID
    pub workspace_id: Option<String>,
}

/// Response for API critique endpoint
#[derive(Debug, Serialize)]
pub struct ApiCritiqueResponse {
    /// The critique result
    pub critique: mockforge_core::ai_studio::api_critique::ApiCritique,

    /// Artifact ID if stored
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,

    /// Whether the critique was frozen as an artifact
    pub frozen: bool,
}

/// Analyze an API schema and generate critique
///
/// POST /api/v1/ai-studio/api-critique
pub async fn api_critique_handler(
    State(state): State<AiStudioState>,
    Json(request): Json<ApiCritiqueRequest>,
) -> std::result::Result<Json<ApiCritiqueResponse>, StatusCode> {
    info!("API critique request received for schema type: {}", request.schema_type);

    // Build critique request
    let critique_request = CritiqueRequest {
        schema: request.schema,
        schema_type: request.schema_type,
        focus_areas: request.focus_areas,
        workspace_id: request.workspace_id.or_else(|| state.workspace_id.clone()),
    };

    // Generate critique
    let critique = match state.critique_engine.analyze(&critique_request).await {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to generate API critique: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Store as artifact
    let critique_json = match serde_json::to_value(&critique) {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to serialize critique: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Create metadata for artifact freezing
    let freeze_request = FreezeRequest {
        artifact_type: "api_critique".to_string(),
        content: critique_json,
        format: "json".to_string(),
        path: None,
        metadata: Some(FreezeMetadata {
            llm_provider: Some(state.config.behavior_model.llm_provider.clone()),
            llm_model: Some(state.config.behavior_model.model.clone()),
            llm_version: None,
            prompt_hash: None,
            output_hash: None,
            original_prompt: None,
        }),
    };

    let frozen_artifact = match state.artifact_freezer.freeze(&freeze_request).await {
        Ok(a) => a,
        Err(e) => {
            error!("Failed to freeze critique artifact: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    info!(
        "API critique completed. Artifact path: {}, Tokens used: {:?}, Cost: ${:.4}",
        frozen_artifact.path,
        critique.tokens_used,
        critique.cost_usd.unwrap_or(0.0)
    );

    Ok(Json(ApiCritiqueResponse {
        critique,
        artifact_id: Some(frozen_artifact.path),
        frozen: true,
    }))
}

/// Request body for system generation endpoint
#[derive(Debug, Deserialize)]
pub struct SystemGenerationHttpRequest {
    /// Natural language description
    pub description: String,

    /// Output formats to generate
    #[serde(default)]
    pub output_formats: Vec<String>,

    /// Optional workspace ID
    pub workspace_id: Option<String>,

    /// Optional system ID (for versioning)
    pub system_id: Option<String>,
}

/// Response for system generation endpoint
#[derive(Debug, Serialize)]
pub struct SystemGenerationResponse {
    /// Generated system
    pub system: mockforge_core::ai_studio::system_generator::GeneratedSystem,
}

/// Request body for apply system design endpoint
#[derive(Debug, Deserialize)]
pub struct ApplySystemRequest {
    /// Optional artifact IDs to apply (if None, applies all)
    #[serde(default)]
    pub artifact_ids: Option<Vec<String>>,
}

/// Response for apply system design endpoint
#[derive(Debug, Serialize)]
pub struct ApplySystemResponse {
    /// Applied system result
    pub applied: mockforge_core::ai_studio::system_generator::AppliedSystem,
}

/// Request body for freeze artifacts endpoint
#[derive(Debug, Deserialize)]
pub struct FreezeArtifactsRequest {
    /// Artifact IDs to freeze
    pub artifact_ids: Vec<String>,
}

/// Response for freeze artifacts endpoint
#[derive(Debug, Serialize)]
pub struct FreezeArtifactsResponse {
    /// Frozen artifact paths
    pub frozen_paths: Vec<String>,
}

/// Generate a complete system from natural language description
///
/// POST /api/v1/ai-studio/generate-system
pub async fn generate_system_handler(
    State(state): State<AiStudioState>,
    Json(request): Json<SystemGenerationHttpRequest>,
) -> std::result::Result<Json<SystemGenerationResponse>, StatusCode> {
    info!("System generation request received");

    let generation_request = SystemGenerationRequest {
        description: request.description,
        output_formats: request.output_formats,
        workspace_id: request.workspace_id.or_else(|| state.workspace_id.clone()),
        system_id: request.system_id,
    };

    let system = match state
        .system_generator
        .generate(&generation_request, state.deterministic_config.as_ref())
        .await
    {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to generate system: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Store the generated system in storage
    {
        let mut storage = state.system_storage.write().await;
        storage.insert(system.system_id.clone(), system.clone());
    }

    info!(
        "System generation completed. System ID: {}, Version: {}, Status: {}, Tokens: {:?}, Cost: ${:.4}",
        system.system_id,
        system.version,
        system.status,
        system.tokens_used,
        system.cost_usd.unwrap_or(0.0)
    );

    Ok(Json(SystemGenerationResponse { system }))
}

/// Apply system design (freeze artifacts if deterministic mode requires it)
///
/// POST /api/v1/ai-studio/system/{system_id}/apply
pub async fn apply_system_handler(
    State(state): State<AiStudioState>,
    Path(system_id): Path<String>,
    Json(request): Json<ApplySystemRequest>,
) -> std::result::Result<Json<ApplySystemResponse>, StatusCode> {
    info!("Apply system request received for system: {}", system_id);

    // Load the system from storage
    let system = {
        let storage = state.system_storage.read().await;
        storage.get(&system_id).cloned()
    };

    let system = match system {
        Some(s) => s,
        None => {
            warn!("System not found: {}", system_id);
            return Err(StatusCode::NOT_FOUND);
        }
    };

    // Apply the system design
    let applied = match state
        .system_generator
        .apply_system_design(
            &system,
            state.deterministic_config.as_ref(),
            request.artifact_ids.clone(),
        )
        .await
    {
        Ok(a) => a,
        Err(e) => {
            error!("Failed to apply system design: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Update the system status in storage if it was frozen
    if applied.frozen {
        let mut storage = state.system_storage.write().await;
        if let Some(stored_system) = storage.get_mut(&system_id) {
            stored_system.status = "frozen".to_string();
        }
    }

    info!(
        "System design applied. System ID: {}, Applied artifacts: {}, Frozen: {}",
        applied.system_id,
        applied.applied_artifacts.len(),
        applied.frozen
    );

    Ok(Json(ApplySystemResponse { applied }))
}

/// Freeze specific artifacts manually
///
/// POST /api/v1/ai-studio/system/{system_id}/freeze
pub async fn freeze_artifacts_handler(
    State(state): State<AiStudioState>,
    Path(system_id): Path<String>,
    Json(request): Json<FreezeArtifactsRequest>,
) -> std::result::Result<Json<FreezeArtifactsResponse>, StatusCode> {
    info!(
        "Freeze artifacts request received for system: {}, artifacts: {:?}",
        system_id, request.artifact_ids
    );

    // Load the system from storage
    let system = {
        let storage = state.system_storage.read().await;
        storage.get(&system_id).cloned()
    };

    let system = match system {
        Some(s) => s,
        None => {
            warn!("System not found: {}", system_id);
            return Err(StatusCode::NOT_FOUND);
        }
    };

    // Freeze the specified artifacts
    let frozen_paths = match state
        .system_generator
        .freeze_artifacts(&system, request.artifact_ids.clone())
        .await
    {
        Ok(paths) => paths,
        Err(e) => {
            error!("Failed to freeze artifacts: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Update the system status in storage if all artifacts are frozen
    if !frozen_paths.is_empty() {
        let mut storage = state.system_storage.write().await;
        if let Some(stored_system) = storage.get_mut(&system_id) {
            // Check if all artifacts are now frozen
            let all_frozen = stored_system
                .artifacts
                .values()
                .all(|artifact| request.artifact_ids.contains(&artifact.artifact_id));
            if all_frozen {
                stored_system.status = "frozen".to_string();
            }
        }
    }

    info!(
        "Artifacts frozen. System ID: {}, Frozen paths: {}",
        system_id,
        frozen_paths.len()
    );

    Ok(Json(FreezeArtifactsResponse { frozen_paths }))
}

/// Request body for create agent endpoint
#[derive(Debug, Deserialize)]
pub struct CreateAgentHttpRequest {
    /// Optional: Attach to existing persona ID
    pub persona_id: Option<String>,

    /// Optional: Behavior policy type
    pub behavior_policy: Option<String>,

    /// If true, generate new persona if needed
    pub generate_persona: bool,

    /// Optional workspace ID
    pub workspace_id: Option<String>,
}

/// Response for create agent endpoint
#[derive(Debug, Serialize)]
pub struct CreateAgentResponse {
    /// Created agent
    pub agent: mockforge_core::ai_studio::behavioral_simulator::NarrativeAgent,
}

/// Request body for simulate behavior endpoint
#[derive(Debug, Deserialize)]
pub struct SimulateBehaviorHttpRequest {
    /// Optional: Use existing agent ID
    pub agent_id: Option<String>,

    /// Optional: Attach to existing persona
    pub persona_id: Option<String>,

    /// Current app state
    pub current_state: mockforge_core::ai_studio::behavioral_simulator::AppState,

    /// Trigger event
    pub trigger_event: Option<String>,

    /// Optional workspace ID
    pub workspace_id: Option<String>,
}

/// Create a new narrative agent
///
/// POST /api/v1/ai-studio/simulate-behavior/create-agent
#[axum::debug_handler]
pub async fn create_agent_handler(
    State(state): State<AiStudioState>,
    Json(request): Json<CreateAgentHttpRequest>,
) -> std::result::Result<Json<CreateAgentResponse>, StatusCode> {
    info!("Create agent request received");

    let create_request = CreateAgentRequest {
        persona_id: request.persona_id,
        behavior_policy: request.behavior_policy,
        generate_persona: request.generate_persona,
        workspace_id: request.workspace_id.or_else(|| state.workspace_id.clone()),
    };

    let mut simulator = state.behavioral_simulator.lock().await;
    let agent = match simulator.create_agent(&create_request).await {
        Ok(a) => a,
        Err(e) => {
            error!("Failed to create agent: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    info!("Agent created: {}", agent.agent_id);

    Ok(Json(CreateAgentResponse { agent }))
}

/// Simulate behavior based on current state
///
/// POST /api/v1/ai-studio/simulate-behavior
#[axum::debug_handler]
pub async fn simulate_behavior_handler(
    State(state): State<AiStudioState>,
    Json(request): Json<SimulateBehaviorHttpRequest>,
) -> std::result::Result<
    Json<mockforge_core::ai_studio::behavioral_simulator::SimulateBehaviorResponse>,
    StatusCode,
> {
    info!("Simulate behavior request received");

    let simulate_request = SimulateBehaviorRequest {
        agent_id: request.agent_id,
        persona_id: request.persona_id,
        current_state: request.current_state,
        trigger_event: request.trigger_event,
        workspace_id: request.workspace_id.or_else(|| state.workspace_id.clone()),
    };

    let mut simulator = state.behavioral_simulator.lock().await;
    let response = match simulator.simulate_behavior(&simulate_request).await {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to simulate behavior: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    info!(
        "Behavior simulation completed. Intention: {:?}, Tokens: {:?}, Cost: ${:.4}",
        response.intention,
        response.tokens_used,
        response.cost_usd.unwrap_or(0.0)
    );

    Ok(Json(response))
}

/// Build AI Studio router
pub fn ai_studio_router(state: AiStudioState) -> Router {
    let mut router = Router::new()
        .route("/api-critique", post(api_critique_handler))
        .route("/generate-system", post(generate_system_handler))
        .route("/system/{system_id}/apply", post(apply_system_handler))
        .route("/system/{system_id}/freeze", post(freeze_artifacts_handler))
        .route("/simulate-behavior/create-agent", post(create_agent_handler))
        .route("/simulate-behavior", post(simulate_behavior_handler));
    router.with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_core::intelligent_behavior::config::BehaviorModelConfig;

    fn create_test_state() -> AiStudioState {
        let config = IntelligentBehaviorConfig {
            behavior_model: BehaviorModelConfig {
                llm_provider: "ollama".to_string(),
                model: "llama2".to_string(),
                api_endpoint: Some("http://localhost:11434/api/chat".to_string()),
                api_key: None,
                temperature: 0.7,
                max_tokens: 2000,
                rules: mockforge_core::intelligent_behavior::types::BehaviorRules::default(),
            },
            ..Default::default()
        };
        AiStudioState::new(config)
    }

    #[test]
    fn test_ai_studio_state_creation() {
        let state = create_test_state();
        // State should be created successfully
        assert!(true);
    }

    #[test]
    fn test_api_critique_request_serialization() {
        let request = ApiCritiqueRequest {
            schema: serde_json::json!({"openapi": "3.0.0"}),
            schema_type: "openapi".to_string(),
            focus_areas: vec!["anti-patterns".to_string()],
            workspace_id: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("openapi"));
        assert!(json.contains("anti-patterns"));
    }
}
