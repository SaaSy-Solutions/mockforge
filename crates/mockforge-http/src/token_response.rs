//! Token-based response resolution for HTTP handlers
//!
//! This module integrates the token resolver with HTTP response generation.

use axum::{
    body::Body,
    http::{Response, StatusCode},
    response::IntoResponse,
};
use mockforge_data::rag::RagConfig;
use mockforge_data::{resolve_tokens, resolve_tokens_with_rag};
use serde_json::Value;
use tracing::*;

/// Resolve tokens in a JSON response body
pub async fn resolve_response_tokens(body: Value) -> Result<Value, String> {
    resolve_tokens(&body)
        .await
        .map_err(|e| format!("Failed to resolve tokens: {}", e))
}

/// Resolve tokens in a JSON response body with RAG support
pub async fn resolve_response_tokens_with_rag(
    body: Value,
    rag_config: RagConfig,
) -> Result<Value, String> {
    resolve_tokens_with_rag(&body, rag_config)
        .await
        .map_err(|e| format!("Failed to resolve tokens with RAG: {}", e))
}

/// Create an HTTP response with token resolution
pub async fn create_token_resolved_response(
    status: StatusCode,
    body: Value,
    use_rag: bool,
    rag_config: Option<RagConfig>,
) -> Response<Body> {
    let resolved_body = if use_rag {
        let config = rag_config.unwrap_or_default();
        match resolve_response_tokens_with_rag(body.clone(), config).await {
            Ok(resolved) => resolved,
            Err(e) => {
                error!(error = %e, "Failed to resolve tokens with RAG, using original body");
                body
            }
        }
    } else {
        match resolve_response_tokens(body.clone()).await {
            Ok(resolved) => resolved,
            Err(e) => {
                error!(error = %e, "Failed to resolve tokens, using original body");
                body
            }
        }
    };

    let json_string = match serde_json::to_string_pretty(&resolved_body) {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "Failed to serialize response");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to serialize response")
                .into_response();
        }
    };

    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Body::from(json_string))
        .unwrap_or_else(|e| {
            error!(error = %e, "Failed to build response");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response").into_response()
        })
}

/// Token-resolved JSON response builder
pub struct TokenResolvedResponse {
    status: StatusCode,
    body: Value,
    use_rag: bool,
    rag_config: Option<RagConfig>,
}

impl TokenResolvedResponse {
    /// Create a new token-resolved response
    pub fn new(status: StatusCode, body: Value) -> Self {
        Self {
            status,
            body,
            use_rag: false,
            rag_config: None,
        }
    }

    /// Enable RAG-based token resolution
    pub fn with_rag(mut self, config: RagConfig) -> Self {
        self.use_rag = true;
        self.rag_config = Some(config);
        self
    }

