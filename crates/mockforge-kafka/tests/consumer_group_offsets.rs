//! End-to-end regression for OffsetCommit/OffsetFetch persistence.
//!
//! Before this PR the broker accepted OffsetCommit but threw the data
//! away, and OffsetFetch hardcoded "no committed offset" for every
//! partition. A new consumer joining a group with previously committed
//! offsets would always restart from `auto.offset.reset` — meaning the
//! mock couldn't model any real consumer's resume-from-committed flow.
//!
//! This test produces four records, has one consumer commit through
//! offset 2, then spins up a fresh consumer in the same group and
//! asserts it only sees records 3 onward. That exercises the full
//! OffsetCommit v7 → coordinator → OffsetFetch v5 round trip.

use mockforge_core::config::KafkaConfig;
use mockforge_kafka::{KafkaMockBroker, Topic, TopicConfig};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::{Message, Offset, TopicPartitionList};
use std::sync::Arc;
use std::time::Duration;

async fn bind_free_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

fn consumer_cfg(port: u16, group_id: &str) -> ClientConfig {
    let mut cfg = ClientConfig::new();
    cfg.set("bootstrap.servers", format!("127.0.0.1:{port}"));
    cfg.set("group.id", group_id);
    cfg.set("auto.offset.reset", "earliest");
    cfg.set("enable.auto.commit", "false");
    cfg.set("session.timeout.ms", "6000");
    cfg.set("heartbeat.interval.ms", "1000");
    cfg
}

async fn close_consumer(consumer: StreamConsumer) {
    // StreamConsumer::drop is synchronous and routinely takes a couple
    // of seconds for librdkafka to wind down its worker threads +
    // round-trip LeaveGroup. Drop it on a blocking task with a bounded
    // outer timeout so the test teardown can't hang.
    let task = tokio::task::spawn_blocking(move || drop(consumer));
    let _ = tokio::time::timeout(Duration::from_secs(10), task).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn committed_offsets_survive_consumer_restart() {
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
            "offsets-topic".to_string(),
            Topic::new(
                "offsets-topic".to_string(),
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

    // Produce four records up-front. Offsets will be 0..=3.
    let mut producer_cfg = ClientConfig::new();
    producer_cfg.set("bootstrap.servers", format!("127.0.0.1:{port}"));
    producer_cfg.set("message.timeout.ms", "5000");
    producer_cfg.set("linger.ms", "0");
    producer_cfg.set("acks", "1");
    producer_cfg.set("enable.idempotence", "false");
    let producer: FutureProducer = producer_cfg.create().expect("producer");
    for i in 0..4u32 {
        let payload = format!("rec-{i}");
        producer
            .send(
                FutureRecord::<str, [u8]>::to("offsets-topic")
                    .payload(payload.as_bytes())
                    .key("k"),
                Duration::from_secs(5),
            )
            .await
            .expect("produce");
    }

    // First consumer: read 3 records, commit offset 3 (= "next offset
    // to read is 3"), then shut down. This is how librdkafka commits:
    // the committed offset is the offset of the *next* record, not the
    // last consumed one.
    let first: StreamConsumer = consumer_cfg(port, "resume-group").create().expect("c1");
    first.subscribe(&["offsets-topic"]).expect("subscribe");

    for _ in 0..3 {
        let msg = tokio::time::timeout(Duration::from_secs(15), first.recv())
            .await
            .expect("first consumer should receive within 15s")
            .expect("no transport error");
        // Drain headers/payload so librdkafka advances its position.
        let _ = msg.payload();
    }

    let mut commit_positions = TopicPartitionList::new();
    commit_positions
        .add_partition_offset("offsets-topic", 0, Offset::Offset(3))
        .expect("add partition offset");
    first.commit(&commit_positions, CommitMode::Sync).expect("commit");
    close_consumer(first).await;

    // Second consumer, same group. OffsetFetch should return offset 3
    // — the record at offset 3 is "rec-3", which is the only one we
    // expect to see.
    let second: StreamConsumer = consumer_cfg(port, "resume-group").create().expect("c2");
    second.subscribe(&["offsets-topic"]).expect("subscribe");

    let msg = tokio::time::timeout(Duration::from_secs(15), second.recv())
        .await
        .expect("second consumer should receive within 15s")
        .expect("no transport error");

    assert_eq!(msg.offset(), 3, "second consumer should resume at offset 3");
    assert_eq!(
        msg.payload().expect("payload present"),
        b"rec-3",
        "second consumer should see the 4th record (offset 3)"
    );

    close_consumer(second).await;
    server_handle.abort();
}
