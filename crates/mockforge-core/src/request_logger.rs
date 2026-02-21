//! Centralized request logging system for all MockForge servers

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Reality continuum type based on blend ratio
///
/// Categorizes responses based on how much real vs mock data is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RealityContinuumType {
    /// 100% synthetic/mock data (blend ratio = 0.0)
    Synthetic,
    /// Mix of mock and real data (0.0 < blend ratio < 1.0)
    Blended,
    /// 100% real/upstream data (blend ratio = 1.0)
    Live,
}

impl RealityContinuumType {
    /// Determine continuum type from blend ratio
    pub fn from_blend_ratio(ratio: f64) -> Self {
        if ratio <= 0.0 {
            Self::Synthetic
        } else if ratio >= 1.0 {
            Self::Live
        } else {
            Self::Blended
        }
    }

    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            RealityContinuumType::Synthetic => "Synthetic",
            RealityContinuumType::Blended => "Blended",
            RealityContinuumType::Live => "Live",
        }
    }
}

/// Data source breakdown showing percentages from different sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceBreakdown {
    /// Percentage from recorded/production data (0.0 - 100.0)
    #[serde(default)]
    pub recorded_percent: f64,
    /// Percentage from generator/synthetic data (0.0 - 100.0)
    #[serde(default)]
    pub generator_percent: f64,
    /// Percentage from upstream/real data (0.0 - 100.0)
    #[serde(default)]
    pub upstream_percent: f64,
}

impl Default for DataSourceBreakdown {
    fn default() -> Self {
        Self {
            recorded_percent: 0.0,
            generator_percent: 100.0,
            upstream_percent: 0.0,
        }
    }
}

impl DataSourceBreakdown {
    /// Create breakdown from blend ratio
    ///
    /// Assumes blend ratio represents the mix between mock (generator) and real (upstream).
    /// Recorded data is treated as a separate category.
    pub fn from_blend_ratio(blend_ratio: f64, recorded_ratio: f64) -> Self {
        let upstream = blend_ratio * (1.0 - recorded_ratio);
        let generator = (1.0 - blend_ratio) * (1.0 - recorded_ratio);
        let recorded = recorded_ratio;

        Self {
            recorded_percent: recorded * 100.0,
            generator_percent: generator * 100.0,
            upstream_percent: upstream * 100.0,
        }
    }

    /// Normalize percentages to ensure they sum to 100.0
    pub fn normalize(&mut self) {
        let total = self.recorded_percent + self.generator_percent + self.upstream_percent;
        if total > 0.0 {
            self.recorded_percent = (self.recorded_percent / total) * 100.0;
            self.generator_percent = (self.generator_percent / total) * 100.0;
            self.upstream_percent = (self.upstream_percent / total) * 100.0;
        }
    }
}

/// Reality trace metadata for a request
///
/// Captures information about how the response was generated, including
/// reality level, data sources, active personas, scenarios, and chaos profiles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealityTraceMetadata {
    /// Reality level (1-5) from RealityLevel enum
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reality_level: Option<crate::reality::RealityLevel>,
    /// Reality continuum type (Synthetic/Blended/Live)
    pub reality_continuum_type: RealityContinuumType,
    /// Blend ratio used (0.0 = mock, 1.0 = real)
    #[serde(default)]
    pub blend_ratio: f64,
    /// Data source breakdown showing percentages
    pub data_source_breakdown: DataSourceBreakdown,
    /// Active persona ID (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_persona_id: Option<String>,
    /// Active scenario identifier (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_scenario: Option<String>,
    /// Active chaos profiles/rules
    #[serde(default)]
    pub active_chaos_profiles: Vec<String>,
    /// Active latency profiles
    #[serde(default)]
    pub active_latency_profiles: Vec<String>,
}

impl Default for RealityTraceMetadata {
    fn default() -> Self {
        Self {
            reality_level: None,
            reality_continuum_type: RealityContinuumType::Synthetic,
            blend_ratio: 0.0,
            data_source_breakdown: DataSourceBreakdown::default(),
            active_persona_id: None,
            active_scenario: None,
            active_chaos_profiles: Vec::new(),
            active_latency_profiles: Vec::new(),
        }
    }
}

