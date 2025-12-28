//! Rate limiting middleware
//!
//! Provides both global and per-user rate limiting with distributed support:
//! - Per-user limits are applied to authenticated requests (via JWT token)
//! - IP-based limits are used as fallback for unauthenticated requests
//! - Global limits provide a safety net for all requests
//! - When Redis is configured, rate limits are distributed across all instances
//! - Falls back to in-memory rate limiting when Redis is not available
//!
//! Configuration:
//! - `RATE_LIMIT_PER_MINUTE`: Global rate limit (default: 60)
//! - `RATE_LIMIT_PER_USER`: Per-user rate limit (default: 100)
//! - `RATE_LIMIT_CLEANUP_INTERVAL_SECS`: Cleanup interval for stale entries (default: 300)
//! - `REDIS_URL`: Redis connection URL for distributed rate limiting (optional)

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Extension, Json,
};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use super::trusted_proxy::extract_client_ip_from_headers;
use crate::redis::RedisPool;

/// JWT Claims structure for extracting user ID
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String, // user_id
    exp: usize,
}

/// Per-user rate limiter entry with last access time for cleanup
struct UserRateLimiterEntry {
    limiter: RateLimiter<NotKeyed, InMemoryState, DefaultClock>,
    last_access: Instant,
}

/// Rate limiter state with both global and per-user limits
/// Supports distributed rate limiting via Redis when configured
#[derive(Clone)]
pub struct RateLimiterState {
    /// Global rate limiter (fallback for in-memory mode)
    global_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    /// Per-user rate limiters (keyed by user_id or IP) - used when Redis is not available
    user_limiters: Arc<RwLock<HashMap<String, UserRateLimiterEntry>>>,
    /// Per-user quota configuration
    per_user_quota: Quota,
    /// Per-user rate limit value (requests per minute)
    per_user_limit: u32,
    /// Global rate limit value (requests per minute)
    global_limit: u32,
    /// Stale entry threshold (entries older than this are cleaned up)
    stale_threshold: Duration,
    /// JWT secret for token verification (optional, will skip user extraction if not set)
    jwt_secret: Option<String>,
    /// Redis pool for distributed rate limiting (optional)
    redis: Option<RedisPool>,
}

impl RateLimiterState {
    /// Create a new rate limiter with the given requests per minute (in-memory only)
    pub fn new(requests_per_minute: u32) -> Self {
        Self::new_internal(requests_per_minute, None)
    }

    /// Create a new rate limiter with Redis support for distributed rate limiting
    pub fn with_redis(requests_per_minute: u32, redis: RedisPool) -> Self {
        Self::new_internal(requests_per_minute, Some(redis))
    }

    /// Internal constructor
    fn new_internal(requests_per_minute: u32, redis: Option<RedisPool>) -> Self {
        let global_limit = if requests_per_minute == 0 {
            tracing::warn!("requests_per_minute was 0, defaulting to 60");
            60
        } else {
            requests_per_minute
        };

        let global_quota = Quota::per_minute(NonZeroU32::new(global_limit).unwrap());
        let global_limiter = Arc::new(RateLimiter::direct(global_quota));

        // Per-user rate limit from environment variable (default: 100 requests per minute)
        let per_user_limit: u32 = std::env::var("RATE_LIMIT_PER_USER")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(100);

        let per_user_limit = if per_user_limit == 0 {
            tracing::warn!("RATE_LIMIT_PER_USER was 0, defaulting to 100");
            100
        } else {
            per_user_limit
        };

        let per_user_quota = Quota::per_minute(NonZeroU32::new(per_user_limit).unwrap());

        // Cleanup interval from environment variable (default: 5 minutes)
        let cleanup_interval_secs: u64 = std::env::var("RATE_LIMIT_CLEANUP_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(300);

        // JWT secret for token verification
        let jwt_secret = std::env::var("JWT_SECRET").ok();

        let distributed = redis.is_some();
        tracing::info!(
            "Rate limiter initialized: global={}/min, per_user={}/min, cleanup_interval={}s, distributed={}",
            global_limit,
            per_user_limit,
            cleanup_interval_secs,
            distributed
        );

        let state = Self {
            global_limiter,
            user_limiters: Arc::new(RwLock::new(HashMap::new())),
            per_user_quota,
            per_user_limit,
            global_limit,
            stale_threshold: Duration::from_secs(cleanup_interval_secs),
            jwt_secret,
            redis,
        };

        // Start background cleanup task if a Tokio runtime is available
        // (may not be available in tests)
        // Only needed for in-memory mode; Redis handles its own expiry
        if state.redis.is_none() {
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let cleanup_state = state.clone();
                handle.spawn(async move {
                    cleanup_state.cleanup_loop().await;
                });
            }
        }

        state
    }

    /// Check if distributed rate limiting is enabled
    pub fn is_distributed(&self) -> bool {
        self.redis.is_some()
    }

    /// Check if a request should be rate limited (global only)
    /// Returns true if the request is allowed, false if rate limited
    pub fn check(&self) -> bool {
        self.global_limiter.check().is_ok()
    }

