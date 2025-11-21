//! AI Studio API handlers for Admin UI
//!
//! Provides endpoints for the unified AI Studio interface, including natural language
//! chat, mock generation, debugging, persona generation, and artifact freezing.

use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
    response::Json as ResponseJson,
};
use json_patch::{patch, Patch};
use jsonptr::PointerBuf;
use mockforge_core::ai_studio::{
    get_conversation_store, initialize_conversation_store, ArtifactFreezer, BudgetConfig,
    BudgetManager, ChatContext, ChatMessage, ChatOrchestrator, ChatRequest, ChatResponse,
    DebugAnalyzer, DebugRequest, DebugResponse, DebugContextIntegrator, FreezeRequest, FrozenArtifact, 
    MockGenerator, PersonaGenerationRequest, PersonaGenerationResponse, PersonaGenerator, UsageStats,
    ContractDiffHandler, ContractDiffQueryResult, OrgControls, OrgAiControlsConfig,
    config::AiMode,
};
use mockforge_core::intelligent_behavior::IntelligentBehaviorConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::handlers::AdminState;
use crate::models::ApiResponse;
use mockforge_core::ai_studio::config::DeterministicModeConfig;

/// Request for chat interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequestPayload {
    /// User's message
    pub message: String,

    /// Optional conversation ID
    pub conversation_id: Option<String>,

    /// Optional workspace ID
    pub workspace_id: Option<String>,
}

/// Process a chat message
///
/// POST /api/v1/ai-studio/chat
pub async fn chat(
    Json(request): Json<ChatRequestPayload>,
) -> Result<ResponseJson<ApiResponse<ChatResponse>>, StatusCode> {
    if request.message.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Create orchestrator with default config
    let config = IntelligentBehaviorConfig::default();
    let orchestrator = ChatOrchestrator::new(config);

    // Initialize conversation store if not already done
    // Note: In production, this should be done at application startup
    let _ = initialize_conversation_store().await;

    // Load conversation context if conversation_id provided
    let context = if let Some(conv_id) = &request.conversation_id {
        let store = get_conversation_store();
        match store.get_context(conv_id).await {
            Ok(Some(ctx)) => Some(ctx),
            Ok(None) => {
                // Conversation not found, create new context
                Some(ChatContext {
                    history: vec![],
                    workspace_id: request.workspace_id.clone(),
                })
            }
            Err(_) => {
                // Error loading, use empty context
                Some(ChatContext {
                    history: vec![],
                    workspace_id: request.workspace_id.clone(),
                })
            }
        }
    } else {
        None
    };

    // Build chat request
    let chat_request = ChatRequest {
        message: request.message.clone(),
        context,
        workspace_id: request.workspace_id.clone(),
        org_id: None,
        user_id: None,
    };

    // Process request
    let response_result = orchestrator.process(&chat_request).await;

    // Save conversation history if conversation_id provided
    if let Some(conv_id) = &request.conversation_id {
        let store = get_conversation_store();

        // Add user message to conversation
        let user_message = ChatMessage {
            role: "user".to_string(),
            content: request.message.clone(),
        };
        let _ = store.add_message(conv_id, user_message).await;

        // Add assistant response to conversation
        if let Ok(ref response) = response_result {
            let assistant_message = ChatMessage {
                role: "assistant".to_string(),
                content: response.message.clone(),
            };
            let _ = store.add_message(conv_id, assistant_message).await;
        }
    } else {
        // Create new conversation if none specified
        let store = get_conversation_store();
        if let Ok(conv_id) = store.create_conversation(request.workspace_id.clone()).await {
            // Add messages to new conversation
            let user_message = ChatMessage {
                role: "user".to_string(),
                content: request.message.clone(),
            };
            let _ = store.add_message(&conv_id, user_message).await;

            if let Ok(ref response) = response_result {
                let assistant_message = ChatMessage {
                    role: "assistant".to_string(),
                    content: response.message.clone(),
                };
                let _ = store.add_message(&conv_id, assistant_message).await;
            }
        }
    }

    // Return response
    match response_result {
        Ok(response) => Ok(ResponseJson(ApiResponse::success(response))),
        Err(e) => Ok(ResponseJson(ApiResponse::error(format!("Failed to process chat: {}", e)))),
    }
}

