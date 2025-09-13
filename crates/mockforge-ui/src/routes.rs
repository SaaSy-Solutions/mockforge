//! Route definitions for the admin UI

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;

use crate::handlers::AdminState;
use crate::handlers::*;

/// Create the admin router with static assets and optional API endpoints
pub fn create_admin_router(
    http_server_addr: Option<std::net::SocketAddr>,
    ws_server_addr: Option<std::net::SocketAddr>,
    grpc_server_addr: Option<std::net::SocketAddr>,
    api_enabled: bool,
) -> Router {
    let state = AdminState::new(http_server_addr, ws_server_addr, grpc_server_addr);
    let mut router = Router::new()
        .route("/", get(serve_admin_html))
        .route("/admin.css", get(serve_admin_css))
        .route("/admin.js", get(serve_admin_js));

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
            // Smoke tests
            .route("/__mockforge/smoke", get(get_smoke_tests))
            .route("/__mockforge/smoke/run", get(run_smoke_tests_endpoint))
            // Management actions
            .route("/__mockforge/logs/clear", post(clear_logs))
            .route("/__mockforge/servers/restart", post(restart_servers))
            .route("/__mockforge/servers/restart/status", get(get_restart_status));
    }

    router.layer(CorsLayer::permissive()).with_state(state)
}

/// Health check endpoint (can be used by load balancers)
pub fn health_router() -> Router {
    Router::new()
        .route("/health", get(get_health))
        .route("/api/health", get(get_health))
}
