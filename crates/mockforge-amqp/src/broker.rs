use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;

use crate::exchanges::ExchangeManager;
use crate::protocol::ConnectionHandler;
use crate::queues::QueueManager;
use crate::spec_registry::AmqpSpecRegistry;
use mockforge_core::config::AmqpConfig;
use mockforge_core::Result;

/// Mock AMQP broker implementation
///
/// The `AmqpBroker` simulates a RabbitMQ-compatible AMQP 0.9.1 broker,
/// handling connections and responding to AMQP protocol requests.
pub struct AmqpBroker {
    pub config: AmqpConfig,
    pub exchanges: Arc<RwLock<ExchangeManager>>,
    pub queues: Arc<RwLock<QueueManager>>,
    pub spec_registry: Arc<AmqpSpecRegistry>,
}

impl AmqpBroker {
    /// Create a new AMQP broker instance
    pub fn new(config: AmqpConfig, spec_registry: Arc<AmqpSpecRegistry>) -> Self {
        Self {
            config,
            exchanges: Arc::new(RwLock::new(ExchangeManager::new())),
            queues: Arc::new(RwLock::new(QueueManager::new())),
            spec_registry,
        }
    }

    /// Start the AMQP broker server
    pub async fn start(&self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr).await.map_err(|e| {
            mockforge_core::Error::generic(format!("Failed to bind to {}: {}", addr, e))
        })?;

        tracing::info!("Starting AMQP broker on {}", addr);

