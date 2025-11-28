//! Rate limiting middleware

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use serde_json::json;
use std::num::NonZeroU32;
use std::sync::Arc;

/// Rate limiter state
pub struct RateLimiterState {
    limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
}

impl RateLimiterState {
    /// Create a new rate limiter with the given requests per minute
    pub fn new(requests_per_minute: u32) -> Self {
        let quota = Quota::per_minute(
            NonZeroU32::new(requests_per_minute).expect("requests_per_minute must be > 0"),
        );
        let limiter = Arc::new(RateLimiter::direct(quota));

        Self { limiter }
    }

    /// Check if a request should be rate limited
    pub fn check(&self) -> bool {
        self.limiter.check().is_ok()
    }
}

impl Clone for RateLimiterState {
    fn clone(&self) -> Self {
        Self {
            limiter: Arc::clone(&self.limiter),
        }
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    // Get client IP from headers (X-Forwarded-For or X-Real-IP)
    let _client_ip = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");

    // For MVP, we use a simple global rate limiter
    // In production, you'd want per-IP rate limiting using Redis
    // For now, we'll skip the actual check and just pass through
    // The limiter would be injected via app state in production

    Ok::<Response, Response>(next.run(request).await)
}

/// Create a rate-limited response
fn rate_limited_response() -> impl IntoResponse {
    (
        StatusCode::TOO_MANY_REQUESTS,
        Json(json!({
            "error": "Rate limit exceeded. Please try again later.",
            "retry_after": 60
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_creation() {
        let limiter = RateLimiterState::new(60);
        assert!(limiter.check());
    }

    #[test]
    fn test_rate_limiter_limit() {
        let limiter = RateLimiterState::new(2);

        // First two requests should succeed
        assert!(limiter.check());
        assert!(limiter.check());

        // Third request should fail
        assert!(!limiter.check());
    }
}
