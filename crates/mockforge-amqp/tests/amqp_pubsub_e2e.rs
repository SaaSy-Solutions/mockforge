//! End-to-end regression: a real `lapin` client publishes and consumes
//! through the mock AMQP broker.
//!
//! The existing integration tests cover fixture loading, exchange/queue
//! managers, and channel state — but none of them binds the TCP listener
//! and drives a real AMQP client. A regression in the frame parser /
//! exchange-to-queue routing / delivery path could ship silently. This
//! locks in the end-to-end pub/sub contract.

use lapin::options::{
    BasicConsumeOptions, BasicPublishOptions, QueueBindOptions, QueueDeclareOptions,
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
