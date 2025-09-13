use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::Path;
use axum::{response::IntoResponse, routing::get, Router};
use mockforge_core::{latency::LatencyInjector, LatencyProfile, WsProxyHandler};
#[cfg(feature = "data-faker")]
use mockforge_data::provider::register_core_faker_provider;
use regex::Regex;
use std::fs;
use std::future::IntoFuture;
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
pub fn router_with_proxy(ws_proxy_handler: WsProxyHandler) -> Router {
    #[cfg(feature = "data-faker")]
    register_core_faker_provider();

    Router::new()
        .route("/ws", get(ws_handler_with_proxy_root))
        .route("/ws/{*path}", get(ws_handler_with_proxy))
        .with_state(ws_proxy_handler)
}

/// Build the WebSocket router with both latency injector and proxy handler
pub fn router_with_latency_and_proxy(
    latency_injector: LatencyInjector,
    ws_proxy_handler: WsProxyHandler,
) -> Router {
    #[cfg(feature = "data-faker")]
    register_core_faker_provider();

    Router::new()
        .route("/ws", get(ws_handler_with_latency_and_proxy_root))
        .route("/ws/{*path}", get(ws_handler_with_latency_and_proxy))
        .with_state((latency_injector, ws_proxy_handler))
}

pub async fn start(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    start_with_latency(port, None).await
}

pub async fn start_with_latency(
    port: u16,
    latency_profile: Option<LatencyProfile>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Use shared server utilities for consistent address creation
    let addr = mockforge_core::wildcard_socket_addr(port);
    info!("WS listening on {}", addr);

    if let Some(profile) = latency_profile {
        let latency_injector = LatencyInjector::new(profile, Default::default());
        let app = router_with_latency(latency_injector);
        axum::serve(tokio::net::TcpListener::bind(addr).await?, app.into_make_service())
            .into_future()
            .await?;
    } else {
        let app = router();
        axum::serve(tokio::net::TcpListener::bind(addr).await?, app.into_make_service())
            .into_future()
            .await?;
    }

    Ok(())
}

/// Start WebSocket server with proxy configuration
pub async fn start_with_proxy(
    port: u16,
    ws_proxy_handler: WsProxyHandler,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = mockforge_core::wildcard_socket_addr(port);
    info!("WS listening on {} with proxy support", addr);

    let app = router_with_proxy(ws_proxy_handler);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app.into_make_service())
        .into_future()
        .await?;

    Ok(())
}

/// Start WebSocket server with both latency injection and proxy support
pub async fn start_with_latency_and_proxy(
    port: u16,
    latency_profile: Option<LatencyProfile>,
    ws_proxy_handler: WsProxyHandler,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = mockforge_core::wildcard_socket_addr(port);
    info!("WS listening on {} with latency injection and proxy support", addr);

    let latency_injector = latency_profile.map(|profile| LatencyInjector::new(profile, Default::default()));

    let app = if let Some(injector) = latency_injector {
        router_with_latency_and_proxy(injector, ws_proxy_handler)
    } else {
        router_with_proxy(ws_proxy_handler)
    };

    axum::serve(tokio::net::TcpListener::bind(addr).await?, app.into_make_service())
        .into_future()
        .await?;

    Ok(())
}

async fn ws_handler_no_state(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| run_ws(socket, None, None))
}

async fn ws_handler_with_state(
    ws: WebSocketUpgrade,
    axum::extract::State(latency_injector): axum::extract::State<LatencyInjector>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| run_ws(socket, Some(latency_injector), None))
}

/// WebSocket handler with proxy support for root /ws path
async fn ws_handler_with_proxy_root(
    ws: WebSocketUpgrade,
    axum::extract::State(ws_proxy_handler): axum::extract::State<WsProxyHandler>,
) -> impl IntoResponse {
    let full_path = "/ws".to_string();
    info!("WebSocket handler (root): full_path='{}'", full_path);
    ws.on_upgrade(move |socket| run_ws_with_path(socket, None, Some(ws_proxy_handler), full_path))
}

/// WebSocket handler with proxy support - extracts path from request
async fn ws_handler_with_proxy(
    Path(path): Path<String>,
    ws: WebSocketUpgrade,
    axum::extract::State(ws_proxy_handler): axum::extract::State<WsProxyHandler>,
) -> impl IntoResponse {
    // Reconstruct the full path by prepending /ws
    let full_path = format!("/ws/{}", path);
    info!("WebSocket handler: captured path='{}', full_path='{}'", path, full_path);
    ws.on_upgrade(move |socket| run_ws_with_path(socket, None, Some(ws_proxy_handler), full_path))
}