    /// Check if a request should be rate limited for a specific user/IP
    /// Returns true if the request is allowed, false if rate limited
    ///
    /// When Redis is configured, uses distributed rate limiting via Redis counters.
    /// Falls back to in-memory rate limiting when Redis is not available.
    pub async fn check_user(&self, key: &str) -> bool {
        // Use Redis-based distributed rate limiting if available
        if let Some(ref redis) = self.redis {
            return self.check_user_redis(redis, key).await;
        }

        // Fall back to in-memory rate limiting
        self.check_user_in_memory(key).await
    }

    /// Check rate limit using Redis (distributed)
    async fn check_user_redis(&self, redis: &RedisPool, key: &str) -> bool {
        // Check global limit first (using a shared key)
        let global_key = "ratelimit:global";
        match redis.increment_with_expiry(global_key, 60).await {
            Ok(count) => {
                if count > self.global_limit as i64 {
                    tracing::debug!(
                        key = global_key,
                        count = count,
                        limit = self.global_limit,
                        "Global rate limit exceeded (Redis)"
                    );
                    return false;
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Redis global rate limit check failed: {}, falling back to in-memory",
                    e
                );
                if !self.global_limiter.check().is_ok() {
                    return false;
                }
            }
        }

        // Check per-user/IP limit
        let user_key = format!("ratelimit:{}", key);
        match redis.increment_with_expiry(&user_key, 60).await {
            Ok(count) => {
                if count > self.per_user_limit as i64 {
                    tracing::debug!(
                        key = user_key,
                        count = count,
                        limit = self.per_user_limit,
                        "Per-user rate limit exceeded (Redis)"
                    );
                    return false;
                }
                true
            }
            Err(e) => {
                tracing::warn!(
                    "Redis user rate limit check failed: {}, falling back to in-memory",
                    e
                );
                // Fallback to in-memory check
                self.check_user_in_memory(key).await
            }
        }
    }

    /// Check rate limit using in-memory state
    async fn check_user_in_memory(&self, key: &str) -> bool {
        // First check global limiter
        if !self.global_limiter.check().is_ok() {
            return false;
        }

        // Then check per-user limiter
        let mut limiters = self.user_limiters.write().await;

        if let Some(entry) = limiters.get_mut(key) {
            entry.last_access = Instant::now();
            entry.limiter.check().is_ok()
        } else {
            // Create new limiter for this user/IP
            let limiter = RateLimiter::direct(self.per_user_quota);
            let result = limiter.check().is_ok();
            limiters.insert(
                key.to_string(),
                UserRateLimiterEntry {
                    limiter,
                    last_access: Instant::now(),
                },
            );
            result
        }
    }

    /// Extract user ID from JWT token in Authorization header
    fn extract_user_id(&self, headers: &HeaderMap) -> Option<String> {
        let jwt_secret = self.jwt_secret.as_ref()?;

        let auth_header = headers.get("Authorization")?.to_str().ok()?;

        // Extract Bearer token
        let token = auth_header.strip_prefix("Bearer ")?;

        // Decode and verify JWT
        let validation = Validation::default();
        let token_data =
            decode::<Claims>(token, &DecodingKey::from_secret(jwt_secret.as_bytes()), &validation)
                .ok()?;

        Some(token_data.claims.sub)
    }

    /// Get rate limit key from request (user_id or IP)
    ///
    /// Uses the trusted proxy module to safely extract client IP.
    /// Note: In middleware context without socket access, we use header-based
    /// extraction which assumes the request is from a trusted proxy.
    pub fn get_rate_limit_key(&self, headers: &HeaderMap) -> String {
        // Try to extract user ID from JWT token
        if let Some(user_id) = self.extract_user_id(headers) {
            return format!("user:{}", user_id);
        }

        // Fall back to IP address using trusted proxy extraction
        let ip = extract_client_ip_from_headers(headers);
        format!("ip:{}", ip)
    }

    /// Background task to clean up stale rate limiter entries
    async fn cleanup_loop(&self) {
        let cleanup_interval = self.stale_threshold;
        loop {
            tokio::time::sleep(cleanup_interval).await;
            self.cleanup_stale_entries().await;
        }
    }

    /// Remove rate limiter entries that haven't been accessed recently
    async fn cleanup_stale_entries(&self) {
        let mut limiters = self.user_limiters.write().await;
        let now = Instant::now();
        let initial_count = limiters.len();

        limiters.retain(|_, entry| now.duration_since(entry.last_access) < self.stale_threshold);

        let removed_count = initial_count - limiters.len();
        if removed_count > 0 {
            tracing::debug!(
                "Cleaned up {} stale rate limiter entries, {} remaining",
                removed_count,
                limiters.len()
            );
        }
    }

    /// Get the number of active rate limiter entries (for monitoring)
    pub async fn active_entries_count(&self) -> usize {
        self.user_limiters.read().await.len()
    }
}

