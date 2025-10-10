//! Route definitions for the admin UI


use axum::{
    routing::{delete, get, post},
    Router,
};
use tower_http::{compression::CompressionLayer, cors::CorsLayer};

use crate::handlers::AdminState;
use crate::handlers::*;
use crate::handlers::analytics::AnalyticsState;
use crate::time_travel_handlers;
use mockforge_core::{get_global_logger, init_global_logger};

/// Create the admin router with static assets and optional API endpoints
pub fn create_admin_router(
    http_server_addr: Option<std::net::SocketAddr>,
    ws_server_addr: Option<std::net::SocketAddr>,
    grpc_server_addr: Option<std::net::SocketAddr>,
    graphql_server_addr: Option<std::net::SocketAddr>,
    api_enabled: bool,
    admin_port: u16,
    prometheus_url: String,
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
        .route("/__mockforge/config/traffic-shaping", post(update_traffic_shaping))
        .route("/__mockforge/logs", delete(clear_logs))
        .route("/__mockforge/restart", post(restart_servers))
        .route("/__mockforge/restart/status", get(get_restart_status))
        .route("/__mockforge/fixtures", get(get_fixtures))
        .route("/__mockforge/fixtures/{id}", delete(delete_fixture))
        .route("/__mockforge/fixtures/bulk", delete(delete_fixtures_bulk))
        .route("/__mockforge/fixtures/{id}/download", get(download_fixture))
        .route("/__mockforge/fixtures/{id}/rename", post(rename_fixture))
        .route("/__mockforge/fixtures/{id}/move", post(move_fixture))
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
        .route("/__mockforge/workspaces/{workspace_id}/environments/{environment_id}/variables/{variable_name}", delete(remove_environment_variable))
        // Chain management routes - proxy to main HTTP server
        .route("/__mockforge/chains", get(proxy_chains_list))
        .route("/__mockforge/chains", post(proxy_chains_create))
        .route("/__mockforge/chains/{id}", get(proxy_chain_get))
        .route("/__mockforge/chains/{id}", axum::routing::put(proxy_chain_update))
        .route("/__mockforge/chains/{id}", delete(proxy_chain_delete))
        .route("/__mockforge/chains/{id}/execute", post(proxy_chain_execute))
        .route("/__mockforge/chains/{id}/validate", post(proxy_chain_validate))
        .route("/__mockforge/chains/{id}/history", get(proxy_chain_history))
        // Validation configuration routes
        .route("/__mockforge/validation", get(get_validation))
        .route("/__mockforge/validation", post(update_validation))
        // Environment variables routes
        .route("/__mockforge/env", get(get_env_vars))
        .route("/__mockforge/env", post(update_env_var))
        // File management routes
        .route("/__mockforge/files/content", post(get_file_content))
        .route("/__mockforge/files/save", post(save_file_content))
        // Smoke test routes
        .route("/__mockforge/smoke", get(get_smoke_tests))
        .route("/__mockforge/smoke/run", get(run_smoke_tests_endpoint))
        // Time travel / temporal testing routes
        .route("/__mockforge/time-travel/status", get(time_travel_handlers::get_time_travel_status))
        .route("/__mockforge/time-travel/enable", post(time_travel_handlers::enable_time_travel))
        .route("/__mockforge/time-travel/disable", post(time_travel_handlers::disable_time_travel))
        .route("/__mockforge/time-travel/advance", post(time_travel_handlers::advance_time))
        .route("/__mockforge/time-travel/scale", post(time_travel_handlers::set_time_scale))
        .route("/__mockforge/time-travel/reset", post(time_travel_handlers::reset_time_travel))
        .route("/__mockforge/time-travel/schedule", post(time_travel_handlers::schedule_response))
        .route("/__mockforge/time-travel/scheduled", get(time_travel_handlers::list_scheduled_responses))
        .route("/__mockforge/time-travel/scheduled/{id}", delete(time_travel_handlers::cancel_scheduled_response))
        .route("/__mockforge/time-travel/scheduled/clear", post(time_travel_handlers::clear_scheduled_responses))
        // Health check endpoints for Kubernetes probes
        .route("/health/live", get(health::liveness_probe))
        .route("/health/ready", get(health::readiness_probe))
        .route("/health/startup", get(health::startup_probe))
        .route("/health", get(health::deep_health_check));

    // Analytics routes with Prometheus integration
    let analytics_state = AnalyticsState::new(prometheus_url);

    let analytics_router = Router::new()
        .route("/__mockforge/analytics/summary", get(analytics::get_summary))
        .route("/__mockforge/analytics/requests", get(analytics::get_requests))
        .route("/__mockforge/analytics/endpoints", get(analytics::get_endpoints))
        .route("/__mockforge/analytics/websocket", get(analytics::get_websocket))
        .route("/__mockforge/analytics/smtp", get(analytics::get_smtp))
        .route("/__mockforge/analytics/system", get(analytics::get_system))
        .with_state(analytics_state);

    router = router.merge(analytics_router);

    // SPA fallback: serve index.html for any unmatched routes to support client-side routing
    // IMPORTANT: This must be AFTER all API routes
    router = router.route("/{*path}", get(serve_admin_html));

    router
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_admin_router() {
        let http_addr: std::net::SocketAddr = "127.0.0.1:3000".parse().unwrap();
        let router = create_admin_router(
            Some(http_addr),
            None,
            None,
            None,
            true,
            8080,
            "http://localhost:9090".to_string(),
        );

        // Router should be created successfully
        let _ = router;
    }

    #[tokio::test]
    async fn test_create_admin_router_no_servers() {
        let router = create_admin_router(
            None,
            None,
            None,
            None,
            false,
            8080,
            "http://localhost:9090".to_string(),
        );

        // Router should still work without server addresses
        let _ = router;
    }
}