/// Request for mock generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateMockRequest {
    /// Natural language description
    pub description: String,

    /// Optional workspace ID
    pub workspace_id: Option<String>,
}

/// Response from mock generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateMockResponse {
    /// Generated OpenAPI spec (if any)
    pub spec: Option<Value>,

    /// Status message
    pub message: String,
}

/// Generate a mock from natural language
///
/// POST /api/v1/ai-studio/generate-mock
pub async fn generate_mock(
    State(state): State<AdminState>,
    Json(request): Json<GenerateMockRequest>,
) -> Result<ResponseJson<ApiResponse<GenerateMockResponse>>, StatusCode> {
    if request.description.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let generator = MockGenerator::new();
    
    // Get workspace config to check ai_mode and deterministic config
    let ai_mode = if let Some(workspace_id) = &request.workspace_id {
        // Try to load workspace to get ai_mode
        if let Ok(workspace) = state.workspace_persistence.load_workspace(workspace_id).await {
            workspace.config.ai_mode
        } else {
            None // Default to live mode if workspace not found
        }
    } else {
        None // Default to live mode if no workspace_id provided
    };

    // Get deterministic config from workspace if available
    let deterministic_config = if let Some(workspace_id) = &request.workspace_id {
        if let Ok(workspace) = state.workspace_persistence.load_workspace(workspace_id).await {
            // Check if workspace has deterministic mode config
            // For now, use default config - in production this would come from workspace config
            Some(DeterministicModeConfig::default())
        } else {
            None
        }
    } else {
        None
    };

    match generator.generate(&request.description, request.workspace_id.as_deref(), ai_mode, deterministic_config.as_ref()).await {
        Ok(result) => {
            let response = GenerateMockResponse {
                spec: result.spec,
                message: result.message,
            };
            Ok(ResponseJson(ApiResponse::success(response)))
        }
        Err(e) => Ok(ResponseJson(ApiResponse::error(format!("Failed to generate mock: {}", e)))),
    }
}

/// Request for debug analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugTestRequest {
    /// Test failure logs
    pub test_logs: String,

    /// Test name/identifier
    pub test_name: Option<String>,

    /// Workspace ID
    pub workspace_id: Option<String>,
}

/// Analyze a test failure
///
/// POST /api/v1/ai-studio/debug-test
pub async fn debug_test(
    Json(request): Json<DebugTestRequest>,
) -> Result<ResponseJson<ApiResponse<DebugResponse>>, StatusCode> {
    if request.test_logs.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let analyzer = DebugAnalyzer::new();

    let debug_request = DebugRequest {
        test_logs: request.test_logs,
        test_name: request.test_name,
        workspace_id: request.workspace_id,
    };

    match analyzer.analyze(&debug_request).await {
        Ok(response) => Ok(ResponseJson(ApiResponse::success(response))),
        Err(e) => Ok(ResponseJson(ApiResponse::error(format!(
            "Failed to analyze test failure: {}",
            e
        )))),
    }
}

/// Request for debug analysis with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugWithContextRequest {
    /// Test failure logs
    pub test_logs: String,

    /// Test name/identifier
    pub test_name: Option<String>,

    /// Workspace ID
    pub workspace_id: Option<String>,

    /// Organization ID (for context)
    pub org_id: Option<String>,
}

/// Analyze a test failure with comprehensive context from subsystems
///
/// POST /api/v1/ai-studio/debug/analyze-with-context
pub async fn debug_analyze_with_context(
    Json(request): Json<DebugWithContextRequest>,
) -> Result<ResponseJson<ApiResponse<DebugResponse>>, StatusCode> {
    if request.test_logs.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let analyzer = DebugAnalyzer::new();

    // Create debug context integrator (optional - would need subsystem references in production)
    // For now, pass None as we don't have direct access to RealityEngine, etc. here
    // In production, these would be injected via State
    let integrator: Option<&DebugContextIntegrator> = None;

    let debug_request = DebugRequest {
        test_logs: request.test_logs,
        test_name: request.test_name,
        workspace_id: request.workspace_id,
    };

    match analyzer.analyze(&debug_request).await {
        Ok(response) => Ok(ResponseJson(ApiResponse::success(response))),
        Err(e) => Ok(ResponseJson(ApiResponse::error(format!(
            "Failed to analyze test failure with context: {}",
            e
        )))),
    }
}

