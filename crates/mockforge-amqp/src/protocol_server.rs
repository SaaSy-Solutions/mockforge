//! Unified protocol server implementation for the AMQP mock broker.

use std::sync::Arc;

use async_trait::async_trait;
use mockforge_core::config::AmqpConfig;
use mockforge_core::protocol_abstraction::Protocol;
use mockforge_core::protocol_server::MockProtocolServer;

use crate::broker::AmqpBroker;
use crate::spec_registry::AmqpSpecRegistry;

/// A `MockProtocolServer` wrapper around [`AmqpBroker`].
///
/// Because the AMQP broker requires an `AmqpSpecRegistry` (which is async to create),
/// the broker and registry are constructed inside `start()`.
pub struct AmqpMockServer {
    config: AmqpConfig,
}

impl AmqpMockServer {
    /// Create a new `AmqpMockServer` with the given configuration.
    pub fn new(config: AmqpConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl MockProtocolServer for AmqpMockServer {
    fn protocol(&self) -> Protocol {
        Protocol::Amqp
    }

    async fn start(
        &self,
        mut shutdown: tokio::sync::watch::Receiver<()>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let spec_registry = Arc::new(AmqpSpecRegistry::new(self.config.clone()).await?);
        let broker = AmqpBroker::new(self.config.clone(), spec_registry);

        tokio::select! {
            result = broker.start() => {
                result.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
            _ = shutdown.changed() => {
                tracing::info!("Shutting down AMQP broker on port {}", self.config.port);
                Ok(())
            }
        }
    }

    fn port(&self) -> u16 {
        self.config.port
    }

    fn description(&self) -> String {
        format!("AMQP broker on {}:{}", self.config.host, self.config.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amqp_mock_server_new() {
        let config = AmqpConfig::default();
        let server = AmqpMockServer::new(config.clone());
        assert_eq!(server.port(), config.port);
    }

    #[test]
    fn test_amqp_mock_server_protocol() {
        let server = AmqpMockServer::new(AmqpConfig::default());
        assert_eq!(server.protocol(), Protocol::Amqp);
    }

    #[test]
    fn test_amqp_mock_server_description() {
        let config = AmqpConfig {
            host: "127.0.0.1".to_string(),
            port: 5672,
            ..Default::default()
        };
        let server = AmqpMockServer::new(config);
        assert_eq!(server.description(), "AMQP broker on 127.0.0.1:5672");
    }
}
