use axum::{routing::get, Router, extract::{Path, Query}, response::IntoResponse, http::StatusCode};
use serde::Deserialize;
use tracing::*;
use std::{net::SocketAddr, fs};

pub async fn start(port: u16, spec: Option<String>) {
    if let Some(p) = spec {
        match fs::read_to_string(&p) {
            Ok(s) => info!("Loaded spec: {} ({} bytes)", p, s.len()),
            Err(e) => warn!("Failed reading spec {}: {}", p, e),
        }
    }
    let app = Router::new()
        .route("/ping", get(|| async { "pong" }))
        .fallback(handler);
    let addr = SocketAddr::from(([0,0,0,0], port));
    info!("HTTP listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app).await.unwrap();
}

#[derive(Deserialize)]
struct AnyQuery(std::collections::HashMap<String, String>);

async fn handler(Path(path): Path<String>, Query(q): Query<AnyQuery>) -> impl IntoResponse {
    let body = serde_json::json!({
        "ok": true,
        "path": format!("/{}", path),
        "query": q.0,
        "ts": chrono::Utc::now().to_rfc3339(),
        "id": uuid::Uuid::new_v4().to_string()
    });
    (StatusCode::OK, axum::Json(body))
}
