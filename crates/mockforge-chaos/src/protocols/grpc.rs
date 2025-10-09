//! gRPC chaos engineering

use crate::{
    config::ChaosConfig, fault::FaultInjector, latency::LatencyInjector,
    rate_limit::RateLimiter, traffic_shaping::TrafficShaper, ChaosError, Result,
};
use std::sync::Arc;
use tracing::{debug, warn};

/// gRPC-specific fault types
#[derive(Debug, Clone)]
pub enum GrpcFault {
    /// gRPC status code error
    StatusCode(i32), // 0=OK, 1=CANCELLED, 2=UNKNOWN, etc.
    /// Stream interruption
    StreamInterruption,
    /// Metadata corruption
    MetadataCorruption,
    /// Message corruption
    MessageCorruption,
}

/// gRPC chaos handler
#[derive(Clone)]
pub struct GrpcChaos {
    latency_injector: Arc<LatencyInjector>,
    fault_injector: Arc<FaultInjector>,
    rate_limiter: Arc<RateLimiter>,
    traffic_shaper: Arc<TrafficShaper>,
    config: Arc<ChaosConfig>,
}

impl GrpcChaos {
    /// Create new gRPC chaos handler
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

    /// Apply chaos before gRPC request processing
    pub async fn apply_pre_request(
        &self,
        service: &str,
        method: &str,
        client_ip: Option<&str>,
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let endpoint = format!("{}/{}", service, method);
        debug!("Applying gRPC chaos for: {}", endpoint);

        // Check rate limits
        if let Err(e) = self.rate_limiter.check(client_ip, Some(&endpoint)) {
            warn!("gRPC rate limit exceeded: {}", endpoint);
            return Err(e);
        }

        // Check connection limits
        if !self.traffic_shaper.check_connection_limit() {
            warn!("gRPC connection limit exceeded");
            return Err(ChaosError::ConnectionThrottled);
        }

        // Inject latency
        self.latency_injector.inject().await;

        // Check for fault injection
        self.fault_injector.inject()?;

        Ok(())
    }

    /// Apply chaos after gRPC response
    pub async fn apply_post_response(&self, message_size: usize) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Throttle bandwidth based on message size
        self.traffic_shaper.throttle_bandwidth(message_size).await;

        // Check for packet loss (simulated)
        if self.traffic_shaper.should_drop_packet() {
            warn!("Simulating gRPC packet loss");
            return Err(ChaosError::InjectedFault("Packet loss".to_string()));
        }

        Ok(())
    }

    /// Get gRPC status code for fault injection
    pub fn get_grpc_status_code(&self) -> Option<i32> {
        self.fault_injector.get_http_error_status().map(|http_code| match http_code {
                400 => 3,  // INVALID_ARGUMENT
                401 => 16, // UNAUTHENTICATED
                403 => 7,  // PERMISSION_DENIED
                404 => 5,  // NOT_FOUND
                429 => 8,  // RESOURCE_EXHAUSTED
                500 => 13, // INTERNAL
                501 => 12, // UNIMPLEMENTED
                503 => 14, // UNAVAILABLE
                504 => 4,  // DEADLINE_EXCEEDED
                _ => 2,    // UNKNOWN
            })
    }

    /// Check if should interrupt stream
    pub fn should_interrupt_stream(&self) -> bool {
        self.fault_injector.should_truncate_response()
    }

    /// Get traffic shaper for connection management
    pub fn traffic_shaper(&self) -> &Arc<TrafficShaper> {
        &self.traffic_shaper
    }
}

/// gRPC status codes
pub mod status {
    pub const OK: i32 = 0;
    pub const CANCELLED: i32 = 1;
    pub const UNKNOWN: i32 = 2;
    pub const INVALID_ARGUMENT: i32 = 3;
    pub const DEADLINE_EXCEEDED: i32 = 4;
    pub const NOT_FOUND: i32 = 5;
    pub const ALREADY_EXISTS: i32 = 6;
    pub const PERMISSION_DENIED: i32 = 7;
    pub const RESOURCE_EXHAUSTED: i32 = 8;
    pub const FAILED_PRECONDITION: i32 = 9;
    pub const ABORTED: i32 = 10;
    pub const OUT_OF_RANGE: i32 = 11;
    pub const UNIMPLEMENTED: i32 = 12;
    pub const INTERNAL: i32 = 13;
    pub const UNAVAILABLE: i32 = 14;
    pub const DATA_LOSS: i32 = 15;
    pub const UNAUTHENTICATED: i32 = 16;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{FaultInjectionConfig, LatencyConfig};

    #[tokio::test]
    async fn test_grpc_chaos_creation() {
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

        let chaos = GrpcChaos::new(config);
        assert!(chaos.config.enabled);
    }

    #[tokio::test]
    async fn test_grpc_status_code_mapping() {
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

        let chaos = GrpcChaos::new(config);
        let status = chaos.get_grpc_status_code();

        // Should map 500 to gRPC INTERNAL (13)
        if let Some(code) = status {
            assert_eq!(code, 13);
        }
    }
}
