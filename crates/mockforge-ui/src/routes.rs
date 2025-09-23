//! Route definitions for the admin UI

use axum::{
    routing::{get, post, put, delete},
    Router,
};
use tower_http::cors::CorsLayer;

use crate::handlers::AdminState;
use crate::handlers::*;
use mockforge_core::{get_global_logger, init_global_logger};

/// Create the admin router with static assets and optional API endpoints
pub fn create_admin_router(
    http_server_addr: Option<std::net::SocketAddr>,
    ws_server_addr: Option<std::net::SocketAddr>,
    grpc_server_addr: Option<std::net::SocketAddr>,
    graphql_server_addr: Option<std::net::SocketAddr>,
    api_enabled: bool,
) -> Router {
    // Initialize global logger if not already initialized
    let logger = get_global_logger()
        .map(|l| Arc::new(l.clone()))
        .unwrap_or_else(|| Arc::new(init_global_logger(1000).clone()));

    let state = AdminState::new(http_server_addr, ws_server_addr, grpc_server_addr, graphql_server_addr, api_enabled, logger);

    // Start system monitoring background task to poll CPU, memory, and thread metrics
    let state_clone = state.clone();
    tokio::spawn(async move {
        state_clone.start_system_monitoring().await;
    });
    let mut router = Router::new()
        .route("/", get(serve_admin_html))
        .route("/assets/index.css", get(serve_admin_css))
        .route("/assets/index.js", get(serve_admin_js))
        .route("/mockforge-icon.png", get(serve_icon))
        .route("/mockforge-icon-32.png", get(serve_icon_32))
        .route("/mockforge-icon-48.png", get(serve_icon_48))
        .route("/mockforge-logo.png", get(serve_logo))
        .route("/mockforge-logo-40.png", get(serve_logo_40))
        .route("/mockforge-logo-80.png", get(serve_logo_80))
        // SPA fallback: serve index.html for any unmatched routes to support client-side routing
        .route("/{*path}", get(serve_admin_html));

    if api_enabled {
        router = router
            // Dedicated admin API endpoints (avoiding user route conflicts)
            .route("/__mockforge/dashboard", get(get_dashboard))
            .route("/__mockforge/health", get(get_health))
            .route("/__mockforge/routes", get(get_routes))
            .route("/admin/server-info", get(get_server_info))
            .route("/__mockforge/logs", get(get_logs))
            .route("/__mockforge/metrics", get(get_metrics))
            .route("/__mockforge/config", get(get_config))
            .route("/__mockforge/fixtures", get(get_fixtures))
            .route("/__mockforge/fixtures/delete", post(delete_fixture))
            .route("/__mockforge/fixtures/delete-bulk", post(delete_fixtures_bulk))
            .route("/__mockforge/fixtures/download", get(download_fixture))
            .route("/__mockforge/env", get(get_env_vars))
            .route("/__mockforge/env", post(update_env_var))
            .route("/__mockforge/files/content", post(get_file_content))
            .route("/__mockforge/files/save", post(save_file_content))
            // Validation settings
            .route("/__mockforge/validation", get(get_validation))
            .route("/__mockforge/validation", post(update_validation))
            // Configuration updates
            .route("/__mockforge/config/latency", post(update_latency))
            .route("/__mockforge/config/faults", post(update_faults))
            .route("/__mockforge/config/proxy", post(update_proxy))
            .route("/__mockforge/config/traffic-shaping", post(update_traffic_shaping))
            // Smoke tests
            .route("/__mockforge/smoke", get(get_smoke_tests))
            .route("/__mockforge/smoke/run", get(run_smoke_tests_endpoint))
            // Management actions
            .route("/__mockforge/logs/clear", post(clear_logs))
            .route("/__mockforge/servers/restart", post(restart_servers))
            .route("/__mockforge/servers/restart/status", get(get_restart_status))
            // Import functionality
            .route("/__mockforge/import/postman", post(import_postman))
            .route("/__mockforge/import/insomnia", post(import_insomnia))
            .route("/__mockforge/import/openapi", post(import_openapi))
            .route("/__mockforge/import/curl", post(import_curl))
            .route("/__mockforge/import/preview", post(preview_import))
            // Import history
            .route("/__mockforge/import/history", get(get_import_history))
            .route("/__mockforge/import/history/clear", post(clear_import_history))
            // Additional admin API endpoints (unified to /__mockforge)
            .route("/__mockforge/api/state", get(get_admin_api_state))
            .route("/__mockforge/api/replay", get(get_admin_api_replay))
            // SSE monitoring endpoints
            .route("/__mockforge/sse/status", get(get_sse_status))
            .route("/__mockforge/sse/connections", get(get_sse_connections))
            // Workspace management endpoints
            .route("/__mockforge/workspaces", get(get_workspaces))
            .route("/__mockforge/workspaces", post(create_workspace))
            .route("/__mockforge/workspaces/open-from-directory", post(open_workspace_from_directory))
            .route("/__mockforge/workspaces/{workspace_id}", get(get_workspace))
            .route("/__mockforge/workspaces/{workspace_id}", delete(delete_workspace))
            .route("/__mockforge/workspaces/{workspace_id}/activate", post(set_active_workspace))
            .route("/__mockforge/workspaces/{workspace_id}/folders", post(create_folder))
            .route("/__mockforge/workspaces/{workspace_id}/requests", post(create_request))
            .route("/__mockforge/workspaces/{workspace_id}/requests/{request_id}/execute", post(execute_workspace_request))
            .route("/__mockforge/workspaces/{workspace_id}/requests/{request_id}/history", get(get_request_history))
            .route("/__mockforge/workspaces/{workspace_id}/folders/{folder_id}", get(get_folder))
            .route("/__mockforge/workspaces/{workspace_id}/import", post(import_to_workspace))
            .route("/__mockforge/workspaces/export", post(export_workspaces))
            // Environment management endpoints
            .route("/__mockforge/workspaces/{workspace_id}/environments", get(get_environments))
            .route("/__mockforge/workspaces/{workspace_id}/environments", post(create_environment))
            .route("/__mockforge/workspaces/{workspace_id}/environments/{environment_id}", put(update_environment))
            .route("/__mockforge/workspaces/{workspace_id}/environments/{environment_id}", delete(delete_environment))
            .route("/__mockforge/workspaces/{workspace_id}/environments/{environment_id}/activate", post(set_active_environment))
            .route("/__mockforge/workspaces/{workspace_id}/environments/{environment_id}/variables", get(get_environment_variables))
            .route("/__mockforge/workspaces/{workspace_id}/environments/{environment_id}/variables", post(set_environment_variable))
            .route("/__mockforge/workspaces/{workspace_id}/environments/{environment_id}/variables/{variable_name}", delete(remove_environment_variable))
             // Autocomplete endpoints
             .route("/__mockforge/workspaces/{workspace_id}/autocomplete", post(get_autocomplete_suggestions))
             // Sync management endpoints
             .route("/__mockforge/workspaces/{workspace_id}/sync/status", get(get_sync_status))
             .route("/__mockforge/workspaces/{workspace_id}/sync/configure", post(configure_sync))
             .route("/__mockforge/workspaces/{workspace_id}/sync/disable", post(disable_sync))
             .route("/__mockforge/workspaces/{workspace_id}/sync/trigger", post(trigger_sync))
             .route("/__mockforge/workspaces/{workspace_id}/sync/changes", get(get_sync_changes))
             .route("/__mockforge/workspaces/{workspace_id}/sync/confirm", post(confirm_sync_changes))
             // Plugin management endpoints
            .route("/__mockforge/plugins", get(get_plugins))
            .route("/__mockforge/plugins/{plugin_id}", get(get_plugin))
            .route("/__mockforge/plugins/{plugin_id}", delete(delete_plugin))
            .route("/__mockforge/plugins/install", post(install_plugin))
            .route("/__mockforge/plugins/reload", post(reload_plugins))
            .route("/__mockforge/plugins/validate", post(validate_plugin))
            .route("/__mockforge/plugins/status", get(get_plugin_status));
    }

    router.layer(CorsLayer::permissive()).with_state(state)
}
