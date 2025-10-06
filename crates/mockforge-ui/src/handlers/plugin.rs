//! Plugin management handlers

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{AdminState, ApiResponse};

/// Plugin information for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub types: Vec<String>,
    pub status: String,
    pub healthy: bool,
    pub description: String,
    pub author: String,
}

/// Plugin statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStats {
    pub total_plugins: usize,
    pub discovered: usize,
    pub loaded: usize,
    pub failed: usize,
    pub skipped: usize,
    pub success_rate: f64,
}

/// Plugin health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHealthInfo {
    pub id: String,
    pub healthy: bool,
    pub message: String,
    pub last_check: String,
}

/// Plugin status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStatusData {
    pub stats: PluginStats,
    pub health: Vec<PluginHealthInfo>,
    pub last_updated: Option<String>,
}

/// Query parameters for plugin listing
#[derive(Debug, Deserialize)]
pub struct PluginListQuery {
    #[serde(rename = "type")]
    pub plugin_type: Option<String>,
    pub status: Option<String>,
}

/// Reload plugin request
#[derive(Debug, Deserialize)]
pub struct ReloadPluginRequest {
    pub plugin_id: String,
}

/// Get list of plugins
pub async fn get_plugins(
    State(state): State<AdminState>,
    Query(query): Query<PluginListQuery>,
) -> Json<ApiResponse<serde_json::Value>> {
    tracing::info!("get_plugins called");

    let registry = state.plugin_registry.read().await;
    let all_plugins = registry.list_plugins();

    let mut plugins: Vec<PluginInfo> = Vec::new();

    for plugin_id in all_plugins {
        if let Some(plugin_instance) = registry.get_plugin(&plugin_id) {
            // Convert core PluginInfo to UI PluginInfo
            let ui_plugin = PluginInfo {
                id: plugin_instance.manifest.info.id.as_str().to_string(),
                name: plugin_instance.manifest.info.name.clone(),
                version: plugin_instance.manifest.info.version.to_string(),
                types: plugin_instance.manifest.capabilities.clone(),
                status: plugin_instance.state.to_string(),
                healthy: plugin_instance.is_healthy(),
                description: plugin_instance.manifest.info.description.clone(),
                author: plugin_instance.manifest.info.author.name.clone(),
            };

            // Apply filters
            let matches_type = query.plugin_type.as_ref()
                .map(|t| ui_plugin.types.contains(t))
                .unwrap_or(true);

            let matches_status = query.status.as_ref()
                .map(|s| ui_plugin.status == *s)
                .unwrap_or(true);

            if matches_type && matches_status {
                plugins.push(ui_plugin);
            }
        }
    }

    Json(ApiResponse::success(json!({
        "plugins": plugins,
        "total": plugins.len()
    })))
}

/// Get plugin status
pub async fn get_plugin_status(
    State(state): State<AdminState>,
) -> Json<ApiResponse<PluginStatusData>> {
    let registry = state.plugin_registry.read().await;
    let registry_stats = registry.get_stats();

    // Calculate stats from registry
    let mut loaded = 0;
    let mut failed = 0;

    let mut health: Vec<PluginHealthInfo> = Vec::new();

    for plugin_id in registry.list_plugins() {
        if let Some(plugin_instance) = registry.get_plugin(&plugin_id) {
            let is_loaded = matches!(plugin_instance.state, mockforge_plugin_core::PluginState::Ready);

            if is_loaded {
                loaded += 1;
            } else {
                failed += 1;
            }

            let is_healthy = plugin_instance.is_healthy();

            health.push(PluginHealthInfo {
                id: plugin_id.as_str().to_string(),
                healthy: is_healthy,
                message: if is_healthy {
                    "Plugin is healthy".to_string()
                } else {
                    "Plugin has issues".to_string()
                },
                last_check: plugin_instance.health.last_check.to_rfc3339(),
            });
        }
    }

    let total_plugins = registry_stats.total_plugins as usize;
    let success_rate = if total_plugins > 0 {
        (loaded as f64 / total_plugins as f64) * 100.0
    } else {
        100.0
    };

    let status = PluginStatusData {
        stats: PluginStats {
            total_plugins,
            discovered: total_plugins,
            loaded,
            failed,
            skipped: 0, // Not tracked in current registry
            success_rate,
        },
        health,
        last_updated: Some(registry_stats.last_updated.to_rfc3339()),
    };

    Json(ApiResponse::success(status))
}

