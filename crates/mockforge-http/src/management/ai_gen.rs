use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use tracing::*;

use super::ManagementState;

// ========== AI-Powered Features ==========

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

/// Request for OpenAPI generation from recorded traffic
#[derive(Debug, Deserialize)]
pub struct GenerateOpenApiFromTrafficRequest {
    /// Path to recorder database (optional, defaults to ./recordings.db)
    #[serde(default)]
    pub database_path: Option<String>,
    /// Start time for filtering (ISO 8601 format, e.g., 2025-01-01T00:00:00Z)
    #[serde(default)]
    pub since: Option<String>,
    /// End time for filtering (ISO 8601 format)
    #[serde(default)]
    pub until: Option<String>,
    /// Path pattern filter (supports wildcards, e.g., /api/*)
    #[serde(default)]
    pub path_pattern: Option<String>,
    /// Minimum confidence score for including paths (0.0 to 1.0)
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f64,
}

fn default_min_confidence() -> f64 {
    0.7
}

/// Generate API specification from natural language using AI
#[cfg(feature = "data-faker")]
pub(crate) async fn generate_ai_spec(
    State(_state): State<ManagementState>,
    Json(request): Json<GenerateSpecRequest>,
) -> impl IntoResponse {
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
    // Note: StorageFactory::create_memory() returns Box<dyn DocumentStorage>
    // We need to use unsafe transmute or create a wrapper, but for now we'll use
    // a simpler approach: create InMemoryStorage directly
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
pub(crate) async fn generate_ai_spec(
    State(_state): State<ManagementState>,
    Json(_request): Json<GenerateSpecRequest>,
) -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": "AI features not enabled",
            "message": "Please enable the 'data-faker' feature to use AI-powered specification generation"
        })),
    )
        .into_response()
}

/// Generate OpenAPI specification from recorded traffic
#[cfg(feature = "behavioral-cloning")]
pub(crate) async fn generate_openapi_from_traffic(
    State(_state): State<ManagementState>,
    Json(request): Json<GenerateOpenApiFromTrafficRequest>,
) -> impl IntoResponse {
    use chrono::{DateTime, Utc};
    use mockforge_core::intelligent_behavior::{
        openapi_generator::{OpenApiGenerationConfig, OpenApiSpecGenerator},
        IntelligentBehaviorConfig,
    };
    use mockforge_recorder::{
        database::RecorderDatabase,
        openapi_export::{QueryFilters, RecordingsToOpenApi},
    };
    use std::path::PathBuf;

    // Determine database path
    let db_path = if let Some(ref path) = request.database_path {
        PathBuf::from(path)
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("recordings.db")
    };

    // Open database
    let db = match RecorderDatabase::new(&db_path).await {
        Ok(db) => db,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Database error",
                    "message": format!("Failed to open recorder database: {}", e)
                })),
            )
                .into_response();
        }
    };

    // Parse time filters
    let since_dt = if let Some(ref since_str) = request.since {
        match DateTime::parse_from_rfc3339(since_str) {
            Ok(dt) => Some(dt.with_timezone(&Utc)),
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "Invalid date format",
                        "message": format!("Invalid --since format: {}. Use ISO 8601 format (e.g., 2025-01-01T00:00:00Z)", e)
                    })),
                )
                    .into_response();
            }
        }
    } else {
        None
    };

    let until_dt = if let Some(ref until_str) = request.until {
        match DateTime::parse_from_rfc3339(until_str) {
            Ok(dt) => Some(dt.with_timezone(&Utc)),
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "Invalid date format",
                        "message": format!("Invalid --until format: {}. Use ISO 8601 format (e.g., 2025-01-01T00:00:00Z)", e)
                    })),
                )
                    .into_response();
            }
        }
    } else {
        None
    };

    // Build query filters
    let query_filters = QueryFilters {
        since: since_dt,
        until: until_dt,
        path_pattern: request.path_pattern.clone(),
        min_status_code: None,
        max_requests: Some(1000),
    };

    // Query HTTP exchanges
    // Note: We need to convert from mockforge-recorder's HttpExchange to mockforge-core's HttpExchange
    // to avoid version mismatch issues. The converter returns the version from mockforge-recorder's
    // dependency, so we need to manually convert to the local version.
    let exchanges_from_recorder =
        match RecordingsToOpenApi::query_http_exchanges(&db, Some(query_filters)).await {
            Ok(exchanges) => exchanges,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "Query error",
                        "message": format!("Failed to query HTTP exchanges: {}", e)
                    })),
                )
                    .into_response();
            }
        };

    if exchanges_from_recorder.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "No exchanges found",
                "message": "No HTTP exchanges found matching the specified filters"
            })),
        )
            .into_response();
    }

    // Convert to local HttpExchange type to avoid version mismatch
    use mockforge_core::intelligent_behavior::openapi_generator::HttpExchange as LocalHttpExchange;
    let exchanges: Vec<LocalHttpExchange> = exchanges_from_recorder
        .into_iter()
        .map(|e| LocalHttpExchange {
            method: e.method,
            path: e.path,
            query_params: e.query_params,
            headers: e.headers,
            body: e.body,
            body_encoding: e.body_encoding,
            status_code: e.status_code,
            response_headers: e.response_headers,
            response_body: e.response_body,
            response_body_encoding: e.response_body_encoding,
            timestamp: e.timestamp,
        })
        .collect();

    // Create OpenAPI generator config
    let behavior_config = IntelligentBehaviorConfig::default();
    let gen_config = OpenApiGenerationConfig {
        min_confidence: request.min_confidence,
        behavior_model: Some(behavior_config.behavior_model),
    };

    // Generate OpenAPI spec
    let generator = OpenApiSpecGenerator::new(gen_config);
    let result = match generator.generate_from_exchanges(exchanges).await {
        Ok(result) => result,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Generation error",
                    "message": format!("Failed to generate OpenAPI spec: {}", e)
                })),
            )
                .into_response();
        }
    };

    // Prepare response
    let spec_json = if let Some(ref raw) = result.spec.raw_document {
        raw.clone()
    } else {
        match serde_json::to_value(&result.spec.spec) {
            Ok(json) => json,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "Serialization error",
                        "message": format!("Failed to serialize OpenAPI spec: {}", e)
                    })),
                )
                    .into_response();
            }
        }
    };

    // Build response with metadata
    let response = serde_json::json!({
        "spec": spec_json,
        "metadata": {
            "requests_analyzed": result.metadata.requests_analyzed,
            "paths_inferred": result.metadata.paths_inferred,
            "path_confidence": result.metadata.path_confidence,
            "generated_at": result.metadata.generated_at.to_rfc3339(),
            "duration_ms": result.metadata.duration_ms,
        }
    });

    Json(response).into_response()
}

