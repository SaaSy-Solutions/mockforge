//! Real-client round-trip through the compression path.
//!
//! Before this PR, compressed Produce batches (any `compression.type` other
//! than `none`) returned `UNSUPPORTED_COMPRESSION_TYPE` (74). This test
//! drives the broker with librdkafka producers configured for snappy and
//! gzip, then verifies consumers can read the records back.

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

async fn start_broker_with_topic(
    topic: &str,
) -> (u16, Arc<KafkaMockBroker>, tokio::task::JoinHandle<mockforge_core::Result<()>>) {
    let port = bind_free_port().await;
    let config = KafkaConfig {
        port,
        host: "127.0.0.1".into(),
        ..KafkaConfig::default()
    };
    let broker = Arc::new(KafkaMockBroker::new(config).await.expect("broker init"));
    {
        let mut topics = broker.topics.write().await;
        topics.insert(
            topic.to_string(),
            Topic::new(
                topic.to_string(),
                TopicConfig {
                    num_partitions: 1,
                    replication_factor: 1,
                    ..Default::default()
                },
            ),
        );
    }
    let server = Arc::clone(&broker);
    let handle = tokio::spawn(async move { server.start().await });
    tokio::time::sleep(Duration::from_millis(250)).await;
    (port, broker, handle)
}

async fn produce_one(port: u16, topic: &str, compression: &str, key: &str, value: &[u8]) {
    let mut cfg = ClientConfig::new();
    cfg.set("bootstrap.servers", format!("127.0.0.1:{port}"));
    cfg.set("message.timeout.ms", "5000");
    cfg.set("linger.ms", "0");
    cfg.set("acks", "1");
    cfg.set("enable.idempotence", "false");
    cfg.set("compression.type", compression);
    // Force real batch-level compression even for a single record.
    cfg.set("batch.size", "1");
    let producer: FutureProducer = cfg.create().expect("producer");

    producer
        .send(
            FutureRecord::<str, [u8]>::to(topic).payload(value).key(key),
            Duration::from_secs(5),
        )
        .await
        .unwrap_or_else(|(e, _)| panic!("produce with {compression} failed: {e:?}"));
}

async fn consume_one(port: u16, topic: &str, group: &str) -> (Vec<u8>, Option<Vec<u8>>) {
    let port_c = port;
    let topic = topic.to_string();
    let group = group.to_string();
    tokio::task::spawn_blocking(move || {
        let mut cfg = ClientConfig::new();
        cfg.set("bootstrap.servers", format!("127.0.0.1:{port_c}"));
        cfg.set("group.id", group);
        cfg.set("enable.auto.commit", "false");
        cfg.set("session.timeout.ms", "6000");
        cfg.set("fetch.min.bytes", "1");
        cfg.set("fetch.wait.max.ms", "50");
        let consumer: BaseConsumer = cfg.create().expect("consumer");

        let mut assignment = TopicPartitionList::new();
        assignment.add_partition_offset(&topic, 0, rdkafka::Offset::Beginning).unwrap();
        consumer.assign(&assignment).expect("assign");

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if Instant::now() >= deadline {
                panic!("consumer timed out waiting for record on {topic}");
            }
            if let Some(result) = consumer.poll(Duration::from_millis(100)) {
                let msg = result.expect("no broker error");
                return (msg.payload().expect("payload").to_vec(), msg.key().map(|k| k.to_vec()));
            }
        }
    })
    .await
    .expect("consumer task did not panic")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn produce_and_fetch_with_snappy_compression() {
    let (port, broker, handle) = start_broker_with_topic("snappy-topic").await;

    produce_one(port, "snappy-topic", "snappy", "k-snappy", b"snappy-payload").await;

    let (payload, key) = consume_one(port, "snappy-topic", "snappy-e2e-group").await;
    assert_eq!(payload, b"snappy-payload");
    assert_eq!(key.as_deref(), Some(b"k-snappy".as_ref()));

    // Broker-side storage must also hold the decompressed record.
    let topics = broker.topics.read().await;
    let topic = topics.get("snappy-topic").expect("topic");
    let stored: Vec<_> = topic.partitions.iter().flat_map(|p| p.messages.iter()).collect();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].value, b"snappy-payload");

    handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn produce_and_fetch_with_gzip_compression() {
    let (port, broker, handle) = start_broker_with_topic("gzip-topic").await;

    produce_one(
        port,
        "gzip-topic",
        "gzip",
        "k-gzip",
        b"gzip-payload-padded-to-be-worth-compressing",
    )
    .await;

    let (payload, key) = consume_one(port, "gzip-topic", "gzip-e2e-group").await;
    assert_eq!(payload, b"gzip-payload-padded-to-be-worth-compressing");
    assert_eq!(key.as_deref(), Some(b"k-gzip".as_ref()));

    let topics = broker.topics.read().await;
    let topic = topics.get("gzip-topic").expect("topic");
    let stored: Vec<_> = topic.partitions.iter().flat_map(|p| p.messages.iter()).collect();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].value, b"gzip-payload-padded-to-be-worth-compressing");

    handle.abort();
}
