//! Common server utilities for MockForge

use std::net::SocketAddr;

/// Create a SocketAddr for server binding from host and port
///
/// # Arguments
/// * `host` - Host address string (e.g., "127.0.0.1", "0.0.0.0", "example.com")
/// * `port` - Port number
///
/// # Returns
/// * `Ok(SocketAddr)` - Parsed socket address
/// * `Err(String)` - Error message if parsing fails
pub fn create_socket_addr(host: &str, port: u16) -> Result<SocketAddr, String> {
    format!("{}:{}", host, port)
        .parse()
        .map_err(|e| format!("Invalid socket address {}:{}: {}", host, port, e))
}

/// Create a standard IPv4 localhost SocketAddr (127.0.0.1:port)
///
/// # Arguments
/// * `port` - Port number to bind to
pub fn localhost_socket_addr(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

/// Create a standard IPv4 wildcard SocketAddr (0.0.0.0:port) to listen on all interfaces
///
/// # Arguments
/// * `port` - Port number to bind to
pub fn wildcard_socket_addr(port: u16) -> SocketAddr {
    SocketAddr::from(([0, 0, 0, 0], port))
}

/// Server startup configuration for binding and listening
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Host address to bind to (e.g., "0.0.0.0" or "127.0.0.1")
    pub host: String,
    /// Port number to bind to
    pub port: u16,
    /// Type of server to start
    pub server_type: ServerType,
}

/// Server type enumeration
#[derive(Debug, Clone)]
pub enum ServerType {
    /// HTTP/REST server
    HTTP,
    /// WebSocket server
    WebSocket,
    /// gRPC server
    GRPC,
}

impl ServerConfig {
    /// Create a new server configuration
    pub fn new(host: String, port: u16, server_type: ServerType) -> Self {
        Self {
            host,
            port,
            server_type,
        }
    }

    /// Create HTTP server configuration
    pub fn http(port: u16) -> Self {
        Self::new("0.0.0.0".to_string(), port, ServerType::HTTP)
    }

    /// Create WebSocket server configuration
    pub fn websocket(port: u16) -> Self {
        Self::new("0.0.0.0".to_string(), port, ServerType::WebSocket)
    }

    /// Create gRPC server configuration
    pub fn grpc(port: u16) -> Self {
        Self::new("0.0.0.0".to_string(), port, ServerType::GRPC)
    }

    /// Get the socket address for this configuration
    pub fn socket_addr(&self) -> Result<SocketAddr, String> {
        create_socket_addr(&self.host, self.port)
    }

    /// Get a formatted server description
    pub fn description(&self) -> String {
        match self.server_type {
            ServerType::HTTP => format!("HTTP server on {}:{}", self.host, self.port),
            ServerType::WebSocket => format!("WebSocket server on {}:{}", self.host, self.port),
            ServerType::GRPC => format!("gRPC server on {}:{}", self.host, self.port),
        }
    }
}

/// Common server traits for consistent startup behavior
///
/// This trait allows different server implementations (HTTP, WebSocket, gRPC)
/// to be started using a unified interface.
pub trait ServerStarter {
    /// Get the server type
    fn server_type(&self) -> ServerType;

    /// Get the port this server will bind to
    fn port(&self) -> u16;

    /// Start the server (implementation-specific)
    ///
    /// Returns a future that resolves when the server is running or fails to start.
    fn start_server(
        self,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send;
}

/// Helper function to start any server that implements ServerStarter
///
/// Logs server startup information and handles server initialization.
///
/// # Arguments
/// * `server` - Server instance implementing ServerStarter
///
/// # Returns
/// Result indicating success or failure of server startup
pub async fn start_server<S: ServerStarter>(
    server: S,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let port = server.port();
    let server_type = server.server_type();

    match server_type {
        ServerType::HTTP => tracing::info!("HTTP listening on port {}", port),
        ServerType::WebSocket => tracing::info!("WebSocket listening on port {}", port),
        ServerType::GRPC => tracing::info!("gRPC listening on port {}", port),
    }

    server.start_server().await
}

/// Server health check utilities
pub mod health {
    use serde::{Deserialize, Serialize};

    /// Server health status information
    #[derive(Debug, Serialize, Deserialize)]
    pub struct HealthStatus {
        /// Health status string (e.g., "healthy", "unhealthy: reason")
        pub status: String,
        /// ISO 8601 timestamp of the health check
        pub timestamp: String,
        /// Server uptime in seconds
        pub uptime_seconds: u64,
        /// Server version string
        pub version: String,
    }

    impl HealthStatus {
        /// Create a healthy status response
        pub fn healthy(uptime_seconds: u64, version: &str) -> Self {
            Self {
                status: "healthy".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                uptime_seconds,
                version: version.to_string(),
            }
        }

        /// Create an unhealthy status response with a reason
        pub fn unhealthy(reason: &str, uptime_seconds: u64, version: &str) -> Self {
            Self {
                status: format!("unhealthy: {}", reason),
                timestamp: chrono::Utc::now().to_rfc3339(),
                uptime_seconds,
                version: version.to_string(),
            }
        }
    }
}

/// Common error response utilities
pub mod errors {
    use axum::{http::StatusCode, Json};
    use serde_json::json;

    /// Create a standard JSON error response for HTTP handlers
    ///
    /// # Arguments
    /// * `status` - HTTP status code (e.g., 400, 500)
    /// * `message` - Error message
    ///
    /// # Returns
    /// Tuple of (status_code, JSON response) for use with Axum handlers
    pub fn json_error(status: StatusCode, message: &str) -> (StatusCode, Json<serde_json::Value>) {
        let error_response = json!({
            "error": {
                "message": message,
                "status_code": status.as_u16()
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        (status, Json(error_response))
    }

    /// Create a standard JSON success response for HTTP handlers
    ///
    /// # Arguments
    /// * `data` - Serializable data to include in the response
    ///
    /// # Returns
    /// Tuple of (HTTP 200 OK, JSON response) for use with Axum handlers
    pub fn json_success<T: serde::Serialize>(data: T) -> (StatusCode, Json<serde_json::Value>) {
        let success_response = json!({
            "success": true,
            "data": data,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        (StatusCode::OK, Json(success_response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_socket_addr() {
        let addr = create_socket_addr("127.0.0.1", 9080).unwrap();
        assert_eq!(addr.to_string(), "127.0.0.1:9080");
    }

    #[test]
    fn test_server_config() {
        let config = ServerConfig::http(3000);
        assert_eq!(config.port, 3000);
        assert_eq!(config.host, "0.0.0.0");
        matches!(config.server_type, ServerType::HTTP);
    }

    #[test]
    fn test_localhost_socket_addr() {
        let addr = localhost_socket_addr(9080);
        assert_eq!(addr.to_string(), "127.0.0.1:9080");
    }

    #[test]
    fn test_wildcard_socket_addr() {
        let addr = wildcard_socket_addr(9080);
        assert_eq!(addr.to_string(), "0.0.0.0:9080");
    }
}