/// List all rule explanations
pub(crate) async fn list_rule_explanations(
    State(state): State<ManagementState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    use mockforge_core::intelligent_behavior::RuleType;

    let explanations = state.rule_explanations.read().await;
    let mut explanations_vec: Vec<_> = explanations.values().cloned().collect();

    // Filter by rule type if provided
    if let Some(rule_type_str) = params.get("rule_type") {
        if let Ok(rule_type) = serde_json::from_str::<RuleType>(&format!("\"{}\"", rule_type_str)) {
            explanations_vec.retain(|e| e.rule_type == rule_type);
        }
    }

    // Filter by minimum confidence if provided
    if let Some(min_confidence_str) = params.get("min_confidence") {
        if let Ok(min_confidence) = min_confidence_str.parse::<f64>() {
            explanations_vec.retain(|e| e.confidence >= min_confidence);
        }
    }

    // Sort by confidence (descending) and then by generated_at (descending)
    explanations_vec.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.generated_at.cmp(&a.generated_at))
    });

    Json(serde_json::json!({
        "explanations": explanations_vec,
        "total": explanations_vec.len(),
    }))
    .into_response()
}

/// Get a specific rule explanation by ID
pub(crate) async fn get_rule_explanation(
    State(state): State<ManagementState>,
    Path(rule_id): Path<String>,
) -> impl IntoResponse {
    let explanations = state.rule_explanations.read().await;

    match explanations.get(&rule_id) {
        Some(explanation) => Json(serde_json::json!({
            "explanation": explanation,
        }))
        .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Rule explanation not found",
                "message": format!("No explanation found for rule ID: {}", rule_id)
            })),
        )
            .into_response(),
    }
}

