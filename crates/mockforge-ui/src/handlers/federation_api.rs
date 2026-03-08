//! Federation API handlers for the admin UI
//!
//! These handlers provide access to the federation state for the TUI dashboard.

use axum::{extract::State, response::IntoResponse, Json};
use serde_json::json;

use super::AdminState;

/// Get federation peers
pub async fn get_federation_peers(State(state): State<AdminState>) -> impl IntoResponse {
    let peers = match &state.federation {
        Some(federation) => federation
            .services
            .iter()
            .map(|service| {
                json!({
                    "id": service.workspace_id.to_string(),
                    "url": service.base_path,
                    "status": "connected",
                    "last_sync": chrono::Utc::now().to_rfc3339(),
                    "name": service.name,
                    "reality_level": service.reality_level.as_str()
                })
            })
            .collect::<Vec<_>>(),
        None => vec![],
    };

    Json(json!({
        "success": true,
        "data": peers,
        "error": null,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
