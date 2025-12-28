//! MockForge AMQP (RabbitMQ) Protocol Support
//!
//! This crate provides AMQP 0.9.1 protocol support for MockForge,
//! enabling testing of message queue patterns, pub/sub, and enterprise messaging scenarios.
//!
//! ## Features
//!
//! - Full AMQP 0.9.1 protocol implementation
//! - Connection and channel management
//! - Exchange types: direct, fanout, topic, headers
//! - Queue operations with TTL and dead-letter support
//! - Publisher confirms and transactions
//! - Message acknowledgment tracking
//!
//! ## Metrics and Observability
//!
//! The AMQP broker includes built-in metrics collection for monitoring:
//! - Connection and channel counts
//! - Message publish/consume/ack/reject rates
//! - Queue and exchange tracking
//! - Error rates and latency
//!
//! Use [`AmqpMetrics`] to collect metrics and [`AmqpMetricsExporter`] to export
//! in Prometheus format.
//!
//! ## Example
//!
//! ```rust,ignore
//! use mockforge_amqp::{AmqpBroker, AmqpSpecRegistry};
//! use mockforge_core::config::AmqpConfig;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = AmqpConfig {
//!         enabled: true,
//!         host: "127.0.0.1".to_string(),
//!         port: 5672,
//!         ..Default::default()
//!     };
//!
//!     let spec_registry = Arc::new(AmqpSpecRegistry::new(config.clone()).await.unwrap());
//!     let broker = AmqpBroker::new(config, spec_registry);
//!
//!     // Start the broker
//!     broker.start().await.unwrap();
//! }
//! ```

pub mod bindings;
pub mod broker;
pub mod connection;
pub mod consumers;
pub mod exchanges;
pub mod fixtures;
pub mod messages;
pub mod metrics;
pub mod protocol;
pub mod queues;
pub mod spec_registry;
pub mod tls;

pub use broker::AmqpBroker;
pub use connection::AmqpConnection;
pub use metrics::{AmqpMetrics, AmqpMetricsExporter, AmqpMetricsSnapshot};
pub use spec_registry::AmqpSpecRegistry;
pub use tls::{create_tls_acceptor, create_tls_acceptor_with_client_auth, TlsError};