impl RealityTraceMetadata {
    /// Create reality trace metadata from unified state and blend ratio
    ///
    /// Builds metadata from the consistency engine's unified state and
    /// the actual blend ratio used for the request.
    pub fn from_unified_state(
        unified_state: &crate::consistency::types::UnifiedState,
        blend_ratio: f64,
        _path: &str,
    ) -> Self {
        let reality_continuum_type = RealityContinuumType::from_blend_ratio(blend_ratio);

        // Extract chaos rule names
        let active_chaos_profiles: Vec<String> = unified_state
            .active_chaos_rules
            .iter()
            .filter_map(|r| r.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .collect();

        // For now, latency profiles are not stored in unified state
        // This would need to be added or extracted from elsewhere
        let active_latency_profiles = Vec::new();

        // Build data source breakdown
        // Assume recorded ratio is 0 for now (could be enhanced later)
        let mut breakdown = DataSourceBreakdown::from_blend_ratio(blend_ratio, 0.0);
        breakdown.normalize();

        Self {
            reality_level: Some(unified_state.reality_level),
            reality_continuum_type,
            blend_ratio,
            data_source_breakdown: breakdown,
            active_persona_id: unified_state.active_persona.as_ref().map(|p| p.id.clone()),
            active_scenario: unified_state.active_scenario.clone(),
            active_chaos_profiles,
            active_latency_profiles,
        }
    }
}

/// A request log entry that can represent HTTP, WebSocket, or gRPC requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLogEntry {
    /// Unique request ID
    pub id: String,
    /// Request timestamp
    pub timestamp: DateTime<Utc>,
    /// Server type (HTTP, WebSocket, gRPC)
    pub server_type: String,
    /// Request method (GET, POST, CONNECT, etc. or gRPC method name)
    pub method: String,
    /// Request path or endpoint
    pub path: String,
    /// Response status code (HTTP status, WebSocket status, gRPC status code)
    pub status_code: u16,
    /// Response time in milliseconds
    pub response_time_ms: u64,
    /// Client IP address
    pub client_ip: Option<String>,
    /// User agent (if available)
    pub user_agent: Option<String>,
    /// Request headers (filtered for security)
    pub headers: HashMap<String, String>,
    /// Query parameters from the request URL
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub query_params: HashMap<String, String>,
    /// Response size in bytes
    pub response_size_bytes: u64,
    /// Error message (if any)
    pub error_message: Option<String>,
    /// Additional metadata specific to server type
    pub metadata: HashMap<String, String>,
    /// Reality trace metadata (if available)
    ///
    /// Contains information about how the response was generated,
    /// including reality level, data sources, personas, and chaos profiles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reality_metadata: Option<RealityTraceMetadata>,
}

/// Centralized request logger that all servers can write to
#[derive(Debug, Clone)]
pub struct CentralizedRequestLogger {
    /// Ring buffer of request logs (most recent first)
    logs: Arc<RwLock<VecDeque<RequestLogEntry>>>,
    /// Maximum number of logs to keep in memory
    max_logs: usize,
}

impl Default for CentralizedRequestLogger {
    fn default() -> Self {
        Self::new(1000) // Keep last 1000 requests by default
    }
}

impl CentralizedRequestLogger {
    /// Create a new centralized request logger
    pub fn new(max_logs: usize) -> Self {
        Self {
            logs: Arc::new(RwLock::new(VecDeque::new())),
            max_logs,
        }
    }

    /// Log a new request entry
    pub async fn log_request(&self, entry: RequestLogEntry) {
        let mut logs = self.logs.write().await;

        // Add to front (most recent first)
        logs.push_front(entry);

        // Maintain size limit
        while logs.len() > self.max_logs {
            logs.pop_back();
        }
    }

    /// Get recent logs (most recent first)
    pub async fn get_recent_logs(&self, limit: Option<usize>) -> Vec<RequestLogEntry> {
        let logs = self.logs.read().await;
        let take_count = limit.unwrap_or(logs.len()).min(logs.len());
        logs.iter().take(take_count).cloned().collect()
    }

    /// Get logs filtered by server type
    pub async fn get_logs_by_server(
        &self,
        server_type: &str,
        limit: Option<usize>,
    ) -> Vec<RequestLogEntry> {
        let logs = self.logs.read().await;
        logs.iter()
            .filter(|log| log.server_type == server_type)
            .take(limit.unwrap_or(logs.len()))
            .cloned()
            .collect()
    }