    /// Build the response
    pub async fn build(self) -> Response<Body> {
        create_token_resolved_response(self.status, self.body, self.use_rag, self.rag_config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_data::rag::LlmProvider;
    use serde_json::json;

    // ==================== Basic Token Resolution Tests ====================

    #[tokio::test]
    async fn test_resolve_response_tokens() {
        let body = json!({
            "id": "$random.uuid",
            "name": "$faker.name",
            "email": "$faker.email"
        });

        let result = resolve_response_tokens(body).await;
        assert!(result.is_ok());

        let resolved = result.unwrap();
        assert!(resolved["id"].is_string());
        assert!(resolved["name"].is_string());
        assert!(resolved["email"].is_string());
    }

    #[tokio::test]
    async fn test_resolve_nested_tokens() {
        let body = json!({
            "user": {
                "id": "$random.uuid",
                "profile": {
                    "name": "$faker.name",
                    "contact": {
                        "email": "$faker.email",
                        "phone": "$faker.phone"
                    }
                }
            }
        });

        let result = resolve_response_tokens(body).await;
        assert!(result.is_ok());

        let resolved = result.unwrap();
        assert!(resolved["user"]["id"].is_string());
        assert!(resolved["user"]["profile"]["name"].is_string());
        assert!(resolved["user"]["profile"]["contact"]["email"].is_string());
    }

    #[tokio::test]
    async fn test_resolve_array_tokens() {
        let body = json!({
            "users": [
                {"id": "$random.uuid", "name": "$faker.name"},
                {"id": "$random.uuid", "name": "$faker.name"}
            ]
        });

        let result = resolve_response_tokens(body).await;
        assert!(result.is_ok());

        let resolved = result.unwrap();
        let users = resolved["users"].as_array().unwrap();
        assert_eq!(users.len(), 2);
        assert!(users[0]["id"].is_string());
        assert!(users[0]["name"].is_string());
    }

    #[tokio::test]
    async fn test_resolve_static_values() {
        let body = json!({
            "message": "Hello, World!",
            "count": 42,
            "active": true
        });

        let result = resolve_response_tokens(body.clone()).await;
        assert!(result.is_ok());

        let resolved = result.unwrap();
        assert_eq!(resolved["message"], "Hello, World!");
        assert_eq!(resolved["count"], 42);
        assert_eq!(resolved["active"], true);
    }

    #[tokio::test]
    async fn test_resolve_mixed_tokens_and_static() {
        let body = json!({
            "id": "$random.uuid",
            "message": "Static message",
            "count": 100
        });

        let result = resolve_response_tokens(body).await;
        assert!(result.is_ok());

        let resolved = result.unwrap();
        assert!(resolved["id"].is_string());
        assert_eq!(resolved["message"], "Static message");
        assert_eq!(resolved["count"], 100);
    }

    #[tokio::test]
    async fn test_resolve_empty_object() {
        let body = json!({});
        let result = resolve_response_tokens(body).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!({}));
    }

    #[tokio::test]
    async fn test_resolve_null_value() {
        let body = json!(null);
        let result = resolve_response_tokens(body).await;
        assert!(result.is_ok());
    }

    // ==================== TokenResolvedResponse Builder Tests ====================

    #[tokio::test]
    async fn test_token_resolved_response_builder() {
        let body = json!({"message": "test"});
        let response = TokenResolvedResponse::new(StatusCode::OK, body).build().await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_token_resolved_response_created() {
        let body = json!({"id": "123", "created": true});
        let response = TokenResolvedResponse::new(StatusCode::CREATED, body).build().await;

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_token_resolved_response_not_found() {
        let body = json!({"error": "Resource not found"});
        let response = TokenResolvedResponse::new(StatusCode::NOT_FOUND, body).build().await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_token_resolved_response_bad_request() {
        let body = json!({"error": "Invalid input", "field": "email"});
        let response = TokenResolvedResponse::new(StatusCode::BAD_REQUEST, body).build().await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_token_resolved_response_with_rag_config() {
        let body = json!({"message": "test"});
        let rag_config = RagConfig {
            provider: LlmProvider::Ollama,
            api_key: None,
            model: "llama2".to_string(),
            api_endpoint: "http://localhost:11434/api/generate".to_string(),
            ..Default::default()
        };

        let builder = TokenResolvedResponse::new(StatusCode::OK, body).with_rag(rag_config);
        assert!(builder.use_rag);
        assert!(builder.rag_config.is_some());
    }

    #[test]
    fn test_token_resolved_response_new_defaults() {
        let body = json!({"test": "value"});
        let response = TokenResolvedResponse::new(StatusCode::OK, body);

        assert_eq!(response.status, StatusCode::OK);
        assert!(!response.use_rag);
        assert!(response.rag_config.is_none());
    }

    // ==================== create_token_resolved_response Tests ====================

    #[tokio::test]
    async fn test_create_response_ok() {
        let body = json!({"status": "success"});
        let response = create_token_resolved_response(StatusCode::OK, body, false, None).await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_response_with_tokens() {
        let body = json!({
            "id": "$random.uuid",
            "timestamp": "$now"
        });
        let response = create_token_resolved_response(StatusCode::OK, body, false, None).await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_response_rag_disabled() {
        let body = json!({"message": "test"});
        let response = create_token_resolved_response(StatusCode::OK, body, false, None).await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_response_rag_enabled_no_config() {
        // RAG enabled but no config provided - should use defaults
        let body = json!({"message": "test"});
        let response = create_token_resolved_response(StatusCode::OK, body, true, None).await;

        // Should still succeed even if RAG fails (fallback to original body)
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_response_content_type_json() {
        let body = json!({"data": "test"});
        let response = create_token_resolved_response(StatusCode::OK, body, false, None).await;

        let content_type = response.headers().get("Content-Type").and_then(|v| v.to_str().ok());
        assert_eq!(content_type, Some("application/json"));
    }

    // ==================== RAG Config Tests ====================

    #[test]
    fn test_rag_config_default() {
        let config = RagConfig::default();
        assert!(config.temperature >= 0.0);
        assert!(config.max_tokens > 0);
    }

    #[test]
    fn test_rag_config_with_provider() {
        let config = RagConfig {
            provider: LlmProvider::OpenAI,
            api_key: Some("test-key".to_string()),
            model: "gpt-4".to_string(),
            api_endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
            ..Default::default()
        };

        assert!(matches!(config.provider, LlmProvider::OpenAI));
        assert_eq!(config.api_key, Some("test-key".to_string()));
    }

    // ==================== Edge Cases ====================

    #[tokio::test]
    async fn test_resolve_deeply_nested_array() {
        let body = json!({
            "data": {
                "items": [
                    {"ids": ["$random.uuid", "$random.uuid"]},
                    {"ids": ["$random.uuid"]}
                ]
            }
        });

        let result = resolve_response_tokens(body).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_resolve_complex_structure() {
        let body = json!({
            "meta": {
                "total": 100,
                "page": 1
            },
            "data": [
                {
                    "id": "$random.uuid",
                    "attributes": {
                        "name": "$faker.name",
                        "created_at": "$now"
                    },
                    "relationships": {
                        "author": {
                            "id": "$random.uuid"
                        }
                    }
                }
            ]
        });

        let result = resolve_response_tokens(body).await;
        assert!(result.is_ok());
    }
}
