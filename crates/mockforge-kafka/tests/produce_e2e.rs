//! End-to-end regression: a real librdkafka-based client producing to the
//! mock broker must succeed AND the produced record must actually land in
//! the broker's in-memory storage. Before this PR, Produce requests got the
//! generic stub response and clients hit `PROTOUFLOW`; after, a full
//! Produce v9 round-trip works.

use mockforge_core::config::KafkaConfig;
use mockforge_kafka::KafkaMockBroker;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::sync::Arc;
use std::time::Duration;

async fn bind_free_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

#[tokio::test]
async fn librdkafka_produce_round_trip() {
    let port = bind_free_port().await;
    let config = KafkaConfig {
        port,
        host: "127.0.0.1".into(),
        ..KafkaConfig::default()
    };

    let broker = Arc::new(KafkaMockBroker::new(config.clone()).await.expect("broker init"));

    // Pre-register the topic so librdkafka's Metadata probe sees it and
    // proceeds to produce. Without this, librdkafka's
    // `allow.auto.create.topics=true` flow never lands on a topic (our
    // Metadata handler only advertises topics from the fixture registry
    // and this test doesn't load one).
    {
        let mut topics = broker.topics.write().await;
        topics.insert(
            "e2e-topic".to_string(),
            mockforge_kafka::Topic::new(
                "e2e-topic".to_string(),
                mockforge_kafka::TopicConfig {
                    num_partitions: 1,
                    replication_factor: 1,
                    ..Default::default()
                },
            ),
        );
    }

    let server = Arc::clone(&broker);
    let server_handle = tokio::spawn(async move { server.start().await });

    // Wait for the broker's listener to be ready.
    tokio::time::sleep(Duration::from_millis(250)).await;

    let mut client = ClientConfig::new();
    client.set("bootstrap.servers", format!("127.0.0.1:{port}"));
    client.set("message.timeout.ms", "5000");
    // Force linger to zero so the single test record ships immediately,
    // and disable idempotence/transactions — those need more protocol
    // support than this PR delivers.
    client.set("linger.ms", "0");
    client.set("acks", "1");
    client.set("enable.idempotence", "false");
    let producer: FutureProducer = client.create().expect("producer");

    let delivery = producer
        .send(
            FutureRecord::<str, [u8]>::to("e2e-topic").payload(b"hello-e2e").key("k1"),
            Duration::from_secs(5),
        )
        .await;

    match delivery {
        Ok(d) => {
            assert!(d.partition >= 0, "negative partition from broker: {}", d.partition);
            assert!(d.offset >= 0, "negative offset from broker: {}", d.offset);
        }
        Err((e, _)) => panic!("producer.send failed: {e:?}"),
    }

    // Record must actually be in storage.
    let topics = broker.topics.read().await;
    let topic = topics.get("e2e-topic").expect("topic auto-created on produce");
    let total_records: usize = topic.partitions.iter().map(|p| p.messages.len()).sum();
    assert_eq!(total_records, 1, "expected exactly one record in storage");
    let stored = topic.partitions.iter().flat_map(|p| p.messages.iter()).next().unwrap();
    assert_eq!(stored.value, b"hello-e2e");
    assert_eq!(stored.key.as_deref(), Some(b"k1".as_ref()));

    server_handle.abort();
}
