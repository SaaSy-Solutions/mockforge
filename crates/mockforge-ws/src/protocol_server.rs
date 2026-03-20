//! Unified protocol server implementation for the WebSocket mock server.

use async_trait::async_trait;
use mockforge_core::protocol_abstraction::Protocol;
use mockforge_core::protocol_server::MockProtocolServer;

use crate::LatencyProfile;

/// A `MockProtocolServer` wrapper around the WebSocket server startup.
///
/// Wraps [`crate::start_with_latency_and_host`] with shutdown-signal integration.
pub struct WsMockServer {
    port: u16,
    host: String,
    latency: Option<LatencyProfile>,
}

impl WsMockServer {
    /// Create a new `WsMockServer` with the given configuration.
    pub fn new(port: u16, host: String, latency: Option<LatencyProfile>) -> Self {
        Self {
            port,
            host,
            latency,
        }
    }
}

#[async_trait]
impl MockProtocolServer for WsMockServer {
    fn protocol(&self) -> Protocol {
        Protocol::WebSocket
    }

    async fn start(
        &self,
        mut shutdown: tokio::sync::watch::Receiver<()>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let port = self.port;
        let host = self.host.clone();
        let latency = self.latency.clone();

        tokio::select! {
            result = crate::start_with_latency_and_host(port, &host, latency) => {
                result.map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    Box::new(std::io::Error::other(e.to_string()))
                })
            }
            _ = shutdown.changed() => {
                tracing::info!("Shutting down WebSocket server on port {}", port);
                Ok(())
            }
        }
    }

    fn port(&self) -> u16 {
        self.port
    }

    fn description(&self) -> String {
        format!("WebSocket server on {}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_mock_server_protocol() {
        let server = WsMockServer::new(3001, "0.0.0.0".to_string(), None);
        assert_eq!(server.protocol(), Protocol::WebSocket);
    }

    #[test]
    fn test_ws_mock_server_port() {
        let server = WsMockServer::new(3001, "0.0.0.0".to_string(), None);
        assert_eq!(server.port(), 3001);
    }

    #[test]
    fn test_ws_mock_server_description() {
        let server = WsMockServer::new(3001, "127.0.0.1".to_string(), None);
        assert_eq!(server.description(), "WebSocket server on 127.0.0.1:3001");
    }
}
