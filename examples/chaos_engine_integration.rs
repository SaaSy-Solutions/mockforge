//! Example showing how to integrate the ChaosEngine into request processing
//!
//! This example demonstrates the pattern for using the random chaos mode
//! in HTTP middleware or request handlers.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use mockforge_core::{ChaosConfig, ChaosEngine, ChaosResult, Config};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Application state containing the chaos engine
#[derive(Clone)]
struct AppState {
    chaos_engine: Option<Arc<ChaosEngine>>,
}

/// Chaos middleware that applies random errors and delays
async fn chaos_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Check if chaos engine is enabled
    if let Some(engine) = &state.chaos_engine {
        // Process the request through the chaos engine
        match engine.process_request(&[]).await {
            ChaosResult::Success => {
                // Continue normally
                next.run(request).await
            }
            ChaosResult::Error { status_code, message } => {
                // Inject error response
                let status = StatusCode::from_u16(status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                (status, message).into_response()
            }
            ChaosResult::Delay { delay_ms } => {
                // Inject delay then continue
                sleep(Duration::from_millis(delay_ms)).await;
                next.run(request).await
            }
            ChaosResult::Timeout { timeout_ms: _ } => {
                // Simulate timeout
                (StatusCode::GATEWAY_TIMEOUT, "Request timeout (chaos-injected)").into_response()
            }
        }
    } else {
        // No chaos engine, continue normally
        next.run(request).await
    }
}

/// Sample handler
async fn hello_handler() -> &'static str {
    "Hello, World!"
}

/// Build a router with chaos middleware
fn build_router_with_chaos(config: &Config) -> Router {
    // Create chaos engine from config if enabled
    let chaos_engine = config.create_chaos_engine().map(Arc::new);

    let state = AppState { chaos_engine };

    Router::new()
        .route("/hello", get(hello_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            chaos_middleware,
        ))
        .with_state(state)
}

#[tokio::main]
async fn main() {
    // Example 1: Create config with chaos mode enabled
    let mut config = Config::default();
    config.chaos_random = Some(
        ChaosConfig::new(0.1, 0.3) // 10% errors, 30% delays
            .with_delay_range(100, 500)
    );

    // Build router with chaos middleware
    let app = build_router_with_chaos(&config);

    println!("🚀 Starting server with chaos mode enabled");
    println!("   Error rate: 10%");
    println!("   Delay rate: 30% (100-500ms)");

    // Run server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("📡 Listening on http://127.0.0.1:3000");

    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chaos_engine_creation() {
        let mut config = Config::default();
        assert!(config.create_chaos_engine().is_none());

        config.chaos_random = Some(ChaosConfig::new(0.5, 0.5));
        assert!(config.create_chaos_engine().is_some());
    }

    #[tokio::test]
    async fn test_chaos_engine_integration() {
        let mut config = Config::default();
        config.chaos_random = Some(ChaosConfig::new(1.0, 0.0)); // 100% errors, 0% delays

        let engine = config.create_chaos_engine().unwrap();

        // With 100% error rate, we should always get errors
        let result = engine.process_request(&[]).await;
        assert!(matches!(result, ChaosResult::Error { .. }));
    }
}