/// Rate limiting middleware that enforces request limits
/// Uses Extension to get the rate limiter (added via layer in main.rs)
pub async fn rate_limit_middleware(
    Extension(limiter): Extension<RateLimiterState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Get rate limit key (user_id or IP)
    let rate_limit_key = limiter.get_rate_limit_key(&headers);

    // Check the rate limiter for this key
    if !limiter.check_user(&rate_limit_key).await {
        let is_authenticated = rate_limit_key.starts_with("user:");
        tracing::warn!(
            rate_limit_key = %rate_limit_key,
            path = %request.uri().path(),
            authenticated = is_authenticated,
            "Rate limit exceeded"
        );
        return Err(rate_limited_response().into_response());
    }

    Ok(next.run(request).await)
}

/// Create a rate-limited response
fn rate_limited_response() -> impl IntoResponse {
    (
        StatusCode::TOO_MANY_REQUESTS,
        [("Retry-After", "60")],
        Json(json!({
            "error": {
                "code": "RATE_LIMIT_EXCEEDED",
                "message": "Too many requests. Please try again later.",
                "retry_after_seconds": 60
            }
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
    fn test_rate_limiter_global_limit() {
        let limiter = RateLimiterState::new(2);

        // First two requests should succeed
        assert!(limiter.check());
        assert!(limiter.check());

        // Third request should fail
        assert!(!limiter.check());
    }

    #[tokio::test]
    async fn test_per_user_rate_limiter() {
        // Set up environment for test
        std::env::set_var("RATE_LIMIT_PER_USER", "2");

        let limiter = RateLimiterState::new(1000); // High global limit

        // First two requests for user1 should succeed
        assert!(limiter.check_user("user:user1").await);
        assert!(limiter.check_user("user:user1").await);

        // Third request for user1 should fail
        assert!(!limiter.check_user("user:user1").await);

        // But user2 should still be allowed
        assert!(limiter.check_user("user:user2").await);

        // Clean up
        std::env::remove_var("RATE_LIMIT_PER_USER");
    }

    #[tokio::test]
    async fn test_ip_rate_limiter() {
        std::env::set_var("RATE_LIMIT_PER_USER", "2");

        let limiter = RateLimiterState::new(1000);

        // First two requests from IP should succeed
        assert!(limiter.check_user("ip:192.168.1.1").await);
        assert!(limiter.check_user("ip:192.168.1.1").await);

        // Third request should fail
        assert!(!limiter.check_user("ip:192.168.1.1").await);

        // Different IP should be allowed
        assert!(limiter.check_user("ip:192.168.1.2").await);

        std::env::remove_var("RATE_LIMIT_PER_USER");
    }

    #[test]
    fn test_get_rate_limit_key_ip_fallback() {
        let limiter = RateLimiterState::new(60);

        // No auth header, should fall back to IP
        let mut headers = HeaderMap::new();
        headers.insert("X-Forwarded-For", "192.168.1.100".parse().unwrap());

        let key = limiter.get_rate_limit_key(&headers);
        assert_eq!(key, "ip:192.168.1.100");
    }

    #[test]
    fn test_get_rate_limit_key_x_real_ip() {
        let limiter = RateLimiterState::new(60);

        let mut headers = HeaderMap::new();
        headers.insert("X-Real-IP", "10.0.0.50".parse().unwrap());

        let key = limiter.get_rate_limit_key(&headers);
        assert_eq!(key, "ip:10.0.0.50");
    }

    #[test]
    fn test_get_rate_limit_key_forwarded_for_multiple() {
        let limiter = RateLimiterState::new(60);

        // X-Forwarded-For can contain multiple IPs, use the first one (client)
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Forwarded-For",
            "203.0.113.195, 70.41.3.18, 150.172.238.178".parse().unwrap(),
        );

        let key = limiter.get_rate_limit_key(&headers);
        assert_eq!(key, "ip:203.0.113.195");
    }

    #[test]
    fn test_get_rate_limit_key_unknown() {
        let limiter = RateLimiterState::new(60);

        let headers = HeaderMap::new();
        let key = limiter.get_rate_limit_key(&headers);
        assert_eq!(key, "ip:unknown");
    }

    #[tokio::test]
    async fn test_active_entries_count() {
        std::env::set_var("RATE_LIMIT_PER_USER", "100");

        let limiter = RateLimiterState::new(1000);

        assert_eq!(limiter.active_entries_count().await, 0);

        limiter.check_user("user:user1").await;
        assert_eq!(limiter.active_entries_count().await, 1);

        limiter.check_user("user:user2").await;
        assert_eq!(limiter.active_entries_count().await, 2);

        // Same user shouldn't create new entry
        limiter.check_user("user:user1").await;
        assert_eq!(limiter.active_entries_count().await, 2);

        std::env::remove_var("RATE_LIMIT_PER_USER");
    }

    #[tokio::test]
    async fn test_global_limit_takes_precedence() {
        std::env::set_var("RATE_LIMIT_PER_USER", "100");

        // Very low global limit
        let limiter = RateLimiterState::new(1);

        // First request succeeds
        assert!(limiter.check_user("user:user1").await);

        // Second request fails due to global limit (even though per-user limit is 100)
        assert!(!limiter.check_user("user:user2").await);

        std::env::remove_var("RATE_LIMIT_PER_USER");
    }
}
