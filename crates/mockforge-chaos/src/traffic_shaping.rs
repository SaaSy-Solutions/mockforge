//! Traffic shaping for simulating network conditions

use crate::config::TrafficShapingConfig;
use rand::Rng;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::debug;

/// Traffic shaper for simulating network conditions
#[derive(Clone)]
pub struct TrafficShaper {
    config: TrafficShapingConfig,
    active_connections: Arc<AtomicU32>,
}

impl TrafficShaper {
    /// Create a new traffic shaper
    pub fn new(config: TrafficShapingConfig) -> Self {
        Self {
            config,
            active_connections: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Check if traffic shaping is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Check if packet should be dropped (simulating packet loss)
    pub fn should_drop_packet(&self) -> bool {
        if !self.config.enabled || self.config.packet_loss_percent == 0.0 {
            return false;
        }

        let mut rng = rand::thread_rng();
        let drop = rng.gen::<f64>() * 100.0 < self.config.packet_loss_percent;

        if drop {
            debug!("Simulating packet loss");
        }

        drop
    }

    /// Apply bandwidth throttling for a given data size
    pub async fn throttle_bandwidth(&self, bytes: usize) {
        if !self.config.enabled || self.config.bandwidth_limit_bps == 0 {
            return;
        }

        // Calculate delay needed to enforce bandwidth limit
        let delay_secs = bytes as f64 / self.config.bandwidth_limit_bps as f64;
        let delay_ms = (delay_secs * 1000.0) as u64;

        if delay_ms > 0 {
            debug!("Throttling bandwidth: {}ms delay for {} bytes", delay_ms, bytes);
            sleep(Duration::from_millis(delay_ms)).await;
        }
    }

    /// Check connection limit and increment if allowed
    pub fn check_connection_limit(&self) -> bool {
        if !self.config.enabled || self.config.max_connections == 0 {
            return true; // No limit
        }

        let current = self.active_connections.load(Ordering::SeqCst);
        if current >= self.config.max_connections {
            debug!("Connection limit reached: {}/{}", current, self.config.max_connections);
            return false;
        }

        self.active_connections.fetch_add(1, Ordering::SeqCst);
        debug!("Connection accepted: {}/{}", current + 1, self.config.max_connections);
        true
    }

    /// Release a connection slot
    pub fn release_connection(&self) {
        if self.config.enabled && self.config.max_connections > 0 {
            let prev = self.active_connections.fetch_sub(1, Ordering::SeqCst);
            debug!("Connection released: {}/{}", prev - 1, self.config.max_connections);
        }
    }

    /// Get active connections count
    pub fn active_connections(&self) -> u32 {
        self.active_connections.load(Ordering::SeqCst)
    }

    /// Get connection timeout
    pub fn connection_timeout(&self) -> Duration {
        Duration::from_millis(self.config.connection_timeout_ms)
    }

    /// Get configuration
    pub fn config(&self) -> &TrafficShapingConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: TrafficShapingConfig) {
        self.config = config;
    }
}

/// RAII guard for connection tracking
pub struct ConnectionGuard {
    shaper: TrafficShaper,
}

impl ConnectionGuard {
    /// Create a new connection guard
    pub fn new(shaper: TrafficShaper) -> Option<Self> {
        if shaper.check_connection_limit() {
            Some(Self { shaper })
        } else {
            None
        }
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.shaper.release_connection();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_loss() {
        let config = TrafficShapingConfig {
            enabled: true,
            packet_loss_percent: 50.0, // 50% loss
            ..Default::default()
        };

        let shaper = TrafficShaper::new(config);

        // Test 1000 packets, should lose approximately 50%
        let mut dropped = 0;
        for _ in 0..1000 {
            if shaper.should_drop_packet() {
                dropped += 1;
            }
        }

        // Allow some variance (40-60%)
        assert!(dropped >= 400 && dropped <= 600);
    }

    #[test]
    fn test_no_packet_loss_when_disabled() {
        let config = TrafficShapingConfig {
            enabled: false,
            packet_loss_percent: 100.0,
            ..Default::default()
        };

        let shaper = TrafficShaper::new(config);

        for _ in 0..100 {
            assert!(!shaper.should_drop_packet());
        }
    }

    #[tokio::test]
    async fn test_bandwidth_throttling() {
        let config = TrafficShapingConfig {
            enabled: true,
            bandwidth_limit_bps: 1000, // 1KB/s
            ..Default::default()
        };

        let shaper = TrafficShaper::new(config);

        let start = std::time::Instant::now();
        shaper.throttle_bandwidth(1000).await; // 1KB
        let elapsed = start.elapsed();

        // Should take approximately 1 second
        assert!(elapsed >= Duration::from_millis(900));
    }

    #[test]
    fn test_connection_limit() {
        let config = TrafficShapingConfig {
            enabled: true,
            max_connections: 2,
            ..Default::default()
        };

        let shaper = TrafficShaper::new(config);

        // Should allow first two connections
        assert!(shaper.check_connection_limit());
        assert!(shaper.check_connection_limit());

        // Should block third connection
        assert!(!shaper.check_connection_limit());

        // Release one connection
        shaper.release_connection();

        // Should allow another connection now
        assert!(shaper.check_connection_limit());
    }

    #[test]
    fn test_connection_guard() {
        let config = TrafficShapingConfig {
            enabled: true,
            max_connections: 1,
            ..Default::default()
        };

        let shaper = TrafficShaper::new(config);

        {
            let _guard = ConnectionGuard::new(shaper.clone());
            assert!(shaper.active_connections() == 1);

            // Should fail to create second guard
            assert!(ConnectionGuard::new(shaper.clone()).is_none());
        }

        // Guard dropped, should be able to create new one
        assert!(shaper.active_connections() == 0);
        assert!(ConnectionGuard::new(shaper.clone()).is_some());
    }
}
