//! Rate limiting for controlling request throughput

use crate::{config::RateLimitConfig, ChaosError, Result};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use nonzero_ext::nonzero;
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::debug;

/// Rate limiter for controlling request throughput
#[derive(Clone)]
pub struct RateLimiter {
    config: RateLimitConfig,
    global_limiter: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    ip_limiters: Arc<RwLock<HashMap<String, Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>>>>,
    endpoint_limiters: Arc<RwLock<HashMap<String, Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>>>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        let quota = Quota::per_second(
            NonZeroU32::new(config.requests_per_second).unwrap_or(nonzero!(100u32))
        )
        .allow_burst(
            NonZeroU32::new(config.burst_size).unwrap_or(nonzero!(10u32))
        );

        let global_limiter = Arc::new(GovernorRateLimiter::direct(quota));

        Self {
            config,
            global_limiter,
            ip_limiters: Arc::new(RwLock::new(HashMap::new())),
            endpoint_limiters: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if rate limiting is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Check global rate limit
    pub fn check_global(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        if self.global_limiter.check().is_err() {
            debug!("Global rate limit exceeded");
            return Err(ChaosError::RateLimitExceeded);
        }

        Ok(())
    }

    /// Check per-IP rate limit
    pub fn check_ip(&self, ip: &str) -> Result<()> {
        if !self.config.enabled || !self.config.per_ip {
            return Ok(());
        }

        let limiter = {
            let mut limiters = self.ip_limiters.write();
            limiters
                .entry(ip.to_string())
                .or_insert_with(|| {
                    let quota = Quota::per_second(
                        NonZeroU32::new(self.config.requests_per_second).unwrap_or(nonzero!(100u32))
                    )
                    .allow_burst(
                        NonZeroU32::new(self.config.burst_size).unwrap_or(nonzero!(10u32))
                    );
                    Arc::new(GovernorRateLimiter::direct(quota))
                })
                .clone()
        };

        if limiter.check().is_err() {
            debug!("Per-IP rate limit exceeded for {}", ip);
            return Err(ChaosError::RateLimitExceeded);
        }

        Ok(())
    }

    /// Check per-endpoint rate limit
    pub fn check_endpoint(&self, endpoint: &str) -> Result<()> {
        if !self.config.enabled || !self.config.per_endpoint {
            return Ok(());
        }

        let limiter = {
            let mut limiters = self.endpoint_limiters.write();
            limiters
                .entry(endpoint.to_string())
                .or_insert_with(|| {
                    let quota = Quota::per_second(
                        NonZeroU32::new(self.config.requests_per_second).unwrap_or(nonzero!(100u32))
                    )
                    .allow_burst(
                        NonZeroU32::new(self.config.burst_size).unwrap_or(nonzero!(10u32))
                    );
                    Arc::new(GovernorRateLimiter::direct(quota))
                })
                .clone()
        };

        if limiter.check().is_err() {
            debug!("Per-endpoint rate limit exceeded for {}", endpoint);
            return Err(ChaosError::RateLimitExceeded);
        }

        Ok(())
    }

    /// Check all applicable rate limits
    pub fn check(&self, ip: Option<&str>, endpoint: Option<&str>) -> Result<()> {
        self.check_global()?;

        if let Some(ip_addr) = ip {
            self.check_ip(ip_addr)?;
        }

        if let Some(endpoint_path) = endpoint {
            self.check_endpoint(endpoint_path)?;
        }

        Ok(())
    }

    /// Get configuration
    pub fn config(&self) -> &RateLimitConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: RateLimitConfig) {
        self.config = config;
        // Note: Updating limiters would require recreating them
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_rate_limit() {
        let config = RateLimitConfig {
            enabled: true,
            requests_per_second: 1,
            burst_size: 2,  // burst_size is the total capacity, not additional requests
            per_ip: false,
            per_endpoint: false,
        };

        let limiter = RateLimiter::new(config);

        // First request should succeed
        assert!(limiter.check_global().is_ok());

        // Burst request should succeed
        assert!(limiter.check_global().is_ok());

        // Next request should fail (exceeded rate + burst)
        assert!(matches!(
            limiter.check_global(),
            Err(ChaosError::RateLimitExceeded)
        ));
    }

    #[test]
    fn test_disabled_rate_limit() {
        let config = RateLimitConfig {
            enabled: false,
            ..Default::default()
        };

        let limiter = RateLimiter::new(config);

        // Should always succeed when disabled
        for _ in 0..1000 {
            assert!(limiter.check_global().is_ok());
        }
    }

    #[test]
    fn test_per_ip_rate_limit() {
        let config = RateLimitConfig {
            enabled: true,
            requests_per_second: 1,
            burst_size: 2,  // burst_size is the total capacity, not additional requests
            per_ip: true,
            per_endpoint: false,
        };

        let limiter = RateLimiter::new(config);

        // Requests from different IPs should be independent
        assert!(limiter.check_ip("192.168.1.1").is_ok());
        assert!(limiter.check_ip("192.168.1.2").is_ok());

        // Burst
        assert!(limiter.check_ip("192.168.1.1").is_ok());
        assert!(limiter.check_ip("192.168.1.2").is_ok());

        // Should fail for each IP independently
        assert!(matches!(
            limiter.check_ip("192.168.1.1"),
            Err(ChaosError::RateLimitExceeded)
        ));
        assert!(matches!(
            limiter.check_ip("192.168.1.2"),
            Err(ChaosError::RateLimitExceeded)
        ));
    }
}
