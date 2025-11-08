//! Chaos engineering configuration

use serde::{Deserialize, Serialize};

/// Payload corruption type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CorruptionType {
    /// No corruption
    None,
    /// Replace random bytes with random values
    RandomBytes,
    /// Truncate payload at random position
    Truncate,
    /// Flip random bits in the payload
    BitFlip,
}

/// Error injection pattern
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ErrorPattern {
    /// Burst pattern: inject N errors within a time interval
    Burst {
        /// Number of errors to inject in the burst
        count: usize,
        /// Time interval in milliseconds for the burst
        interval_ms: u64,
    },
    /// Random pattern: inject errors with a probability
    Random {
        /// Probability of injecting an error (0.0-1.0)
        probability: f64,
    },
    /// Sequential pattern: inject errors in a specific sequence
    Sequential {
        /// Sequence of status codes to inject in order
        sequence: Vec<u16>,
    },
}

impl Default for ErrorPattern {
    fn default() -> Self {
        ErrorPattern::Random { probability: 0.1 }
    }
}

/// Main chaos engineering configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    /// Enable payload corruption
    pub payload_corruption: bool,
    /// Probability of payload corruption (0.0-1.0)
    pub payload_corruption_probability: f64,
    /// Type of corruption to apply
    pub corruption_type: CorruptionType,
    /// Error injection pattern (burst, random, sequential)
    #[serde(default)]
    pub error_pattern: Option<ErrorPattern>,
    /// Enable MockAI for dynamic error message generation
    #[serde(default)]
    pub mockai_enabled: bool,
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
            payload_corruption: false,
            payload_corruption_probability: 0.05,
            corruption_type: CorruptionType::None,
            error_pattern: None,
            mockai_enabled: false,
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

/// Network profile for simulating different network conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkProfile {
    /// Profile name
    pub name: String,
    /// Profile description
    pub description: String,
    /// Chaos configuration for this profile
    pub chaos_config: ChaosConfig,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Whether this is a built-in profile (not user-created)
    #[serde(default)]
    pub builtin: bool,
}

impl NetworkProfile {
    /// Create a new network profile
    pub fn new(name: String, description: String, chaos_config: ChaosConfig) -> Self {
        Self {
            name,
            description,
            chaos_config,
            tags: Vec::new(),
            builtin: false,
        }
    }

