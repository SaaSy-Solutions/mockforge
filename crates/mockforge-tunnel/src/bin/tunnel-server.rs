//! Standalone tunnel server for testing and development

use mockforge_tunnel::server::{create_tunnel_server_router, TunnelStore};
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    // Get port from env or use default
    let port = std::env::var("TUNNEL_SERVER_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(4040);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let store = TunnelStore::new();
    let router = create_tunnel_server_router().with_state(store);

    println!("🚇 MockForge Tunnel Server");
    println!("   Listening on: http://{}", addr);
    println!("   Health check: http://{}/health", addr);
    println!("   API endpoint: http://{}/api/tunnels", addr);
    println!("\nPress Ctrl+C to stop");

    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
