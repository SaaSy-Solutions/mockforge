//! Migration pipeline handlers
//!
//! Handlers for managing the mock-to-real migration pipeline,
//! including route toggling, group management, and status reporting.
//!
//! These handlers proxy requests to the HTTP server's management API.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::handlers::AdminState;
use crate::models::ApiResponse;

// Use percent encoding for URL path segments
// Note: We encode patterns that may contain special characters like / and *
fn encode_path_segment(s: &str) -> String {
    // Simple encoding for common special characters in URL paths
    s.replace('/', "%2F")
        .replace('*', "%2A")
        .replace('?', "%3F")
        .replace('#', "%23")
        .replace('[', "%5B")
        .replace(']', "%5D")
}

/// Helper function to proxy requests to the HTTP server's management API
async fn proxy_to_http_server(
    state: &AdminState,
    path: &str,
    body: Option<Value>,
    method: &str,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    let http_addr = match state.http_server_addr {
        Some(addr) => addr,
        None => {
            return Ok(Json(ApiResponse::error("HTTP server not available".to_string())));
        }
    };

    let url = format!("http://{}/__mockforge/api{}", http_addr, path);

    let client = reqwest::Client::new();
    let mut request = match (method, body.is_some()) {
        ("PUT", _) => client.put(&url),
        ("POST", _) => client.post(&url),
        _ => client.get(&url),
    };

    if let Some(body_value) = body {
        request = request.json(&body_value);
    }

    match request.send().await {
        Ok(response) => {
            let status = response.status();
            match response.json::<Value>().await {
                Ok(data) => {
                    if status.is_success() {
                        Ok(Json(ApiResponse::success(data)))
                    } else {
                        Ok(Json(ApiResponse::error(
                            data.get("error")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Request failed")
                                .to_string(),
                        )))
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to parse response: {}", e);
                    Ok(Json(ApiResponse::error(format!("Failed to parse response: {}", e))))
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to proxy request: {}", e);
            Ok(Json(ApiResponse::error(format!("Failed to connect to HTTP server: {}", e))))
        }
    }
}

/// Request to set migration mode for a route
#[derive(Debug, Deserialize)]
pub struct SetRouteMigrationRequest {
    /// Migration mode: mock, shadow, real, or auto
    pub mode: String,
}

/// Request to set migration mode for a group
#[derive(Debug, Deserialize)]
pub struct SetGroupMigrationRequest {
    /// Migration mode: mock, shadow, real, or auto
    pub mode: String,
}

/// Migration status response
#[derive(Debug, Serialize)]
pub struct MigrationStatus {
    /// Total number of routes
    pub total_routes: usize,
    /// Number of routes in mock mode
    pub mock_routes: usize,
    /// Number of routes in shadow mode
    pub shadow_routes: usize,
    /// Number of routes in real mode
    pub real_routes: usize,
    /// Number of routes in auto mode
    pub auto_routes: usize,
    /// Total number of groups
    pub total_groups: usize,
    /// Migration enabled flag
    pub migration_enabled: bool,
}

/// Get all migration routes with their status
pub async fn get_migration_routes(
    State(state): State<AdminState>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    proxy_to_http_server(&state, "/migration/routes", None, "GET").await
}

/// Toggle a route's migration mode through stages: mock → shadow → real → mock
pub async fn toggle_route_migration(
    State(state): State<AdminState>,
    Path(pattern): Path<String>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    let encoded_pattern = encode_path_segment(&pattern);
    proxy_to_http_server(
        &state,
        &format!("/migration/routes/{}/toggle", encoded_pattern),
        None,
        "POST",
    )
    .await
}

/// Set a route's migration mode explicitly
pub async fn set_route_migration_mode(
    State(state): State<AdminState>,
    Path(pattern): Path<String>,
    Json(request): Json<SetRouteMigrationRequest>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    let encoded_pattern = encode_path_segment(&pattern);
    let body = json!({ "mode": request.mode });
    proxy_to_http_server(
        &state,
        &format!("/migration/routes/{}", encoded_pattern),
        Some(body),
        "PUT",
    )
    .await
}

/// Toggle a group's migration mode through stages: mock → shadow → real → mock
pub async fn toggle_group_migration(
    State(state): State<AdminState>,
    Path(group): Path<String>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    let encoded_group = encode_path_segment(&group);
    proxy_to_http_server(
        &state,
        &format!("/migration/groups/{}/toggle", encoded_group),
        None,
        "POST",
    )
    .await
}

/// Set a group's migration mode explicitly
pub async fn set_group_migration_mode(
    State(state): State<AdminState>,
    Path(group): Path<String>,
    Json(request): Json<SetGroupMigrationRequest>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    let encoded_group = encode_path_segment(&group);
    let body = json!({ "mode": request.mode });
    proxy_to_http_server(&state, &format!("/migration/groups/{}", encoded_group), Some(body), "PUT")
        .await
}

/// Get all migration groups with their status
pub async fn get_migration_groups(
    State(state): State<AdminState>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    proxy_to_http_server(&state, "/migration/groups", None, "GET").await
}

/// Get overall migration status
pub async fn get_migration_status(
    State(state): State<AdminState>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    proxy_to_http_server(&state, "/migration/status", None, "GET").await
}
