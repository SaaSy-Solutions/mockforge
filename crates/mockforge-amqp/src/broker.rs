use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio_rustls::TlsAcceptor;

use crate::connection::AmqpConnection;
use crate::exchanges::{ExchangeManager, ExchangeType};
use crate::metrics::AmqpMetrics;
use crate::queues::QueueManager;
use crate::spec_registry::AmqpSpecRegistry;
use crate::tls::{create_tls_acceptor_with_client_auth, TlsError};
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
    pub metrics: Arc<AmqpMetrics>,
}

impl AmqpBroker {
    /// Create a new AMQP broker instance
    pub fn new(config: AmqpConfig, spec_registry: Arc<AmqpSpecRegistry>) -> Self {
        let mut exchanges = ExchangeManager::new();

        // Declare default exchanges
        exchanges.declare_exchange(String::new(), ExchangeType::Direct, true, false); // default exchange
        exchanges.declare_exchange("amq.direct".to_string(), ExchangeType::Direct, true, false);
        exchanges.declare_exchange("amq.fanout".to_string(), ExchangeType::Fanout, true, false);
        exchanges.declare_exchange("amq.topic".to_string(), ExchangeType::Topic, true, false);
        exchanges.declare_exchange("amq.headers".to_string(), ExchangeType::Headers, true, false);
        exchanges.declare_exchange("amq.match".to_string(), ExchangeType::Headers, true, false);

        let exchanges = Arc::new(RwLock::new(exchanges));
        let queues = Arc::new(RwLock::new(QueueManager::new()));

        // Wire broker managers into spec registry for PUBLISH/CONSUME integration
        spec_registry.set_broker_managers(Arc::clone(&exchanges), Arc::clone(&queues));

        Self {
            config,
            exchanges,
            queues,
            spec_registry,
            metrics: Arc::new(AmqpMetrics::new()),
        }
    }

    /// Create a new AMQP broker instance with custom metrics
    pub fn with_metrics(
        config: AmqpConfig,
        spec_registry: Arc<AmqpSpecRegistry>,
        metrics: Arc<AmqpMetrics>,
    ) -> Self {
        let mut exchanges = ExchangeManager::new();

        // Declare default exchanges
        exchanges.declare_exchange(String::new(), ExchangeType::Direct, true, false);
        exchanges.declare_exchange("amq.direct".to_string(), ExchangeType::Direct, true, false);
        exchanges.declare_exchange("amq.fanout".to_string(), ExchangeType::Fanout, true, false);
        exchanges.declare_exchange("amq.topic".to_string(), ExchangeType::Topic, true, false);
        exchanges.declare_exchange("amq.headers".to_string(), ExchangeType::Headers, true, false);
        exchanges.declare_exchange("amq.match".to_string(), ExchangeType::Headers, true, false);

        let exchanges = Arc::new(RwLock::new(exchanges));
        let queues = Arc::new(RwLock::new(QueueManager::new()));

        // Wire broker managers into spec registry for PUBLISH/CONSUME integration
        spec_registry.set_broker_managers(Arc::clone(&exchanges), Arc::clone(&queues));

        Self {
            config,
            exchanges,
            queues,
            spec_registry,
            metrics,
        }
    }

    /// Get the metrics for this broker
    pub fn metrics(&self) -> Arc<AmqpMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Start the AMQP broker server
    pub async fn start(&self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr).await.map_err(|e| {
            mockforge_core::Error::generic(format!("Failed to bind to {}: {}", addr, e))
        })?;

        tracing::info!("Starting AMQP broker on {}", addr);

