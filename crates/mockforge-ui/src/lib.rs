//! # MockForge UI
//!
//! Web-based admin interface for managing mock servers.

pub mod handlers;
pub mod routes;
// Templates module removed; static assets in `static/` are the single source of truth
pub mod models;
pub mod prometheus_client;
pub mod time_travel_handlers;

pub use models::{RequestLog, RouteInfo, ServerStatus, SystemInfo};
pub use routes::create_admin_router;

use std::net::SocketAddr;

/// Start the admin UI server
pub async fn start_admin_server(
    addr: SocketAddr,
    http_server_addr: Option<SocketAddr>,
    ws_server_addr: Option<SocketAddr>,
    grpc_server_addr: Option<SocketAddr>,
    graphql_server_addr: Option<SocketAddr>,
    api_enabled: bool,
    prometheus_url: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = create_admin_router(
        http_server_addr,
        ws_server_addr,
        grpc_server_addr,
        graphql_server_addr,
        api_enabled,
        addr.port(),
        prometheus_url,
    );

    tracing::info!("Starting MockForge Admin UI on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        format!(
            "Failed to bind Admin UI server to port {}: {}\n\
             Hint: The port may already be in use. Try using a different port with --admin-port or check if another process is using this port with: lsof -i :{} or netstat -tulpn | grep {}",
            addr.port(), e, addr.port(), addr.port()
        )
    })?;

    axum::serve(listener, app).await?;

    Ok(())
}

/// Get React UI HTML content
pub fn get_admin_html() -> &'static str {
    include_str!("../ui/dist/index.html")
}

/// Get React UI CSS content
pub fn get_admin_css() -> &'static str {
    include_str!("../ui/dist/assets/index.css")
}

/// Get React UI JavaScript content
pub fn get_admin_js() -> &'static str {
    include_str!("../ui/dist/assets/index.js")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_admin_html() {
        let html = get_admin_html();
        assert!(!html.is_empty());
        assert!(html.contains("<!DOCTYPE html>") || html.contains("<html"));
    }

    #[test]
    fn test_get_admin_css() {
        let css = get_admin_css();
        assert!(!css.is_empty());
    }

    #[test]
    fn test_get_admin_js() {
        let js = get_admin_js();
        assert!(!js.is_empty());
    }
}
