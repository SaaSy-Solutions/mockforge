//! Federation API handlers for the admin UI
//!
//! These handlers provide access to the federation state for the TUI dashboard.

use axum::{extract::State, response::IntoResponse, Json};
use serde_json::json;

use super::AdminState;

/// Get federation peers.
///
/// The admin runtime does not yet track per-service sync/health state, so
/// `status` is returned as `"unknown"` and `last_sync` as `null` rather than
/// fabricating "connected"/`now()` values. The TUI's `FederationPeer` struct
/// tolerates these defaults. When real peer-health signals land, populate
/// these fields from the federation runtime instead of the current snapshot.
pub async fn get_federation_peers(State(state): State<AdminState>) -> impl IntoResponse {
    let peers = match &state.federation {
        Some(federation) => federation
            .services
            .iter()
            .map(|service| {
                json!({
                    "id": service.workspace_id.to_string(),
                    "url": service.base_path,
                    "status": "unknown",
                    "last_sync": serde_json::Value::Null,
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
