pub mod static_spa;
pub mod admin_api;
pub mod latency_profiles;
pub mod schema_diff;
pub mod overrides;
pub mod replay_listing;
pub mod op_middleware;

use axum::Router;
use std::net::SocketAddr;
use tracing::*;

pub async fn start(port: u16, _spec: Option<String>) {
    // Create app state for admin API
    let state = admin_api::AppState {
        started_at: std::time::Instant::now(),
        profiles_count_op: 0, // TODO: load from spec
        profiles_count_tag: 0, // TODO: load from spec
        fixtures_root: "fixtures".to_string(), // TODO: make configurable
    };

    // Set up the router
    let app = Router::new()
        .nest("/__admin", static_spa::service())
        .nest("/__admin/api", admin_api::router(state));

    // Start the server
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("HTTP listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app).await.unwrap();
}
