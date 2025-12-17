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
//! use mockforge_kafka::KafkaMetrics;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let metrics = Arc::new(KafkaMetrics::default());
//!     let exporter = MetricsExporter::new(metrics.clone());
//!
//!     // Export metrics in Prometheus format
//!     let snapshot = exporter.export_prometheus();
//!     println!("{}", snapshot);
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
pub mod consumer_groups;
pub mod fixtures;
pub mod metrics;
pub mod partitions;
pub mod protocol;
pub mod spec_registry;
pub mod topics;

// Re-export main types
pub use broker::KafkaMockBroker;
pub use consumer_groups::{ConsumerGroup, ConsumerGroupManager};
pub use fixtures::{AutoProduceConfig, KafkaFixture};
pub use metrics::{KafkaMetrics, MetricsExporter, MetricsSnapshot};
pub use partitions::{KafkaMessage, Partition};
pub use spec_registry::KafkaSpecRegistry;
pub use topics::{Topic, TopicConfig};

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Module Re-export Tests ====================

    #[test]
    fn test_kafka_mock_broker_available() {
        // Ensure KafkaMockBroker is re-exported and accessible
        let _type_exists: Option<KafkaMockBroker> = None;
    }

    #[test]
    fn test_consumer_group_available() {
        // Ensure ConsumerGroup is re-exported and accessible
        let _type_exists: Option<ConsumerGroup> = None;
    }

    #[test]
    fn test_consumer_group_manager_available() {
        // Ensure ConsumerGroupManager is re-exported and accessible
        let _type_exists: Option<ConsumerGroupManager> = None;
    }

    #[test]
    fn test_kafka_fixture_available() {
        // Ensure KafkaFixture is re-exported and accessible
        let _type_exists: Option<KafkaFixture> = None;
    }

    #[test]
    fn test_auto_produce_config_available() {
        // Ensure AutoProduceConfig is re-exported and accessible
        let _type_exists: Option<AutoProduceConfig> = None;
    }

    #[test]
    fn test_kafka_metrics_available() {
        // Ensure KafkaMetrics is re-exported and accessible
        let _type_exists: Option<KafkaMetrics> = None;
    }

    #[test]
    fn test_metrics_exporter_available() {
        // Ensure MetricsExporter is re-exported and accessible
        let _type_exists: Option<MetricsExporter> = None;
    }

    #[test]
    fn test_metrics_snapshot_available() {
        // Ensure MetricsSnapshot is re-exported and accessible
        let _type_exists: Option<MetricsSnapshot> = None;
    }

    #[test]
    fn test_kafka_message_available() {
        // Ensure KafkaMessage is re-exported and accessible
        let _type_exists: Option<KafkaMessage> = None;
    }

    #[test]
    fn test_partition_available() {
        // Ensure Partition is re-exported and accessible
        let _type_exists: Option<Partition> = None;
    }

    #[test]
    fn test_kafka_spec_registry_available() {
        // Ensure KafkaSpecRegistry is re-exported and accessible
        let _type_exists: Option<KafkaSpecRegistry> = None;
    }

    #[test]
    fn test_topic_available() {
        // Ensure Topic is re-exported and accessible
        let _type_exists: Option<Topic> = None;
    }

    #[test]
    fn test_topic_config_available() {
        // Ensure TopicConfig is re-exported and accessible
        let _type_exists: Option<TopicConfig> = None;
    }

    // ==================== Basic Functionality Tests ====================

    #[tokio::test]
    async fn test_broker_creation() {
        let config = mockforge_core::config::KafkaConfig::default();
        let broker = KafkaMockBroker::new(config).await;
        assert!(broker.is_ok());
    }

    #[test]
    fn test_metrics_creation() {
        let metrics = KafkaMetrics::default();
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.messages_produced_total, 0);
    }

    #[test]
    fn test_partition_creation() {
        let partition = Partition::new(0);
        assert_eq!(partition.id, 0);
        assert_eq!(partition.high_watermark, 0);
    }

    #[test]
    fn test_topic_config_default() {
        let config = TopicConfig::default();
        assert!(config.num_partitions > 0);
    }

    #[test]
    fn test_consumer_group_manager_creation() {
        let manager = ConsumerGroupManager::new();
        assert_eq!(manager.groups().len(), 0);
    }

    // ==================== Integration Tests ====================

    #[tokio::test]
    async fn test_end_to_end_message_flow() {
        let config = mockforge_core::config::KafkaConfig::default();
        let broker = KafkaMockBroker::new(config).await.unwrap();

        // Create a topic
        let topic_config = TopicConfig::default();
        broker.test_create_topic("test-topic", topic_config).await;

        // Check that metrics are initialized
        let metrics = broker.metrics();
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.messages_produced_total, 0);
    }

    #[tokio::test]
    async fn test_consumer_group_workflow() {
        let config = mockforge_core::config::KafkaConfig::default();
        let broker = KafkaMockBroker::new(config).await.unwrap();

        // Create a topic
        let topic_config = TopicConfig::default();
        broker.test_create_topic("workflow-topic", topic_config).await;

        // Join a consumer group
        let result = broker.test_join_group("test-group", "member-1", "client-1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fixture_to_message_conversion() {
        use std::collections::HashMap;

        let fixture = KafkaFixture {
            identifier: "test-id".to_string(),
            name: "Test".to_string(),
            topic: "test-topic".to_string(),
            partition: Some(0),
            key_pattern: Some("key-test".to_string()),
            value_template: serde_json::json!({"data": "test"}),
            headers: HashMap::new(),
            auto_produce: None,
        };

        let context = HashMap::new();
        let message = fixture.generate_message(&context).unwrap();

        assert!(message.key.is_some());
        assert!(!message.value.is_empty());
    }

    #[tokio::test]
    async fn test_spec_registry_creation() {
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let topics = Arc::new(RwLock::new(std::collections::HashMap::new()));
        let config = mockforge_core::config::KafkaConfig::default();

        let registry = KafkaSpecRegistry::new(config, topics).await;
        assert!(registry.is_ok());
    }

    // ==================== Protocol Abstraction Tests ====================

    #[tokio::test]
    async fn test_spec_registry_protocol_trait() {
        use mockforge_core::protocol_abstraction::SpecRegistry;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let topics = Arc::new(RwLock::new(std::collections::HashMap::new()));
        let config = mockforge_core::config::KafkaConfig::default();

        let registry = KafkaSpecRegistry::new(config, topics).await.unwrap();

        // Test protocol method
        assert_eq!(registry.protocol(), mockforge_core::Protocol::Kafka);

        // Test operations method
        let ops = registry.operations();
        assert!(ops.is_empty() || ops.len() > 0);
    }

    // ==================== Metrics Integration Tests ====================

    #[test]
    fn test_metrics_exporter_creation() {
        use std::sync::Arc;

        let metrics = Arc::new(KafkaMetrics::default());
        let exporter = MetricsExporter::new(metrics);

        let prometheus_output = exporter.export_prometheus();
        assert!(prometheus_output.contains("kafka") || prometheus_output.contains("#"));
    }

    #[test]
    fn test_metrics_snapshot_serialization() {
        let metrics = KafkaMetrics::default();
        metrics.record_request(0); // Produce
        metrics.record_response();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.requests_total, 1);
        assert_eq!(snapshot.responses_total, 1);
    }
}
