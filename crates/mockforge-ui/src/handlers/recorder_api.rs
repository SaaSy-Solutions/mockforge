//! Recorder API handlers for the admin UI
//!
//! These handlers provide access to the traffic recorder state for the TUI dashboard.

use axum::{extract::State, response::IntoResponse, Json};
use serde_json::json;

use super::AdminState;

/// Get recorder status
pub async fn get_recorder_status(State(state): State<AdminState>) -> impl IntoResponse {
    let data = match &state.recorder {
        Some(recorder) => {
            let recording = recorder.is_enabled().await;
            json!({
                "recording": recording,
                "recorded_count": 0
            })
        }
        None => {
            json!({
                "recording": false,
                "recorded_count": 0
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

/// Start recording
pub async fn start_recorder(State(state): State<AdminState>) -> impl IntoResponse {
    match &state.recorder {
        Some(recorder) => {
            recorder.enable().await;
            Json(json!({
                "success": true,
                "data": { "recording": true },
                "error": null,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }
        None => Json(json!({
            "success": false,
            "data": null,
            "error": "Recorder not configured",
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    }
}

/// Stop recording
pub async fn stop_recorder(State(state): State<AdminState>) -> impl IntoResponse {
    match &state.recorder {
        Some(recorder) => {
            recorder.disable().await;
            Json(json!({
                "success": true,
                "data": { "recording": false },
                "error": null,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }
        None => Json(json!({
            "success": false,
            "data": null,
            "error": "Recorder not configured",
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    }
}
