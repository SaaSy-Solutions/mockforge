//! Voice + LLM Interface API handlers for Admin UI
//!
//! Provides endpoints for processing voice commands and generating OpenAPI specs
//! using natural language commands powered by LLM.

use axum::{extract::{Json, State}, http::StatusCode, response::Json as ResponseJson};
use mockforge_core::intelligent_behavior::IntelligentBehaviorConfig;
use mockforge_core::voice::{ParsedWorkspaceCreation, WorkspaceBuilder};
use mockforge_core::{
    GeneratedWorkspaceScenario, HookTranspiler, VoiceCommandParser, VoiceSpecGenerator,
    WorkspaceScenarioGenerator,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::ApiResponse;
use crate::handlers::workspaces::WorkspaceState;

/// Request to process a voice command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessVoiceCommandRequest {
    /// The voice command text (transcribed from speech or typed)
    pub command: String,
    /// Optional conversation ID for multi-turn interactions
    #[serde(default)]
    pub conversation_id: Option<String>,
}

/// Response from processing a voice command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessVoiceCommandResponse {
    /// The original command
    pub command: String,
    /// Parsed command structure
    pub parsed: ParsedCommandData,
    /// Generated OpenAPI spec (as JSON)
    pub spec: Option<Value>,
    /// Optional error message
    pub error: Option<String>,
}

/// Parsed command data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCommandData {
    /// API type/category
    pub api_type: String,
    /// API title
    pub title: String,
    /// API description
    pub description: String,
    /// List of endpoints
    pub endpoints: Vec<Value>,
    /// List of data models
    pub models: Vec<Value>,
}

/// Process a voice command and generate an OpenAPI spec
///
/// POST /api/v2/voice/process
pub async fn process_voice_command(
    Json(request): Json<ProcessVoiceCommandRequest>,
) -> Result<ResponseJson<ApiResponse<ProcessVoiceCommandResponse>>, StatusCode> {
    if request.command.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Create parser with default config
    let config = IntelligentBehaviorConfig::default();
    let parser = VoiceCommandParser::new(config);

    // Parse the command
    let parsed = match parser.parse_command(&request.command).await {
        Ok(parsed) => parsed,
        Err(e) => {
            return Ok(ResponseJson(ApiResponse::error(format!("Failed to parse command: {}", e))));
        }
    };

    // Generate OpenAPI spec
    let spec_generator = VoiceSpecGenerator::new();
    let spec_result = spec_generator.generate_spec(&parsed).await;
    let spec = match spec_result {
        Ok(spec) => {
            // Convert spec to JSON and include title/version in response
            let mut spec_json = serde_json::to_value(&spec.spec).unwrap_or(Value::Null);
            // Add title and version to the spec JSON for easier frontend access
            if let Value::Object(ref mut obj) = spec_json {
                if let Some(Value::Object(ref mut info)) = obj.get_mut("info") {
                    // Ensure title and version are present
                    if !info.contains_key("title") {
                        info.insert("title".to_string(), Value::String(parsed.title.clone()));
                    }
                    if !info.contains_key("version") {
                        info.insert("version".to_string(), Value::String("1.0.0".to_string()));
                    }
                }
            }
            Some(spec_json)
        }
        Err(e) => {
            return Ok(ResponseJson(ApiResponse::error(format!("Failed to generate spec: {}", e))));
        }
    };

    // Convert parsed command to response format
    let parsed_data = ParsedCommandData {
        api_type: parsed.api_type.clone(),
        title: parsed.title.clone(),
        description: parsed.description.clone(),
        endpoints: parsed
            .endpoints
            .iter()
            .map(|e| serde_json::to_value(e).unwrap_or(Value::Null))
            .collect(),
        models: parsed
            .models
            .iter()
            .map(|m| serde_json::to_value(m).unwrap_or(Value::Null))
            .collect(),
    };

    let response = ProcessVoiceCommandResponse {
        command: request.command,
        parsed: parsed_data,
        spec,
        error: None,
    };

    Ok(ResponseJson(ApiResponse::success(response)))
}

/// Request to transpile a natural language hook description
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranspileHookRequest {
    /// Natural language description of the hook logic
    pub description: String,
}

/// Response from transpiling a hook description
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranspileHookResponse {
    /// The original description
    pub description: String,
    /// Transpiled hook configuration (as YAML)
    pub hook_yaml: Option<String>,
    /// Transpiled hook configuration (as JSON)
    pub hook_json: Option<Value>,
    /// Optional error message
    pub error: Option<String>,
}

/// Transpile a natural language hook description to hook configuration
///
/// POST /api/v2/voice/transpile-hook
pub async fn transpile_hook(
    Json(request): Json<TranspileHookRequest>,
) -> Result<ResponseJson<ApiResponse<TranspileHookResponse>>, StatusCode> {
    if request.description.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Create transpiler with default config
    let config = IntelligentBehaviorConfig::default();
    let transpiler = HookTranspiler::new(config);

    // Transpile the description
    let hook = match transpiler.transpile(&request.description).await {
        Ok(hook) => hook,
        Err(e) => {
            return Ok(ResponseJson(ApiResponse::error(format!(
                "Failed to transpile hook: {}",
                e
            ))));
        }
    };

    // Convert hook to YAML and JSON
    let hook_yaml = match serde_yaml::to_string(&hook) {
        Ok(yaml) => Some(yaml),
        Err(e) => {
            return Ok(ResponseJson(ApiResponse::error(format!(
                "Failed to serialize hook to YAML: {}",
                e
            ))));
        }
    };

    let hook_json = match serde_json::to_value(&hook) {
        Ok(json) => Some(json),
        Err(e) => {
            return Ok(ResponseJson(ApiResponse::error(format!(
                "Failed to serialize hook to JSON: {}",
                e
            ))));
        }
    };

    let response = TranspileHookResponse {
        description: request.description,
        hook_yaml,
        hook_json,
        error: None,
    };

    Ok(ResponseJson(ApiResponse::success(response)))
}

