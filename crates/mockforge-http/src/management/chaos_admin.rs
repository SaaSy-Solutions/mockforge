//! Chaos engineering + network profile admin endpoints
//! (`GET|POST /__mockforge/chaos/config`,
//! `GET /__mockforge/network/profiles`,
//! `POST /__mockforge/network/profile/apply`).
//!
//! Split out of the original `management/ai_gen.rs` under #656 — these
//! handlers were never AI-related, they just happened to live in the
//! same file. They read and write `ManagementState.chaos_api_state` and
//! `ManagementState.server_config`.
//!
//! Stays in `mockforge-http`: chaos-coupled handlers are explicitly
//! out of scope for the #555 / #656 drain — `mockforge-chaos` would
//! need its own extraction bucket before these could move anywhere.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use tracing::*;

use super::ManagementState;

/// Get current chaos engineering configuration
pub(crate) async fn get_chaos_config(State(_state): State<ManagementState>) -> impl IntoResponse {
    #[cfg(feature = "chaos")]
    {
        if let Some(chaos_state) = &_state.chaos_api_state {
            let config = chaos_state.config.read().await;
            // Convert ChaosConfig to JSON response format
            Json(serde_json::json!({
                "enabled": config.enabled,
                "latency": config.latency.as_ref().map(|l| serde_json::to_value(l).unwrap_or(serde_json::Value::Null)),
                "fault_injection": config.fault_injection.as_ref().map(|f| serde_json::to_value(f).unwrap_or(serde_json::Value::Null)),
                "rate_limit": config.rate_limit.as_ref().map(|r| serde_json::to_value(r).unwrap_or(serde_json::Value::Null)),
                "traffic_shaping": config.traffic_shaping.as_ref().map(|t| serde_json::to_value(t).unwrap_or(serde_json::Value::Null)),
            }))
            .into_response()
        } else {
            // Chaos API not available, return default
            Json(serde_json::json!({
                "enabled": false,
                "latency": null,
                "fault_injection": null,
                "rate_limit": null,
                "traffic_shaping": null,
            }))
            .into_response()
        }
    }
    #[cfg(not(feature = "chaos"))]
    {
        // Chaos feature not enabled
        Json(serde_json::json!({
            "enabled": false,
            "latency": null,
            "fault_injection": null,
            "rate_limit": null,
            "traffic_shaping": null,
        }))
        .into_response()
    }
}

/// Request to update chaos configuration
#[derive(Debug, Deserialize)]
pub struct ChaosConfigUpdate {
    /// Whether to enable chaos engineering
    pub enabled: Option<bool>,
    /// Latency configuration
    pub latency: Option<serde_json::Value>,
    /// Fault injection configuration
    pub fault_injection: Option<serde_json::Value>,
    /// Rate limiting configuration
    pub rate_limit: Option<serde_json::Value>,
    /// Traffic shaping configuration
    pub traffic_shaping: Option<serde_json::Value>,
}

/// Update chaos engineering configuration
pub(crate) async fn update_chaos_config(
    State(_state): State<ManagementState>,
    Json(_config_update): Json<ChaosConfigUpdate>,
) -> impl IntoResponse {
    #[cfg(feature = "chaos")]
    {
        if let Some(chaos_state) = &_state.chaos_api_state {
            use mockforge_chaos::config::{
                FaultInjectionConfig, LatencyConfig, RateLimitConfig, TrafficShapingConfig,
            };

            let mut config = chaos_state.config.write().await;

            // Update enabled flag if provided
            if let Some(enabled) = _config_update.enabled {
                config.enabled = enabled;
            }

            // Update latency config if provided
            if let Some(latency_json) = _config_update.latency {
                if let Ok(latency) = serde_json::from_value::<LatencyConfig>(latency_json) {
                    config.latency = Some(latency);
                }
            }

            // Update fault injection config if provided
            if let Some(fault_json) = _config_update.fault_injection {
                if let Ok(fault) = serde_json::from_value::<FaultInjectionConfig>(fault_json) {
                    config.fault_injection = Some(fault);
                }
            }

            // Update rate limit config if provided
            if let Some(rate_json) = _config_update.rate_limit {
                if let Ok(rate) = serde_json::from_value::<RateLimitConfig>(rate_json) {
                    config.rate_limit = Some(rate);
                }
            }

            // Update traffic shaping config if provided
            if let Some(traffic_json) = _config_update.traffic_shaping {
                if let Ok(traffic) = serde_json::from_value::<TrafficShapingConfig>(traffic_json) {
                    config.traffic_shaping = Some(traffic);
                }
            }

            // Reinitialize middleware injectors with new config
            // The middleware will pick up the changes on the next request
            drop(config);

            info!("Chaos configuration updated successfully");
            Json(serde_json::json!({
                "success": true,
                "message": "Chaos configuration updated and applied"
            }))
            .into_response()
        } else {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "success": false,
                    "error": "Chaos API not available",
                    "message": "Chaos engineering is not enabled or configured"
                })),
            )
                .into_response()
        }
    }
    #[cfg(not(feature = "chaos"))]
    {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({
                "success": false,
                "error": "Chaos feature not enabled",
                "message": "Chaos engineering feature is not compiled into this build"
            })),
        )
            .into_response()
    }
}

