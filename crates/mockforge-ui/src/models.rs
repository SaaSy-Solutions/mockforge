//! Data models for the admin UI

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Server status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    /// Server type (HTTP, WebSocket, gRPC)
    pub server_type: String,
    /// Server address
    pub address: Option<String>,
    /// Whether server is running
    pub running: bool,
    /// Start time
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Uptime in seconds
    pub uptime_seconds: Option<u64>,
    /// Number of active connections
    pub active_connections: u64,
    /// Total requests served
    pub total_requests: u64,
}

/// Route information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteInfo {
    /// HTTP method
    pub method: Option<String>,
    /// Route path
    pub path: String,
    /// Route priority
    pub priority: i32,
    /// Whether route has fixtures
    pub has_fixtures: bool,
    /// Latency profile
    pub latency_ms: Option<u64>,
    /// Request count
    pub request_count: u64,
    /// Last request time
    pub last_request: Option<chrono::DateTime<chrono::Utc>>,
    /// Error count
    pub error_count: u64,
}

/// Request log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLog {
    /// Request ID
    pub id: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Response status code
    pub status_code: u16,
    /// Response time in milliseconds
    pub response_time_ms: u64,
    /// Client IP address
    pub client_ip: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// Request headers (filtered)
    pub headers: HashMap<String, String>,
    /// Response size in bytes
    pub response_size_bytes: u64,
    /// Error message (if any)
    pub error_message: Option<String>,
}

/// System information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// MockForge version
    pub version: String,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Memory usage in MB
    pub memory_usage_mb: u64,
    /// CPU usage percentage
    pub cpu_usage_percent: f64,
    /// Number of active threads
    pub active_threads: usize,
    /// Total routes configured
    pub total_routes: usize,
    /// Total fixtures available
    pub total_fixtures: usize,
}

/// Latency profile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyProfile {
    /// Profile name
    pub name: String,
    /// Base latency in milliseconds
    pub base_ms: u64,
    /// Jitter range in milliseconds
    pub jitter_ms: u64,
    /// Tag-based overrides
    pub tag_overrides: HashMap<String, u64>,
}

/// Fault injection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultConfig {
    /// Whether fault injection is enabled
    pub enabled: bool,
    /// Failure rate (0.0 to 1.0)
    pub failure_rate: f64,
    /// HTTP status codes for failures
    pub status_codes: Vec<u16>,
    /// Current active failures
    pub active_failures: u64,
}

/// Proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Whether proxy is enabled
    pub enabled: bool,
    /// Upstream URL
    pub upstream_url: Option<String>,
    /// Request timeout seconds
    pub timeout_seconds: u64,
    /// Total requests proxied
    pub requests_proxied: u64,
}

/// Dashboard data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    /// System information
    pub system: SystemInfo,
    /// Server statuses
    pub servers: Vec<ServerStatus>,
    /// Route information
    pub routes: Vec<RouteInfo>,
    /// Recent request logs (last 100)
    pub recent_logs: Vec<RequestLog>,
    /// Latency profile
    pub latency_profile: LatencyProfile,
    /// Fault configuration
    pub fault_config: FaultConfig,
    /// Proxy configuration
    pub proxy_config: ProxyConfig,
}

/// API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Whether request was successful
    pub success: bool,
    /// Response data
    pub data: Option<T>,
    /// Error message (if any)
    pub error: Option<String>,
    /// Response timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> ApiResponse<T> {
    /// Create a successful response
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create an error response
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Configuration update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigUpdate {
    /// Configuration type
    pub config_type: String,
    /// Configuration data
    pub data: serde_json::Value,
}

/// Route management request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteUpdate {
    /// Route path
    pub path: String,
    /// HTTP method (optional)
    pub method: Option<String>,
    /// Update operation
    pub operation: String,
    /// Update data
    pub data: Option<serde_json::Value>,
}

/// Log filter options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFilter {
    /// Filter by HTTP method
    pub method: Option<String>,
    /// Filter by path pattern
    pub path_pattern: Option<String>,
    /// Filter by status code
    pub status_code: Option<u16>,
    /// Filter by time range (hours ago)
    pub hours_ago: Option<u64>,
    /// Maximum number of results
    pub limit: Option<usize>,
}

impl Default for LogFilter {
    fn default() -> Self {
        Self {
            method: None,
            path_pattern: None,
            status_code: None,
            hours_ago: Some(24),
            limit: Some(100),
        }
    }
}

/// Metrics data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsData {
    /// Request count by endpoint
    pub requests_by_endpoint: HashMap<String, u64>,
    /// Response time percentiles
    pub response_time_percentiles: HashMap<String, u64>,
    /// Error rate by endpoint
    pub error_rate_by_endpoint: HashMap<String, f64>,
    /// Memory usage over time
    pub memory_usage_over_time: Vec<(chrono::DateTime<chrono::Utc>, u64)>,
    /// CPU usage over time
    pub cpu_usage_over_time: Vec<(chrono::DateTime<chrono::Utc>, f64)>,
}

/// Validation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSettings {
    /// Validation mode: "enforce", "warn", or "off"
    pub mode: String,
    /// Whether to aggregate errors
    pub aggregate_errors: bool,
    /// Whether to validate responses
    pub validate_responses: bool,
    /// Per-route validation overrides
    pub overrides: HashMap<String, String>,
}

/// Validation update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationUpdate {
    /// Validation mode
    pub mode: String,
    /// Whether to aggregate errors
    pub aggregate_errors: bool,
    /// Whether to validate responses
    pub validate_responses: bool,
    /// Per-route validation overrides
    pub overrides: Option<HashMap<String, String>>,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Overall health status
    pub status: String,
    /// Individual service health
    pub services: HashMap<String, String>,
    /// Last health check time
    pub last_check: chrono::DateTime<chrono::Utc>,
    /// Any health issues
    pub issues: Vec<String>,
}

impl HealthCheck {
    /// Create a healthy status
    pub fn healthy() -> Self {
        Self {
            status: "healthy".to_string(),
            services: HashMap::new(),
            last_check: chrono::Utc::now(),
            issues: Vec::new(),
        }
    }

    /// Create an unhealthy status
    pub fn unhealthy(issues: Vec<String>) -> Self {
        Self {
            status: "unhealthy".to_string(),
            services: HashMap::new(),
            last_check: chrono::Utc::now(),
            issues,
        }
    }

    /// Add service status
    pub fn with_service(mut self, name: String, status: String) -> Self {
        self.services.insert(name, status);
        self
    }
}
