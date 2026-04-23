//! End-to-end regression: a real `rdkafka::StreamConsumer` with a
//! `group.id` joins the coordinator, receives a partition assignment,
//! and reads records produced by another client.
//!
//! Before this PR the mock refused FindCoordinator / JoinGroup /
//! SyncGroup / Heartbeat entirely (not advertised in ApiVersions), so
//! any consumer configured with a `group.id` immediately errored with
//! `UnsupportedFeature (Required feature not supported by broker)`.
//! Manual partition assignment via `BaseConsumer` was the only path.
//!
//! This test locks in the single-consumer-per-group flow end to end.
//! Multi-consumer rebalancing is a later PR.

use mockforge_core::config::KafkaConfig;
use mockforge_kafka::{KafkaMockBroker, Topic, TopicConfig};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::Message;
use std::sync::Arc;
use std::time::Duration;

async fn bind_free_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stream_consumer_with_group_id_subscribes_and_receives() {
    let port = bind_free_port().await;
    let config = KafkaConfig {
        port,
        host: "127.0.0.1".into(),
        ..KafkaConfig::default()
    };
    let broker = Arc::new(KafkaMockBroker::new(config.clone()).await.expect("broker"));

    {
        let mut topics = broker.topics.write().await;
        topics.insert(
            "group-e2e-topic".to_string(),
            Topic::new(
                "group-e2e-topic".to_string(),
                TopicConfig {
                    num_partitions: 1,
                    replication_factor: 1,
                    ..Default::default()
                },
            ),
        );
    }

    let server = Arc::clone(&broker);
    let server_handle = tokio::spawn(async move { server.start().await });
    tokio::time::sleep(Duration::from_millis(250)).await;

    // Produce one record up-front.
    let mut producer_cfg = ClientConfig::new();
    producer_cfg.set("bootstrap.servers", format!("127.0.0.1:{port}"));
    producer_cfg.set("message.timeout.ms", "5000");
    producer_cfg.set("linger.ms", "0");
    producer_cfg.set("acks", "1");
    producer_cfg.set("enable.idempotence", "false");
    let producer: FutureProducer = producer_cfg.create().expect("producer");
    producer
        .send(
            FutureRecord::<str, [u8]>::to("group-e2e-topic")
                .payload(b"grouped-record")
                .key("k"),
            Duration::from_secs(5),
        )
        .await
        .expect("produce");

    // StreamConsumer with a real group.id — drives the full
    // FindCoordinator → JoinGroup → SyncGroup → Fetch → Heartbeat flow.
    let mut consumer_cfg = ClientConfig::new();
    consumer_cfg.set("bootstrap.servers", format!("127.0.0.1:{port}"));
    consumer_cfg.set("group.id", "mockforge-e2e-group");
    consumer_cfg.set("auto.offset.reset", "earliest");
    consumer_cfg.set("enable.auto.commit", "false"); // OffsetCommit is the next PR
    consumer_cfg.set("session.timeout.ms", "6000");
    consumer_cfg.set("heartbeat.interval.ms", "1000");
    let consumer: StreamConsumer = consumer_cfg.create().expect("stream consumer");
    consumer.subscribe(&["group-e2e-topic"]).expect("subscribe");

    let msg = tokio::time::timeout(Duration::from_secs(15), consumer.recv())
        .await
        .expect("consumer should receive within 15s")
        .expect("no transport error");

    let payload = msg.payload().expect("payload present").to_vec();
    let key = msg.key().map(|k| k.to_vec());
    assert_eq!(payload, b"grouped-record");
    assert_eq!(key.as_deref(), Some(b"k".as_ref()));

    // StreamConsumer's drop is synchronous and can block for several
    // seconds as librdkafka tears down its worker threads and issues a
    // LeaveGroup. Spawning it on a blocking thread keeps the tokio
    // runtime responsive; we cap it with an outer timeout so a genuine
    // hang doesn't stall the test for minutes.
    let drop_task = tokio::task::spawn_blocking(move || drop(consumer));
    let _ = tokio::time::timeout(Duration::from_secs(3), drop_task).await;
    server_handle.abort();
}
