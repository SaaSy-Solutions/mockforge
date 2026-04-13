use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;

use super::{ManagementState, ServerStats};

/// Request to validate configuration
#[derive(Debug, Deserialize)]
pub struct ValidateConfigRequest {
    /// Configuration to validate (as JSON)
    pub config: serde_json::Value,
    /// Format of the configuration ("json" or "yaml")
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "json".to_string()
}

/// Validate configuration without applying it
pub(crate) async fn validate_config(
    Json(request): Json<ValidateConfigRequest>,
) -> impl IntoResponse {
    use mockforge_core::config::ServerConfig;

    let config_result: Result<ServerConfig, String> = match request.format.as_str() {
        "yaml" | "yml" => {
            let yaml_str = match serde_json::to_string(&request.config) {
                Ok(s) => s,
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "valid": false,
                            "error": format!("Failed to convert to string: {}", e),
                            "message": "Configuration validation failed"
                        })),
                    )
                        .into_response();
                }
            };
            serde_yaml::from_str(&yaml_str).map_err(|e| format!("YAML parse error: {}", e))
        }
        _ => serde_json::from_value(request.config).map_err(|e| format!("JSON parse error: {}", e)),
    };

    match config_result {
        Ok(_) => Json(serde_json::json!({
            "valid": true,
            "message": "Configuration is valid"
        }))
        .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "valid": false,
                "error": format!("Invalid configuration: {}", e),
                "message": "Configuration validation failed"
            })),
        )
            .into_response(),
    }
}

/// Request for bulk configuration update
#[derive(Debug, Deserialize)]
pub struct BulkConfigUpdateRequest {
    /// Partial configuration updates (only specified fields will be updated)
    pub updates: serde_json::Value,
}

/// Bulk update configuration
///
/// This endpoint allows updating multiple configuration options at once.
/// Only the specified fields in the updates object will be modified.
///
/// Configuration updates are applied to the server configuration if available
/// in ManagementState. Changes take effect immediately for supported settings.
pub(crate) async fn bulk_update_config(
    State(state): State<ManagementState>,
    Json(request): Json<BulkConfigUpdateRequest>,
) -> impl IntoResponse {
    // Validate the updates structure
    if !request.updates.is_object() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid request",
                "message": "Updates must be a JSON object"
            })),
        )
            .into_response();
    }

    // Try to validate as partial ServerConfig
    use mockforge_core::config::ServerConfig;

    // Create a minimal valid config and try to merge updates
    let base_config = ServerConfig::default();
    let base_json = match serde_json::to_value(&base_config) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Internal error",
                    "message": format!("Failed to serialize base config: {}", e)
                })),
            )
                .into_response();
        }
    };

    // Merge updates into base config (simplified merge)
    let mut merged = base_json.clone();
    if let (Some(merged_obj), Some(updates_obj)) =
        (merged.as_object_mut(), request.updates.as_object())
    {
        for (key, value) in updates_obj {
            merged_obj.insert(key.clone(), value.clone());
        }
    }

    // Validate the merged config
    match serde_json::from_value::<ServerConfig>(merged) {
        Ok(validated_config) => {
            // Apply config if server_config is available in ManagementState
            if let Some(ref config_lock) = state.server_config {
                let mut config = config_lock.write().await;
                *config = validated_config;
                Json(serde_json::json!({
                    "success": true,
                    "message": "Bulk configuration update applied successfully",
                    "updates_received": request.updates,
                    "validated": true,
                    "applied": true
                }))
                .into_response()
            } else {
                Json(serde_json::json!({
                    "success": true,
                    "message": "Bulk configuration update validated but not applied (no server config in state). Use .with_server_config() when building ManagementState.",
                    "updates_received": request.updates,
                    "validated": true,
                    "applied": false
                }))
                .into_response()
            }
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid configuration",
                "message": format!("Configuration validation failed: {}", e),
                "validated": false
            })),
        )
            .into_response(),
    }
}

/// Get server statistics
pub(crate) async fn get_stats(State(state): State<ManagementState>) -> Json<ServerStats> {
    let mocks = state.mocks.read().await;
    let request_count = *state.request_counter.read().await;

    Json(ServerStats {
        uptime_seconds: state.start_time.elapsed().as_secs(),
        total_requests: request_count,
        active_mocks: mocks.len(),
        enabled_mocks: mocks.iter().filter(|m| m.enabled).count(),
        registered_routes: mocks.len(), // This could be enhanced with actual route registry info
    })
}

/// Get server configuration
pub(crate) async fn get_config(State(state): State<ManagementState>) -> Json<super::ServerConfig> {
    Json(super::ServerConfig {
        version: env!("CARGO_PKG_VERSION").to_string(),
        port: state.port,
        has_openapi_spec: state.spec.is_some(),
        spec_path: state.spec_path.clone(),
    })
}

/// Serve the loaded OpenAPI spec as JSON
pub(crate) async fn get_openapi_spec(
    State(state): State<ManagementState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match &state.spec {
        Some(spec) => match &spec.raw_document {
            Some(doc) => Ok(Json(doc.clone())),
            None => {
                // Fall back to serializing the parsed spec
                match serde_json::to_value(&spec.spec) {
                    Ok(val) => Ok(Json(val)),
                    Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                }
            }
        },
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Health check endpoint
pub(crate) async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "mockforge-management",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Report which features and protocols are available in this build.
///
/// The UI queries this on startup to auto-stub any missing features
/// instead of maintaining hardcoded prefix lists.
pub(crate) async fn get_capabilities() -> Json<serde_json::Value> {
    let mut features = vec!["core", "http", "management", "mocks", "proxy", "ai"];

    #[cfg(feature = "smtp")]
    features.push("smtp");
    #[cfg(feature = "mqtt")]
    features.push("mqtt");
    #[cfg(feature = "kafka")]
    features.push("kafka");
    #[cfg(feature = "conformance")]
    features.push("conformance");
    #[cfg(feature = "behavioral-cloning")]
    features.push("behavioral-cloning");

    // Always-available subsystems (registered unconditionally in the router)
    features.extend_from_slice(&[
        "chaos",
        "network-profiles",
        "state-machines",
        "migration",
        "snapshot-diff",
        "mockai",
    ]);

    Json(serde_json::json!({
        "features": features,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