/// List available network profiles
pub(crate) async fn list_network_profiles() -> impl IntoResponse {
    use mockforge_chaos::core_network_profiles::NetworkProfileCatalog;

    let catalog = NetworkProfileCatalog::default();
    let profiles: Vec<serde_json::Value> = catalog
        .list_profiles_with_description()
        .iter()
        .map(|(name, description)| {
            serde_json::json!({
                "name": name,
                "description": description,
            })
        })
        .collect();

    Json(serde_json::json!({
        "profiles": profiles
    }))
    .into_response()
}

#[derive(Debug, Deserialize)]
/// Request to apply a network profile
pub struct ApplyNetworkProfileRequest {
    /// Name of the network profile to apply
    pub profile_name: String,
}

/// Apply a network profile
pub(crate) async fn apply_network_profile(
    State(state): State<ManagementState>,
    Json(request): Json<ApplyNetworkProfileRequest>,
) -> impl IntoResponse {
    use mockforge_chaos::core_network_profiles::NetworkProfileCatalog;

    let catalog = NetworkProfileCatalog::default();
    if let Some(profile) = catalog.get(&request.profile_name) {
        // Apply profile to server configuration if available
        // NetworkProfile contains latency and traffic_shaping configs
        if let Some(server_config) = &state.server_config {
            let mut config = server_config.write().await;

            // Apply network profile's traffic shaping to core config
            use mockforge_core::config::NetworkShapingConfig;

            // Convert NetworkProfile's TrafficShapingConfig to NetworkShapingConfig
            // NetworkProfile uses mockforge_core::traffic_shaping::TrafficShapingConfig
            // which has bandwidth and burst_loss fields
            let network_shaping = NetworkShapingConfig {
                enabled: profile.traffic_shaping.bandwidth.enabled
                    || profile.traffic_shaping.burst_loss.enabled,
                bandwidth_limit_bps: profile.traffic_shaping.bandwidth.max_bytes_per_sec * 8, // Convert bytes to bits
                packet_loss_percent: profile.traffic_shaping.burst_loss.loss_rate_during_burst,
                max_connections: 1000, // Default value
            };

            // Update chaos config if it exists, or create it
            // Chaos config is in observability.chaos, not core.chaos
            if let Some(ref mut chaos) = config.observability.chaos {
                chaos.traffic_shaping = Some(network_shaping);
            } else {
                // Create minimal chaos config with traffic shaping
                use mockforge_core::config::ChaosEngConfig;
                config.observability.chaos = Some(ChaosEngConfig {
                    enabled: true,
                    latency: None,
                    fault_injection: None,
                    rate_limit: None,
                    traffic_shaping: Some(network_shaping),
                    scenario: None,
                });
            }

            info!("Network profile '{}' applied to server configuration", request.profile_name);
        } else {
            warn!("Server configuration not available in ManagementState - profile applied but not persisted");
        }

        // Also update chaos API state if available
        #[cfg(feature = "chaos")]
        {
            if let Some(chaos_state) = &state.chaos_api_state {
                use mockforge_chaos::config::TrafficShapingConfig;

                let mut chaos_config = chaos_state.config.write().await;
                // Apply profile's traffic shaping to chaos API state
                let chaos_traffic_shaping = TrafficShapingConfig {
                    enabled: profile.traffic_shaping.bandwidth.enabled
                        || profile.traffic_shaping.burst_loss.enabled,
                    bandwidth_limit_bps: profile.traffic_shaping.bandwidth.max_bytes_per_sec * 8, // Convert bytes to bits
                    packet_loss_percent: profile.traffic_shaping.burst_loss.loss_rate_during_burst,
                    max_connections: 0,
                    connection_timeout_ms: 30000,
                };
                chaos_config.traffic_shaping = Some(chaos_traffic_shaping);
                chaos_config.enabled = true; // Enable chaos when applying a profile
                drop(chaos_config);
                info!("Network profile '{}' applied to chaos API state", request.profile_name);
            }
        }

        Json(serde_json::json!({
            "success": true,
            "message": format!("Network profile '{}' applied", request.profile_name),
            "profile": {
                "name": profile.name,
                "description": profile.description,
            }
        }))
        .into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Profile not found",
                "message": format!("Network profile '{}' not found", request.profile_name)
            })),
        )
            .into_response()
    }
}
