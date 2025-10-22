//! WebSocket Handlers Demo
//!
//! This example demonstrates how to use programmable WebSocket handlers with MockForge.
//!
//! ## Running the example
//!
//! ```bash
//! cargo run --example ws-handlers-demo
//! ```
//!
//! Then connect with a WebSocket client:
//!
//! ```bash
//! # Test echo handler
//! websocat ws://localhost:3030/ws
//!
//! # Test chat handler
//! websocat ws://localhost:3030/ws/chat
//! ```

mod chat_handler;
mod echo_handler;

use chat_handler::ChatHandler;
use echo_handler::EchoHandler;
use mockforge_ws::HandlerRegistry;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("Starting WebSocket Handlers Demo...");
    println!("Echo handler available at: ws://localhost:3030/ws");
    println!("Chat handler available at: ws://localhost:3030/ws/chat");

    // Create handler registry
    let mut registry = HandlerRegistry::new();

    // Register echo handler for /ws path
    registry.register(EchoHandler);

    // Register chat handler for /ws/chat path
    registry.register(ChatHandler::new());

    // Wrap in Arc for sharing across connections
    let registry = Arc::new(registry);

    // Build router with handlers
    let app = mockforge_ws::router_with_handlers(registry);

    // Start server
    let addr: std::net::SocketAddr = "0.0.0.0:3030".parse()?;
    println!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
