//! Route definitions for the admin UI

use std::sync::Arc;

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
        .unwrap_or_else(|| init_global_logger(1000));

    let state = AdminState::new(http_server_addr, ws_server_addr, grpc_server_addr, graphql_server_addr);

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
            .route("/__mockforge/dashboard", get(get_dashboard))
            .route("/__mockforge/health", get(get_health))
            .route("/admin/server-info", get(get_server_info))
            .route("/__mockforge/logs", get(get_logs))
            .route("/__mockforge/metrics", get(get_metrics))
            .route("/__mockforge/config", get(get_config))
            .route("/__mockforge/config/latency", post(update_latency))
            .route("/__mockforge/config/faults", post(update_faults))
            .route("/__mockforge/config/proxy", post(update_proxy))
            .route("/__mockforge/logs", delete(clear_logs))
            .route("/__mockforge/restart", post(restart_servers))
            .route("/__mockforge/restart/status", get(get_restart_status))
            .route("/__mockforge/import/insomnia", post(import_insomnia));
    }

    router.layer(CorsLayer::permissive()).with_state(state)
}
