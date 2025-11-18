//! AI Studio API handlers for Admin UI
//!
//! Provides endpoints for the unified AI Studio interface, including natural language
//! chat, mock generation, debugging, persona generation, and artifact freezing.

use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
    response::Json as ResponseJson,
};
use mockforge_core::ai_studio::{
    get_conversation_store, initialize_conversation_store, ArtifactFreezer, BudgetConfig,
    BudgetManager, ChatContext, ChatMessage, ChatOrchestrator, ChatRequest, ChatResponse,
    DebugAnalyzer, DebugRequest, DebugResponse, FreezeRequest, FrozenArtifact, MockGenerator,
    PersonaGenerationRequest, PersonaGenerationResponse, PersonaGenerator, UsageStats,
};
use mockforge_core::intelligent_behavior::IntelligentBehaviorConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::handlers::AdminState;
use crate::models::ApiResponse;

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
    Json(request): Json<GenerateMockRequest>,
) -> Result<ResponseJson<ApiResponse<GenerateMockResponse>>, StatusCode> {
    if request.description.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let generator = MockGenerator::new();

    match generator.generate(&request.description).await {
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
    Json(request): Json<GeneratePersonaRequest>,
) -> Result<ResponseJson<ApiResponse<PersonaGenerationResponse>>, StatusCode> {
    if request.description.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let generator = PersonaGenerator::new();

    let persona_request = PersonaGenerationRequest {
        description: request.description,
        base_persona_id: request.base_persona_id,
        workspace_id: request.workspace_id,
    };

    match generator.generate(&persona_request).await {
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

/// Freeze an AI-generated artifact to deterministic format
///
/// POST /api/v1/ai-studio/freeze
pub async fn freeze_artifact(
    Json(request): Json<FreezeArtifactRequest>,
) -> Result<ResponseJson<ApiResponse<FrozenArtifact>>, StatusCode> {
    let freezer = ArtifactFreezer::new();

    let freeze_request = FreezeRequest {
        artifact_type: request.artifact_type,
        content: request.content,
        format: request.format,
        path: request.path,
    };

    match freezer.freeze(&freeze_request).await {
        Ok(artifact) => Ok(ResponseJson(ApiResponse::success(artifact))),
        Err(e) => Ok(ResponseJson(ApiResponse::error(format!("Failed to freeze artifact: {}", e)))),
    }
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
