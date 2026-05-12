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
    /// Returns the delay amount in milliseconds that was injected (0 if no delay was injected)
    pub async fn inject(&self) -> u64 {
        self.inject_with_breakdown().await.0
    }

    /// Inject latency and report `(total_delay_ms, base_delay_ms, jitter_abs_ms)`.
    ///
    /// `base_delay_ms` is what the fixed/random configuration alone would have
    /// produced; `jitter_abs_ms` is the absolute jitter offset that was applied
    /// on top (always ≥ 0). This split lets the chaos middleware record a
    /// separate `jitter` fault counter so the TUI Chaos screen and `/metrics`
    /// surface jitter activity independently of total latency. Issue #79 —
    /// Srikanth's round-3 reply.
    pub async fn inject_with_breakdown(&self) -> (u64, u64, u64) {
        if !self.config.enabled {
            return (0, 0, 0);
        }

        let mut rng = rand::rng();
        if rng.random::<f64>() > self.config.probability {
            return (0, 0, 0);
        }

        let (delay_ms, base_ms, jitter_abs) = self.calculate_delay_with_breakdown();
        if delay_ms > 0 {
            debug!("Injecting latency: {}ms (base={}, jitter={})", delay_ms, base_ms, jitter_abs);
            sleep(Duration::from_millis(delay_ms)).await;
        }

        (delay_ms, base_ms, jitter_abs)
    }

    /// Calculate the delay in milliseconds. Only used by unit tests now —
    /// production callers go through `calculate_delay_with_breakdown` so they
    /// can also see the jitter component.
    #[cfg(test)]
    fn calculate_delay(&self) -> u64 {
        self.calculate_delay_with_breakdown().0
    }

    /// Calculate `(total, base, jitter_abs)` without sleeping. `jitter_abs` is
    /// always non-negative — it's the magnitude of the jitter offset before its
    /// sign was randomized.
    fn calculate_delay_with_breakdown(&self) -> (u64, u64, u64) {
        let mut rng = rand::rng();

        let base_delay = if let Some(fixed) = self.config.fixed_delay_ms {
            fixed
        } else if let Some((min, max)) = self.config.random_delay_range_ms {
            rng.random_range(min..=max)
        } else {
            0
        };

        if self.config.jitter_percent > 0.0 {
            let jitter = (base_delay as f64 * self.config.jitter_percent / 100.0) as u64;
            let jitter_offset = rng.random_range(0..=jitter);
            let total = if rng.random_bool(0.5) {
                base_delay + jitter_offset
            } else {
                base_delay.saturating_sub(jitter_offset)
            };
            (total, base_delay, jitter_offset)
        } else {
            (base_delay, base_delay, 0)
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
            assert!((50..=150).contains(&delay));
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
            assert!((90..=110).contains(&delay));
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
        let delay_ms = injector.inject().await;
        let elapsed = start.elapsed();

        // Should have delayed at least 10ms
        assert!(elapsed >= Duration::from_millis(10));
        // Should return the delay amount
        assert_eq!(delay_ms, 10);
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
        let delay_ms = injector.inject().await;
        let elapsed = start.elapsed();

        // Should not have delayed
        assert!(elapsed < Duration::from_millis(5));
        // Should return 0 when probability prevents injection
        assert_eq!(delay_ms, 0);
    }
}
