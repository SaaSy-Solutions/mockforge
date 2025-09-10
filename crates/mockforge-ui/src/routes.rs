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
            .route("/__mockforge/logs", get(get_logs))
            .route("/__mockforge/metrics", get(get_metrics))
            .route("/__mockforge/config", get(get_config))
            .route("/__mockforge/fixtures", get(get_fixtures))
            // Configuration updates
            .route("/__mockforge/config/latency", post(update_latency))
            .route("/__mockforge/config/faults", post(update_faults))
            .route("/__mockforge/config/proxy", post(update_proxy))
            // Management actions
            .route("/__mockforge/logs/clear", post(clear_logs))
            .route("/__mockforge/servers/restart", post(restart_servers));
    }

    router.layer(CorsLayer::permissive()).with_state(state)
}

/// Health check endpoint (can be used by load balancers)
pub fn health_router() -> Router {
    Router::new()
        .route("/health", get(get_health))
        .route("/api/health", get(get_health))
}
