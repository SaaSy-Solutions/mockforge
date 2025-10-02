//! Route definitions for the admin UI


use axum::{
    routing::{delete, get, post},
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
    admin_port: u16,
) -> Router {
    // Initialize global logger if not already initialized
    let _logger = get_global_logger().unwrap_or_else(|| init_global_logger(1000));

    let state = AdminState::new(
        http_server_addr,
        ws_server_addr,
        grpc_server_addr,
        graphql_server_addr,
        api_enabled,
        admin_port,
    );

    // Start system monitoring background task to poll CPU, memory, and thread metrics
    let state_clone = state.clone();
    tokio::spawn(async move {
        state_clone.start_system_monitoring().await;
    });
    let mut router = Router::new()
        .route("/", get(serve_admin_html))
        .route("/assets/index.css", get(serve_admin_css))
        .route("/assets/index.js", get(serve_admin_js))
        .route("/api-docs", get(serve_api_docs))
        .route("/mockforge-icon.png", get(serve_icon))
        .route("/mockforge-icon-32.png", get(serve_icon_32))
        .route("/mockforge-icon-48.png", get(serve_icon_48))
        .route("/mockforge-logo.png", get(serve_logo))
        .route("/mockforge-logo-40.png", get(serve_logo_40))
        .route("/mockforge-logo-80.png", get(serve_logo_80));

    router = router
        .route("/__mockforge/dashboard", get(get_dashboard))
        .route("/__mockforge/health", get(get_health))
        .route("/admin/server-info", get(get_server_info))
        .route("/__mockforge/server-info", get(get_server_info))
        .route("/__mockforge/routes", get(get_routes))
        .route("/__mockforge/logs", get(get_logs))
        .route("/__mockforge/logs/sse", get(logs_sse))
        .route("/__mockforge/metrics", get(get_metrics))
        .route("/__mockforge/config", get(get_config))
        .route("/__mockforge/config/latency", post(update_latency))
        .route("/__mockforge/config/faults", post(update_faults))
        .route("/__mockforge/config/proxy", post(update_proxy))
        .route("/__mockforge/logs", delete(clear_logs))
        .route("/__mockforge/restart", post(restart_servers))
        .route("/__mockforge/restart/status", get(get_restart_status))
        .route("/__mockforge/fixtures", get(get_fixtures))
        .route("/__mockforge/fixtures/{id}", delete(delete_fixture))
        .route("/__mockforge/fixtures/bulk", delete(delete_fixtures_bulk))
        .route("/__mockforge/fixtures/{id}/download", get(download_fixture))
        .route("/__mockforge/import/insomnia", post(import_insomnia))
        // Plugin management routes
        .route("/__mockforge/plugins", get(get_plugins))
        .route("/__mockforge/plugins/status", get(get_plugin_status))
        .route("/__mockforge/plugins/{id}", get(get_plugin_details))
        .route("/__mockforge/plugins/{id}", delete(delete_plugin))
        .route("/__mockforge/plugins/reload", post(reload_plugin))
        // Workspace management routes
        .route("/__mockforge/workspaces", get(get_workspaces))
        .route("/__mockforge/workspaces", post(create_workspace))
        .route("/__mockforge/workspaces/{workspace_id}", get(get_workspace))
        .route("/__mockforge/workspaces/{workspace_id}", delete(delete_workspace))
        .route("/__mockforge/workspaces/{workspace_id}/activate", post(set_active_workspace))
        // Environment management routes
        .route("/__mockforge/workspaces/{workspace_id}/environments", get(get_environments))
        .route("/__mockforge/workspaces/{workspace_id}/environments", post(create_environment))
        .route("/__mockforge/workspaces/{workspace_id}/environments/order", axum::routing::put(update_environments_order))
        .route("/__mockforge/workspaces/{workspace_id}/environments/{environment_id}", axum::routing::put(update_environment))
        .route("/__mockforge/workspaces/{workspace_id}/environments/{environment_id}", delete(delete_environment))
        .route("/__mockforge/workspaces/{workspace_id}/environments/{environment_id}/activate", post(set_active_environment))
        .route("/__mockforge/workspaces/{workspace_id}/environments/{environment_id}/variables", get(get_environment_variables))
        .route("/__mockforge/workspaces/{workspace_id}/environments/{environment_id}/variables", post(set_environment_variable))
        .route("/__mockforge/workspaces/{workspace_id}/environments/{environment_id}/variables/{variable_name}", delete(remove_environment_variable));

    // SPA fallback: serve index.html for any unmatched routes to support client-side routing
    // IMPORTANT: This must be AFTER all API routes
    router = router.route("/{*path}", get(serve_admin_html));

    router.layer(CorsLayer::permissive()).with_state(state)
}
