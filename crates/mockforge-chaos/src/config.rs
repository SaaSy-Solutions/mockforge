//! Chaos engineering configuration

use serde::{Deserialize, Serialize};

/// Main chaos engineering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct ChaosConfig {
    /// Enable chaos engineering
    pub enabled: bool,
    /// Latency injection configuration
    pub latency: Option<LatencyConfig>,
    /// Fault injection configuration
    pub fault_injection: Option<FaultInjectionConfig>,
    /// Rate limiting configuration
    pub rate_limit: Option<RateLimitConfig>,
    /// Traffic shaping configuration
    pub traffic_shaping: Option<TrafficShapingConfig>,
    /// Circuit breaker configuration
    pub circuit_breaker: Option<CircuitBreakerConfig>,
    /// Bulkhead configuration
    pub bulkhead: Option<BulkheadConfig>,
}


/// Latency injection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyConfig {
    /// Enable latency injection
    pub enabled: bool,
    /// Fixed delay in milliseconds
    pub fixed_delay_ms: Option<u64>,
    /// Random delay range (min, max) in milliseconds
    pub random_delay_range_ms: Option<(u64, u64)>,
    /// Jitter percentage (0-100)
    pub jitter_percent: f64,
    /// Probability of applying latency (0.0-1.0)
    pub probability: f64,
}

impl Default for LatencyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            fixed_delay_ms: None,
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            probability: 1.0,
        }
    }
}

/// Fault injection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultInjectionConfig {
    /// Enable fault injection
    pub enabled: bool,
    /// HTTP error codes to inject
    pub http_errors: Vec<u16>,
    /// Probability of HTTP errors (0.0-1.0)
    pub http_error_probability: f64,
    /// Inject connection errors
    pub connection_errors: bool,
    /// Probability of connection errors (0.0-1.0)
    pub connection_error_probability: f64,
    /// Inject timeout errors
    pub timeout_errors: bool,
    /// Timeout duration in milliseconds
    pub timeout_ms: u64,
    /// Probability of timeout errors (0.0-1.0)
    pub timeout_probability: f64,
    /// Inject partial responses (incomplete data)
    pub partial_responses: bool,
    /// Probability of partial responses (0.0-1.0)
    pub partial_response_probability: f64,
}

impl Default for FaultInjectionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            http_errors: vec![500, 502, 503, 504],
            http_error_probability: 0.1,
            connection_errors: false,
            connection_error_probability: 0.05,
            timeout_errors: false,
            timeout_ms: 5000,
            timeout_probability: 0.05,
            partial_responses: false,
            partial_response_probability: 0.05,
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,
    /// Maximum requests per second
    pub requests_per_second: u32,
    /// Burst size (number of requests allowed in burst)
    pub burst_size: u32,
    /// Per-IP rate limiting
    pub per_ip: bool,
    /// Per-endpoint rate limiting
    pub per_endpoint: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            requests_per_second: 100,
            burst_size: 10,
            per_ip: false,
            per_endpoint: false,
        }
    }
}

/// Traffic shaping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficShapingConfig {
    /// Enable traffic shaping
    pub enabled: bool,
    /// Bandwidth limit in bytes per second (0 = unlimited)
    pub bandwidth_limit_bps: u64,
    /// Packet loss percentage (0-100)
    pub packet_loss_percent: f64,
    /// Maximum concurrent connections (0 = unlimited)
    pub max_connections: u32,
    /// Connection timeout in milliseconds
    pub connection_timeout_ms: u64,
}

impl Default for TrafficShapingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bandwidth_limit_bps: 0,
            packet_loss_percent: 0.0,
            max_connections: 0,
            connection_timeout_ms: 30000,
        }
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Enable circuit breaker
    pub enabled: bool,
    /// Failure threshold before opening circuit
    pub failure_threshold: u64,
    /// Success threshold before closing circuit from half-open
    pub success_threshold: u64,
    /// Timeout before attempting to close circuit (in milliseconds)
    pub timeout_ms: u64,
    /// Half-open request limit
    pub half_open_max_requests: u32,
    /// Failure rate threshold (percentage, 0-100)
    pub failure_rate_threshold: f64,
    /// Minimum number of requests before calculating failure rate
    pub min_requests_for_rate: u64,
    /// Rolling window duration for failure rate calculation (in milliseconds)
    pub rolling_window_ms: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            failure_threshold: 5,
            success_threshold: 2,
            timeout_ms: 60000,
            half_open_max_requests: 3,
            failure_rate_threshold: 50.0,
            min_requests_for_rate: 10,
            rolling_window_ms: 10000,
        }
    }
}

/// Bulkhead configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkheadConfig {
    /// Enable bulkhead
    pub enabled: bool,
    /// Maximum concurrent requests
    pub max_concurrent_requests: u32,
    /// Maximum queue size (0 = no queue)
    pub max_queue_size: u32,
    /// Queue timeout in milliseconds
    pub queue_timeout_ms: u64,
}

impl Default for BulkheadConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_concurrent_requests: 100,
            max_queue_size: 10,
            queue_timeout_ms: 5000,
        }
    }
}