    /// Get total request count by server type
    pub async fn get_request_counts_by_server(&self) -> HashMap<String, u64> {
        let logs = self.logs.read().await;
        let mut counts = HashMap::new();

        for log in logs.iter() {
            *counts.entry(log.server_type.clone()).or_insert(0) += 1;
        }

        counts
    }

    /// Clear all logs
    pub async fn clear_logs(&self) {
        let mut logs = self.logs.write().await;
        logs.clear();
    }

    /// Find all request log entries that match the verification pattern
    ///
    /// This method is used by the verification API to find matching requests.
    /// It returns all log entries that match the given pattern, ordered by
    /// timestamp (most recent first).
    pub async fn find_matching_requests(
        &self,
        pattern: &crate::verification::VerificationRequest,
    ) -> Vec<RequestLogEntry> {
        let logs = self.logs.read().await;
        logs.iter()
            .filter(|entry| crate::verification::matches_verification_pattern(entry, pattern))
            .cloned()
            .collect()
    }

    /// Count request log entries that match the verification pattern
    ///
    /// This is a convenience method that returns just the count of matching requests
    /// without collecting all the matching entries, which is more efficient when
    /// you only need the count.
    pub async fn count_matching_requests(
        &self,
        pattern: &crate::verification::VerificationRequest,
    ) -> usize {
        let logs = self.logs.read().await;
        logs.iter()
            .filter(|entry| crate::verification::matches_verification_pattern(entry, pattern))
            .count()
    }

    /// Get request sequence matching the given patterns in order
    ///
    /// This method finds requests that match the patterns in the specified order,
    /// which is useful for verifying request sequences. It returns the matching
    /// entries in the order they were found (chronological order).
    pub async fn get_request_sequence(
        &self,
        patterns: &[crate::verification::VerificationRequest],
    ) -> Vec<RequestLogEntry> {
        let logs = self.logs.read().await;
        let mut log_idx = 0;
        let mut all_matches = Vec::new();

        for pattern in patterns {
            // Find the next matching request after the last match
            let mut found = false;
            while log_idx < logs.len() {
                if crate::verification::matches_verification_pattern(&logs[log_idx], pattern) {
                    all_matches.push(logs[log_idx].clone());
                    log_idx += 1;
                    found = true;
                    break;
                }
                log_idx += 1;
            }

            if !found {
                // If we can't find a match for this pattern, return what we have so far
                break;
            }
        }

        all_matches
    }
}

/// Global singleton instance of the centralized logger
static GLOBAL_LOGGER: once_cell::sync::OnceCell<CentralizedRequestLogger> =
    once_cell::sync::OnceCell::new();

/// Initialize the global request logger
pub fn init_global_logger(max_logs: usize) -> &'static CentralizedRequestLogger {
    GLOBAL_LOGGER.get_or_init(|| CentralizedRequestLogger::new(max_logs))
}

/// Get reference to the global request logger
pub fn get_global_logger() -> Option<&'static CentralizedRequestLogger> {
    GLOBAL_LOGGER.get()
}

/// Log a request to the global logger (convenience function)
pub async fn log_request_global(entry: RequestLogEntry) {
    if let Some(logger) = get_global_logger() {
        logger.log_request(entry).await;
    }
}

/// Helper to create HTTP request log entry
#[allow(clippy::too_many_arguments)]
pub fn create_http_log_entry(
    method: &str,
    path: &str,
    status_code: u16,
    response_time_ms: u64,
    client_ip: Option<String>,
    user_agent: Option<String>,
    headers: HashMap<String, String>,
    response_size_bytes: u64,
    error_message: Option<String>,
) -> RequestLogEntry {
    create_http_log_entry_with_query(
        method,
        path,
        status_code,
        response_time_ms,
        client_ip,
        user_agent,
        headers,
        HashMap::new(), // Default empty query params
        response_size_bytes,
        error_message,
    )
}

/// Helper to create HTTP request log entry with query parameters
#[allow(clippy::too_many_arguments)]
pub fn create_http_log_entry_with_query(
    method: &str,
    path: &str,
    status_code: u16,
    response_time_ms: u64,
    client_ip: Option<String>,
    user_agent: Option<String>,
    headers: HashMap<String, String>,
    query_params: HashMap<String, String>,
    response_size_bytes: u64,
    error_message: Option<String>,
) -> RequestLogEntry {
    RequestLogEntry {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: Utc::now(),
        server_type: "HTTP".to_string(),
        method: method.to_string(),
        path: path.to_string(),
        status_code,
        response_time_ms,
        client_ip,
        user_agent,
        headers,
        query_params,
        response_size_bytes,
        error_message,
        metadata: HashMap::new(),
        reality_metadata: None,
    }
}