/// Request for learning from examples
#[derive(Debug, Deserialize)]
pub struct LearnFromExamplesRequest {
    /// Example request/response pairs to learn from
    pub examples: Vec<ExamplePairRequest>,
    /// Optional configuration override
    #[serde(default)]
    pub config: Option<serde_json::Value>,
}

/// Example pair request format
#[derive(Debug, Deserialize)]
pub struct ExamplePairRequest {
    /// Request data (method, path, body, etc.)
    pub request: serde_json::Value,
    /// Response data (status_code, body, etc.)
    pub response: serde_json::Value,
}

/// Learn behavioral rules from example pairs
///
/// This endpoint accepts example request/response pairs, generates behavioral rules
/// with explanations, and stores the explanations for later retrieval.
pub(crate) async fn learn_from_examples(
    State(state): State<ManagementState>,
    Json(request): Json<LearnFromExamplesRequest>,
) -> impl IntoResponse {
    use mockforge_core::intelligent_behavior::{
        config::{BehaviorModelConfig, IntelligentBehaviorConfig},
        rule_generator::{ExamplePair, RuleGenerator},
    };

    if request.examples.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "No examples provided",
                "message": "At least one example pair is required"
            })),
        )
            .into_response();
    }

    // Convert request examples to ExamplePair format
    let example_pairs: Result<Vec<ExamplePair>, String> = request
        .examples
        .into_iter()
        .enumerate()
        .map(|(idx, ex)| {
            // Parse request JSON to extract method, path, body, etc.
            let method = ex
                .request
                .get("method")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "GET".to_string());
            let path = ex
                .request
                .get("path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "/".to_string());
            let request_body = ex.request.get("body").cloned();
            let query_params = ex
                .request
                .get("query_params")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default();
            let headers = ex
                .request
                .get("headers")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default();

            // Parse response JSON to extract status, body, etc.
            let status = ex
                .response
                .get("status_code")
                .or_else(|| ex.response.get("status"))
                .and_then(|v| v.as_u64())
                .map(|n| n as u16)
                .unwrap_or(200);
            let response_body = ex.response.get("body").cloned();

            Ok(ExamplePair {
                method,
                path,
                request: request_body,
                status,
                response: response_body,
                query_params,
                headers,
                metadata: {
                    let mut meta = std::collections::HashMap::new();
                    meta.insert("source".to_string(), "api".to_string());
                    meta.insert("example_index".to_string(), idx.to_string());
                    meta
                },
            })
        })
        .collect();

    let example_pairs = match example_pairs {
        Ok(pairs) => pairs,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid examples",
                    "message": e
                })),
            )
                .into_response();
        }
    };

    // Create behavior config (use provided config or default)
    let behavior_config = if let Some(config_json) = request.config {
        // Try to deserialize custom config, fall back to default
        serde_json::from_value(config_json)
            .unwrap_or_else(|_| IntelligentBehaviorConfig::default())
            .behavior_model
    } else {
        BehaviorModelConfig::default()
    };

    // Create rule generator
    let generator = RuleGenerator::new(behavior_config);

    // Generate rules with explanations
    let (rules, explanations) =
        match generator.generate_rules_with_explanations(example_pairs).await {
            Ok(result) => result,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "Rule generation failed",
                        "message": format!("Failed to generate rules: {}", e)
                    })),
                )
                    .into_response();
            }
        };

    // Store explanations in ManagementState
    {
        let mut stored_explanations = state.rule_explanations.write().await;
        for explanation in &explanations {
            stored_explanations.insert(explanation.rule_id.clone(), explanation.clone());
        }
    }

    // Prepare response
    let response = serde_json::json!({
        "success": true,
        "rules_generated": {
            "consistency_rules": rules.consistency_rules.len(),
            "schemas": rules.schemas.len(),
            "state_machines": rules.state_transitions.len(),
            "system_prompt": !rules.system_prompt.is_empty(),
        },
        "explanations": explanations.iter().map(|e| serde_json::json!({
            "rule_id": e.rule_id,
            "rule_type": e.rule_type,
            "confidence": e.confidence,
            "reasoning": e.reasoning,
        })).collect::<Vec<_>>(),
        "total_explanations": explanations.len(),
    });

    Json(response).into_response()
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

// ========== Chaos Engineering Management ==========

