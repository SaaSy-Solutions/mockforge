//! # MockForge UI
//!
//! Web-based admin interface for managing mock servers.

pub mod routes;
pub mod handlers;
// Templates module removed; static assets in `static/` are the single source of truth
pub mod models;

pub use routes::create_admin_router;
pub use models::{ServerStatus, RouteInfo, RequestLog, SystemInfo};

use std::net::SocketAddr;

/// Start the admin UI server
pub async fn start_admin_server(
    addr: SocketAddr,
    http_server_addr: Option<SocketAddr>,
    ws_server_addr: Option<SocketAddr>,
    grpc_server_addr: Option<SocketAddr>,
    api_enabled: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = create_admin_router(http_server_addr, ws_server_addr, grpc_server_addr, api_enabled);

    tracing::info!("Starting MockForge Admin UI on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Get admin UI HTML content
pub fn get_admin_html() -> &'static str {
    include_str!("../static/admin.html")
}

/// Get admin UI CSS content
pub fn get_admin_css() -> &'static str {
    include_str!("../static/admin.css")
}

/// Get admin UI JavaScript content
pub fn get_admin_js() -> &'static str {
    include_str!("../static/admin.js")
}
