//! Unified protocol server implementation for the Kafka mock broker.

use async_trait::async_trait;
use mockforge_core::config::KafkaConfig;
use mockforge_core::protocol_abstraction::Protocol;
use mockforge_core::protocol_server::MockProtocolServer;

use crate::broker::KafkaMockBroker;
use crate::filter::MessageFilter;
use std::sync::Arc;

/// A `MockProtocolServer` wrapper around [`KafkaMockBroker`].
///
/// This struct holds the configuration needed to create and start a Kafka broker.
/// Because `KafkaMockBroker::new` is async (it initializes the spec registry),
/// the broker is created inside `start()` rather than at construction time.
pub struct KafkaMockServer {
    config: KafkaConfig,
    message_filter: Option<Arc<dyn MessageFilter>>,
}

impl KafkaMockServer {
    /// Create a new `KafkaMockServer` with the given configuration.
    pub fn new(config: KafkaConfig) -> Self {
        Self {
            config,
            message_filter: None,
        }
    }

    /// Install a Produce-message retention filter for the broker instance
    /// started by this server.
    pub fn with_filter(mut self, filter: Arc<dyn MessageFilter>) -> Self {
        self.message_filter = Some(filter);
        self
    }
}

#[async_trait]
impl MockProtocolServer for KafkaMockServer {
    fn protocol(&self) -> Protocol {
        Protocol::Kafka
    }

    async fn start(
        &self,
        mut shutdown: tokio::sync::watch::Receiver<()>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let broker =
            KafkaMockBroker::new_with_filter(self.config.clone(), self.message_filter.clone())
                .await?;

        tokio::select! {
            result = broker.start() => {
                result.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
            _ = shutdown.changed() => {
                tracing::info!("Shutting down Kafka broker on port {}", self.config.port);
                Ok(())
            }
        }
    }

    fn port(&self) -> u16 {
        self.config.port
    }

    fn description(&self) -> String {
        format!("Kafka broker on {}:{}", self.config.host, self.config.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::partitions::KafkaMessage;

    #[test]
    fn test_kafka_mock_server_new() {
        let config = KafkaConfig::default();
        let server = KafkaMockServer::new(config.clone());
        assert_eq!(server.port(), config.port);
    }

    #[test]
    fn test_kafka_mock_server_protocol() {
        let server = KafkaMockServer::new(KafkaConfig::default());
        assert_eq!(server.protocol(), Protocol::Kafka);
    }

    struct KeepAllFilter;

    impl MessageFilter for KeepAllFilter {
        fn should_keep(&self, _topic: &str, _partition: i32, _message: &KafkaMessage) -> bool {
            true
        }
    }

    #[test]
    fn test_kafka_mock_server_with_filter() {
        let server =
            KafkaMockServer::new(KafkaConfig::default()).with_filter(Arc::new(KeepAllFilter));
        assert!(server.message_filter.is_some());
    }

    #[test]
    fn test_kafka_mock_server_description() {
        let config = KafkaConfig {
            host: "127.0.0.1".to_string(),
            port: 9092,
            ..Default::default()
        };
        let server = KafkaMockServer::new(config);
        assert_eq!(server.description(), "Kafka broker on 127.0.0.1:9092");
    }
}
