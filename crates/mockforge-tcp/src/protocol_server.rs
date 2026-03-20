//! Unified protocol server implementation for the TCP mock server.

use std::sync::Arc;

use async_trait::async_trait;
use mockforge_core::protocol_abstraction::Protocol;
use mockforge_core::protocol_server::MockProtocolServer;

use crate::server::TcpServer;
use crate::spec_registry::TcpSpecRegistry;
use crate::TcpConfig;

/// A `MockProtocolServer` wrapper around [`TcpServer`].
///
/// Constructs the TCP server and spec registry, then delegates to
/// [`TcpServer::start`] with shutdown-signal integration.
pub struct TcpMockServer {
    config: TcpConfig,
}

impl TcpMockServer {
    /// Create a new `TcpMockServer` with the given configuration.
    pub fn new(config: TcpConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl MockProtocolServer for TcpMockServer {
    fn protocol(&self) -> Protocol {
        Protocol::Tcp
    }

    async fn start(
        &self,
        mut shutdown: tokio::sync::watch::Receiver<()>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let spec_registry = Arc::new(TcpSpecRegistry::new());
        let server = TcpServer::new(self.config.clone(), spec_registry)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        tokio::select! {
            result = server.start() => {
                result.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
            _ = shutdown.changed() => {
                tracing::info!("Shutting down TCP server on port {}", self.config.port);
                Ok(())
            }
        }
    }

    fn port(&self) -> u16 {
        self.config.port
    }

    fn description(&self) -> String {
        format!("TCP server on {}:{}", self.config.host, self.config.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tcp_mock_server_protocol() {
        let server = TcpMockServer::new(TcpConfig::default());
        assert_eq!(server.protocol(), Protocol::Tcp);
    }

    #[test]
    fn test_tcp_mock_server_port() {
        let config = TcpConfig {
            port: 9999,
            ..Default::default()
        };
        let server = TcpMockServer::new(config);
        assert_eq!(server.port(), 9999);
    }

    #[test]
    fn test_tcp_mock_server_description() {
        let config = TcpConfig {
            host: "127.0.0.1".to_string(),
            port: 9999,
            ..Default::default()
        };
        let server = TcpMockServer::new(config);
        assert_eq!(server.description(), "TCP server on 127.0.0.1:9999");
    }
}
