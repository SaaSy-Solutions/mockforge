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
            NonZeroU32::new(config.requests_per_minute)
                .unwrap_or(NonZeroU32::new(100).expect("constant 100 is non-zero")),
        )
        .allow_burst(
            NonZeroU32::new(config.burst)
                .unwrap_or(NonZeroU32::new(200).expect("constant 200 is non-zero")),
        );

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
        let mut window_start =
            self.window_start.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut remaining =
            self.remaining_counter.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

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
/// 3. Returns 429 with Retry-After header when rate limited
pub async fn rate_limit_middleware(
    State(state): axum::extract::State<crate::HttpServerState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<Body>,
    next: Next,
) -> Response {
    // Get rate limiter from app state
    let quota_info = if let Some(limiter) = &state.rate_limiter {
        // Check rate limit
        if !limiter.check_rate_limit() {
            warn!("Rate limit exceeded for IP: {}", addr.ip());
            // Return 429 with Retry-After header per HTTP spec
            let mut response = Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .body(Body::from("Too Many Requests"))
                .unwrap_or_else(|_| Response::new(Body::from("Too Many Requests")));

            // Add Retry-After header (60 seconds = 1 minute window)
            if let Ok(retry_after) = HeaderValue::from_static("60").try_into() {
                response
                    .headers_mut()
                    .insert(HeaderName::from_static("retry-after"), retry_after);
            }

            // Add rate limit headers to the 429 response
            let quota = limiter.get_quota_info();
            if let Ok(limit_value) = HeaderValue::from_str(&quota.limit.to_string()) {
                response
                    .headers_mut()
                    .insert(HeaderName::from_static("x-rate-limit-limit"), limit_value);
            }
            if let Ok(remaining_value) = HeaderValue::from_str("0") {
                response
                    .headers_mut()
                    .insert(HeaderName::from_static("x-rate-limit-remaining"), remaining_value);
            }
            if let Ok(reset_value) = HeaderValue::from_str(&quota.reset.to_string()) {
                response
                    .headers_mut()
                    .insert(HeaderName::from_static("x-rate-limit-reset"), reset_value);
            }

            return response;
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

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== RateLimitConfig Tests ====================

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.requests_per_minute, 100);
        assert_eq!(config.burst, 200);
        assert!(config.per_ip);
        assert!(!config.per_endpoint);
    }

    #[test]
    fn test_rate_limit_config_custom() {
        let config = RateLimitConfig {
            requests_per_minute: 50,
            burst: 100,
            per_ip: false,
            per_endpoint: true,
        };

        assert_eq!(config.requests_per_minute, 50);
        assert_eq!(config.burst, 100);
        assert!(!config.per_ip);
        assert!(config.per_endpoint);
    }

    #[test]
    fn test_rate_limit_config_clone() {
        let config = RateLimitConfig {
            requests_per_minute: 75,
            burst: 150,
            per_ip: true,
            per_endpoint: true,
        };

        let cloned = config.clone();

        assert_eq!(cloned.requests_per_minute, config.requests_per_minute);
        assert_eq!(cloned.burst, config.burst);
        assert_eq!(cloned.per_ip, config.per_ip);
        assert_eq!(cloned.per_endpoint, config.per_endpoint);
    }

    #[test]
    fn test_rate_limit_config_debug() {
        let config = RateLimitConfig::default();
        let debug_str = format!("{:?}", config);

        assert!(debug_str.contains("requests_per_minute"));
        assert!(debug_str.contains("burst"));
        assert!(debug_str.contains("per_ip"));
        assert!(debug_str.contains("per_endpoint"));
    }

    // ==================== RateLimitQuota Tests ====================

    #[test]
    fn test_rate_limit_quota_creation() {
        let quota = RateLimitQuota {
            limit: 100,
            remaining: 50,
            reset: 1234567890,
        };

        assert_eq!(quota.limit, 100);
        assert_eq!(quota.remaining, 50);
        assert_eq!(quota.reset, 1234567890);
    }

    #[test]
    fn test_rate_limit_quota_clone() {
        let quota = RateLimitQuota {
            limit: 200,
            remaining: 175,
            reset: 9876543210,
        };

        let cloned = quota.clone();

        assert_eq!(cloned.limit, quota.limit);
        assert_eq!(cloned.remaining, quota.remaining);
        assert_eq!(cloned.reset, quota.reset);
    }

    #[test]
    fn test_rate_limit_quota_debug() {
        let quota = RateLimitQuota {
            limit: 100,
            remaining: 50,
            reset: 1234567890,
        };

        let debug_str = format!("{:?}", quota);

        assert!(debug_str.contains("limit"));
        assert!(debug_str.contains("remaining"));
        assert!(debug_str.contains("reset"));
    }

    // ==================== GlobalRateLimiter Tests ====================

    #[test]
    fn test_rate_limiter_creation() {
        let config = RateLimitConfig::default();
        let limiter = GlobalRateLimiter::new(config);

        // Should allow first request
        assert!(limiter.check_rate_limit());
    }

    #[test]
    fn test_rate_limiter_with_custom_config() {
        let config = RateLimitConfig {
            requests_per_minute: 60,
            burst: 10,
            per_ip: false,
            per_endpoint: false,
        };

        let limiter = GlobalRateLimiter::new(config);
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
    fn test_rate_limiter_multiple_requests() {
        let config = RateLimitConfig {
            requests_per_minute: 1000,
            burst: 100,
            per_ip: false,
            per_endpoint: false,
        };

        let limiter = GlobalRateLimiter::new(config);

        // Should allow many requests within burst limit
        for i in 0..50 {
            assert!(limiter.check_rate_limit(), "Request {} should be allowed", i);
        }
    }

    #[test]
    fn test_get_quota_info() {
        let config = RateLimitConfig {
            requests_per_minute: 100,
            burst: 50,
            per_ip: false,
            per_endpoint: false,
        };

        let limiter = GlobalRateLimiter::new(config);

        let quota = limiter.get_quota_info();

        assert_eq!(quota.limit, 100);
        assert!(quota.remaining > 0);
        assert!(quota.reset > 0);
    }

    #[test]
    fn test_quota_info_limit_matches_config() {
        let config = RateLimitConfig {
            requests_per_minute: 500,
            burst: 100,
            per_ip: false,
            per_endpoint: false,
        };

        let limiter = GlobalRateLimiter::new(config);
        let quota = limiter.get_quota_info();

        assert_eq!(quota.limit, 500);
    }

    #[test]
    fn test_quota_decrements_remaining() {
        let config = RateLimitConfig {
            requests_per_minute: 100,
            burst: 50,
            per_ip: false,
            per_endpoint: false,
        };

        let limiter = GlobalRateLimiter::new(config);

        let first_quota = limiter.get_quota_info();
        let second_quota = limiter.get_quota_info();

        // Remaining should decrement between calls
        assert!(second_quota.remaining <= first_quota.remaining, "Remaining should not increase");
    }

    #[test]
    fn test_quota_reset_timestamp_is_future() {
        let config = RateLimitConfig::default();
        let limiter = GlobalRateLimiter::new(config);

        let quota = limiter.get_quota_info();

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        // Reset timestamp should be in the future (approximately 60 seconds from window start)
        assert!(quota.reset >= now, "Reset timestamp should be >= current time");
        assert!(quota.reset <= now + 120, "Reset timestamp should be within 2 minutes");
    }

    #[test]
    fn test_rate_limiter_high_burst() {
        let config = RateLimitConfig {
            requests_per_minute: 10,
            burst: 1000, // Very high burst
            per_ip: false,
            per_endpoint: false,
        };

        let limiter = GlobalRateLimiter::new(config);

        // Should allow many requests due to high burst
        for _ in 0..100 {
            assert!(limiter.check_rate_limit());
        }
    }

    #[test]
    fn test_rate_limiter_low_limit() {
        let config = RateLimitConfig {
            requests_per_minute: 1,
            burst: 1,
            per_ip: false,
            per_endpoint: false,
        };

        let limiter = GlobalRateLimiter::new(config);

        // First request should succeed
        assert!(limiter.check_rate_limit());
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_config_with_zero_values_handled() {
        // Zero values should be handled gracefully by governor
        let config = RateLimitConfig {
            requests_per_minute: 0, // Will use default (100)
            burst: 0,               // Will use default (200)
            per_ip: false,
            per_endpoint: false,
        };

        // Should not panic - NonZeroU32::new(0) returns None, unwrap_or handles it
        let limiter = GlobalRateLimiter::new(config);
        assert!(limiter.check_rate_limit());
    }

    #[test]
    fn test_multiple_quota_calls_same_limiter() {
        let config = RateLimitConfig::default();
        let limiter = GlobalRateLimiter::new(config);

        // Call get_quota_info multiple times
        let quotas: Vec<RateLimitQuota> = (0..5).map(|_| limiter.get_quota_info()).collect();

        // All should have same limit
        for quota in &quotas {
            assert_eq!(quota.limit, 100);
        }

        // Reset timestamps should be similar (within same window)
        let first_reset = quotas[0].reset;
        for quota in &quotas {
            assert!(
                (quota.reset as i64 - first_reset as i64).abs() <= 1,
                "Reset timestamps should be within 1 second of each other"
            );
        }
    }

    #[test]
    fn test_quota_remaining_never_negative() {
        let config = RateLimitConfig {
            requests_per_minute: 5,
            burst: 5,
            per_ip: false,
            per_endpoint: false,
        };

        let limiter = GlobalRateLimiter::new(config);

        // Call many times to exhaust quota
        for _ in 0..20 {
            let quota = limiter.get_quota_info();
            // Remaining should never go below 0 due to saturating_sub
            assert!(quota.remaining <= 100, "Remaining should be reasonable");
        }
    }
}
