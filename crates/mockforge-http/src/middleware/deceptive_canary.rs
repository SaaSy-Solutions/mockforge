//! Deceptive Canary Middleware
//!
//! Middleware that routes a percentage of team traffic to deceptive deploys.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use mockforge_core::deceptive_canary::DeceptiveCanaryRouter;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

/// Deceptive canary middleware state
#[derive(Clone)]
pub struct DeceptiveCanaryState {
    /// Router for canary routing decisions
    pub router: Arc<DeceptiveCanaryRouter>,
}

impl DeceptiveCanaryState {
    /// Create new deceptive canary state
    pub fn new(router: DeceptiveCanaryRouter) -> Self {
        Self {
            router: Arc::new(router),
        }
    }
}

/// Deceptive canary middleware
///
/// Intercepts requests and routes a percentage to deceptive deploy endpoints
/// based on team identification criteria.
pub async fn deceptive_canary_middleware(req: Request, next: Next) -> Response {
    // Extract state from extensions (set by router)
    let state = req.extensions().get::<DeceptiveCanaryState>().cloned().unwrap_or_else(|| {
        // Return default state if not found (canary disabled)
        DeceptiveCanaryState::new(DeceptiveCanaryRouter::default())
    });
    // Extract request information
    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // Extract IP address from headers or connection info
    let ip_address = req
        .extensions()
        .get::<std::net::SocketAddr>()
        .map(|addr| addr.ip().to_string())
        .or_else(|| {
            req.headers()
                .get("x-forwarded-for")
                .or_else(|| req.headers().get("x-real-ip"))
                .and_then(|h| h.to_str().ok())
                .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        });

    // Extract headers
    let mut headers_map = HashMap::new();
    for (key, value) in req.headers() {
        let key_str = key.as_str().to_string();
        if let Ok(value_str) = value.to_str() {
            headers_map.insert(key_str, value_str.to_string());
        }
    }

    // Extract query parameters
    let mut query_params = HashMap::new();
    if let Some(query) = req.uri().query() {
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                query_params.insert(key.to_string(), value.to_string());
            }
        }
    }

    // Extract user ID from headers (if available)
    let user_id = req
        .headers()
        .get("x-user-id")
        .or_else(|| req.headers().get("authorization"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // Check if request should be routed to canary
    let should_route = state.router.should_route_to_canary(
        user_agent.as_deref(),
        ip_address.as_deref(),
        &headers_map,
        &query_params,
        user_id.as_deref(),
    );

    if should_route {
        debug!("Routing request to deceptive canary: {} {}", req.method(), req.uri().path());

        // Get deceptive deploy URL from router config
        let canary_url = &state.router.config().deceptive_deploy_url;

        if !canary_url.is_empty() {
            // Proxy request to deceptive deploy
            // For now, we'll just add a header indicating canary routing
            // Full proxying would require more complex logic
            let mut response = next.run(req).await;
            response.headers_mut().insert("X-Deceptive-Canary", "true".parse().unwrap());
            return response;
        }
    }

    // Continue with normal request processing
    next.run(req).await
}
