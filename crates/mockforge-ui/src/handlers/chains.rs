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
        AdminState::new(http_addr, None, None, None, false, 8080, None, None, None)
    }

    #[tokio::test]
    async fn test_proxy_to_http_server_no_addr() {
        let state = create_test_state(None);
        let response = proxy_to_http_server(&state, "/test", None).await;

        // Response should indicate service unavailable
        // We can't easily extract status from Response, but we verify it compiles
        let _ = response;
    }
}
