//! Voice + LLM Interface API handlers for Admin UI
//!
//! Provides endpoints for processing voice commands and generating OpenAPI specs
//! using natural language commands powered by LLM.

use axum::{extract::Json, http::StatusCode, response::Json as ResponseJson};
use mockforge_core::intelligent_behavior::IntelligentBehaviorConfig;
use mockforge_core::{VoiceCommandParser, VoiceSpecGenerator};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::ApiResponse;

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
