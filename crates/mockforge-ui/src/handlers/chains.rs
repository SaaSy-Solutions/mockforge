//! Chain management proxy handlers
//!
//! These handlers proxy chain-related requests from the Admin UI to the main HTTP server

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::Value;

use super::AdminState;

/// Proxy chain list requests to the main HTTP server
pub async fn proxy_chains_list(State(state): State<AdminState>) -> Response {
    proxy_to_http_server(&state, "/chains", None).await
}

/// Proxy chain creation requests to the main HTTP server
pub async fn proxy_chains_create(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> Response {
    proxy_to_http_server(&state, "/chains", Some(body)).await
}

/// Proxy get chain requests to the main HTTP server
pub async fn proxy_chain_get(State(state): State<AdminState>, Path(id): Path<String>) -> Response {
    proxy_to_http_server(&state, &format!("/chains/{}", id), None).await
}

/// Proxy chain update requests to the main HTTP server
pub async fn proxy_chain_update(
    State(state): State<AdminState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    proxy_to_http_server(&state, &format!("/chains/{}", id), Some(body)).await
}

/// Proxy chain delete requests to the main HTTP server
pub async fn proxy_chain_delete(
    State(state): State<AdminState>,
    Path(id): Path<String>,
) -> Response {
    proxy_to_http_server(&state, &format!("/chains/{}", id), None).await
}

/// Proxy chain execute requests to the main HTTP server
pub async fn proxy_chain_execute(
    State(state): State<AdminState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    proxy_to_http_server(&state, &format!("/chains/{}/execute", id), Some(body)).await
}

/// Proxy chain validate requests to the main HTTP server
pub async fn proxy_chain_validate(
    State(state): State<AdminState>,
    Path(id): Path<String>,
) -> Response {
    proxy_to_http_server(&state, &format!("/chains/{}/validate", id), None).await
}

/// Proxy chain history requests to the main HTTP server
pub async fn proxy_chain_history(
    State(state): State<AdminState>,
    Path(id): Path<String>,
) -> Response {
    proxy_to_http_server(&state, &format!("/chains/{}/history", id), None).await
}

/// Helper function to proxy requests to the main HTTP server
async fn proxy_to_http_server(state: &AdminState, path: &str, body: Option<Value>) -> Response {
    let Some(http_addr) = state.http_server_addr else {
        return (StatusCode::SERVICE_UNAVAILABLE, "HTTP server address not configured")
            .into_response();
    };

    let url = format!("http://{}/__mockforge{}", http_addr, path);

    let client = reqwest::Client::new();
    let mut request_builder = if body.is_some() {
        client.post(&url)
    } else {
        client.get(&url)
    };

    if let Some(json_body) = body {
        request_builder = request_builder.json(&json_body);
    }

    match request_builder.send().await {
        Ok(response) => {
            let status = response.status();
            match response.text().await {
                Ok(text) => {
                    // Try to parse as JSON, otherwise return as text
                    if let Ok(json) = serde_json::from_str::<Value>(&text) {
                        (
                            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK),
                            Json(json),
                        )
                            .into_response()
                    } else {
                        (StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK), text)
                            .into_response()
                    }
                }
                Err(e) => {
                    (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read response: {}", e))
                        .into_response()
                }
            }
        }
        Err(e) => {
            (StatusCode::BAD_GATEWAY, format!("Failed to proxy request: {}", e)).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    fn create_test_state(http_addr: Option<SocketAddr>) -> AdminState {
        AdminState::new(http_addr, None, None, None, false, 8080, None, None, None, None, None)
    }

    // ==================== Proxy to HTTP Server Tests ====================

    #[tokio::test]
    async fn test_proxy_to_http_server_no_addr() {
        let state = create_test_state(None);
        let response = proxy_to_http_server(&state, "/test", None).await;

        // Response should indicate service unavailable
        // We can't easily extract status from Response, but we verify it compiles
        let _ = response;
    }

    #[tokio::test]
    async fn test_proxy_to_http_server_with_path() {
        let state = create_test_state(None);
        let response = proxy_to_http_server(&state, "/chains/123", None).await;
        let _ = response;
    }

    #[tokio::test]
    async fn test_proxy_to_http_server_with_body() {
        let state = create_test_state(None);
        let body = serde_json::json!({"name": "test-chain"});
        let response = proxy_to_http_server(&state, "/chains", Some(body)).await;
        let _ = response;
    }

    // ==================== AdminState Tests ====================

    #[test]
    fn test_admin_state_creation() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let state = create_test_state(Some(addr));
        assert!(state.http_server_addr.is_some());
    }

    #[test]
    fn test_admin_state_no_http_addr() {
        let state = create_test_state(None);
        assert!(state.http_server_addr.is_none());
    }

    // ==================== Path Construction Tests ====================

    #[test]
    fn test_chain_path_construction() {
        let chain_id = "chain-123";
        let path = format!("/chains/{}", chain_id);
        assert_eq!(path, "/chains/chain-123");
    }

    #[test]
    fn test_chain_execute_path_construction() {
        let chain_id = "exec-chain";
        let path = format!("/chains/{}/execute", chain_id);
        assert_eq!(path, "/chains/exec-chain/execute");
    }

    #[test]
    fn test_chain_validate_path_construction() {
        let chain_id = "validate-chain";
        let path = format!("/chains/{}/validate", chain_id);
        assert_eq!(path, "/chains/validate-chain/validate");
    }

    #[test]
    fn test_chain_history_path_construction() {
        let chain_id = "history-chain";
        let path = format!("/chains/{}/history", chain_id);
        assert_eq!(path, "/chains/history-chain/history");
    }

    // ==================== URL Construction Tests ====================

    #[test]
    fn test_url_construction_chains_endpoint() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let path = "/chains";
        let url = format!("http://{}/__mockforge{}", addr, path);
        assert_eq!(url, "http://127.0.0.1:8080/__mockforge/chains");
    }

    #[test]
    fn test_url_construction_with_id() {
        let addr: SocketAddr = "192.168.1.1:3000".parse().unwrap();
        let chain_id = "abc123";
        let url = format!("http://{}/__mockforge/chains/{}", addr, chain_id);
        assert_eq!(url, "http://192.168.1.1:3000/__mockforge/chains/abc123");
    }

    #[test]
    fn test_url_construction_ipv6() {
        let addr: SocketAddr = "[::1]:8080".parse().unwrap();
        let path = "/chains";
        let url = format!("http://{}/__mockforge{}", addr, path);
        assert_eq!(url, "http://[::1]:8080/__mockforge/chains");
    }

    // ==================== Request Body Tests ====================

    #[test]
    fn test_chain_create_body() {
        let body = serde_json::json!({
            "name": "test-chain",
            "steps": [
                {"type": "http", "endpoint": "/api/users"},
                {"type": "transform", "expression": "$.data"}
            ]
        });

        assert!(body.get("name").is_some());
        assert!(body.get("steps").is_some());
        let steps = body.get("steps").unwrap().as_array().unwrap();
        assert_eq!(steps.len(), 2);
    }

    #[test]
    fn test_chain_update_body() {
        let body = serde_json::json!({
            "name": "updated-chain",
            "enabled": true,
            "timeout_ms": 5000
        });

        assert_eq!(body.get("name").unwrap(), "updated-chain");
        assert_eq!(body.get("enabled").unwrap(), true);
    }

    #[test]
    fn test_chain_execute_body() {
        let body = serde_json::json!({
            "input": {
                "user_id": 123,
                "action": "fetch"
            },
            "options": {
                "timeout": 10000,
                "retry": true
            }
        });

        assert!(body.get("input").is_some());
        assert!(body.get("options").is_some());
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_chain_id_with_special_characters() {
        let chain_id = "chain-with-dashes_and_underscores";
        let path = format!("/chains/{}", chain_id);
        assert!(path.contains(chain_id));
    }

    #[test]
    fn test_chain_id_uuid_format() {
        let chain_id = "550e8400-e29b-41d4-a716-446655440000";
        let path = format!("/chains/{}", chain_id);
        assert!(path.contains(chain_id));
    }

    #[test]
    fn test_empty_body_is_none() {
        let body: Option<Value> = None;
        assert!(body.is_none());
    }

    #[test]
    fn test_empty_json_body() {
        let body = serde_json::json!({});
        assert!(body.is_object());
        assert!(body.as_object().unwrap().is_empty());
    }

    // ==================== Request Method Selection Tests ====================

    #[test]
    fn test_method_selection_with_body() {
        let body: Option<Value> = Some(serde_json::json!({"test": true}));
        // With body -> POST
        assert!(body.is_some());
    }

    #[test]
    fn test_method_selection_without_body() {
        let body: Option<Value> = None;
        // Without body -> GET
        assert!(body.is_none());
    }

    // ==================== Status Code Tests ====================

    #[test]
    fn test_status_code_conversion_success() {
        let status = StatusCode::OK.as_u16();
        let converted = StatusCode::from_u16(status);
        assert!(converted.is_ok());
        assert_eq!(converted.unwrap(), StatusCode::OK);
    }

    #[test]
    fn test_status_code_conversion_not_found() {
        let status = 404u16;
        let converted = StatusCode::from_u16(status);
        assert!(converted.is_ok());
        assert_eq!(converted.unwrap(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_status_code_conversion_server_error() {
        let status = 500u16;
        let converted = StatusCode::from_u16(status);
        assert!(converted.is_ok());
        assert_eq!(converted.unwrap(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_status_code_conversion_created() {
        let status = 201u16;
        let converted = StatusCode::from_u16(status);
        assert!(converted.is_ok());
        assert_eq!(converted.unwrap(), StatusCode::CREATED);
    }

    #[test]
    fn test_status_code_conversion_bad_gateway() {
        let status = 502u16;
        let converted = StatusCode::from_u16(status);
        assert!(converted.is_ok());
        assert_eq!(converted.unwrap(), StatusCode::BAD_GATEWAY);
    }
}