/// Request for persona generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratePersonaRequest {
    /// Natural language description
    pub description: String,

    /// Optional base persona ID to tweak
    pub base_persona_id: Option<String>,

    /// Workspace ID
    pub workspace_id: Option<String>,
}

/// Generate or tweak a persona
///
/// POST /api/v1/ai-studio/generate-persona
pub async fn generate_persona(
    State(state): State<AdminState>,
    Json(request): Json<GeneratePersonaRequest>,
) -> Result<ResponseJson<ApiResponse<PersonaGenerationResponse>>, StatusCode> {
    if request.description.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let generator = PersonaGenerator::new();

    let persona_request = PersonaGenerationRequest {
        description: request.description.clone(),
        base_persona_id: request.base_persona_id,
        workspace_id: request.workspace_id.clone(),
    };
    
    // Get workspace config to check ai_mode and deterministic config
    let ai_mode = if let Some(workspace_id) = &request.workspace_id {
        if let Ok(workspace) = state.workspace_persistence.load_workspace(workspace_id).await {
            workspace.config.ai_mode
        } else {
            None
        }
    } else {
        None
    };

    let deterministic_config = if let Some(workspace_id) = &request.workspace_id {
        if let Ok(_workspace) = state.workspace_persistence.load_workspace(workspace_id).await {
            Some(DeterministicModeConfig::default())
        } else {
            None
        }
    } else {
        None
    };

    match generator.generate(&persona_request, ai_mode, deterministic_config.as_ref()).await {
        Ok(response) => Ok(ResponseJson(ApiResponse::success(response))),
        Err(e) => {
            Ok(ResponseJson(ApiResponse::error(format!("Failed to generate persona: {}", e))))
        }
    }
}

/// Request for artifact freezing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeArtifactRequest {
    /// Type of artifact
    pub artifact_type: String,

    /// Artifact content
    pub content: Value,

    /// Output format (yaml or json)
    pub format: String,

    /// Output path
    pub path: Option<String>,
}

/// Request for artifact freezing with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeArtifactRequestWithMetadata {
    /// Type of artifact
    pub artifact_type: String,

    /// Artifact content
    pub content: Value,

    /// Output format (yaml or json)
    pub format: String,

    /// Output path
    pub path: Option<String>,

    /// Optional metadata
    pub metadata: Option<FreezeMetadataPayload>,
}

/// Metadata payload for freezing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeMetadataPayload {
    /// LLM provider used
    pub llm_provider: Option<String>,

    /// LLM model used
    pub llm_model: Option<String>,

    /// LLM version
    pub llm_version: Option<String>,

    /// Hash of the input prompt
    pub prompt_hash: Option<String>,

    /// Original prompt/description
    pub original_prompt: Option<String>,
}

/// Freeze an AI-generated artifact to deterministic format
///
/// POST /api/v1/ai-studio/freeze
pub async fn freeze_artifact(
    Json(request): Json<FreezeArtifactRequestWithMetadata>,
) -> Result<ResponseJson<ApiResponse<FrozenArtifact>>, StatusCode> {
    let freezer = ArtifactFreezer::new();

    let metadata = request.metadata.map(|m| {
        use mockforge_core::ai_studio::FreezeMetadata;
        FreezeMetadata {
            llm_provider: m.llm_provider,
            llm_model: m.llm_model,
            llm_version: m.llm_version,
            prompt_hash: m.prompt_hash,
            output_hash: None, // Will be calculated by freezer
            original_prompt: m.original_prompt,
        }
    });

    let freeze_request = FreezeRequest {
        artifact_type: request.artifact_type,
        content: request.content,
        format: request.format,
        path: request.path,
        metadata,
    };

    match freezer.freeze(&freeze_request).await {
        Ok(artifact) => Ok(ResponseJson(ApiResponse::success(artifact))),
        Err(e) => Ok(ResponseJson(ApiResponse::error(format!("Failed to freeze artifact: {}", e)))),
    }
}

