//! AI-powered spec generation (`POST /__mockforge/ai/generate-spec`).
//!
//! Moved from `mockforge_http::management::ai_gen::generate_ai_spec`
//! under #656. The original took `State<ManagementState>` but never
//! read it, so this version drops the extractor — axum allows handlers
//! to skip the state argument even on routers that carry state. The
//! data-faker / stub-503 contract is preserved via this crate's mirror
//! `data-faker` feature flag (controlled by `mockforge-http`'s flag of
//! the same name).
//!
//! Only foreign dep is `mockforge_data::rag::*`, already an
//! unconditional dep of this crate, so the move is cycle-safe.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;

/// Request for AI-powered API specification generation
#[derive(Debug, Deserialize)]
pub struct GenerateSpecRequest {
    /// Natural language description of the API to generate
    pub query: String,
    /// Type of specification to generate: "openapi", "graphql", or "asyncapi"
    pub spec_type: String,
    /// Optional API version (e.g., "3.0.0" for OpenAPI)
    pub api_version: Option<String>,
}

/// Generate API specification from natural language using AI
#[cfg(feature = "data-faker")]
pub async fn generate_ai_spec(Json(request): Json<GenerateSpecRequest>) -> impl IntoResponse {
    use mockforge_data::rag::{
        config::{LlmProvider, RagConfig},
        engine::RagEngine,
        storage::DocumentStorage,
    };
    use std::sync::Arc;

    // Build RAG config from environment variables
    let api_key = std::env::var("MOCKFORGE_RAG_API_KEY")
        .ok()
        .or_else(|| std::env::var("OPENAI_API_KEY").ok());

    // Check if RAG is configured - require API key
    if api_key.is_none() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "AI service not configured",
                "message": "Please provide an API key via MOCKFORGE_RAG_API_KEY or OPENAI_API_KEY"
            })),
        )
            .into_response();
    }

    // Build RAG configuration
    let provider_str = std::env::var("MOCKFORGE_RAG_PROVIDER")
        .unwrap_or_else(|_| "openai".to_string())
        .to_lowercase();

    let provider = match provider_str.as_str() {
        "openai" => LlmProvider::OpenAI,
        "anthropic" => LlmProvider::Anthropic,
        "ollama" => LlmProvider::Ollama,
        "openai-compatible" | "openai_compatible" => LlmProvider::OpenAICompatible,
        _ => LlmProvider::OpenAI,
    };

    let api_endpoint =
        std::env::var("MOCKFORGE_RAG_API_ENDPOINT").unwrap_or_else(|_| match provider {
            LlmProvider::OpenAI => "https://api.openai.com/v1".to_string(),
            LlmProvider::Anthropic => "https://api.anthropic.com/v1".to_string(),
            LlmProvider::Ollama => "http://localhost:11434/api".to_string(),
            LlmProvider::OpenAICompatible => "http://localhost:8000/v1".to_string(),
        });

    let model = std::env::var("MOCKFORGE_RAG_MODEL").unwrap_or_else(|_| match provider {
        LlmProvider::OpenAI => "gpt-3.5-turbo".to_string(),
        LlmProvider::Anthropic => "claude-3-sonnet-20240229".to_string(),
        LlmProvider::Ollama => "llama2".to_string(),
        LlmProvider::OpenAICompatible => "gpt-3.5-turbo".to_string(),
    });

    // Build RagConfig using struct literal with defaults
    let rag_config = RagConfig {
        provider,
        api_endpoint,
        api_key,
        model,
        max_tokens: std::env::var("MOCKFORGE_RAG_MAX_TOKENS")
            .unwrap_or_else(|_| "4096".to_string())
            .parse()
            .unwrap_or(4096),
        temperature: std::env::var("MOCKFORGE_RAG_TEMPERATURE")
            .unwrap_or_else(|_| "0.3".to_string())
            .parse()
            .unwrap_or(0.3), // Lower temperature for more structured output
        timeout_secs: std::env::var("MOCKFORGE_RAG_TIMEOUT")
            .unwrap_or_else(|_| "60".to_string())
            .parse()
            .unwrap_or(60),
        max_context_length: std::env::var("MOCKFORGE_RAG_CONTEXT_WINDOW")
            .unwrap_or_else(|_| "4000".to_string())
            .parse()
            .unwrap_or(4000),
        ..Default::default()
    };

    // Build the prompt for spec generation
    let spec_type_label = match request.spec_type.as_str() {
        "openapi" => "OpenAPI 3.0",
        "graphql" => "GraphQL",
        "asyncapi" => "AsyncAPI",
        _ => "OpenAPI 3.0",
    };

    let api_version = request.api_version.as_deref().unwrap_or("3.0.0");

    let prompt = format!(
        r#"You are an expert API architect. Generate a complete {} specification based on the following user requirements.

User Requirements:
{}

Instructions:
1. Generate a complete, valid {} specification
2. Include all paths, operations, request/response schemas, and components
3. Use realistic field names and data types
4. Include proper descriptions and examples
5. Follow {} best practices
6. Return ONLY the specification, no additional explanation
7. For OpenAPI, use version {}

Return the specification in {} format."#,
        spec_type_label,
        request.query,
        spec_type_label,
        spec_type_label,
        api_version,
        if request.spec_type == "graphql" {
            "GraphQL SDL"
        } else {
            "YAML"
        }
    );

    // Create in-memory storage for RAG engine
    use mockforge_data::rag::storage::InMemoryStorage;
    let storage: Arc<dyn DocumentStorage> = Arc::new(InMemoryStorage::new());

    // Create RAG engine
    let mut rag_engine = match RagEngine::new(rag_config.clone(), storage) {
        Ok(engine) => engine,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to initialize RAG engine",
                    "message": e.to_string()
                })),
            )
                .into_response();
        }
    };

    // Generate using RAG engine
    match rag_engine.generate(&prompt, None).await {
        Ok(generated_text) => {
            // Try to extract just the YAML/JSON/SDL content if LLM added explanation
            let spec = if request.spec_type == "graphql" {
                // For GraphQL, extract SDL
                extract_graphql_schema(&generated_text)
            } else {
                // For OpenAPI/AsyncAPI, extract YAML
                extract_yaml_spec(&generated_text)
            };

            Json(serde_json::json!({
                "success": true,
                "spec": spec,
                "spec_type": request.spec_type,
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "AI generation failed",
                "message": e.to_string()
            })),
        )
            .into_response(),
    }
}

