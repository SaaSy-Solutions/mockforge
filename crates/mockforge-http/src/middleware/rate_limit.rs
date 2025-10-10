//! Global rate limiting middleware for HTTP server
//!
//! This module provides rate limiting to protect against abuse and DDoS attacks

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;
use tracing::warn;

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Requests per minute
    pub requests_per_minute: u32,
    /// Burst capacity
    pub burst: u32,
    /// Enable per-IP rate limiting
    pub per_ip: bool,
    /// Enable per-endpoint rate limiting
    pub per_endpoint: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 100,
            burst: 200,
            per_ip: true,
            per_endpoint: false,
        }
    }
}

/// Global rate limiter state
pub struct GlobalRateLimiter {
    limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    config: RateLimitConfig,
}

impl GlobalRateLimiter {
    /// Create a new global rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        let quota = Quota::per_minute(
            NonZeroU32::new(config.requests_per_minute).unwrap_or(NonZeroU32::new(100).unwrap()),
        )
        .allow_burst(NonZeroU32::new(config.burst).unwrap_or(NonZeroU32::new(200).unwrap()));

        let limiter = Arc::new(RateLimiter::direct(quota));

        Self { limiter, config }
    }

    /// Check if request should be rate limited
    pub fn check_rate_limit(&self) -> bool {
        self.limiter.check().is_ok()
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(state): axum::extract::State<crate::HttpServerState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Get rate limiter from app state
    if let Some(limiter) = &state.rate_limiter {
        if !limiter.check_rate_limit() {
            warn!("Rate limit exceeded for IP: {}", addr.ip());
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
    } else {
        // No rate limiter configured, allow request to proceed
        tracing::debug!("No rate limiter configured, allowing request");
    }

    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_creation() {
        let config = RateLimitConfig::default();
        let limiter = GlobalRateLimiter::new(config);

        // Should allow first request
        assert!(limiter.check_rate_limit());
    }

    #[test]
    fn test_rate_limiter_burst() {
        let config = RateLimitConfig {
            requests_per_minute: 10,
            burst: 5,
            per_ip: false,
            per_endpoint: false,
        };

        let limiter = GlobalRateLimiter::new(config);

        // Should allow burst requests
        for _ in 0..5 {
            assert!(limiter.check_rate_limit(), "Burst request should be allowed");
        }
    }

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.requests_per_minute, 100);
        assert_eq!(config.burst, 200);
        assert!(config.per_ip);
        assert!(!config.per_endpoint);
    }
}