/// Query parameters for listing frozen artifacts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListFrozenQuery {
    /// Filter by artifact type
    pub artifact_type: Option<String>,

    /// Workspace ID
    pub workspace_id: Option<String>,
}

/// List frozen artifacts
///
/// GET /api/v1/ai-studio/frozen
pub async fn list_frozen(
    Query(params): Query<ListFrozenQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<FrozenArtifact>>>, StatusCode> {
    let freezer = ArtifactFreezer::new();
    let base_dir = freezer.base_dir().to_path_buf();
    
    // Read all files from the freeze directory
    let mut artifacts = Vec::new();
    
    if let Ok(mut entries) = tokio::fs::read_dir(&base_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_file() {
                // Check if file matches artifact_type filter
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if let Some(ref artifact_type) = params.artifact_type {
                    if !file_name.starts_with(&format!("{}_", artifact_type)) {
                        continue;
                    }
                }
                
                // Try to load the frozen artifact
                let content = match tokio::fs::read_to_string(&path).await {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                
                let content_value: Value = if path.extension().and_then(|e| e.to_str()) == Some("yaml")
                    || path.extension().and_then(|e| e.to_str()) == Some("yml")
                {
                    match serde_yaml::from_str(&content) {
                        Ok(v) => v,
                        Err(_) => continue,
                    }
                } else {
                    match serde_json::from_str(&content) {
                        Ok(v) => v,
                        Err(_) => continue,
                    }
                };
                
                // Extract metadata from content
                let metadata = content_value.get("_frozen_metadata")
                    .and_then(|m| serde_json::from_value(m.clone()).ok());
                
                // Extract output_hash from metadata
                let output_hash = content_value
                    .get("_frozen_metadata")
                    .and_then(|m| m.get("output_hash"))
                    .and_then(|h| h.as_str())
                    .map(|s| s.to_string());
                
                // Determine artifact type from filename or metadata
                let artifact_type = content_value
                    .get("_frozen_metadata")
                    .and_then(|m| m.get("artifact_type"))
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| {
                        // Fallback: extract from filename
                        file_name.split('_').next().unwrap_or("unknown").to_string()
                    });
                
                artifacts.push(FrozenArtifact {
                    artifact_type,
                    content: content_value,
                    format: if path.extension().and_then(|e| e.to_str()) == Some("yaml")
                        || path.extension().and_then(|e| e.to_str()) == Some("yml")
                    {
                        "yaml".to_string()
                    } else {
                        "json".to_string()
                    },
                    path: path.to_string_lossy().to_string(),
                    metadata,
                    output_hash,
                });
            }
        }
    }
    
    // Sort by path (most recent first if timestamps are in filename)
    artifacts.sort_by(|a, b| b.path.cmp(&a.path));
    
    Ok(ResponseJson(ApiResponse::success(artifacts)))
}

/// Get usage statistics
///
/// GET /api/v1/ai-studio/usage
pub async fn get_usage(
    Query(params): Query<HashMap<String, String>>,
) -> Result<ResponseJson<ApiResponse<UsageStats>>, StatusCode> {
    let workspace_id = params.get("workspace_id").cloned().unwrap_or_default();

    let budget_config = BudgetConfig::default();
    let budget_manager = BudgetManager::new(budget_config);

    match budget_manager.get_usage(&workspace_id).await {
        Ok(stats) => Ok(ResponseJson(ApiResponse::success(stats))),
        Err(e) => Ok(ResponseJson(ApiResponse::error(format!("Failed to get usage stats: {}", e)))),
    }
}

/// Request for applying a debug patch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyPatchRequest {
    /// JSON Patch operation
    pub patch: Value,

    /// Configuration file path to apply patch to
    pub config_path: Option<String>,
}