/// WebSocket handler with both latency injector and proxy support for root /ws path
async fn ws_handler_with_latency_and_proxy_root(
    ws: WebSocketUpgrade,
    axum::extract::State((latency_injector, ws_proxy_handler)): axum::extract::State<(
        LatencyInjector,
        WsProxyHandler,
    )>,
) -> impl IntoResponse {
    let full_path = "/ws".to_string();
    info!("WebSocket handler (latency+proxy root): full_path='{}'", full_path);
    ws.on_upgrade(move |socket| {
        run_ws_with_path(socket, Some(latency_injector), Some(ws_proxy_handler), full_path)
    })
}

/// WebSocket handler with both latency injector and proxy support - extracts path from request
async fn ws_handler_with_latency_and_proxy(
    Path(path): Path<String>,
    ws: WebSocketUpgrade,
    axum::extract::State((latency_injector, ws_proxy_handler)): axum::extract::State<(
        LatencyInjector,
        WsProxyHandler,
    )>,
) -> impl IntoResponse {
    // Reconstruct the full path by prepending /ws
    let full_path = format!("/ws/{}", path);
    info!(
        "WebSocket handler (latency+proxy): captured path='{}', full_path='{}'",
        path, full_path
    );
    ws.on_upgrade(move |socket| {
        run_ws_with_path(socket, Some(latency_injector), Some(ws_proxy_handler), full_path)
    })
}

async fn run_ws(
    socket: WebSocket,
    latency_injector: Option<LatencyInjector>,
    ws_proxy_handler: Option<WsProxyHandler>,
) {
    // For backward compatibility, use default path when no path is provided
    run_ws_with_path(socket, latency_injector, ws_proxy_handler, "/ws".to_string()).await
}

/// Run WebSocket connection with explicit path for proxy routing
async fn run_ws_with_path(
    socket: WebSocket,
    latency_injector: Option<LatencyInjector>,
    ws_proxy_handler: Option<WsProxyHandler>,
    path: String,
) {
    // Check if we should proxy this connection
    if let Some(proxy_handler) = ws_proxy_handler {
        info!("Checking if WebSocket connection at path '{}' should be proxied", path);
        info!(
            "Proxy config enabled: {}, passthrough_by_default: {}",
            proxy_handler.config.enabled, proxy_handler.config.passthrough_by_default
        );
        info!("Proxy rules: {:?}", proxy_handler.config.rules);

        if proxy_handler.config.should_proxy(&path) {
            info!("Proxying WebSocket connection at path '{}' to upstream", path);
            if let Err(e) = proxy_handler.proxy_connection(&path, socket).await {
                error!("WebSocket proxy failed for path '{}': {}", path, e);
            }
            return;
        } else {
            info!("WebSocket connection at path '{}' will be handled locally", path);
        }
    }

    run_ws_local(socket, latency_injector).await
}

async fn run_ws_local(mut socket: WebSocket, latency_injector: Option<LatencyInjector>) {
    // If MOCKFORGE_WS_REPLAY_FILE is set, drive scripted replay with optional waitFor gates.
    if let Ok(path) = std::env::var("MOCKFORGE_WS_REPLAY_FILE") {
        if let Ok(text) = fs::read_to_string(&path) {
            let mut pending: Option<Regex> = None;
            for line in text.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                    if v.get("dir").and_then(|x| x.as_str()) == Some("out") {
                        if let Some(w) = v.get("waitFor").and_then(|x| x.as_str()) {
                            if let Ok(re) = Regex::new(w) {
                                pending = Some(re);
                            }
                        }
                        if let Some(re) = &pending {
                            while let Some(Ok(Message::Text(inmsg))) = socket.recv().await {
                                if re.is_match(&inmsg) {
                                    break;
                                }
                            }
                            pending = None;
                        }
                        if let Some(t) = v.get("text").and_then(|x| x.as_str()) {
                            let mut out = t.to_string();
                            let expand = std::env::var("MOCKFORGE_RESPONSE_TEMPLATE_EXPAND")
                                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                                .unwrap_or(false);
                            if expand {
                                out = mockforge_core::templating::expand_str(&out);
                            }

                            // Inject latency before sending message
                            if let Some(ref injector) = latency_injector {
                                let _ = injector.inject_latency(&[]).await;
                            }

                            let _ = socket.send(Message::Text(out.into())).await;
                        }
                    }
                }
            }
        }
        return;
    }

    // Echo mode
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(t) => {
                // Inject latency before sending echo response
                if let Some(ref injector) = latency_injector {
                    let _ = injector.inject_latency(&[]).await;
                }

                let _ = socket.send(Message::Text(format!("echo: {}", t).into())).await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}