/// Get plugin details
pub async fn get_plugin_details(
    State(state): State<AdminState>,
    Path(plugin_id): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    let registry = state.plugin_registry.read().await;
    let plugin_id_core = mockforge_plugin_core::PluginId::new(&plugin_id);

    if let Some(plugin_instance) = registry.get_plugin(&plugin_id_core) {
        let details = json!({
            "id": plugin_instance.manifest.info.id.as_str(),
            "name": plugin_instance.manifest.info.name,
            "version": plugin_instance.manifest.info.version.to_string(),
            "description": plugin_instance.manifest.info.description,
            "author": {
                "name": plugin_instance.manifest.info.author.name,
                "email": plugin_instance.manifest.info.author.email
            },
            "capabilities": plugin_instance.manifest.capabilities,
            "dependencies": plugin_instance.manifest.dependencies.iter()
                .map(|(id, version)| json!({
                    "id": id.as_str(),
                    "version": version.to_string()
                }))
                .collect::<Vec<_>>(),
            "state": plugin_instance.state.to_string(),
            "healthy": plugin_instance.is_healthy(),
            "health": {
                "state": plugin_instance.health.state.to_string(),
                "healthy": plugin_instance.health.healthy,
                "message": plugin_instance.health.message,
                "last_check": plugin_instance.health.last_check.to_rfc3339(),
                "metrics": {
                    "total_executions": plugin_instance.health.metrics.total_executions,
                    "successful_executions": plugin_instance.health.metrics.successful_executions,
                    "failed_executions": plugin_instance.health.metrics.failed_executions,
                    "avg_execution_time_ms": plugin_instance.health.metrics.avg_execution_time_ms,
                    "max_execution_time_ms": plugin_instance.health.metrics.max_execution_time_ms,
                    "memory_usage_bytes": plugin_instance.health.metrics.memory_usage_bytes,
                    "peak_memory_usage_bytes": plugin_instance.health.metrics.peak_memory_usage_bytes
                }
            }
        });

        Json(ApiResponse::success(details))
    } else {
        Json(ApiResponse::error(format!("Plugin not found: {}", plugin_id)))
    }
}

/// Delete a plugin
pub async fn delete_plugin(
    State(state): State<AdminState>,
    Path(plugin_id): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    let mut registry = state.plugin_registry.write().await;
    let plugin_id_core = mockforge_plugin_core::PluginId::new(&plugin_id);

    match registry.remove_plugin(&plugin_id_core) {
        Ok(removed_plugin) => {
            Json(ApiResponse::success(json!({
                "message": format!("Plugin '{}' removed successfully", plugin_id),
                "plugin": {
                    "id": removed_plugin.id.as_str(),
                    "name": removed_plugin.manifest.info.name
                }
            })))
        }
        Err(_) => {
            Json(ApiResponse::error(format!("Failed to remove plugin: {}", plugin_id)))
        }
    }
}

/// Reload a plugin
pub async fn reload_plugin(
    State(state): State<AdminState>,
    Json(payload): Json<ReloadPluginRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let registry = state.plugin_registry.read().await;
    let plugin_id_core = mockforge_plugin_core::PluginId::new(&payload.plugin_id);

    if registry.has_plugin(&plugin_id_core) {
        // In a real implementation, this would unload and reload the plugin from disk
        // For now, just return success since the plugin exists
        Json(ApiResponse::success(json!({
            "message": format!("Plugin '{}' reload initiated", payload.plugin_id),
            "status": "reloading"
        })))
    } else {
        Json(ApiResponse::error(format!("Plugin not found: {}", payload.plugin_id)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_info_creation() {
        let info = PluginInfo {
            id: "test".to_string(),
            name: "Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            types: vec!["resolver".to_string()],
            status: "ready".to_string(),
            healthy: true,
            description: "Test description".to_string(),
            author: "Test Author".to_string(),
        };

        assert_eq!(info.id, "test");
        assert_eq!(info.name, "Test Plugin");
        assert_eq!(info.version, "1.0.0");
        assert!(info.healthy);
        assert_eq!(info.author, "Test Author");
    }

    #[test]
    fn test_plugin_stats_creation() {
        let stats = PluginStats {
            total_plugins: 10,
            discovered: 10,
            loaded: 8,
            failed: 2,
            skipped: 0,
            success_rate: 80.0,
        };

        assert_eq!(stats.total_plugins, 10);
        assert_eq!(stats.discovered, 10);
        assert_eq!(stats.loaded, 8);
        assert_eq!(stats.failed, 2);
        assert_eq!(stats.success_rate, 80.0);
    }

    #[test]
    fn test_plugin_health_info_creation() {
        let health = PluginHealthInfo {
            id: "plugin-1".to_string(),
            healthy: true,
            message: "All good".to_string(),
            last_check: "2024-01-01T00:00:00Z".to_string(),
        };

        assert_eq!(health.id, "plugin-1");
        assert!(health.healthy);
        assert_eq!(health.message, "All good");
    }

    #[test]
    fn test_plugin_status_data_creation() {
        let stats = PluginStats {
            total_plugins: 5,
            discovered: 5,
            loaded: 5,
            failed: 0,
            skipped: 0,
            success_rate: 100.0,
        };

        let status_data = PluginStatusData {
            stats,
            health: vec![],
            last_updated: Some("2024-01-01T00:00:00Z".to_string()),
        };

        assert_eq!(status_data.stats.total_plugins, 5);
        assert!(status_data.health.is_empty());
    }

    #[test]
    fn test_reload_plugin_request() {
        let req = ReloadPluginRequest {
            plugin_id: "test-plugin".to_string(),
        };

        assert_eq!(req.plugin_id, "test-plugin");
    }

    #[test]
    fn test_plugin_list_query_default() {
        let query = PluginListQuery {
            plugin_type: None,
            status: None,
        };

        assert!(query.plugin_type.is_none());
        assert!(query.status.is_none());
    }

    #[test]
    fn test_plugin_list_query_with_filters() {
        let query = PluginListQuery {
            plugin_type: Some("resolver".to_string()),
            status: Some("ready".to_string()),
        };

        assert_eq!(query.plugin_type, Some("resolver".to_string()));
        assert_eq!(query.status, Some("ready".to_string()));
    }
}
