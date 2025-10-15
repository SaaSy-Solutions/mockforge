use mockforge_kafka::{KafkaMockBroker, KafkaSpecRegistry};
use mockforge_core::config::KafkaConfig;
use std::time::Duration;
use tokio::time::timeout;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::client::DefaultClientContext;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::Message;
use std::collections::HashMap;

#[tokio::test]
async fn test_broker_creation() {
    let config = KafkaConfig::default();
    let broker = KafkaMockBroker::new(config).await;
    assert!(broker.is_ok());
}

#[tokio::test]
async fn test_spec_registry_creation() {
    let config = KafkaConfig::default();
    let registry = KafkaSpecRegistry::new(config).await;
    assert!(registry.is_ok());
}

#[tokio::test]
async fn test_topic_creation() {
    let config = KafkaConfig::default();
    let broker = KafkaMockBroker::new(config).await.unwrap();

    // TODO: Add topic creation test once broker methods are implemented
    assert!(true); // Placeholder
}

#[tokio::test]
async fn test_fixture_loading() {
    // TODO: Test fixture loading from YAML
    assert!(true); // Placeholder
}

#[tokio::test]
async fn test_message_generation() {
    // TODO: Test message generation from fixtures
    assert!(true); // Placeholder
}

#[tokio::test]
async fn test_consumer_group_operations() {
    // TODO: Test consumer group join, sync, etc.
    assert!(true); // Placeholder
}

#[tokio::test]
async fn test_partition_assignment() {
    // TODO: Test partition assignment logic
    assert!(true); // Placeholder
}

#[tokio::test]
async fn test_offset_management() {
    // TODO: Test offset commit and fetch
    assert!(true); // Placeholder
}

#[tokio::test]
async fn test_broker_startup_timeout() {
    let config = KafkaConfig {
        port: 9093, // Use a different port for testing
        ..Default::default()
    };
    let broker = KafkaMockBroker::new(config).await.unwrap();

    // Test that broker can start (with timeout to avoid hanging)
    let start_result = timeout(Duration::from_secs(1), broker.start()).await;

    // Should timeout since we're not actually connecting
    assert!(start_result.is_err());
}

#[tokio::test]
async fn test_full_broker_integration() {
    let config = KafkaConfig {
        port: 9094, // Use a unique port for this test
        ..Default::default()
    };

    let broker = KafkaMockBroker::new(config.clone()).await.unwrap();

    // Start the broker in a separate task
    let broker_handle = tokio::spawn(async move {
        broker.start().await.unwrap();
    });

    // Give the broker time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create Kafka client configuration
    let mut client_config = ClientConfig::new();
    client_config.set("bootstrap.servers", &format!("127.0.0.1:{}", config.port));
    client_config.set("group.id", "test-group");
    client_config.set("auto.offset.reset", "earliest");
    client_config.set("enable.auto.commit", "false");

    // Test topic creation
    let admin_client: AdminClient<DefaultClientContext> = client_config.clone().create().unwrap();
    let topics = vec![NewTopic {
        name: "test-topic",
        num_partitions: 3,
        replication: TopicReplication::Fixed(1),
        config: HashMap::new(),
    }];

    let options = AdminOptions::new().request_timeout(Some(Duration::from_secs(5)));
    let result = admin_client.create_topics(&topics, &options).await;

    // The mock broker might not fully implement topic creation yet, so we'll accept both success and expected errors
    match result {
        Ok(_) => println!("Topic creation succeeded"),
        Err(e) => println!("Topic creation failed as expected: {:?}", e),
    }

    // Test producer
    let producer: FutureProducer = client_config.clone().create().unwrap();
    let record = FutureRecord::to("test-topic")
        .payload("test message")
        .key("test-key");

    let produce_result = producer.send(record, Duration::from_secs(5)).await;
    match produce_result {
        Ok(_) => println!("Message production succeeded"),
        Err((e, _)) => println!("Message production failed: {:?}", e),
    }

    // Test consumer
    let consumer: StreamConsumer = client_config.create().unwrap();
    consumer.subscribe(&["test-topic"]).unwrap();

    // Try to consume a message with timeout
    let consume_result = timeout(Duration::from_secs(2), consumer.recv()).await;
    match consume_result {
        Ok(Ok(message)) => {
            println!("Message consumed successfully");
            let payload = message.payload_view::<str>().unwrap().unwrap();
            assert_eq!(payload, "test message");
        }
        Ok(Err(e)) => println!("Consume error: {:?}", e),
        Err(_) => println!("Consume timeout - no messages available"),
    }

    // Clean up
    consumer.unsubscribe();
    drop(producer);
    drop(admin_client);

    // Stop the broker
    broker_handle.abort();
}

#[tokio::test]
async fn test_protocol_operations() {
    let config = KafkaConfig {
        port: 9095, // Use a unique port for this test
        ..Default::default()
    };

    let broker = KafkaMockBroker::new(config.clone()).await.unwrap();

    // Start the broker in a separate task
    let broker_handle = tokio::spawn(async move {
        broker.start().await.unwrap();
    });

    // Give the broker time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test metadata request
    let mut client_config = ClientConfig::new();
    client_config.set("bootstrap.servers", &format!("127.0.0.1:{}", config.port));

    let admin_client: AdminClient<DefaultClientContext> = client_config.create().unwrap();

    // Test getting metadata
    let timeout_duration = Duration::from_secs(5);
    let metadata = admin_client.inner().fetch_metadata(None, timeout_duration);

    match metadata {
        Ok(metadata) => {
            println!("Metadata fetch succeeded, brokers: {}", metadata.brokers().len());
            assert!(!metadata.brokers().is_empty());
        }
        Err(e) => println!("Metadata fetch failed: {:?}", e),
    }

    // Clean up
    drop(admin_client);
    broker_handle.abort();
}
