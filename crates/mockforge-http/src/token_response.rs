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
    use serde_json::json;

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
    async fn test_token_resolved_response_builder() {
        let body = json!({"message": "test"});
        let response = TokenResolvedResponse::new(StatusCode::OK, body).build().await;

        assert_eq!(response.status(), StatusCode::OK);
    }
}