/// Request to create a workspace scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceScenarioRequest {
    /// Natural language description of the scenario
    pub description: String,
}

/// Response from creating a workspace scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceScenarioResponse {
    /// The original description
    pub description: String,
    /// Generated workspace scenario
    pub scenario: Option<GeneratedWorkspaceScenario>,
    /// Optional error message
    pub error: Option<String>,
}

/// Create a workspace scenario from natural language description
///
/// POST /api/v2/voice/create-workspace-scenario
pub async fn create_workspace_scenario(
    Json(request): Json<CreateWorkspaceScenarioRequest>,
) -> Result<ResponseJson<ApiResponse<CreateWorkspaceScenarioResponse>>, StatusCode> {
    if request.description.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Create parser with default config
    let config = IntelligentBehaviorConfig::default();
    let parser = VoiceCommandParser::new(config);

    // Parse the scenario description
    let parsed = match parser
        .parse_workspace_scenario_command(&request.description)
        .await
    {
        Ok(parsed) => parsed,
        Err(e) => {
            return Ok(ResponseJson(ApiResponse::error(format!(
                "Failed to parse scenario description: {}",
                e
            ))));
        }
    };

    // Generate the workspace scenario
    let generator = WorkspaceScenarioGenerator::new();
    let scenario = match generator.generate_scenario(&parsed).await {
        Ok(scenario) => Some(scenario),
        Err(e) => {
            return Ok(ResponseJson(ApiResponse::error(format!(
                "Failed to generate workspace scenario: {}",
                e
            ))));
        }
    };

    let response = CreateWorkspaceScenarioResponse {
        description: request.description,
        scenario,
        error: None,
    };

    Ok(ResponseJson(ApiResponse::success(response)))
}

/// Request to create a workspace from natural language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceRequest {
    /// Natural language description of the workspace
    pub description: String,
}

/// Response from parsing workspace creation command (preview)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspacePreviewResponse {
    /// The original description
    pub description: String,
    /// Parsed workspace creation data (for preview)
    pub parsed: ParsedWorkspaceCreation,
    /// Optional error message
    pub error: Option<String>,
}

/// Request to confirm and create workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmCreateWorkspaceRequest {
    /// Parsed workspace creation data (from preview)
    pub parsed: ParsedWorkspaceCreation,
}

/// Response from creating a workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceResponse {
    /// Workspace ID
    pub workspace_id: String,
    /// Workspace name
    pub name: String,
    /// Creation log
    pub creation_log: Vec<String>,
    /// Number of endpoints created
    pub endpoint_count: usize,
    /// Number of personas created
    pub persona_count: usize,
    /// Number of scenarios created
    pub scenario_count: usize,
    /// Whether reality continuum is configured
    pub has_reality_continuum: bool,
    /// Whether drift budget is configured
    pub has_drift_budget: bool,
    /// Optional error message
    pub error: Option<String>,
}

/// Parse workspace creation command and return preview
///
/// POST /api/v2/voice/create-workspace-preview
pub async fn create_workspace_preview(
    Json(request): Json<CreateWorkspaceRequest>,
) -> Result<ResponseJson<ApiResponse<CreateWorkspacePreviewResponse>>, StatusCode> {
    if request.description.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Create parser with default config
    let config = IntelligentBehaviorConfig::default();
    let parser = VoiceCommandParser::new(config);

    // Parse the workspace creation command
    let parsed = match parser.parse_workspace_creation_command(&request.description).await {
        Ok(parsed) => parsed,
        Err(e) => {
            return Ok(ResponseJson(ApiResponse::error(format!(
                "Failed to parse workspace creation command: {}",
                e
            ))));
        }
    };

    let response = CreateWorkspacePreviewResponse {
        description: request.description,
        parsed,
        error: None,
    };

    Ok(ResponseJson(ApiResponse::success(response)))
}

/// Confirm and create workspace from parsed command
///
/// POST /api/v2/voice/create-workspace-confirm
pub async fn create_workspace_confirm(
    State(state): State<WorkspaceState>,
    Json(request): Json<ConfirmCreateWorkspaceRequest>,
) -> Result<ResponseJson<ApiResponse<CreateWorkspaceResponse>>, StatusCode> {
    // Create workspace builder
    let mut builder = WorkspaceBuilder::new();

    // Get mutable access to workspace registry from state
    let mut registry = state.registry.write().await;

    // Build workspace
    let built = match builder.build_workspace(&mut registry, &request.parsed).await {
        Ok(built) => built,
        Err(e) => {
            return Ok(ResponseJson(ApiResponse::error(format!(
                "Failed to create workspace: {}",
                e
            ))));
        }
    };

    let endpoint_count = built
        .openapi_spec
        .as_ref()
        .map(|s| s.all_paths_and_operations().len())
        .unwrap_or(0);

    let response = CreateWorkspaceResponse {
        workspace_id: built.workspace_id,
        name: built.name,
        creation_log: built.creation_log,
        endpoint_count,
        persona_count: built.personas.len(),
        scenario_count: built.scenarios.len(),
        has_reality_continuum: built.reality_continuum.is_some(),
        has_drift_budget: built.drift_budget.is_some(),
        error: None,
    };

    Ok(ResponseJson(ApiResponse::success(response)))
}