    /// Create predefined network profiles
    pub fn predefined_profiles() -> Vec<Self> {
        vec![
            // Slow 3G: High latency, packet loss, low bandwidth
            Self {
                name: "slow_3g".to_string(),
                description: "Simulates slow 3G network: 400ms latency, 1% packet loss, 400KB/s bandwidth".to_string(),
                chaos_config: ChaosConfig {
                    enabled: true,
                    latency: Some(LatencyConfig {
                        enabled: true,
                        fixed_delay_ms: Some(400),
                        random_delay_range_ms: Some((300, 500)),
                        jitter_percent: 10.0,
                        probability: 1.0,
                    }),
                    fault_injection: None,
                    rate_limit: None,
                    traffic_shaping: Some(TrafficShapingConfig {
                        enabled: true,
                        bandwidth_limit_bps: 400_000, // 400 KB/s
                        packet_loss_percent: 1.0,
                        max_connections: 0,
                        connection_timeout_ms: 30000,
                    }),
                    circuit_breaker: None,
                    bulkhead: None,
                },
                tags: vec!["mobile".to_string(), "slow".to_string(), "3g".to_string()],
                builtin: true,
            },
            // Fast 3G: Moderate latency, low packet loss, higher bandwidth
            Self {
                name: "fast_3g".to_string(),
                description: "Simulates fast 3G network: 150ms latency, 0.5% packet loss, 1.5MB/s bandwidth".to_string(),
                chaos_config: ChaosConfig {
                    enabled: true,
                    latency: Some(LatencyConfig {
                        enabled: true,
                        fixed_delay_ms: Some(150),
                        random_delay_range_ms: Some((100, 200)),
                        jitter_percent: 5.0,
                        probability: 1.0,
                    }),
                    fault_injection: None,
                    rate_limit: None,
                    traffic_shaping: Some(TrafficShapingConfig {
                        enabled: true,
                        bandwidth_limit_bps: 1_500_000, // 1.5 MB/s
                        packet_loss_percent: 0.5,
                        max_connections: 0,
                        connection_timeout_ms: 30000,
                    }),
                    circuit_breaker: None,
                    bulkhead: None,
                },
                tags: vec!["mobile".to_string(), "fast".to_string(), "3g".to_string()],
                builtin: true,
            },
            // Flaky Wi-Fi: Low latency but high packet loss and random disconnects
            Self {
                name: "flaky_wifi".to_string(),
                description: "Simulates flaky Wi-Fi: 50ms latency, 5% packet loss, random connection errors".to_string(),
                chaos_config: ChaosConfig {
                    enabled: true,
                    latency: Some(LatencyConfig {
                        enabled: true,
                        fixed_delay_ms: Some(50),
                        random_delay_range_ms: Some((30, 100)),
                        jitter_percent: 20.0,
                        probability: 1.0,
                    }),
                    fault_injection: Some(FaultInjectionConfig {
                        enabled: true,
                        http_errors: vec![500, 502, 503],
                        http_error_probability: 0.05, // 5% chance of connection errors
                        connection_errors: true,
                        connection_error_probability: 0.03, // 3% chance of disconnects
                        timeout_errors: false,
                        timeout_ms: 5000,
                        timeout_probability: 0.0,
                        partial_responses: false,
                        partial_response_probability: 0.0,
                        payload_corruption: false,
                        payload_corruption_probability: 0.0,
                        corruption_type: CorruptionType::None,
                        error_pattern: None,
                        mockai_enabled: false,
                    }),
                    rate_limit: None,
                    traffic_shaping: Some(TrafficShapingConfig {
                        enabled: true,
                        bandwidth_limit_bps: 0, // No bandwidth limit
                        packet_loss_percent: 5.0,
                        max_connections: 0,
                        connection_timeout_ms: 30000,
                    }),
                    circuit_breaker: None,
                    bulkhead: None,
                },
                tags: vec!["wifi".to_string(), "unstable".to_string(), "wireless".to_string()],
                builtin: true,
            },
            // Cable: Low latency, no packet loss, high bandwidth
            Self {
                name: "cable".to_string(),
                description: "Simulates cable internet: 20ms latency, no packet loss, 10MB/s bandwidth".to_string(),
                chaos_config: ChaosConfig {
                    enabled: true,
                    latency: Some(LatencyConfig {
                        enabled: true,
                        fixed_delay_ms: Some(20),
                        random_delay_range_ms: Some((10, 30)),
                        jitter_percent: 2.0,
                        probability: 1.0,
                    }),
                    fault_injection: None,
                    rate_limit: None,
                    traffic_shaping: Some(TrafficShapingConfig {
                        enabled: true,
                        bandwidth_limit_bps: 10_000_000, // 10 MB/s
                        packet_loss_percent: 0.0,
                        max_connections: 0,
                        connection_timeout_ms: 30000,
                    }),
                    circuit_breaker: None,
                    bulkhead: None,
                },
                tags: vec!["broadband".to_string(), "fast".to_string(), "stable".to_string()],
                builtin: true,
            },
            // Dial-up: Very high latency, packet loss, very low bandwidth
            Self {
                name: "dialup".to_string(),
                description: "Simulates dial-up connection: 2000ms latency, 2% packet loss, 50KB/s bandwidth".to_string(),
                chaos_config: ChaosConfig {
                    enabled: true,
                    latency: Some(LatencyConfig {
                        enabled: true,
                        fixed_delay_ms: Some(2000),
                        random_delay_range_ms: Some((1500, 2500)),
                        jitter_percent: 15.0,
                        probability: 1.0,
                    }),
                    fault_injection: None,
                    rate_limit: None,
                    traffic_shaping: Some(TrafficShapingConfig {
                        enabled: true,
                        bandwidth_limit_bps: 50_000, // 50 KB/s
                        packet_loss_percent: 2.0,
                        max_connections: 0,
                        connection_timeout_ms: 60000, // Longer timeout for dial-up
                    }),
                    circuit_breaker: None,
                    bulkhead: None,
                },
                tags: vec!["dialup".to_string(), "slow".to_string(), "legacy".to_string()],
                builtin: true,
            },
        ]
    }
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