#[cfg(not(feature = "data-faker"))]
pub async fn generate_ai_spec(Json(_request): Json<GenerateSpecRequest>) -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": "AI features not enabled",
            "message": "Please enable the 'data-faker' feature to use AI-powered specification generation"
        })),
    )
        .into_response()
}

#[cfg(feature = "data-faker")]
fn extract_yaml_spec(text: &str) -> String {
    // Try to find YAML code blocks
    if let Some(start) = text.find("```yaml") {
        let yaml_start = text[start + 7..].trim_start();
        if let Some(end) = yaml_start.find("```") {
            return yaml_start[..end].trim().to_string();
        }
    }
    if let Some(start) = text.find("```") {
        let content_start = text[start + 3..].trim_start();
        if let Some(end) = content_start.find("```") {
            return content_start[..end].trim().to_string();
        }
    }

    // Check if it starts with openapi: or asyncapi:
    if text.trim_start().starts_with("openapi:") || text.trim_start().starts_with("asyncapi:") {
        return text.trim().to_string();
    }

    // Return as-is if no code blocks found
    text.trim().to_string()
}

/// Extract GraphQL schema from text content
#[cfg(feature = "data-faker")]
fn extract_graphql_schema(text: &str) -> String {
    // Try to find GraphQL code blocks
    if let Some(start) = text.find("```graphql") {
        let schema_start = text[start + 10..].trim_start();
        if let Some(end) = schema_start.find("```") {
            return schema_start[..end].trim().to_string();
        }
    }
    if let Some(start) = text.find("```") {
        let content_start = text[start + 3..].trim_start();
        if let Some(end) = content_start.find("```") {
            return content_start[..end].trim().to_string();
        }
    }

    // Check if it looks like GraphQL SDL (starts with type, schema, etc.)
    if text.trim_start().starts_with("type ") || text.trim_start().starts_with("schema ") {
        return text.trim().to_string();
    }

    text.trim().to_string()
}
