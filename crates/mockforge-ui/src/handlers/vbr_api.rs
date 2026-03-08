//! VBR (Virtual Backend Reality) API handlers for the admin UI
//!
//! These handlers provide access to the VBR engine state for the TUI dashboard.

use axum::{extract::State, response::IntoResponse, Json};
use serde_json::json;

use super::AdminState;

/// Get VBR engine status
pub async fn get_vbr_status(State(state): State<AdminState>) -> impl IntoResponse {
    let data = match &state.vbr_engine {
        Some(engine) => {
            let summary = engine.state_summary().await;
            let entity_count = engine.registry().list().len();
            json!({
                "enabled": true,
                "model_count": entity_count,
                "training_status": "idle",
                "accuracy": null,
                "summary": summary
            })
        }
        None => {
            json!({
                "enabled": false,
                "model_count": 0,
                "training_status": "idle",
                "accuracy": null,
                "summary": null
            })
        }
    };

    Json(json!({
        "success": true,
        "data": data,
        "error": null,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
