use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;

use super::ManagementState;

/// Request to set migration mode
#[derive(Debug, Deserialize)]
pub(crate) struct SetMigrationModeRequest {
    mode: String,
}

/// Get all migration routes
pub(crate) async fn get_migration_routes(
    State(state): State<ManagementState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Migration not configured. Proxy config not available."
            })));
        }
    };

    let config = proxy_config.read().await;
    let routes = config.get_migration_routes();

    Ok(Json(serde_json::json!({
        "routes": routes
    })))
}

/// Toggle a route's migration mode
pub(crate) async fn toggle_route_migration(
    State(state): State<ManagementState>,
    Path(pattern): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Migration not configured. Proxy config not available."
            })));
        }
    };

    let mut config = proxy_config.write().await;
    let new_mode = match config.toggle_route_migration(&pattern) {
        Some(mode) => mode,
        None => {
            return Ok(Json(serde_json::json!({
                "error": format!("Route pattern not found: {}", pattern)
            })));
        }
    };

    Ok(Json(serde_json::json!({
        "pattern": pattern,
        "mode": format!("{:?}", new_mode).to_lowercase()
    })))
}

/// Set a route's migration mode explicitly
pub(crate) async fn set_route_migration_mode(
    State(state): State<ManagementState>,
    Path(pattern): Path<String>,
    Json(request): Json<SetMigrationModeRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Migration not configured. Proxy config not available."
            })));
        }
    };

    use mockforge_proxy::config::MigrationMode;
    let mode = match request.mode.to_lowercase().as_str() {
        "mock" => MigrationMode::Mock,
        "shadow" => MigrationMode::Shadow,
        "real" => MigrationMode::Real,
        "auto" => MigrationMode::Auto,
        _ => {
            return Ok(Json(serde_json::json!({
                "error": format!("Invalid migration mode: {}. Must be one of: mock, shadow, real, auto", request.mode)
            })));
        }
    };

    let mut config = proxy_config.write().await;
    let updated = config.update_rule_migration_mode(&pattern, mode);

    if !updated {
        return Ok(Json(serde_json::json!({
            "error": format!("Route pattern not found: {}", pattern)
        })));
    }

    Ok(Json(serde_json::json!({
        "pattern": pattern,
        "mode": format!("{:?}", mode).to_lowercase()
    })))
}

/// Toggle a group's migration mode
pub(crate) async fn toggle_group_migration(
    State(state): State<ManagementState>,
    Path(group): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Migration not configured. Proxy config not available."
            })));
        }
    };

    let mut config = proxy_config.write().await;
    let new_mode = config.toggle_group_migration(&group);

    Ok(Json(serde_json::json!({
        "group": group,
        "mode": format!("{:?}", new_mode).to_lowercase()
    })))
}

/// Set a group's migration mode explicitly
pub(crate) async fn set_group_migration_mode(
    State(state): State<ManagementState>,
    Path(group): Path<String>,
    Json(request): Json<SetMigrationModeRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Migration not configured. Proxy config not available."
            })));
        }
    };

    use mockforge_proxy::config::MigrationMode;
    let mode = match request.mode.to_lowercase().as_str() {
        "mock" => MigrationMode::Mock,
        "shadow" => MigrationMode::Shadow,
        "real" => MigrationMode::Real,
        "auto" => MigrationMode::Auto,
        _ => {
            return Ok(Json(serde_json::json!({
                "error": format!("Invalid migration mode: {}. Must be one of: mock, shadow, real, auto", request.mode)
            })));
        }
    };

    let mut config = proxy_config.write().await;
    config.update_group_migration_mode(&group, mode);

    Ok(Json(serde_json::json!({
        "group": group,
        "mode": format!("{:?}", mode).to_lowercase()
    })))
}

/// Get all migration groups
pub(crate) async fn get_migration_groups(
    State(state): State<ManagementState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Migration not configured. Proxy config not available."
            })));
        }
    };

    let config = proxy_config.read().await;
    let groups = config.get_migration_groups();

    // Convert to JSON-serializable format
    let groups_json: serde_json::Map<String, serde_json::Value> = groups
        .into_iter()
        .map(|(name, info)| {
            (
                name,
                serde_json::json!({
                    "name": info.name,
                    "migration_mode": format!("{:?}", info.migration_mode).to_lowercase(),
                    "route_count": info.route_count
                }),
            )
        })
        .collect();

    Ok(Json(serde_json::json!(groups_json)))
}

/// Get overall migration status
pub(crate) async fn get_migration_status(
    State(state): State<ManagementState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Migration not configured. Proxy config not available."
            })));
        }
    };

    let config = proxy_config.read().await;
    let routes = config.get_migration_routes();
    let groups = config.get_migration_groups();

    let mut mock_count = 0;
    let mut shadow_count = 0;
    let mut real_count = 0;
    let mut auto_count = 0;

    for route in &routes {
        match route.migration_mode {
            mockforge_proxy::config::MigrationMode::Mock => mock_count += 1,
            mockforge_proxy::config::MigrationMode::Shadow => shadow_count += 1,
            mockforge_proxy::config::MigrationMode::Real => real_count += 1,
            mockforge_proxy::config::MigrationMode::Auto => auto_count += 1,
        }
    }

    Ok(Json(serde_json::json!({
        "total_routes": routes.len(),
        "mock_routes": mock_count,
        "shadow_routes": shadow_count,
        "real_routes": real_count,
        "auto_routes": auto_count,
        "total_groups": groups.len(),
        "migration_enabled": config.migration_enabled
    })))
}
