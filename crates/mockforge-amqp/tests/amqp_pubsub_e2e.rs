//! End-to-end regression: a real `lapin` client publishes and consumes
//! through the mock AMQP broker.
//!
//! The existing integration tests cover fixture loading, exchange/queue
//! managers, and channel state — but none of them binds the TCP listener
//! and drives a real AMQP client. A regression in the frame parser /
//! exchange-to-queue routing / delivery path could ship silently. This
//! locks in the end-to-end pub/sub contract.

use lapin::options::{
    BasicConsumeOptions, BasicPublishOptions, ConfirmSelectOptions, QueueBindOptions,
    QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Connection, ConnectionProperties};
use mockforge_amqp::broker::AmqpBroker;
use mockforge_amqp::spec_registry::AmqpSpecRegistry;
use mockforge_core::config::AmqpConfig;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;

async fn free_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

async fn wait_for_port(port: u16, max: Duration) {
    let deadline = tokio::time::Instant::now() + max;
    loop {
        if tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("amqp broker never started listening on 127.0.0.1:{port}");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn spawn_broker() -> (u16, tokio::task::JoinHandle<()>) {
    let port = free_port().await;
    let config = AmqpConfig {
        port,
        host: "127.0.0.1".into(),
        ..AmqpConfig::default()
    };
    let spec_registry =
        Arc::new(AmqpSpecRegistry::new(config.clone()).await.expect("spec registry"));
    let broker = Arc::new(AmqpBroker::new(config, spec_registry));
    let handle = tokio::spawn(async move {
        broker.start().await.unwrap();
    });
    wait_for_port(port, Duration::from_secs(5)).await;
    (port, handle)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn amqp_publish_consume_round_trip_via_default_exchange() {
    let (port, server) = spawn_broker().await;

    let uri = format!("amqp://127.0.0.1:{port}/%2f");
    let conn = Connection::connect(&uri, ConnectionProperties::default())
        .await
        .expect("lapin connects to mock broker");
    let channel = conn.create_channel().await.expect("open channel");

    let queue_name = "e2e-queue";
    channel
        .queue_declare(
            queue_name,
            QueueDeclareOptions {
                durable: false,
                exclusive: false,
                auto_delete: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .expect("queue_declare");

    // Start the consumer BEFORE publishing so the delivery isn't
    // dropped on the floor.
    let mut consumer = channel
        .basic_consume(
            queue_name,
            "e2e-consumer",
            BasicConsumeOptions {
                no_ack: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .expect("basic_consume");

    channel
        .basic_publish(
            "",         // default exchange
            queue_name, // routing_key == queue name goes direct on default exchange
            BasicPublishOptions::default(),
            b"hello-amqp",
            BasicProperties::default(),
        )
        .await
        .expect("basic_publish");

    let delivery = tokio::time::timeout(Duration::from_secs(5), consumer.next())
        .await
        .expect("consumer should receive a delivery within 5s")
        .expect("consumer stream produced an item")
        .expect("delivery parses cleanly");
    assert_eq!(delivery.data, b"hello-amqp");

    let _ = conn.close(0, "bye").await;
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn amqp_topic_exchange_routes_wildcard_binding() {
    // Declare a topic exchange, bind a queue with a wildcard routing key,
    // publish a message whose routing key matches the wildcard, confirm
    // the consumer receives it. Guards the exchange->binding->queue path.
    let (port, server) = spawn_broker().await;

    let uri = format!("amqp://127.0.0.1:{port}/%2f");
    let conn = Connection::connect(&uri, ConnectionProperties::default()).await.unwrap();
    let channel = conn.create_channel().await.unwrap();

    // Use the built-in amq.topic exchange the broker declares at startup.
    let queue_name = "topic-queue";
    channel
        .queue_declare(
            queue_name,
            QueueDeclareOptions {
                durable: false,
                exclusive: false,
                auto_delete: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .unwrap();
    channel
        .queue_bind(
            queue_name,
            "amq.topic",
            "sensors.*.temperature", // matches sensors.{anything}.temperature
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();

    let mut consumer = channel
        .basic_consume(
            queue_name,
            "topic-consumer",
            BasicConsumeOptions {
                no_ack: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .unwrap();

    channel
        .basic_publish(
            "amq.topic",
            "sensors.kitchen.temperature",
            BasicPublishOptions::default(),
            b"21.3C",
            BasicProperties::default(),
        )
        .await
        .unwrap();

    let delivery = tokio::time::timeout(Duration::from_secs(5), consumer.next())
        .await
        .expect("wildcard consumer should receive within 5s")
        .expect("stream produced")
        .expect("delivery");
    assert_eq!(delivery.data, b"21.3C");

    let _ = conn.close(0, "bye").await;
    server.abort();
}

/// Durable queues persist across a producer's disconnect. The AMQP
/// broker must:
///   1. Keep the queue declared by client A after A's connection
///      closes (rather than replacing it when a subsequent redeclare
///      arrives).
///   2. Keep any pending messages that were routed into that queue.
///   3. Deliver them to a fresh consumer C that reconnects later and
///      redeclares the same queue name with the same flags.
///
/// Previously `declare_queue` unconditionally overwrote the existing
/// queue on every call, so client B's redeclare dropped A's pending
/// messages on the floor and the consumer got nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn amqp_durable_queue_retains_messages_across_producer_disconnect() {
    let (port, server) = spawn_broker().await;
    let uri = format!("amqp://127.0.0.1:{port}/%2f");

    let queue_name = "durable-retains";
    let durable_opts = QueueDeclareOptions {
        durable: true,
        exclusive: false,
        auto_delete: false,
        ..Default::default()
    };

    // --- Producer: declare durable queue + publish, then disconnect ---
    {
        let producer_conn = Connection::connect(&uri, ConnectionProperties::default())
            .await
            .expect("producer connects");
        let producer_ch = producer_conn.create_channel().await.expect("producer channel");
        producer_ch
            .queue_declare(queue_name, durable_opts, FieldTable::default())
            .await
            .expect("producer declares durable queue");
        for i in 0..3u32 {
            producer_ch
                .basic_publish(
                    "",
                    queue_name,
                    BasicPublishOptions::default(),
                    format!("persist-{i}").as_bytes(),
                    BasicProperties::default(),
                )
                .await
                .expect("publish");
        }
        // Give the broker a moment to route + persist.
        tokio::time::sleep(Duration::from_millis(200)).await;
        let _ = producer_conn.close(0, "producer done").await;
    }

    // --- Consumer: fresh connection, redeclare same queue, drain ---
    let consumer_conn = Connection::connect(&uri, ConnectionProperties::default())
        .await
        .expect("consumer reconnects after producer disconnect");
    let consumer_ch = consumer_conn.create_channel().await.expect("consumer channel");
    consumer_ch
        .queue_declare(queue_name, durable_opts, FieldTable::default())
        .await
        .expect("consumer redeclares same durable queue — should be idempotent");

    let mut consumer = consumer_ch
        .basic_consume(
            queue_name,
            "durable-consumer",
            BasicConsumeOptions {
                no_ack: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .expect("basic_consume");

    let mut received: Vec<Vec<u8>> = Vec::new();
    for _ in 0..3 {
        let delivery = tokio::time::timeout(Duration::from_secs(5), consumer.next())
            .await
            .expect("each persisted message should arrive within 5s")
            .expect("stream produced")
            .expect("delivery parses");
        received.push(delivery.data);
    }
    received.sort();
    assert_eq!(
        received,
        vec![
            b"persist-0".to_vec(),
            b"persist-1".to_vec(),
            b"persist-2".to_vec(),
        ],
        "durable queue must deliver every persisted message to the new consumer"
    );

    let _ = consumer_conn.close(0, "consumer done").await;
    server.abort();
}

/// Publisher confirms (`confirm.select` + `basic.ack`). After a client
/// opts a channel into publisher-confirm mode, every `basic.publish`
/// MUST produce either a `basic.ack` (accepted) or `basic.nack`
/// (rejected) from the broker, tagged with a monotonically increasing
/// delivery tag. lapin's `basic_publish` in confirm mode returns a
/// `PublisherConfirm` future that resolves to a `Confirmation` we can
/// assert on. Before publisher confirms are wired end-to-end, this
/// future hangs forever — the real check here is "the await
/// actually resolves".
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn amqp_publisher_confirms_ack_every_publish() {
    let (port, server) = spawn_broker().await;
    let uri = format!("amqp://127.0.0.1:{port}/%2f");

    let conn = Connection::connect(&uri, ConnectionProperties::default())
        .await
        .expect("lapin connects");
    let channel = conn.create_channel().await.expect("open channel");

    // Opt the channel into publisher-confirm mode.
    channel
        .confirm_select(ConfirmSelectOptions::default())
        .await
        .expect("confirm.select must succeed");

    // Declare a target queue (keeps lapin from drop-on-floor + gives
    // the publish something to land in).
    let queue_name = "confirms-queue";
    channel
        .queue_declare(
            queue_name,
            QueueDeclareOptions {
                durable: false,
                exclusive: false,
                auto_delete: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .unwrap();

    // Three publishes. Each returns a `PublisherConfirm`; we await
    // it under a short timeout and assert it resolves to Ack.
    for i in 0..3u32 {
        let confirm = channel
            .basic_publish(
                "",
                queue_name,
                BasicPublishOptions::default(),
                format!("payload-{i}").as_bytes(),
                BasicProperties::default(),
            )
            .await
            .expect("basic_publish returns a PublisherConfirm future in confirm mode");

        let confirmation = tokio::time::timeout(Duration::from_secs(5), confirm)
            .await
            .expect("broker should ack within 5s")
            .expect("confirm future resolves without error");

        assert!(confirmation.is_ack(), "publish {i} should be ack'd; got {confirmation:?}");
    }

    let _ = conn.close(0, "bye").await;
    server.abort();
}