        loop {
            let (socket, _) = listener.accept().await.map_err(|e| {
                mockforge_core::Error::generic(format!("Failed to accept connection: {}", e))
            })?;

            let exchanges = Arc::clone(&self.exchanges);
            let queues = Arc::clone(&self.queues);
            let spec_registry = Arc::clone(&self.spec_registry);

            tokio::spawn(async move {
                if let Err(e) =
                    Self::handle_connection(socket, exchanges, queues, spec_registry).await
                {
                    tracing::error!("Connection error: {}", e);
                }
            });
        }
    }

    async fn handle_connection(
        socket: tokio::net::TcpStream,
        _exchanges: Arc<RwLock<ExchangeManager>>,
        _queues: Arc<RwLock<QueueManager>>,
        _spec_registry: Arc<AmqpSpecRegistry>,
    ) -> Result<()> {
        let handler = ConnectionHandler::new(socket);
        handler
            .handle()
            .await
            .map_err(|e| mockforge_core::Error::generic(format!("Connection handler error: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_core::config::AmqpConfig;

    fn create_test_config() -> AmqpConfig {
        AmqpConfig {
            enabled: true,
            host: "127.0.0.1".to_string(),
            port: 0, // Use port 0 for testing
            max_connections: 100,
            max_channels_per_connection: 100,
            frame_max: 131072,
            heartbeat_interval: 60,
            fixtures_dir: None,
            virtual_hosts: vec!["/".to_string()],
        }
    }

    #[tokio::test]
    async fn test_broker_new() {
        let config = create_test_config();
        let spec_registry = Arc::new(AmqpSpecRegistry::new(config.clone()).await.unwrap());
        let broker = AmqpBroker::new(config.clone(), spec_registry.clone());

        assert_eq!(broker.config.host, "127.0.0.1");
        assert_eq!(broker.config.port, 0);
    }

    #[tokio::test]
    async fn test_broker_exchanges_initialized() {
        let config = create_test_config();
        let spec_registry = Arc::new(AmqpSpecRegistry::new(config.clone()).await.unwrap());
        let broker = AmqpBroker::new(config, spec_registry);

        let exchanges = broker.exchanges.read().await;
        assert!(exchanges.list_exchanges().is_empty());
    }

    #[tokio::test]
    async fn test_broker_queues_initialized() {
        let config = create_test_config();
        let spec_registry = Arc::new(AmqpSpecRegistry::new(config.clone()).await.unwrap());
        let broker = AmqpBroker::new(config, spec_registry);

        let queues = broker.queues.read().await;
        assert!(queues.list_queues().is_empty());
    }

    #[tokio::test]
    async fn test_broker_can_declare_exchange() {
        let config = create_test_config();
        let spec_registry = Arc::new(AmqpSpecRegistry::new(config.clone()).await.unwrap());
        let broker = AmqpBroker::new(config, spec_registry);

        {
            let mut exchanges = broker.exchanges.write().await;
            exchanges.declare_exchange(
                "test-exchange".to_string(),
                crate::exchanges::ExchangeType::Direct,
                true,
                false,
            );
        }

        let exchanges = broker.exchanges.read().await;
        assert_eq!(exchanges.list_exchanges().len(), 1);
        assert!(exchanges.get_exchange("test-exchange").is_some());
    }

    #[tokio::test]
    async fn test_broker_can_declare_queue() {
        let config = create_test_config();
        let spec_registry = Arc::new(AmqpSpecRegistry::new(config.clone()).await.unwrap());
        let broker = AmqpBroker::new(config, spec_registry);

        {
            let mut queues = broker.queues.write().await;
            queues.declare_queue("test-queue".to_string(), true, false, false);
        }

        let queues = broker.queues.read().await;
        assert_eq!(queues.list_queues().len(), 1);
        assert!(queues.get_queue("test-queue").is_some());
    }

    #[tokio::test]
    async fn test_broker_spec_registry() {
        let config = create_test_config();
        let spec_registry = Arc::new(AmqpSpecRegistry::new(config.clone()).await.unwrap());
        let broker = AmqpBroker::new(config, spec_registry.clone());

        assert!(Arc::ptr_eq(&broker.spec_registry, &spec_registry));
    }

    #[tokio::test]
    async fn test_broker_config_with_virtual_hosts() {
        let mut config = create_test_config();
        config.virtual_hosts = vec!["/".to_string(), "/test-vhost".to_string()];

        let spec_registry = Arc::new(AmqpSpecRegistry::new(config.clone()).await.unwrap());
        let broker = AmqpBroker::new(config, spec_registry);

        assert_eq!(broker.config.virtual_hosts.len(), 2);
        assert!(broker.config.virtual_hosts.contains(&"/test-vhost".to_string()));
    }

    #[tokio::test]
    async fn test_broker_multiple_exchanges() {
        let config = create_test_config();
        let spec_registry = Arc::new(AmqpSpecRegistry::new(config.clone()).await.unwrap());
        let broker = AmqpBroker::new(config, spec_registry);

        {
            let mut exchanges = broker.exchanges.write().await;
            exchanges.declare_exchange(
                "exchange1".to_string(),
                crate::exchanges::ExchangeType::Direct,
                true,
                false,
            );
            exchanges.declare_exchange(
                "exchange2".to_string(),
                crate::exchanges::ExchangeType::Fanout,
                false,
                true,
            );
        }

        let exchanges = broker.exchanges.read().await;
        assert_eq!(exchanges.list_exchanges().len(), 2);
    }

    #[tokio::test]
    async fn test_broker_multiple_queues() {
        let config = create_test_config();
        let spec_registry = Arc::new(AmqpSpecRegistry::new(config.clone()).await.unwrap());
        let broker = AmqpBroker::new(config, spec_registry);

        {
            let mut queues = broker.queues.write().await;
            queues.declare_queue("queue1".to_string(), true, false, false);
            queues.declare_queue("queue2".to_string(), false, true, false);
            queues.declare_queue("queue3".to_string(), false, false, true);
        }

        let queues = broker.queues.read().await;
        assert_eq!(queues.list_queues().len(), 3);
    }

    #[tokio::test]
    async fn test_broker_concurrent_access() {
        let config = create_test_config();
        let spec_registry = Arc::new(AmqpSpecRegistry::new(config.clone()).await.unwrap());
        let broker = Arc::new(AmqpBroker::new(config, spec_registry));

        let broker1 = Arc::clone(&broker);
        let broker2 = Arc::clone(&broker);

        let handle1 = tokio::spawn(async move {
            let mut exchanges = broker1.exchanges.write().await;
            exchanges.declare_exchange(
                "exchange-from-task1".to_string(),
                crate::exchanges::ExchangeType::Direct,
                true,
                false,
            );
        });

        let handle2 = tokio::spawn(async move {
            let mut queues = broker2.queues.write().await;
            queues.declare_queue("queue-from-task2".to_string(), true, false, false);
        });

        handle1.await.unwrap();
        handle2.await.unwrap();

        let exchanges = broker.exchanges.read().await;
        let queues = broker.queues.read().await;

        assert_eq!(exchanges.list_exchanges().len(), 1);
        assert_eq!(queues.list_queues().len(), 1);
    }
}
