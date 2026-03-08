//! Chaos engineering API handlers for the admin UI
//!
//! These handlers provide direct access to the chaos API state for the TUI dashboard.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;

use super::AdminState;

/// Get chaos engineering status
pub async fn get_chaos_status(State(state): State<AdminState>) -> impl IntoResponse {
    let data = match &state.chaos_api_state {
        Some(chaos) => {
            let config = chaos.config.read().await;
            let active_scenarios = chaos.scenario_engine.get_active_scenarios();
            json!({
                "enabled": config.enabled,
                "active_scenario": active_scenarios.first().map(|s| &s.name),
                "active_scenario_count": active_scenarios.len(),
                "settings": {
                    "latency": config.latency.is_some(),
                    "fault_injection": config.fault_injection.is_some(),
                }
            })
        }
        None => {
            json!({
                "enabled": false,
                "active_scenario": null,
                "active_scenario_count": 0,
                "settings": {}
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

#[derive(Deserialize)]
pub struct ChaosToggleRequest {
    pub enabled: bool,
}

/// Toggle chaos engineering on/off
pub async fn toggle_chaos(
    State(state): State<AdminState>,
    Json(body): Json<ChaosToggleRequest>,
) -> impl IntoResponse {
    match &state.chaos_api_state {
        Some(chaos) => {
            let mut config = chaos.config.write().await;
            config.enabled = body.enabled;
            Json(json!({
                "success": true,
                "data": { "enabled": config.enabled },
                "error": null,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }
        None => Json(json!({
            "success": false,
            "data": null,
            "error": "Chaos engineering not configured",
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    }
}

/// Get predefined chaos scenarios
pub async fn get_chaos_scenarios_predefined(State(state): State<AdminState>) -> impl IntoResponse {
    let scenarios = match &state.chaos_api_state {
        Some(_) => {
            json!([
                { "name": "network-partition", "description": "Simulates network partition between services", "severity": "high" },
                { "name": "latency-spike", "description": "Injects random latency spikes", "severity": "medium" },
                { "name": "error-storm", "description": "Returns 500 errors at high rate", "severity": "high" },
                { "name": "slow-drain", "description": "Gradually increases response times", "severity": "low" },
                { "name": "cpu-stress", "description": "Simulates CPU pressure", "severity": "medium" },
                { "name": "memory-pressure", "description": "Simulates memory pressure", "severity": "medium" }
            ])
        }
        None => json!([]),
    };

    Json(json!({
        "success": true,
        "data": scenarios,
        "error": null,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Start a chaos scenario by name
pub async fn start_chaos_scenario(
    State(state): State<AdminState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match &state.chaos_api_state {
        Some(chaos) => {
            let scenario = mockforge_chaos::scenarios::ChaosScenario::new(
                name.clone(),
                mockforge_chaos::ChaosConfig::default(),
            );
            chaos.scenario_engine.start_scenario(scenario);
            // Also enable chaos if not already
            let mut config = chaos.config.write().await;
            config.enabled = true;
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": { "scenario": name, "status": "started" },
                    "error": null,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })),
            )
        }
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "success": false,
                "data": null,
                "error": "Chaos engineering not configured",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        ),
    }
}

/// Stop a chaos scenario by name
pub async fn stop_chaos_scenario(
    State(state): State<AdminState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match &state.chaos_api_state {
        Some(chaos) => {
            let stopped = chaos.scenario_engine.stop_scenario(&name);
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": { "scenario": name, "status": if stopped { "stopped" } else { "not_found" } },
                    "error": null,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })),
            )
        }
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "success": false,
                "data": null,
                "error": "Chaos engineering not configured",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        ),
    }
}
