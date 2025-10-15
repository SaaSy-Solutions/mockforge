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
