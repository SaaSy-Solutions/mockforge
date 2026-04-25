//! End-to-end regression: a real librdkafka-based client produces a record
//! and then fetches it back through the mock broker. Before this PR, Fetch
//! returned a stub that clients couldn't parse; after, a consumer using
//! manual partition assignment reads the exact record it just produced.
//!
//! Consumer groups (FindCoordinator/JoinGroup/Heartbeat/OffsetCommit) are
//! still stubs, so this test uses `BaseConsumer::assign()` rather than a
//! group-coordinated `StreamConsumer`. Group support is a later PR.

use mockforge_core::config::KafkaConfig;
use mockforge_kafka::{KafkaMockBroker, Topic, TopicConfig};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::{Message, TopicPartitionList};
use std::sync::Arc;
use std::time::{Duration, Instant};

async fn bind_free_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn librdkafka_produce_then_fetch_round_trip() {
    let port = bind_free_port().await;
    let config = KafkaConfig {
        port,
        host: "127.0.0.1".into(),
        ..KafkaConfig::default()
    };

    let broker = Arc::new(KafkaMockBroker::new(config.clone()).await.expect("broker init"));

    // Pre-register the topic with one partition so Metadata + Fetch both
    // find it; auto-create is only done on produce.
    {
        let mut topics = broker.topics.write().await;
        topics.insert(
            "fetch-topic".to_string(),
            Topic::new(
                "fetch-topic".to_string(),
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

    // --- Produce one record ---------------------------------------------
    let mut producer_cfg = ClientConfig::new();
    producer_cfg.set("bootstrap.servers", format!("127.0.0.1:{port}"));
    producer_cfg.set("message.timeout.ms", "5000");
    producer_cfg.set("linger.ms", "0");
    producer_cfg.set("acks", "1");
    producer_cfg.set("enable.idempotence", "false");
    let producer: FutureProducer = producer_cfg.create().expect("producer");

    producer
        .send(
            FutureRecord::<str, [u8]>::to("fetch-topic").payload(b"record-one").key("k1"),
            Duration::from_secs(5),
        )
        .await
        .expect("produce ok");

    // --- Consume via manual assignment ----------------------------------
    // Run the blocking BaseConsumer::poll on spawn_blocking so we don't tie
    // up the single-threaded test runtime.
    let port_c = port;
    let received = tokio::task::spawn_blocking(move || {
        let mut consumer_cfg = ClientConfig::new();
        consumer_cfg.set("bootstrap.servers", format!("127.0.0.1:{port_c}"));
        consumer_cfg.set("group.id", "e2e-fetch-test-group");
        consumer_cfg.set("enable.auto.commit", "false");
        consumer_cfg.set("session.timeout.ms", "6000");
        consumer_cfg.set("fetch.min.bytes", "1");
        consumer_cfg.set("fetch.wait.max.ms", "50");
        let consumer: BaseConsumer = consumer_cfg.create().expect("consumer");

        // `Offset::Beginning` drives librdkafka through the ListOffsets API
        // (key 2, earliest timestamp = -2). Before ListOffsets was
        // implemented this test hardcoded `Offset::Offset(0)` to bypass it;
        // now we exercise the real consumer-oriented resolution path.
        let mut assignment = TopicPartitionList::new();
        assignment
            .add_partition_offset("fetch-topic", 0, rdkafka::Offset::Beginning)
            .unwrap();
        consumer.assign(&assignment).expect("assign");

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if Instant::now() >= deadline {
                panic!("consumer timed out waiting for produced record");
            }
            if let Some(result) = consumer.poll(Duration::from_millis(100)) {
                let msg = result.expect("no broker error");
                let payload = msg.payload().expect("payload").to_vec();
                let key = msg.key().map(|k| k.to_vec());
                return (payload, key);
            }
        }
    })
    .await
    .expect("consumer task did not panic");

    assert_eq!(received.0, b"record-one");
    assert_eq!(received.1.as_deref(), Some(b"k1".as_ref()));

    server_handle.abort();
}
