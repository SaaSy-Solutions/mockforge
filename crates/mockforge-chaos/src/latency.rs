//! Latency injection for simulating network delays

use crate::config::LatencyConfig;
use rand::Rng;
use std::time::Duration;
use tokio::time::sleep;
use tracing::debug;

/// Latency injector for simulating network delays
#[derive(Clone)]
pub struct LatencyInjector {
    config: LatencyConfig,
}

impl LatencyInjector {
    /// Create a new latency injector
    pub fn new(config: LatencyConfig) -> Self {
        Self { config }
    }

    /// Check if latency injection is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Inject latency based on configuration
    pub async fn inject(&self) {
        if !self.config.enabled {
            return;
        }

        // Check probability
        let mut rng = rand::thread_rng();
        if rng.gen::<f64>() > self.config.probability {
            return;
        }

        let delay_ms = self.calculate_delay();
        if delay_ms > 0 {
            debug!("Injecting latency: {}ms", delay_ms);
            sleep(Duration::from_millis(delay_ms)).await;
        }
    }

    /// Calculate the delay in milliseconds
    fn calculate_delay(&self) -> u64 {
        let mut rng = rand::thread_rng();

        // Base delay
        let base_delay = if let Some(fixed) = self.config.fixed_delay_ms {
            fixed
        } else if let Some((min, max)) = self.config.random_delay_range_ms {
            rng.gen_range(min..=max)
        } else {
            0
        };

        // Apply jitter
        if self.config.jitter_percent > 0.0 {
            let jitter = (base_delay as f64 * self.config.jitter_percent / 100.0) as u64;
            let jitter_offset = rng.gen_range(0..=jitter);
            if rng.gen_bool(0.5) {
                base_delay + jitter_offset
            } else {
                base_delay.saturating_sub(jitter_offset)
            }
        } else {
            base_delay
        }
    }

    /// Get configuration
    pub fn config(&self) -> &LatencyConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: LatencyConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_fixed_delay() {
        let config = LatencyConfig {
            enabled: true,
            fixed_delay_ms: Some(100),
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            probability: 1.0,
        };

        let injector = LatencyInjector::new(config);
        let delay = injector.calculate_delay();
        assert_eq!(delay, 100);
    }

    #[test]
    fn test_calculate_random_delay() {
        let config = LatencyConfig {
            enabled: true,
            fixed_delay_ms: None,
            random_delay_range_ms: Some((50, 150)),
            jitter_percent: 0.0,
            probability: 1.0,
        };

        let injector = LatencyInjector::new(config);
        for _ in 0..100 {
            let delay = injector.calculate_delay();
            assert!(delay >= 50 && delay <= 150);
        }
    }

    #[test]
    fn test_jitter() {
        let config = LatencyConfig {
            enabled: true,
            fixed_delay_ms: Some(100),
            random_delay_range_ms: None,
            jitter_percent: 10.0, // 10% jitter = +/- 10ms
            probability: 1.0,
        };

        let injector = LatencyInjector::new(config);
        for _ in 0..100 {
            let delay = injector.calculate_delay();
            // Should be within 90-110ms range
            assert!(delay >= 90 && delay <= 110);
        }
    }

    #[tokio::test]
    async fn test_inject_latency() {
        let config = LatencyConfig {
            enabled: true,
            fixed_delay_ms: Some(10),
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            probability: 1.0,
        };

        let injector = LatencyInjector::new(config);
        let start = std::time::Instant::now();
        injector.inject().await;
        let elapsed = start.elapsed();

        // Should have delayed at least 10ms
        assert!(elapsed >= Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_probability() {
        let config = LatencyConfig {
            enabled: true,
            fixed_delay_ms: Some(10),
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            probability: 0.0, // Never inject
        };

        let injector = LatencyInjector::new(config);
        let start = std::time::Instant::now();
        injector.inject().await;
        let elapsed = start.elapsed();

        // Should not have delayed
        assert!(elapsed < Duration::from_millis(5));
    }
}