        loop {
            let (socket, peer_addr) = listener.accept().await.map_err(|e| {
                mockforge_core::Error::generic(format!("Failed to accept connection: {}", e))
            })?;

            tracing::debug!("New AMQP connection from {:?}", peer_addr);

            let exchanges = Arc::clone(&self.exchanges);
            let queues = Arc::clone(&self.queues);
            let metrics = Arc::clone(&self.metrics);

            tokio::spawn(async move {
                let connection = AmqpConnection::new(socket, exchanges, queues, metrics).await;
                if let Err(e) = connection.handle().await {
                    tracing::error!("Connection error: {}", e);
                }
            });
        }
    }

    /// Start the broker and return the actual bound address
    /// Useful for testing when binding to port 0
    pub async fn start_with_addr(&self) -> Result<std::net::SocketAddr> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr).await.map_err(|e| {
            mockforge_core::Error::generic(format!("Failed to bind to {}: {}", addr, e))
        })?;

        let local_addr = listener.local_addr().map_err(|e| {
            mockforge_core::Error::generic(format!("Failed to get local address: {}", e))
        })?;

        tracing::info!("Starting AMQP broker on {}", local_addr);

        let exchanges = Arc::clone(&self.exchanges);
        let queues = Arc::clone(&self.queues);
        let metrics = Arc::clone(&self.metrics);

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((socket, peer_addr)) => {
                        tracing::debug!("New AMQP connection from {:?}", peer_addr);

                        let exchanges = Arc::clone(&exchanges);
                        let queues = Arc::clone(&queues);
                        let metrics = Arc::clone(&metrics);

                        tokio::spawn(async move {
                            let connection =
                                AmqpConnection::new(socket, exchanges, queues, metrics).await;
                            if let Err(e) = connection.handle().await {
                                tracing::error!("Connection error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Failed to accept connection: {}", e);
                    }
                }
            }
        });

        Ok(local_addr)
    }

    /// Start the TLS-enabled AMQP broker server
    ///
    /// This starts an AMQPS listener on the configured TLS port (default 5671).
    /// Requires TLS certificate and key to be configured.
    pub async fn start_tls(&self) -> std::result::Result<(), TlsError> {
        if !self.config.tls_enabled {
            return Err(TlsError::ConfigError("TLS is not enabled in configuration".to_string()));
        }

        let tls_acceptor = create_tls_acceptor_with_client_auth(&self.config)?;
        let addr = format!("{}:{}", self.config.host, self.config.tls_port);

        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| TlsError::ConfigError(format!("Failed to bind to {}: {}", addr, e)))?;

        tracing::info!("Starting AMQPS broker with TLS on {}", addr);

        let exchanges = Arc::clone(&self.exchanges);
        let queues = Arc::clone(&self.queues);
        let metrics = Arc::clone(&self.metrics);

        self.run_tls_accept_loop(listener, tls_acceptor, exchanges, queues, metrics)
            .await;

        Ok(())
    }

    /// Start the TLS-enabled broker and return the actual bound address
    /// Useful for testing when binding to port 0
    pub async fn start_tls_with_addr(&self) -> std::result::Result<std::net::SocketAddr, TlsError> {
        if !self.config.tls_enabled {
            return Err(TlsError::ConfigError("TLS is not enabled in configuration".to_string()));
        }

        let tls_acceptor = create_tls_acceptor_with_client_auth(&self.config)?;
        let addr = format!("{}:{}", self.config.host, self.config.tls_port);

        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| TlsError::ConfigError(format!("Failed to bind to {}: {}", addr, e)))?;

        let local_addr = listener
            .local_addr()
            .map_err(|e| TlsError::ConfigError(format!("Failed to get local address: {}", e)))?;

        tracing::info!("Starting AMQPS broker with TLS on {}", local_addr);

        let exchanges = Arc::clone(&self.exchanges);
        let queues = Arc::clone(&self.queues);
        let metrics = Arc::clone(&self.metrics);

        tokio::spawn(async move {
            Self::run_tls_accept_loop_static(listener, tls_acceptor, exchanges, queues, metrics)
                .await;
        });

        Ok(local_addr)
    }

    /// Start both plain and TLS listeners concurrently
    pub async fn start_dual(&self) -> Result<(std::net::SocketAddr, Option<std::net::SocketAddr>)> {
        let plain_addr = self.start_with_addr().await?;

        let tls_addr = if self.config.tls_enabled {
            match self.start_tls_with_addr().await {
                Ok(addr) => Some(addr),
                Err(e) => {
                    tracing::warn!("Failed to start TLS listener: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Ok((plain_addr, tls_addr))
    }

    /// Internal TLS accept loop (blocking)
    async fn run_tls_accept_loop(
        &self,
        listener: TcpListener,
        tls_acceptor: TlsAcceptor,
        exchanges: Arc<RwLock<ExchangeManager>>,
        queues: Arc<RwLock<QueueManager>>,
        metrics: Arc<AmqpMetrics>,
    ) {
        Self::run_tls_accept_loop_static(listener, tls_acceptor, exchanges, queues, metrics).await;
    }

    /// Static TLS accept loop for spawning
    async fn run_tls_accept_loop_static(
        listener: TcpListener,
        tls_acceptor: TlsAcceptor,
        exchanges: Arc<RwLock<ExchangeManager>>,
        queues: Arc<RwLock<QueueManager>>,
        metrics: Arc<AmqpMetrics>,
    ) {
        loop {
            match listener.accept().await {
                Ok((socket, peer_addr)) => {
                    tracing::debug!("New AMQPS connection from {:?}", peer_addr);

                    let tls_acceptor = tls_acceptor.clone();
                    let exchanges = Arc::clone(&exchanges);
                    let queues = Arc::clone(&queues);
                    let metrics = Arc::clone(&metrics);

                    tokio::spawn(async move {
                        match tls_acceptor.accept(socket).await {
                            Ok(tls_stream) => {
                                // Use compat layer for TLS stream
                                let connection =
                                    AmqpConnection::new_tls(tls_stream, exchanges, queues, metrics)
                                        .await;
                                if let Err(e) = connection.handle().await {
                                    tracing::error!("TLS connection error: {}", e);
                                }
                            }
                            Err(e) => {
                                tracing::error!("TLS handshake failed from {:?}: {}", peer_addr, e);
                            }
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to accept TLS connection: {}", e);
                }
            }
        }
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
            ..Default::default()
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
        // Should have default exchanges (amq.direct, amq.fanout, amq.topic, amq.headers, amq.match, and default "")
        assert_eq!(exchanges.list_exchanges().len(), 6);
        assert!(exchanges.get_exchange("amq.direct").is_some());
        assert!(exchanges.get_exchange("amq.fanout").is_some());
        assert!(exchanges.get_exchange("amq.topic").is_some());
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
        // 6 default exchanges + 1 user-declared
        assert_eq!(exchanges.list_exchanges().len(), 7);
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
        // 6 default exchanges + 2 user-declared
        assert_eq!(exchanges.list_exchanges().len(), 8);
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

        // 6 default exchanges + 1 user-declared
        assert_eq!(exchanges.list_exchanges().len(), 7);
        assert_eq!(queues.list_queues().len(), 1);
    }

    #[tokio::test]
    async fn test_broker_metrics() {
        let config = create_test_config();
        let spec_registry = Arc::new(AmqpSpecRegistry::new(config.clone()).await.unwrap());
        let broker = AmqpBroker::new(config, spec_registry);

        let metrics = broker.metrics();
        let snapshot = metrics.snapshot();

        // Initial metrics should be zero
        assert_eq!(snapshot.connections_total, 0);
        assert_eq!(snapshot.messages_published_total, 0);
    }

    #[tokio::test]
    async fn test_broker_with_custom_metrics() {
        let config = create_test_config();
        let spec_registry = Arc::new(AmqpSpecRegistry::new(config.clone()).await.unwrap());
        let metrics = Arc::new(AmqpMetrics::new());

        // Record some metrics before creating broker
        metrics.record_connection();
        metrics.record_publish();

        let broker = AmqpBroker::with_metrics(config, spec_registry, Arc::clone(&metrics));

        let snapshot = broker.metrics().snapshot();
        assert_eq!(snapshot.connections_total, 1);
        assert_eq!(snapshot.messages_published_total, 1);
    }
}