/// Apply a debug patch to configuration
///
/// POST /api/v1/ai-studio/apply-patch
pub async fn apply_patch(
    State(state): State<AdminState>,
    Json(request): Json<ApplyPatchRequest>,
) -> Result<ResponseJson<ApiResponse<Value>>, StatusCode> {
    // Determine config file path
    let config_path = request.config_path
        .unwrap_or_else(|| "mockforge.yaml".to_string());
    
    // Load the config file
    let config_content = match tokio::fs::read_to_string(&config_path).await {
        Ok(content) => content,
        Err(e) => {
            return Ok(ResponseJson(ApiResponse::error(format!(
                "Failed to read config file {}: {}",
                config_path, e
            ))));
        }
    };
    
    // Parse config as JSON (works for YAML too via serde_yaml)
    let mut config_value: Value = if config_path.ends_with(".yaml") || config_path.ends_with(".yml") {
        serde_yaml::from_str(&config_content).map_err(|e| {
            StatusCode::BAD_REQUEST
        })?
    } else {
        serde_json::from_str(&config_content).map_err(|e| {
            StatusCode::BAD_REQUEST
        })?
    };
    
    // Parse patch operations
    let patch_ops: Patch = if let Some(ops_array) = request.patch.get("operations").and_then(|v| v.as_array()) {
        // Multiple operations
        Patch(ops_array.iter()
            .filter_map(|op| {
                parse_patch_operation(op).ok()
            })
            .collect())
    } else {
        // Single operation
        Patch(vec![parse_patch_operation(&request.patch)?])
    };
    
    // Apply patch
    patch(&mut config_value, &patch_ops).map_err(|e| {
        StatusCode::BAD_REQUEST
    })?;
    
    // Save updated config
    let updated_content = if config_path.ends_with(".yaml") || config_path.ends_with(".yml") {
        serde_yaml::to_string(&config_value).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        serde_json::to_string_pretty(&config_value).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };
    
    tokio::fs::write(&config_path, updated_content).await.map_err(|e| {
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    Ok(ResponseJson(ApiResponse::success(serde_json::json!({
        "message": "Patch applied successfully",
        "config_path": config_path,
        "updated_config": config_value
    }))))
}

/// Parse a single patch operation from JSON
fn parse_patch_operation(op: &Value) -> Result<json_patch::PatchOperation, StatusCode> {
    use json_patch::{AddOperation, CopyOperation, MoveOperation, PatchOperation, RemoveOperation, ReplaceOperation, TestOperation};
    
    let op_type = op.get("op")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;
    
    let path_str = op.get("path")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;
    
    let path: PointerBuf = path_str.parse()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    match op_type {
        "add" => {
            let value = op.get("value")
                .ok_or(StatusCode::BAD_REQUEST)?;
            Ok(PatchOperation::Add(AddOperation {
                path,
                value: value.clone(),
            }))
        }
        "remove" => {
            Ok(PatchOperation::Remove(RemoveOperation { path }))
        }
        "replace" => {
            let value = op.get("value")
                .ok_or(StatusCode::BAD_REQUEST)?;
            Ok(PatchOperation::Replace(ReplaceOperation {
                path,
                value: value.clone(),
            }))
        }
        "copy" => {
            let from = op.get("from")
                .and_then(|v| v.as_str())
                .ok_or(StatusCode::BAD_REQUEST)?;
            let from_path: PointerBuf = from.parse()
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            Ok(PatchOperation::Copy(json_patch::CopyOperation {
                path,
                from: from_path,
            }))
        }
        "move" => {
            let from = op.get("from")
                .and_then(|v| v.as_str())
                .ok_or(StatusCode::BAD_REQUEST)?;
            let from_path: PointerBuf = from.parse()
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            Ok(PatchOperation::Move(json_patch::MoveOperation {
                path,
                from: from_path,
            }))
        }
        "test" => {
            let value = op.get("value")
                .ok_or(StatusCode::BAD_REQUEST)?;
            Ok(PatchOperation::Test(json_patch::TestOperation {
                path,
                value: value.clone(),
            }))
        }
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

/// Get organization-level AI controls
///
/// GET /api/v1/ai-studio/org-controls
pub async fn get_org_controls(
    Query(params): Query<HashMap<String, String>>,
) -> Result<ResponseJson<ApiResponse<OrgAiControlsConfig>>, StatusCode> {
    let org_id = params.get("org_id").cloned();
    let workspace_id = params.get("workspace_id").cloned();

    // Create org controls service with default YAML config
    // In production, this would be injected via State
    let org_controls = OrgControls::new(OrgAiControlsConfig::default());

    match org_controls.load_org_config(
        org_id.as_deref().unwrap_or("default"),
        workspace_id.as_deref()
    ).await {
        Ok(controls) => Ok(ResponseJson(ApiResponse::success(controls))),
        Err(e) => Ok(ResponseJson(ApiResponse::error(format!("Failed to get org controls: {}", e)))),
    }
}

/// Update organization-level AI controls
///
/// PUT /api/v1/ai-studio/org-controls
pub async fn update_org_controls(
    Query(params): Query<HashMap<String, String>>,
    Json(controls): Json<OrgAiControlsConfig>,
) -> Result<ResponseJson<ApiResponse<OrgAiControlsConfig>>, StatusCode> {
    let org_id = params.get("org_id").cloned();
    let workspace_id = params.get("workspace_id").cloned();

    // Note: Database persistence requires OrgControlsAccessor to be available in State
    // To enable database persistence:
    // 1. Add OrgControlsAccessor (e.g., DbOrgControls) to AdminState
    // 2. Call accessor.save_controls(org_id, workspace_id, &controls).await
    // 3. Return updated controls from database
    // For now, controls are returned as-is (in-memory only)

    Ok(ResponseJson(ApiResponse::success(controls)))
}

/// Get organization-level AI usage statistics
///
/// GET /api/v1/ai-studio/org-controls/usage
pub async fn get_org_usage(
    Query(params): Query<HashMap<String, String>>,
) -> Result<ResponseJson<ApiResponse<Value>>, StatusCode> {
    let org_id = params.get("org_id").cloned();
    let workspace_id = params.get("workspace_id").cloned();

    // Get period filter (default to current month)
    let period_start = params.get("period_start")
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|| {
            // Default to start of current month
            let now = chrono::Utc::now();
            {
                use chrono::{Datelike, TimeZone};
                chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                    .map(|dt| chrono::Utc.from_utc_datetime(&dt))
                    .unwrap()
            }
        });
    
    let period_end = params.get("period_end")
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|| chrono::Utc::now());

    // Note: Database querying requires a database connection pool
    // In production with database access, you would:
    // 1. Get database pool from State
    // 2. Query org_ai_usage_logs table:
    //    SELECT 
    //      SUM(tokens_used) as total_tokens,
    //      SUM(cost_usd) as total_cost,
    //      COUNT(*) as total_calls,
    //      feature_name,
    //      COUNT(DISTINCT user_id) as unique_users
    //    FROM org_ai_usage_logs
    //    WHERE org_id = $1 
    //      AND (workspace_id = $2 OR $2 IS NULL)
    //      AND created_at >= $3
    //      AND created_at <= $4
    //    GROUP BY feature_name
    
    // For now, return structure that matches what the query would return
    Ok(ResponseJson(ApiResponse::success(serde_json::json!({
        "org_id": org_id,
        "workspace_id": workspace_id,
        "period_start": period_start.to_rfc3339(),
        "period_end": period_end.to_rfc3339(),
        "total_tokens": 0,
        "total_cost": 0.0,
        "total_calls": 0,
        "feature_breakdown": {},
        "message": "Usage stats require database connection. Connect to registry server database to enable."
    }))))
}

/// Request for contract diff query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractDiffQueryRequest {
    /// Natural language query
    pub query: String,

    /// Optional workspace ID
    pub workspace_id: Option<String>,

    /// Optional organization ID
    pub org_id: Option<String>,
}

/// Process a natural language query about contract diffs
///
/// POST /api/v1/ai-studio/contract-diff/query
pub async fn contract_diff_query(
    Json(request): Json<ContractDiffQueryRequest>,
) -> Result<ResponseJson<ApiResponse<ContractDiffQueryResult>>, StatusCode> {
    if request.query.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let handler = ContractDiffHandler::new()
        .map_err(|e| {
            tracing::error!("Failed to create ContractDiffHandler: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // For now, we don't have direct access to specs/requests in the handler
    // In production, these would be loaded from workspace/request storage
    match handler.analyze_from_query(&request.query, None, None).await {
        Ok(result) => Ok(ResponseJson(ApiResponse::success(result))),
        Err(e) => Ok(ResponseJson(ApiResponse::error(format!(
            "Failed to process contract diff query: {}",
            e
        )))),
    }
}