/// Get current chaos engineering configuration
pub(crate) async fn get_chaos_config(State(_state): State<ManagementState>) -> impl IntoResponse {
    #[cfg(feature = "chaos")]
    {
        if let Some(chaos_state) = &_state.chaos_api_state {
            let config = chaos_state.config.read().await;
            // Convert ChaosConfig to JSON response format
            Json(serde_json::json!({
                "enabled": config.enabled,
                "latency": config.latency.as_ref().map(|l| serde_json::to_value(l).unwrap_or(serde_json::Value::Null)),
                "fault_injection": config.fault_injection.as_ref().map(|f| serde_json::to_value(f).unwrap_or(serde_json::Value::Null)),
                "rate_limit": config.rate_limit.as_ref().map(|r| serde_json::to_value(r).unwrap_or(serde_json::Value::Null)),
                "traffic_shaping": config.traffic_shaping.as_ref().map(|t| serde_json::to_value(t).unwrap_or(serde_json::Value::Null)),
            }))
            .into_response()
        } else {
            // Chaos API not available, return default
            Json(serde_json::json!({
                "enabled": false,
                "latency": null,
                "fault_injection": null,
                "rate_limit": null,
                "traffic_shaping": null,
            }))
            .into_response()
        }
    }
    #[cfg(not(feature = "chaos"))]
    {
        // Chaos feature not enabled
        Json(serde_json::json!({
            "enabled": false,
            "latency": null,
            "fault_injection": null,
            "rate_limit": null,
            "traffic_shaping": null,
        }))
        .into_response()
    }
}

/// Request to update chaos configuration
#[derive(Debug, Deserialize)]
pub struct ChaosConfigUpdate {
    /// Whether to enable chaos engineering
    pub enabled: Option<bool>,
    /// Latency configuration
    pub latency: Option<serde_json::Value>,
    /// Fault injection configuration
    pub fault_injection: Option<serde_json::Value>,
    /// Rate limiting configuration
    pub rate_limit: Option<serde_json::Value>,
    /// Traffic shaping configuration
    pub traffic_shaping: Option<serde_json::Value>,
}

/// Update chaos engineering configuration
pub(crate) async fn update_chaos_config(
    State(_state): State<ManagementState>,
    Json(_config_update): Json<ChaosConfigUpdate>,
) -> impl IntoResponse {
    #[cfg(feature = "chaos")]
    {
        if let Some(chaos_state) = &_state.chaos_api_state {
            use mockforge_chaos::config::{
                FaultInjectionConfig, LatencyConfig, RateLimitConfig, TrafficShapingConfig,
            };

            let mut config = chaos_state.config.write().await;

            // Update enabled flag if provided
            if let Some(enabled) = _config_update.enabled {
                config.enabled = enabled;
            }

            // Update latency config if provided
            if let Some(latency_json) = _config_update.latency {
                if let Ok(latency) = serde_json::from_value::<LatencyConfig>(latency_json) {
                    config.latency = Some(latency);
                }
            }

            // Update fault injection config if provided
            if let Some(fault_json) = _config_update.fault_injection {
                if let Ok(fault) = serde_json::from_value::<FaultInjectionConfig>(fault_json) {
                    config.fault_injection = Some(fault);
                }
            }

            // Update rate limit config if provided
            if let Some(rate_json) = _config_update.rate_limit {
                if let Ok(rate) = serde_json::from_value::<RateLimitConfig>(rate_json) {
                    config.rate_limit = Some(rate);
                }
            }

            // Update traffic shaping config if provided
            if let Some(traffic_json) = _config_update.traffic_shaping {
                if let Ok(traffic) = serde_json::from_value::<TrafficShapingConfig>(traffic_json) {
                    config.traffic_shaping = Some(traffic);
                }
            }

            // Reinitialize middleware injectors with new config
            // The middleware will pick up the changes on the next request
            drop(config);

            info!("Chaos configuration updated successfully");
            Json(serde_json::json!({
                "success": true,
                "message": "Chaos configuration updated and applied"
            }))
            .into_response()
        } else {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "success": false,
                    "error": "Chaos API not available",
                    "message": "Chaos engineering is not enabled or configured"
                })),
            )
                .into_response()
        }
    }
    #[cfg(not(feature = "chaos"))]
    {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({
                "success": false,
                "error": "Chaos feature not enabled",
                "message": "Chaos engineering feature is not compiled into this build"
            })),
        )
            .into_response()
    }
}

// ========== Network Profile Management ==========

