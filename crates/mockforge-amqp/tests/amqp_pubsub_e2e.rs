//! End-to-end regression: a real `lapin` client publishes and consumes
//! through the mock AMQP broker.
//!
//! The existing integration tests cover fixture loading, exchange/queue
//! managers, and channel state — but none of them binds the TCP listener
//! and drives a real AMQP client. A regression in the frame parser /
//! exchange-to-queue routing / delivery path could ship silently. This
//! locks in the end-to-end pub/sub contract.

use lapin::options::{
    BasicConsumeOptions, BasicNackOptions, BasicPublishOptions, ConfirmSelectOptions,
    ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Connection, ConnectionProperties, ExchangeKind};
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

/// Per-message TTL (AMQP `expiration` property). Messages with a
/// per-message expiration MUST be silently dropped once the TTL
/// elapses — the consumer MUST NOT see the expired payload. The
/// expected flow:
///   1. Publish two messages: one short-lived (`expiration=100ms`),
///      one without expiration.
///   2. Sleep 300ms, well past the TTL on the first message.
///   3. Subscribe and drain. Only the second (unexpired) message
///      should arrive.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn amqp_per_message_expiration_drops_stale_payload() {
    let (port, server) = spawn_broker().await;
    let uri = format!("amqp://127.0.0.1:{port}/%2f");

    let conn = Connection::connect(&uri, ConnectionProperties::default())
        .await
        .expect("lapin connects");
    let channel = conn.create_channel().await.expect("open channel");

    let queue_name = "ttl-queue";
    channel
        .queue_declare(
            queue_name,
            QueueDeclareOptions {
                durable: false,
                exclusive: false,
                auto_delete: false,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .unwrap();

    // Short-lived message — AMQP `expiration` is a string of ms.
    channel
        .basic_publish(
            "",
            queue_name,
            BasicPublishOptions::default(),
            b"expires-fast",
            BasicProperties::default().with_expiration("100".into()),
        )
        .await
        .unwrap();
    // Immortal message.
    channel
        .basic_publish(
            "",
            queue_name,
            BasicPublishOptions::default(),
            b"lives-forever",
            BasicProperties::default(),
        )
        .await
        .unwrap();

    // Wait past the TTL before subscribing. The broker checks
    // expiration at dequeue time (see `QueuedMessage::is_expired`),
    // so subscribing AFTER the TTL is what drops the stale payload.
    tokio::time::sleep(Duration::from_millis(300)).await;

    let mut consumer = channel
        .basic_consume(
            queue_name,
            "ttl-consumer",
            BasicConsumeOptions {
                no_ack: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .unwrap();

    // Drain. The first delivery must be the unexpired message.
    let delivery = tokio::time::timeout(Duration::from_secs(5), consumer.next())
        .await
        .expect("consumer should receive the unexpired message within 5s")
        .expect("stream produced")
        .expect("delivery");
    assert_eq!(
        delivery.data, b"lives-forever",
        "expired message must be silently dropped — consumer should only see the live one"
    );

    // Verify no further deliveries. A lingering expired message
    // would surface here.
    let extra = tokio::time::timeout(Duration::from_millis(500), consumer.next()).await;
    assert!(
        extra.is_err(),
        "no further messages expected (expired one should be gone), got: {extra:?}"
    );

    let _ = conn.close(0, "bye").await;
    server.abort();
}

/// Queue-level TTL via `x-message-ttl` queue argument. When the queue
/// itself is declared with a TTL, every enqueued message inherits it
/// even if the publisher doesn't set `expiration`. Same assertion as
/// the per-message test: expired messages must not be delivered.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn amqp_queue_level_ttl_drops_stale_messages() {
    let (port, server) = spawn_broker().await;
    let uri = format!("amqp://127.0.0.1:{port}/%2f");

    let conn = Connection::connect(&uri, ConnectionProperties::default()).await.unwrap();
    let channel = conn.create_channel().await.unwrap();

    let queue_name = "queue-ttl";
    // x-message-ttl is an AMQP i64 ms argument on queue.declare.
    let mut args = FieldTable::default();
    args.insert("x-message-ttl".into(), lapin::types::AMQPValue::LongInt(100));

    channel
        .queue_declare(
            queue_name,
            QueueDeclareOptions {
                durable: false,
                exclusive: false,
                auto_delete: false,
                ..Default::default()
            },
            args,
        )
        .await
        .expect("declare queue with x-message-ttl");

    // Publish WITHOUT per-message expiration — the queue TTL applies.
    channel
        .basic_publish(
            "",
            queue_name,
            BasicPublishOptions::default(),
            b"should-expire",
            BasicProperties::default(),
        )
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(300)).await;

    let mut consumer = channel
        .basic_consume(
            queue_name,
            "queue-ttl-consumer",
            BasicConsumeOptions {
                no_ack: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .unwrap();

    let got = tokio::time::timeout(Duration::from_millis(500), consumer.next()).await;
    assert!(
        got.is_err(),
        "queue with x-message-ttl=100ms must drop a 300ms-old payload; got: {got:?}"
    );

    let _ = conn.close(0, "bye").await;
    server.abort();
}

/// Dead-letter-exchange routing. When a queue is declared with
/// `x-dead-letter-exchange` and a consumer rejects a delivery with
/// `requeue=false`, the rejected message MUST be re-routed to the
/// DLX (and the optional `x-dead-letter-routing-key`). A DLX-bound
/// queue then picks it up for retry / audit flows.
///
/// Topology:
///   producer → work-queue (with x-dead-letter-exchange=dlx,
///                          x-dead-letter-routing-key=dead)
///   dlx (direct) → dlq bound on routing_key=dead
///   consumer on work-queue nacks → broker republishes to dlx →
///   dlq consumer receives the dead-lettered payload.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn amqp_dead_letter_exchange_receives_rejected_messages() {
    let (port, server) = spawn_broker().await;
    let uri = format!("amqp://127.0.0.1:{port}/%2f");

    let conn = Connection::connect(&uri, ConnectionProperties::default()).await.unwrap();
    let channel = conn.create_channel().await.unwrap();

    // Declare the DLX + DLQ first. The DLX is a plain direct
    // exchange; the DLQ is a normal durable queue bound on
    // `routing_key=dead`.
    channel
        .exchange_declare(
            "dlx",
            ExchangeKind::Direct,
            ExchangeDeclareOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("declare dlx");
    channel
        .queue_declare(
            "dlq",
            QueueDeclareOptions {
                durable: true,
                exclusive: false,
                auto_delete: false,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .expect("declare dlq");
    channel
        .queue_bind("dlq", "dlx", "dead", QueueBindOptions::default(), FieldTable::default())
        .await
        .expect("bind dlq");

    // Declare the work queue with DLX routing configured.
    let mut args = FieldTable::default();
    args.insert(
        "x-dead-letter-exchange".into(),
        lapin::types::AMQPValue::LongString("dlx".into()),
    );
    args.insert(
        "x-dead-letter-routing-key".into(),
        lapin::types::AMQPValue::LongString("dead".into()),
    );
    channel
        .queue_declare(
            "work-queue",
            QueueDeclareOptions {
                durable: false,
                exclusive: false,
                auto_delete: false,
                ..Default::default()
            },
            args,
        )
        .await
        .expect("declare work-queue with DLX args");

    // Publish one message to the work queue.
    channel
        .basic_publish(
            "",
            "work-queue",
            BasicPublishOptions::default(),
            b"doomed",
            BasicProperties::default(),
        )
        .await
        .unwrap();

    // Consume and nack with requeue=false — this is what triggers
    // the dead-letter.
    let mut work_consumer = channel
        .basic_consume(
            "work-queue",
            "work-consumer",
            BasicConsumeOptions {
                no_ack: false, // must ack/nack explicitly for DLX to fire
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .unwrap();

    let delivery = tokio::time::timeout(Duration::from_secs(5), work_consumer.next())
        .await
        .expect("work consumer receives within 5s")
        .expect("stream produced")
        .expect("delivery");
    assert_eq!(delivery.data, b"doomed");
    // Nack with requeue=false → broker should reroute to dlx.
    delivery
        .nack(BasicNackOptions {
            requeue: false,
            multiple: false,
        })
        .await
        .expect("nack");

    // Now subscribe to the DLQ and expect the dead-lettered payload.
    let mut dlq_consumer = channel
        .basic_consume(
            "dlq",
            "dlq-consumer",
            BasicConsumeOptions {
                no_ack: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .unwrap();

    let dead = tokio::time::timeout(Duration::from_secs(5), dlq_consumer.next())
        .await
        .expect("DLQ consumer should see the rejected payload within 5s")
        .expect("stream produced")
        .expect("delivery");
    assert_eq!(
        dead.data, b"doomed",
        "DLQ should receive the exact rejected payload byte-for-byte"
    );

    let _ = conn.close(0, "bye").await;
    server.abort();
}
