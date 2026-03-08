//! World-state proxy handler
//!
//! Proxies world-state requests to the main HTTP server's `/api/world-state/snapshot` endpoint.

use axum::{extract::State, response::IntoResponse, Json};
use serde_json::json;

use super::AdminState;

/// Get world state by proxying to the HTTP server
pub async fn get_world_state(State(state): State<AdminState>) -> impl IntoResponse {
    let Some(http_addr) = state.http_server_addr else {
        // Return empty world state when HTTP server is not configured
        return Json(json!({
            "success": true,
            "data": { "nodes": [], "edges": [], "layers": [] },
            "error": null,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }));
    };

    let url = format!("http://{}/api/world-state/snapshot", http_addr);
    let client = reqwest::Client::new();

    match client.get(&url).timeout(std::time::Duration::from_secs(5)).send().await {
        Ok(response) => match response.json::<serde_json::Value>().await {
            Ok(data) => Json(json!({
                "success": true,
                "data": data,
                "error": null,
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
            Err(_) => Json(json!({
                "success": true,
                "data": { "nodes": [], "edges": [], "layers": [] },
                "error": null,
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        },
        Err(_) => {
            // HTTP server unreachable — return empty defaults, not an error
            Json(json!({
                "success": true,
                "data": { "nodes": [], "edges": [], "layers": [] },
                "error": null,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }
    }
}