/// List available network profiles
pub(crate) async fn list_network_profiles() -> impl IntoResponse {
    use mockforge_chaos::core_network_profiles::NetworkProfileCatalog;

    let catalog = NetworkProfileCatalog::default();
    let profiles: Vec<serde_json::Value> = catalog
        .list_profiles_with_description()
        .iter()
        .map(|(name, description)| {
            serde_json::json!({
                "name": name,
                "description": description,
            })
        })
        .collect();

    Json(serde_json::json!({
        "profiles": profiles
    }))
    .into_response()
}

#[derive(Debug, Deserialize)]
/// Request to apply a network profile
pub struct ApplyNetworkProfileRequest {
    /// Name of the network profile to apply
    pub profile_name: String,
}

/// Apply a network profile
pub(crate) async fn apply_network_profile(
    State(state): State<ManagementState>,
    Json(request): Json<ApplyNetworkProfileRequest>,
) -> impl IntoResponse {
    use mockforge_chaos::core_network_profiles::NetworkProfileCatalog;

    let catalog = NetworkProfileCatalog::default();
    if let Some(profile) = catalog.get(&request.profile_name) {
        // Apply profile to server configuration if available
        // NetworkProfile contains latency and traffic_shaping configs
        if let Some(server_config) = &state.server_config {
            let mut config = server_config.write().await;

            // Apply network profile's traffic shaping to core config
            use mockforge_core::config::NetworkShapingConfig;

            // Convert NetworkProfile's TrafficShapingConfig to NetworkShapingConfig
            // NetworkProfile uses mockforge_core::traffic_shaping::TrafficShapingConfig
            // which has bandwidth and burst_loss fields
            let network_shaping = NetworkShapingConfig {
                enabled: profile.traffic_shaping.bandwidth.enabled
                    || profile.traffic_shaping.burst_loss.enabled,
                bandwidth_limit_bps: profile.traffic_shaping.bandwidth.max_bytes_per_sec * 8, // Convert bytes to bits
                packet_loss_percent: profile.traffic_shaping.burst_loss.loss_rate_during_burst,
                max_connections: 1000, // Default value
            };

            // Update chaos config if it exists, or create it
            // Chaos config is in observability.chaos, not core.chaos
            if let Some(ref mut chaos) = config.observability.chaos {
                chaos.traffic_shaping = Some(network_shaping);
            } else {
                // Create minimal chaos config with traffic shaping
                use mockforge_core::config::ChaosEngConfig;
                config.observability.chaos = Some(ChaosEngConfig {
                    enabled: true,
                    latency: None,
                    fault_injection: None,
                    rate_limit: None,
                    traffic_shaping: Some(network_shaping),
                    scenario: None,
                });
            }

            info!("Network profile '{}' applied to server configuration", request.profile_name);
        } else {
            warn!("Server configuration not available in ManagementState - profile applied but not persisted");
        }

        // Also update chaos API state if available
        #[cfg(feature = "chaos")]
        {
            if let Some(chaos_state) = &state.chaos_api_state {
                use mockforge_chaos::config::TrafficShapingConfig;

                let mut chaos_config = chaos_state.config.write().await;
                // Apply profile's traffic shaping to chaos API state
                let chaos_traffic_shaping = TrafficShapingConfig {
                    enabled: profile.traffic_shaping.bandwidth.enabled
                        || profile.traffic_shaping.burst_loss.enabled,
                    bandwidth_limit_bps: profile.traffic_shaping.bandwidth.max_bytes_per_sec * 8, // Convert bytes to bits
                    packet_loss_percent: profile.traffic_shaping.burst_loss.loss_rate_during_burst,
                    max_connections: 0,
                    connection_timeout_ms: 30000,
                };
                chaos_config.traffic_shaping = Some(chaos_traffic_shaping);
                chaos_config.enabled = true; // Enable chaos when applying a profile
                drop(chaos_config);
                info!("Network profile '{}' applied to chaos API state", request.profile_name);
            }
        }

        Json(serde_json::json!({
            "success": true,
            "message": format!("Network profile '{}' applied", request.profile_name),
            "profile": {
                "name": profile.name,
                "description": profile.description,
            }
        }))
        .into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Profile not found",
                "message": format!("Network profile '{}' not found", request.profile_name)
            })),
        )
            .into_response()
    }
}
