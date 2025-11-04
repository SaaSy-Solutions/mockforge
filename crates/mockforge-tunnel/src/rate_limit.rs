//! Rate limiting for tunnel server
//!
//! This module provides rate limiting to protect the tunnel server from abuse
//! and DDoS attacks. It supports both global and per-IP rate limiting.

use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Global requests per minute
    pub global_requests_per_minute: u32,
    /// Per-IP requests per minute
    pub per_ip_requests_per_minute: u32,
    /// Burst capacity
    pub burst: u32,
    /// Enable per-IP rate limiting
    pub per_ip: bool,
    /// Enable rate limiting
    pub enabled: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            global_requests_per_minute: 1000,
            per_ip_requests_per_minute: 100,
            burst: 200,
            per_ip: true,
            enabled: true,
        }
    }
}

/// Rate limiter state
pub struct TunnelRateLimiter {
    global_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    ip_limiters:
        Arc<RwLock<HashMap<String, Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>>>,
    config: RateLimitConfig,
}

impl TunnelRateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        let global_quota = Quota::per_minute(
            NonZeroU32::new(config.global_requests_per_minute)
                .unwrap_or(NonZeroU32::new(1000).unwrap()),
        )
        .allow_burst(NonZeroU32::new(config.burst).unwrap_or(NonZeroU32::new(200).unwrap()));

        let global_limiter = Arc::new(RateLimiter::direct(global_quota));

        Self {
            global_limiter,
            ip_limiters: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Check if request should be rate limited
    pub async fn check_rate_limit(&self, ip: &str) -> bool {
        if !self.config.enabled {
            return true;
        }

        // Check global rate limit
        if self.global_limiter.check().is_err() {
            warn!("Global rate limit exceeded");
            return false;
        }

        // Check per-IP rate limit if enabled
        if self.config.per_ip {
            let ip_limiter = {
                let limiters = self.ip_limiters.read().await;
                limiters.get(ip).cloned()
            };

            let ip_limiter = if let Some(limiter) = ip_limiter {
                limiter
            } else {
                // Create new limiter for this IP
                let ip_quota = Quota::per_minute(
                    NonZeroU32::new(self.config.per_ip_requests_per_minute)
                        .unwrap_or(NonZeroU32::new(100).unwrap()),
                )
                .allow_burst(
                    NonZeroU32::new(self.config.burst).unwrap_or(NonZeroU32::new(200).unwrap()),
                );

                let new_limiter = Arc::new(RateLimiter::direct(ip_quota));
                let mut limiters = self.ip_limiters.write().await;
                limiters.insert(ip.to_string(), Arc::clone(&new_limiter));
                new_limiter
            };

            if ip_limiter.check().is_err() {
                warn!("Rate limit exceeded for IP: {}", ip);
                return false;
            }
        }

        true
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(state): State<Arc<TunnelRateLimiter>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract client IP from headers (for proxied requests) or connection
    let client_ip = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| addr.ip().to_string());

    if !state.check_rate_limit(&client_ip).await {
        debug!("Rate limit exceeded for IP: {}", client_ip);
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_creation() {
        let config = RateLimitConfig::default();
        let limiter = TunnelRateLimiter::new(config);

        // Should allow first request
        assert!(limiter.check_rate_limit("127.0.0.1").await);
    }

    #[tokio::test]
    async fn test_rate_limiter_per_ip() {
        let config = RateLimitConfig {
            per_ip_requests_per_minute: 10,
            per_ip: true,
            enabled: true,
            ..Default::default()
        };

        let limiter = TunnelRateLimiter::new(config);

        // Should allow requests from different IPs
        assert!(limiter.check_rate_limit("127.0.0.1").await);
        assert!(limiter.check_rate_limit("127.0.0.2").await);
    }
}
