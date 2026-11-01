//! Shutdown must finish an active response instead of dropping its server future.

use axum::{routing::get, Router};
use std::{sync::Arc, time::Duration};
use tokio::sync::{oneshot, Notify};

#[tokio::test]
async fn shutdown_drains_an_in_flight_http_response() {
    assert_shutdown_drains_response(None).await;
}

#[tokio::test]
async fn shutdown_drains_an_in_flight_chaos_listener_response() {
    let chaos = mockforge_chaos::ChaosConfig {
        enabled: true,
        ..Default::default()
    };
    assert_shutdown_drains_response(Some(Arc::new(tokio::sync::RwLock::new(chaos)))).await;
}

async fn assert_shutdown_drains_response(
    chaos: Option<Arc<tokio::sync::RwLock<mockforge_chaos::ChaosConfig>>>,
) {
    let started = Arc::new(Notify::new());
    let handler_started = started.clone();
    let app = Router::new().route(
        "/slow",
        get(move || {
            let started = handler_started.clone();
            async move {
                started.notify_one();
                tokio::time::sleep(Duration::from_millis(250)).await;
                "completed"
            }
        }),
    );
    let (bound_tx, bound_rx) = oneshot::channel();
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let server = tokio::spawn(mockforge_http::serve_router_with_tls_notify_chaos_shutdown(
        0,
        app,
        None,
        Some(bound_tx),
        chaos,
        async move {
            let _ = shutdown_rx.await;
        },
    ));
    let port = tokio::time::timeout(Duration::from_secs(5), bound_rx)
        .await
        .expect("listener should bind")
        .expect("listener should report its actual port");
    let request = tokio::spawn(async move {
        reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("local test client")
            .get(format!("http://127.0.0.1:{port}/slow"))
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .expect("active request should survive shutdown")
    });
    tokio::time::timeout(Duration::from_secs(5), started.notified())
        .await
        .expect("request should reach the handler");
    shutdown_tx.send(()).expect("server should still be listening");
    let response = request.await.expect("request task should finish");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(response.text().await.unwrap(), "completed");
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .expect("drained server should stop")
        .expect("server task should not panic")
        .expect("server should shut down successfully");
}
