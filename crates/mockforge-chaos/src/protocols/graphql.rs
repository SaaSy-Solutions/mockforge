//! GraphQL chaos engineering

use crate::{
    config::ChaosConfig, fault::FaultInjector, latency::LatencyInjector,
    rate_limit::RateLimiter, traffic_shaping::TrafficShaper, ChaosError, Result,
};
use std::sync::Arc;
use tracing::{debug, warn};

/// GraphQL-specific fault types
#[derive(Debug, Clone)]
pub enum GraphQLFault {
    /// GraphQL error in response
    GraphQLError(String),
    /// Field resolution error
    FieldError(String),
    /// Partial data (some fields null)
    PartialData,
    /// Slow resolver
    SlowResolver,
}

/// GraphQL chaos handler
#[derive(Clone)]
pub struct GraphQLChaos {
    latency_injector: Arc<LatencyInjector>,
    fault_injector: Arc<FaultInjector>,
    rate_limiter: Arc<RateLimiter>,
    traffic_shaper: Arc<TrafficShaper>,
    config: Arc<ChaosConfig>,
}

impl GraphQLChaos {
    /// Create new GraphQL chaos handler
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

    /// Apply chaos before GraphQL query execution
    pub async fn apply_pre_query(
        &self,
        operation_type: &str,
        operation_name: Option<&str>,
        client_ip: Option<&str>,
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let endpoint = format!(
            "/graphql/{}",
            operation_name.unwrap_or("anonymous")
        );
        debug!(
            "Applying GraphQL chaos for: {} {}",
            operation_type, endpoint
        );

        // Check rate limits
        if let Err(e) = self.rate_limiter.check(client_ip, Some(&endpoint)) {
            warn!("GraphQL rate limit exceeded: {}", endpoint);
            return Err(e);
        }

        // Check connection limits
        if !self.traffic_shaper.check_connection_limit() {
            warn!("GraphQL connection limit exceeded");
            return Err(ChaosError::ConnectionThrottled);
        }

        // Inject query latency
        self.latency_injector.inject().await;

        // Check for fault injection
        self.fault_injector.inject()?;

        Ok(())
    }

    /// Apply chaos after GraphQL query execution
    pub async fn apply_post_query(&self, response_size: usize) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Throttle bandwidth based on response size
        self.traffic_shaper.throttle_bandwidth(response_size).await;

        // Check for packet loss (simulated)
        if self.traffic_shaper.should_drop_packet() {
            warn!("Simulating GraphQL packet loss");
            return Err(ChaosError::InjectedFault("Packet loss".to_string()));
        }

        Ok(())
    }

    /// Apply chaos for resolver execution
    pub async fn apply_resolver(&self, field_name: &str) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        debug!("Applying GraphQL chaos for resolver: {}", field_name);

        // Inject resolver latency (typically smaller than query latency)
        if self.latency_injector.is_enabled() {
            // Apply 10% of normal latency for field resolvers
            let config = self.latency_injector.config();
            if let Some(delay_ms) = config.fixed_delay_ms {
                let resolver_delay = delay_ms / 10;
                if resolver_delay > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(resolver_delay)).await;
                }
            }
        }

        Ok(())
    }

    /// Check if should inject GraphQL error
    pub fn should_inject_error(&self) -> Option<String> {
        self.fault_injector.get_http_error_status().map(|_http_code| "Internal server error".to_string())
    }

    /// Check if should return partial data
    pub fn should_return_partial_data(&self) -> bool {
        self.fault_injector.should_truncate_response()
    }

    /// Get GraphQL error code for fault injection
    pub fn get_error_code(&self) -> Option<&str> {
        if let Some(http_code) = self.fault_injector.get_http_error_status() {
            // Map HTTP codes to GraphQL error codes
            Some(match http_code {
                400 => "BAD_USER_INPUT",
                401 => "UNAUTHENTICATED",
                403 => "FORBIDDEN",
                404 => "NOT_FOUND",
                429 => "PERSISTED_QUERY_NOT_SUPPORTED",
                500 => "INTERNAL_SERVER_ERROR",
                503 => "SERVICE_UNAVAILABLE",
                _ => "INTERNAL_SERVER_ERROR",
            })
        } else {
            None
        }
    }

    /// Get traffic shaper for connection management
    pub fn traffic_shaper(&self) -> &Arc<TrafficShaper> {
        &self.traffic_shaper
    }
}

/// GraphQL error codes
pub mod error_code {
    pub const GRAPHQL_PARSE_FAILED: &str = "GRAPHQL_PARSE_FAILED";
    pub const GRAPHQL_VALIDATION_FAILED: &str = "GRAPHQL_VALIDATION_FAILED";
    pub const BAD_USER_INPUT: &str = "BAD_USER_INPUT";
    pub const UNAUTHENTICATED: &str = "UNAUTHENTICATED";
    pub const FORBIDDEN: &str = "FORBIDDEN";
    pub const NOT_FOUND: &str = "NOT_FOUND";
    pub const INTERNAL_SERVER_ERROR: &str = "INTERNAL_SERVER_ERROR";
    pub const SERVICE_UNAVAILABLE: &str = "SERVICE_UNAVAILABLE";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{FaultInjectionConfig, LatencyConfig};

    #[tokio::test]
    async fn test_graphql_chaos_creation() {
        let config = ChaosConfig {
            enabled: true,
            latency: Some(LatencyConfig {
                enabled: true,
                fixed_delay_ms: Some(100),
                random_delay_range_ms: None,
                jitter_percent: 0.0,
                probability: 1.0,
            }),
            ..Default::default()
        };

        let chaos = GraphQLChaos::new(config);
        assert!(chaos.config.enabled);
    }

    #[tokio::test]
    async fn test_graphql_error_code_mapping() {
        let config = ChaosConfig {
            enabled: true,
            fault_injection: Some(FaultInjectionConfig {
                enabled: true,
                http_errors: vec![401],
                http_error_probability: 1.0,
                ..Default::default()
            }),
            ..Default::default()
        };

        let chaos = GraphQLChaos::new(config);
        let error_code = chaos.get_error_code();

        // Should map 401 to UNAUTHENTICATED
        assert_eq!(error_code, Some("UNAUTHENTICATED"));
    }

    #[tokio::test]
    async fn test_resolver_latency() {
        let config = ChaosConfig {
            enabled: true,
            latency: Some(LatencyConfig {
                enabled: true,
                fixed_delay_ms: Some(100), // 100ms query latency
                random_delay_range_ms: None,
                jitter_percent: 0.0,
                probability: 1.0,
            }),
            ..Default::default()
        };

        let chaos = GraphQLChaos::new(config);
        let start = std::time::Instant::now();

        chaos.apply_resolver("user").await.unwrap();

        let elapsed = start.elapsed();
        // Should be ~10ms (10% of query latency)
        assert!(elapsed >= std::time::Duration::from_millis(10));
        assert!(elapsed < std::time::Duration::from_millis(50));
    }
}
