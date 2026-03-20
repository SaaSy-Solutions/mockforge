//! Unified protocol server implementation for the SMTP mock server.

use std::sync::Arc;

use async_trait::async_trait;
use mockforge_core::protocol_abstraction::Protocol;
use mockforge_core::protocol_server::MockProtocolServer;

use crate::server::SmtpServer;
use crate::spec_registry::SmtpSpecRegistry;
use crate::SmtpConfig;

/// A `MockProtocolServer` wrapper around [`SmtpServer`].
///
/// Constructs the SMTP server and spec registry, then delegates to
/// [`SmtpServer::start`] with shutdown-signal integration.
pub struct SmtpMockServer {
    config: SmtpConfig,
}

impl SmtpMockServer {
    /// Create a new `SmtpMockServer` with the given configuration.
    pub fn new(config: SmtpConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl MockProtocolServer for SmtpMockServer {
    fn protocol(&self) -> Protocol {
        Protocol::Smtp
    }

    async fn start(
        &self,
        mut shutdown: tokio::sync::watch::Receiver<()>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let spec_registry = Arc::new(SmtpSpecRegistry::new());
        let server = SmtpServer::new(self.config.clone(), spec_registry)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        tokio::select! {
            result = server.start() => {
                result.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
            _ = shutdown.changed() => {
                tracing::info!("Shutting down SMTP server on port {}", self.config.port);
                Ok(())
            }
        }
    }

    fn port(&self) -> u16 {
        self.config.port
    }

    fn description(&self) -> String {
        format!("SMTP server on {}:{}", self.config.host, self.config.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smtp_mock_server_protocol() {
        let server = SmtpMockServer::new(SmtpConfig::default());
        assert_eq!(server.protocol(), Protocol::Smtp);
    }

    #[test]
    fn test_smtp_mock_server_port() {
        let server = SmtpMockServer::new(SmtpConfig::default());
        assert_eq!(server.port(), server.config.port);
    }

    #[test]
    fn test_smtp_mock_server_description() {
        let config = SmtpConfig {
            host: "127.0.0.1".to_string(),
            port: 2525,
            ..Default::default()
        };
        let server = SmtpMockServer::new(config);
        assert_eq!(server.description(), "SMTP server on 127.0.0.1:2525");
    }
}
