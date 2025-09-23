use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::{response::IntoResponse, routing::get, Router};
use mockforge_core::{latency::LatencyInjector, LatencyProfile, WsProxyHandler};
#[cfg(feature = "data-faker")]
use mockforge_data::provider::register_core_faker_provider;
use tracing::*;

/// Build the WebSocket router (exposed for tests and embedding)
pub fn router() -> Router {
    #[cfg(feature = "data-faker")]
    register_core_faker_provider();

    Router::new().route("/ws", get(ws_handler_no_state))
}

/// Build the WebSocket router with latency injector state
pub fn router_with_latency(latency_injector: LatencyInjector) -> Router {
    #[cfg(feature = "data-faker")]
    register_core_faker_provider();

    Router::new()
        .route("/ws", get(ws_handler_with_state))
        .with_state(latency_injector)
}

/// Build the WebSocket router with proxy handler
pub fn router_with_proxy(proxy_handler: WsProxyHandler) -> Router {
    #[cfg(feature = "data-faker")]
    register_core_faker_provider();

    Router::new()
        .route("/ws", get(ws_handler_with_proxy))
        .with_state(proxy_handler)
}

/// Start WebSocket server with latency simulation
pub async fn start_with_latency(port: u16, latency: Option<LatencyProfile>) -> Result<(), Box<dyn std::error::Error>> {
    let latency_injector = latency.map(|profile| LatencyInjector::new(profile, Default::default()));
    let router = if let Some(injector) = latency_injector {
        router_with_latency(injector)
    } else {
        router()
    };

    let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse()?;
    info!("WebSocket server listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, router).await?;
    Ok(())
}

// WebSocket handlers
async fn ws_handler_no_state(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn ws_handler_with_state(
    ws: WebSocketUpgrade,
    axum::extract::State(_latency): axum::extract::State<LatencyInjector>,
) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn ws_handler_with_proxy(
    ws: WebSocketUpgrade,
    axum::extract::State(_proxy): axum::extract::State<WsProxyHandler>,
) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
        if let Ok(Message::Text(text)) = msg {
            // Echo the message back
            if socket.send(Message::Text(text)).await.is_err() {
                break;
            }
        }
    }
}
