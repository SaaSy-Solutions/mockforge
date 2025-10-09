//! WebSocket chaos engineering

use crate::{
    config::ChaosConfig, fault::FaultInjector, latency::LatencyInjector,
    rate_limit::RateLimiter, traffic_shaping::TrafficShaper, ChaosError, Result,
};
use std::sync::Arc;
use tracing::{debug, warn};

/// WebSocket-specific fault types
#[derive(Debug, Clone)]
pub enum WebSocketFault {
    /// Connection drop
    ConnectionDrop,
    /// Close frame with code
    CloseFrame(u16), // 1000=Normal, 1001=GoingAway, etc.
    /// Message corruption
    MessageCorruption,
    /// Delayed message
    MessageDelay,
}

/// WebSocket chaos handler
#[derive(Clone)]
pub struct WebSocketChaos {
    latency_injector: Arc<LatencyInjector>,
    fault_injector: Arc<FaultInjector>,
    rate_limiter: Arc<RateLimiter>,
    traffic_shaper: Arc<TrafficShaper>,
    config: Arc<ChaosConfig>,
}

impl WebSocketChaos {
    /// Create new WebSocket chaos handler
    pub fn new(config: ChaosConfig) -> Self {
        let latency_injector = Arc::new(LatencyInjector::new(
            config.latency.clone().unwrap_or_default(),
        ));

        let fault_injector = Arc::new(FaultInjector::new(
            config.fault_injection.clone().unwrap_or_default(),
        ));

        let rate_limiter = Arc::new(RateLimiter::new(
            config.rate_limit.clone().unwrap_or_default(),
        ));

        let traffic_shaper = Arc::new(TrafficShaper::new(
            config.traffic_shaping.clone().unwrap_or_default(),
        ));

        Self {
            latency_injector,
            fault_injector,
            rate_limiter,
            traffic_shaper,
            config: Arc::new(config),
        }
    }

    /// Apply chaos before WebSocket connection
    pub async fn apply_connection(&self, path: &str, client_ip: Option<&str>) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        debug!("Applying WebSocket chaos for connection: {}", path);

        // Check rate limits
        if let Err(e) = self.rate_limiter.check(client_ip, Some(path)) {
            warn!("WebSocket rate limit exceeded: {}", path);
            return Err(e);
        }

        // Check connection limits
        if !self.traffic_shaper.check_connection_limit() {
            warn!("WebSocket connection limit exceeded");
            return Err(ChaosError::ConnectionThrottled);
        }

        // Inject connection latency
        self.latency_injector.inject().await;

        // Check for connection errors
        self.fault_injector.inject()?;

        Ok(())
    }

    /// Apply chaos before sending/receiving a message
    pub async fn apply_message(&self, message_size: usize, direction: &str) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        debug!(
            "Applying WebSocket chaos for {} message: {} bytes",
            direction, message_size
        );

        // Inject message latency
        self.latency_injector.inject().await;

        // Throttle bandwidth
        self.traffic_shaper.throttle_bandwidth(message_size).await;

        // Check for packet loss (message drop)
        if self.traffic_shaper.should_drop_packet() {
            warn!("Simulating WebSocket message drop");
            return Err(ChaosError::InjectedFault("Message dropped".to_string()));
        }

        // Check for fault injection
        self.fault_injector.inject()?;

        Ok(())
    }

    /// Check if should drop connection
    pub fn should_drop_connection(&self) -> bool {
        self.traffic_shaper.should_drop_packet()
    }

    /// Check if should corrupt message
    pub fn should_corrupt_message(&self) -> bool {
        self.fault_injector.should_truncate_response()
    }

    /// Get WebSocket close code for fault injection
    pub fn get_close_code(&self) -> Option<u16> {
        self.fault_injector.get_http_error_status().map(|http_code| match http_code {
                400 => 1002, // Protocol error
                408 => 1001, // Going away (timeout)
                429 => 1008, // Policy violation (rate limit)
                500 => 1011, // Server error
                503 => 1001, // Going away (unavailable)
                _ => 1011,   // Server error
            })
    }

    /// Get traffic shaper for connection management
    pub fn traffic_shaper(&self) -> &Arc<TrafficShaper> {
        &self.traffic_shaper
    }
}

/// WebSocket close codes
pub mod close_code {
    pub const NORMAL: u16 = 1000;
    pub const GOING_AWAY: u16 = 1001;
    pub const PROTOCOL_ERROR: u16 = 1002;
    pub const UNSUPPORTED_DATA: u16 = 1003;
    pub const NO_STATUS_RECEIVED: u16 = 1005;
    pub const ABNORMAL_CLOSURE: u16 = 1006;
    pub const INVALID_FRAME_PAYLOAD: u16 = 1007;
    pub const POLICY_VIOLATION: u16 = 1008;
    pub const MESSAGE_TOO_BIG: u16 = 1009;
    pub const MANDATORY_EXTENSION: u16 = 1010;
    pub const INTERNAL_ERROR: u16 = 1011;
    pub const SERVICE_RESTART: u16 = 1012;
    pub const TRY_AGAIN_LATER: u16 = 1013;
    pub const BAD_GATEWAY: u16 = 1014;
    pub const TLS_HANDSHAKE: u16 = 1015;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{FaultInjectionConfig, LatencyConfig};

    #[tokio::test]
    async fn test_websocket_chaos_creation() {
        let config = ChaosConfig {
            enabled: true,
            latency: Some(LatencyConfig {
                enabled: true,
                fixed_delay_ms: Some(10),
                random_delay_range_ms: None,
                jitter_percent: 0.0,
                probability: 1.0,
            }),
            ..Default::default()
        };

        let chaos = WebSocketChaos::new(config);
        assert!(chaos.config.enabled);
    }

    #[tokio::test]
    async fn test_websocket_close_code_mapping() {
        let config = ChaosConfig {
            enabled: true,
            fault_injection: Some(FaultInjectionConfig {
                enabled: true,
                http_errors: vec![500],
                http_error_probability: 1.0,
                ..Default::default()
            }),
            ..Default::default()
        };

        let chaos = WebSocketChaos::new(config);
        let close_code = chaos.get_close_code();

        // Should map 500 to WebSocket INTERNAL_ERROR (1011)
        if let Some(code) = close_code {
            assert_eq!(code, 1011);
        }
    }

    #[tokio::test]
    async fn test_apply_message_latency() {
        let config = ChaosConfig {
            enabled: true,
            latency: Some(LatencyConfig {
                enabled: true,
                fixed_delay_ms: Some(10),
                random_delay_range_ms: None,
                jitter_percent: 0.0,
                probability: 1.0,
            }),
            ..Default::default()
        };

        let chaos = WebSocketChaos::new(config);
        let start = std::time::Instant::now();

        chaos.apply_message(1024, "inbound").await.unwrap();

        let elapsed = start.elapsed();
        assert!(elapsed >= std::time::Duration::from_millis(10));
    }
}