/// Helper to create WebSocket request log entry
pub fn create_websocket_log_entry(
    event_type: &str, // "connect", "disconnect", "message"
    path: &str,
    status_code: u16,
    client_ip: Option<String>,
    message_size_bytes: u64,
    error_message: Option<String>,
) -> RequestLogEntry {
    let mut metadata = HashMap::new();
    metadata.insert("event_type".to_string(), event_type.to_string());

    RequestLogEntry {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: Utc::now(),
        server_type: "WebSocket".to_string(),
        method: event_type.to_uppercase(),
        path: path.to_string(),
        status_code,
        response_time_ms: 0, // WebSocket events are typically instant
        client_ip,
        user_agent: None,
        headers: HashMap::new(),
        query_params: HashMap::new(),
        response_size_bytes: message_size_bytes,
        error_message,
        metadata,
        reality_metadata: None,
    }
}

/// Helper to create gRPC request log entry
#[allow(clippy::too_many_arguments)]
pub fn create_grpc_log_entry(
    service: &str,
    method: &str,
    status_code: u16, // gRPC status code
    response_time_ms: u64,
    client_ip: Option<String>,
    request_size_bytes: u64,
    response_size_bytes: u64,
    error_message: Option<String>,
) -> RequestLogEntry {
    let mut metadata = HashMap::new();
    metadata.insert("service".to_string(), service.to_string());
    metadata.insert("request_size_bytes".to_string(), request_size_bytes.to_string());

    RequestLogEntry {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: Utc::now(),
        server_type: "gRPC".to_string(),
        method: format!("{}/{}", service, method),
        path: format!("/{}/{}", service, method),
        status_code,
        response_time_ms,
        client_ip,
        user_agent: None,
        headers: HashMap::new(),
        query_params: HashMap::new(),
        response_size_bytes,
        error_message,
        metadata,
        reality_metadata: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_entry(server_type: &str, method: &str) -> RequestLogEntry {
        RequestLogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            server_type: server_type.to_string(),
            method: method.to_string(),
            path: "/test".to_string(),
            status_code: 200,
            response_time_ms: 100,
            client_ip: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            headers: HashMap::new(),
            query_params: HashMap::new(),
            response_size_bytes: 1024,
            error_message: None,
            metadata: HashMap::new(),
            reality_metadata: None,
        }
    }

    #[test]
    fn test_centralized_logger_new() {
        let logger = CentralizedRequestLogger::new(500);
        assert_eq!(logger.max_logs, 500);
    }

    #[test]
    fn test_centralized_logger_default() {
        let logger = CentralizedRequestLogger::default();
        assert_eq!(logger.max_logs, 1000);
    }

    #[tokio::test]
    async fn test_log_request() {
        let logger = CentralizedRequestLogger::new(10);
        let entry = create_test_entry("HTTP", "GET");

        logger.log_request(entry).await;

        let logs = logger.get_recent_logs(None).await;
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].method, "GET");
    }

    #[tokio::test]
    async fn test_log_request_maintains_size_limit() {
        let logger = CentralizedRequestLogger::new(5);

        // Add 10 entries
        for i in 0..10 {
            let mut entry = create_test_entry("HTTP", "GET");
            entry.id = format!("entry-{}", i);
            logger.log_request(entry).await;
        }

        let logs = logger.get_recent_logs(None).await;
        assert_eq!(logs.len(), 5); // Should only keep 5 most recent
    }

    #[tokio::test]
    async fn test_get_recent_logs_with_limit() {
        let logger = CentralizedRequestLogger::new(100);

        for _ in 0..20 {
            logger.log_request(create_test_entry("HTTP", "GET")).await;
        }

        let logs = logger.get_recent_logs(Some(10)).await;
        assert_eq!(logs.len(), 10);
    }

    #[tokio::test]
    async fn test_get_logs_by_server() {
        let logger = CentralizedRequestLogger::new(100);

        logger.log_request(create_test_entry("HTTP", "GET")).await;
        logger.log_request(create_test_entry("HTTP", "POST")).await;
        logger.log_request(create_test_entry("WebSocket", "CONNECT")).await;
        logger.log_request(create_test_entry("gRPC", "Call")).await;

        let http_logs = logger.get_logs_by_server("HTTP", None).await;
        assert_eq!(http_logs.len(), 2);

        let ws_logs = logger.get_logs_by_server("WebSocket", None).await;
        assert_eq!(ws_logs.len(), 1);

        let grpc_logs = logger.get_logs_by_server("gRPC", None).await;
        assert_eq!(grpc_logs.len(), 1);
    }

    #[tokio::test]
    async fn test_get_request_counts_by_server() {
        let logger = CentralizedRequestLogger::new(100);

        logger.log_request(create_test_entry("HTTP", "GET")).await;
        logger.log_request(create_test_entry("HTTP", "POST")).await;
        logger.log_request(create_test_entry("HTTP", "PUT")).await;
        logger.log_request(create_test_entry("WebSocket", "CONNECT")).await;
        logger.log_request(create_test_entry("gRPC", "Call")).await;
        logger.log_request(create_test_entry("gRPC", "Stream")).await;

        let counts = logger.get_request_counts_by_server().await;

        assert_eq!(counts.get("HTTP"), Some(&3));
        assert_eq!(counts.get("WebSocket"), Some(&1));
        assert_eq!(counts.get("gRPC"), Some(&2));
    }

    #[tokio::test]
    async fn test_clear_logs() {
        let logger = CentralizedRequestLogger::new(100);

        logger.log_request(create_test_entry("HTTP", "GET")).await;
        logger.log_request(create_test_entry("HTTP", "POST")).await;

        let logs = logger.get_recent_logs(None).await;
        assert_eq!(logs.len(), 2);

        logger.clear_logs().await;

        let logs = logger.get_recent_logs(None).await;
        assert_eq!(logs.len(), 0);
    }

    #[test]
    fn test_create_http_log_entry() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let entry = create_http_log_entry(
            "POST",
            "/api/test",
            201,
            150,
            Some("192.168.1.1".to_string()),
            Some("Mozilla/5.0".to_string()),
            headers.clone(),
            2048,
            None,
        );

        assert_eq!(entry.server_type, "HTTP");
        assert_eq!(entry.method, "POST");
        assert_eq!(entry.path, "/api/test");
        assert_eq!(entry.status_code, 201);
        assert_eq!(entry.response_time_ms, 150);
        assert_eq!(entry.response_size_bytes, 2048);
        assert_eq!(entry.client_ip, Some("192.168.1.1".to_string()));
        assert_eq!(entry.user_agent, Some("Mozilla/5.0".to_string()));
        assert_eq!(entry.headers.get("Content-Type"), Some(&"application/json".to_string()));
        assert!(entry.error_message.is_none());
    }

    #[test]
    fn test_create_websocket_log_entry() {
        let entry = create_websocket_log_entry(
            "connect",
            "/ws/chat",
            101,
            Some("10.0.0.1".to_string()),
            0,
            None,
        );

        assert_eq!(entry.server_type, "WebSocket");
        assert_eq!(entry.method, "CONNECT");
        assert_eq!(entry.path, "/ws/chat");
        assert_eq!(entry.status_code, 101);
        assert_eq!(entry.response_time_ms, 0);
        assert_eq!(entry.metadata.get("event_type"), Some(&"connect".to_string()));
    }

    #[test]
    fn test_create_grpc_log_entry() {
        let entry = create_grpc_log_entry(
            "UserService",
            "GetUser",
            0, // gRPC OK status
            50,
            Some("172.16.0.1".to_string()),
            128,
            512,
            None,
        );

        assert_eq!(entry.server_type, "gRPC");
        assert_eq!(entry.method, "UserService/GetUser");
        assert_eq!(entry.path, "/UserService/GetUser");
        assert_eq!(entry.status_code, 0);
        assert_eq!(entry.response_time_ms, 50);
        assert_eq!(entry.response_size_bytes, 512);
        assert_eq!(entry.metadata.get("service"), Some(&"UserService".to_string()));
        assert_eq!(entry.metadata.get("request_size_bytes"), Some(&"128".to_string()));
    }

    #[test]
    fn test_request_log_entry_with_error() {
        let entry = create_http_log_entry(
            "GET",
            "/api/error",
            500,
            200,
            None,
            None,
            HashMap::new(),
            0,
            Some("Internal server error".to_string()),
        );

        assert_eq!(entry.status_code, 500);
        assert_eq!(entry.error_message, Some("Internal server error".to_string()));
    }
}
