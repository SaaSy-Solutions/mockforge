use axum::extract::ws::{Message, WebSocket};
use axum::{extract::WebSocketUpgrade, response::IntoResponse, routing::get, Router};
use regex::Regex;
use std::fs;
use tracing::*;

pub async fn start(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = Router::new().route("/ws", get(ws_handler));

    // Use shared server utilities for consistent address creation
    let addr = mockforge_core::wildcard_socket_addr(port);
    info!("WS listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(run_ws)
}

async fn run_ws(mut socket: WebSocket) {
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
                            let _ = socket.send(Message::Text(t.to_string().into())).await;
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
                let _ = socket.send(Message::Text(format!("echo: {}", t).into())).await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}
