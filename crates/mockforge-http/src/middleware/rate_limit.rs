//! Global rate limiting middleware for HTTP server
//!
//! This module provides rate limiting to protect against abuse and DDoS attacks
//! and adds production-like rate limit headers to responses.

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{HeaderName, HeaderValue, Request, StatusCode},
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
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
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

/// Rate limit quota information for headers
#[derive(Debug, Clone)]
pub struct RateLimitQuota {
    /// Maximum requests per minute (limit)
    pub limit: u32,
    /// Remaining requests in current window (approximate)
    pub remaining: u32,
    /// Unix timestamp when the rate limit resets
    pub reset: u64,
}

/// Global rate limiter state
pub struct GlobalRateLimiter {
    limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    config: RateLimitConfig,
    /// Track window start time for reset calculation
    window_start: Arc<Mutex<SystemTime>>,
    /// Track approximate remaining requests
    remaining_counter: Arc<Mutex<u32>>,
}

impl GlobalRateLimiter {
    /// Create a new global rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        let quota = Quota::per_minute(
            NonZeroU32::new(config.requests_per_minute).unwrap_or(NonZeroU32::new(100).unwrap()),
        )
        .allow_burst(NonZeroU32::new(config.burst).unwrap_or(NonZeroU32::new(200).unwrap()));

        let limiter = Arc::new(RateLimiter::direct(quota));
        let window_start = Arc::new(Mutex::new(SystemTime::now()));
        let remaining_counter = Arc::new(Mutex::new(config.requests_per_minute));

        Self {
            limiter,
            config,
            window_start,
            remaining_counter,
        }
    }

    /// Check if request should be rate limited
    pub fn check_rate_limit(&self) -> bool {
        self.limiter.check().is_ok()
    }

    /// Get rate limit quota information for headers
    ///
    /// Returns information about the current rate limit state including
    /// limit, remaining requests, and reset timestamp.
    pub fn get_quota_info(&self) -> RateLimitQuota {
        let now = SystemTime::now();
        let mut window_start = self.window_start.lock().unwrap();
        let mut remaining = self.remaining_counter.lock().unwrap();

        // Check if we need to reset the window (every minute)
        let window_duration = Duration::from_secs(60);
        if now.duration_since(*window_start).unwrap_or(Duration::ZERO) >= window_duration {
            // Reset window
            *window_start = now;
            *remaining = self.config.requests_per_minute;
        }

        // Decrement remaining if we successfully checked (approximate)
        // Note: This is approximate because governor's token bucket
        // may have different internal state, but it's good enough for headers
        let current_remaining = *remaining;
        if current_remaining > 0 {
            *remaining = current_remaining.saturating_sub(1);
        }

        // Calculate reset timestamp (start of next window)
        let reset_timestamp =
            window_start.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO).as_secs() + 60; // Add 60 seconds for next window

        RateLimitQuota {
            limit: self.config.requests_per_minute,
            remaining: current_remaining,
            reset: reset_timestamp,
        }
    }
}

/// Rate limiting middleware
///
/// This middleware:
/// 1. Checks if the request should be rate limited
/// 2. Adds rate limit headers to successful responses (for deceptive deploy)
pub async fn rate_limit_middleware(
    State(state): axum::extract::State<crate::HttpServerState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Get rate limiter from app state
    let quota_info = if let Some(limiter) = &state.rate_limiter {
        // Check rate limit
        if !limiter.check_rate_limit() {
            warn!("Rate limit exceeded for IP: {}", addr.ip());
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }

        // Get quota information for headers
        Some(limiter.get_quota_info())
    } else {
        // No rate limiter configured, allow request to proceed
        tracing::debug!("No rate limiter configured, allowing request");
        None
    };

    // Process request and get response
    let mut response = next.run(req).await;

    // Add rate limit headers to response if we have quota info
    // This makes the mock API look more like production
    if let Some(quota) = quota_info {
        // Add X-Rate-Limit-Limit header
        let limit_name = HeaderName::from_static("x-rate-limit-limit");
        if let Ok(limit_value) = HeaderValue::from_str(&quota.limit.to_string()) {
            response.headers_mut().insert(limit_name, limit_value);
        }

        // Add X-Rate-Limit-Remaining header
        let remaining_name = HeaderName::from_static("x-rate-limit-remaining");
        if let Ok(remaining_value) = HeaderValue::from_str(&quota.remaining.to_string()) {
            response.headers_mut().insert(remaining_name, remaining_value);
        }

        // Add X-Rate-Limit-Reset header (Unix timestamp)
        let reset_name = HeaderName::from_static("x-rate-limit-reset");
        if let Ok(reset_value) = HeaderValue::from_str(&quota.reset.to_string()) {
            response.headers_mut().insert(reset_name, reset_value);
        }
    }

    Ok(response)
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
