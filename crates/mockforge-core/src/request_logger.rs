//! Centralized request logging system for all MockForge servers

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

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
    /// Response size in bytes
    pub response_size_bytes: u64,
    /// Error message (if any)
    pub error_message: Option<String>,
    /// Additional metadata specific to server type
    pub metadata: HashMap<String, String>,
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
    pub async fn get_logs_by_server(&self, server_type: &str, limit: Option<usize>) -> Vec<RequestLogEntry> {
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
}

/// Global singleton instance of the centralized logger
static GLOBAL_LOGGER: once_cell::sync::OnceCell<CentralizedRequestLogger> = once_cell::sync::OnceCell::new();

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
        response_size_bytes,
        error_message,
        metadata: HashMap::new(),
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
        response_size_bytes: message_size_bytes,
        error_message,
        metadata,
    }
}

/// Helper to create gRPC request log entry
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
        response_size_bytes,
        error_message,
        metadata,
    }
}