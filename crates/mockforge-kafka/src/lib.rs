//! # MockForge Kafka
//!
//! Kafka protocol support for MockForge.
//!
//! This crate provides Kafka-specific functionality for creating mock Kafka brokers,
//! including topic management, partition handling, consumer groups, and fixture-driven message generation.
//!
//! ## Overview
//!
//! MockForge Kafka enables you to:
//!
//! - **Mock Kafka brokers**: Simulate Apache Kafka brokers for testing
//! - **Topic and partition management**: Create and manage topics with configurable partitions
//! - **Producer/consumer simulation**: Handle produce and fetch requests
//! - **Consumer group coordination**: Simulate consumer group behavior and rebalancing
//! - **Fixture-based messaging**: Generate messages using templates and patterns
//! - **Auto-produce functionality**: Automatically generate messages at specified rates
//!
//! ## Quick Start
//!
//! ### Basic Kafka Broker
//!
//! ```rust,no_run
//! use mockforge_kafka::KafkaMockBroker;
//! use mockforge_core::config::KafkaConfig;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = KafkaConfig::default();
//!     let broker = KafkaMockBroker::new(config).await?;
//!
//!     broker.start().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Key Features
//!
//! ### Broker Simulation
//! - **10+ Kafka APIs supported**: Produce, Fetch, Metadata, ListGroups, DescribeGroups, ApiVersions, CreateTopics, DeleteTopics, DescribeConfigs
//! - **Protocol-compliant responses**: Full Kafka protocol implementation without external dependencies
//! - **Connection handling**: TCP-based broker connections with proper error handling
//! - **Topic and partition management**: Dynamic topic/partition creation and management
//!
//! ### Metrics and Monitoring
//! - **Comprehensive metrics**: Request counts, error rates, connection tracking
//! - **Prometheus integration**: Export metrics in Prometheus format
//! - **Real-time monitoring**: Live metrics collection during broker operation
//!
//! ### Fixture System
//! - **YAML-based fixtures**: Define message templates and auto-production rules
//! - **Template engine integration**: Use MockForge's templating system for dynamic content
//! - **Auto-produce functionality**: Automatically generate messages at specified rates
//! - **Key and value templating**: Flexible message generation with context variables
//!
//! ### Consumer Group Management
//! - **Group coordination**: Simulate consumer group joins and synchronization
//! - **Partition assignment**: Automatic partition distribution among consumers
//! - **Offset management**: Track and manage consumer offsets
//! - **Rebalancing simulation**: Test consumer group rebalancing scenarios
//!
//! ### Testing Features
//! - **Protocol validation**: Ensure requests conform to Kafka protocol specifications
//! - **Error simulation**: Configurable error responses for testing error handling
//! - **Performance testing**: Built-in benchmarking support
//! - **Integration testing**: Compatible with standard Kafka client libraries
//!
//! ## Supported Kafka APIs
//!
//! - **Produce** (API Key 0): Message production with acknowledgments
//! - **Fetch** (API Key 1): Message consumption with offset management
//! - **Metadata** (API Key 3): Topic and broker metadata discovery
//! - **ListGroups** (API Key 9): Consumer group listing
//! - **DescribeGroups** (API Key 15): Consumer group details and member information
//! - **ApiVersions** (API Key 18): Protocol version negotiation
//! - **CreateTopics** (API Key 19): Dynamic topic creation
//! - **DeleteTopics** (API Key 20): Topic deletion
//! - **DescribeConfigs** (API Key 32): Configuration retrieval
//!
//! ## Example Usage
//!
//! ### Basic Broker Setup
//!
//! ```rust,no_run
//! use mockforge_kafka::KafkaMockBroker;
//! use mockforge_core::config::KafkaConfig;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = KafkaConfig {
//!         port: 9092,
//!         ..Default::default()
//!     };
//!
//!     let broker = KafkaMockBroker::new(config).await?;
//!     broker.start().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Metrics Export
//!
//! ```rust,no_run
//! use mockforge_kafka::MetricsExporter;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let exporter = MetricsExporter::new();
//!
//!     // Export metrics in Prometheus format
//!     let metrics = exporter.export_prometheus().await?;
//!     println!("{}", metrics);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Related Crates
//!
//! - [`mockforge-core`](https://docs.rs/mockforge-core): Core mocking functionality and configuration
//! - [`rdkafka`](https://docs.rs/rdkafka): Kafka client library for testing integration

pub mod broker;
pub mod topics;
pub mod partitions;
pub mod consumer_groups;
pub mod fixtures;
pub mod spec_registry;
pub mod protocol;
pub mod metrics;

// Re-export main types
pub use broker::KafkaMockBroker;
pub use topics::{Topic, TopicConfig};
pub use partitions::{Partition, KafkaMessage};
pub use consumer_groups::{ConsumerGroupManager, ConsumerGroup};
pub use fixtures::{KafkaFixture, AutoProduceConfig};
pub use spec_registry::KafkaSpecRegistry;
pub use metrics::{KafkaMetrics, MetricsExporter, MetricsSnapshot};
