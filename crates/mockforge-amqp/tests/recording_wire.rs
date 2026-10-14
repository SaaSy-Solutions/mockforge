//! Actual AMQP socket traffic must persist both push and pull deliveries.
//! This integration binary owns its process-global recorder and a disposable DB.

use futures::StreamExt;
use lapin::options::{
    BasicConsumeOptions, BasicGetOptions, BasicPublishOptions, ConfirmSelectOptions,
    QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Connection, ConnectionProperties};
use mockforge_amqp::{broker::AmqpBroker, recording, spec_registry::AmqpSpecRegistry};
use mockforge_core::config::AmqpConfig;
use mockforge_recorder::{
    models::Protocol,
    query::{execute_query, QueryFilter},
    Recorder, RecorderDatabase,
};
use std::{sync::Arc, time::Duration};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wire_publish_push_and_get_persist_without_empty_get_events() {
    tokio::time::timeout(Duration::from_secs(20), exercise_recording())
        .await
        .expect("wire test deadline");
}

async fn exercise_recording() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("traffic.db");
    let db = RecorderDatabase::new(&path).await.unwrap();
    recording::set_recorder(Arc::new(Recorder::new(db.clone())));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let config = AmqpConfig {
        port,
        host: "127.0.0.1".into(),
        ..Default::default()
    };
    let registry = Arc::new(AmqpSpecRegistry::new(config.clone()).await.unwrap());
    let broker = Arc::new(AmqpBroker::new(config, registry));
    let server = tokio::spawn(async move {
        broker.start().await.unwrap();
    });
    while tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_err() {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let conn = Connection::connect(
        &format!("amqp://127.0.0.1:{port}/%2f"),
        ConnectionProperties::default(),
    )
    .await
    .unwrap();
    let channel = conn.create_channel().await.unwrap();
    channel.confirm_select(ConfirmSelectOptions::default()).await.unwrap();
    for queue in ["recorded-pull", "recorded-push"] {
        channel
            .queue_declare(queue, QueueDeclareOptions::default(), FieldTable::default())
            .await
            .unwrap();
    }
    let binary = [0xff, 0xfe, 0x00];
    channel
        .basic_publish(
            "",
            "recorded-pull",
            BasicPublishOptions::default(),
            &binary,
            BasicProperties::default(),
        )
        .await
        .unwrap()
        .await
        .unwrap();
    let delivery = channel
        .basic_get("recorded-pull", BasicGetOptions { no_ack: true })
        .await
        .unwrap()
        .expect("pull delivery");
    assert_eq!(delivery.data, binary);
    // Check pull capture before exercising another publish on this channel.
    let pull_deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        let rows = execute_query(
            &db,
            QueryFilter {
                protocol: Some(Protocol::Amqp),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        if rows.total >= 2 {
            break;
        }
        assert!(
            tokio::time::Instant::now() < pull_deadline,
            "Basic.Get returned a message but expected publish + delivery records; found {}",
            rows.total
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(channel
        .basic_get("recorded-pull", BasicGetOptions { no_ack: true })
        .await
        .unwrap()
        .is_none());
    let mut consumer = channel
        .basic_consume(
            "recorded-push",
            "recording-test",
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
            "",
            "recorded-push",
            BasicPublishOptions::default(),
            b"hello",
            BasicProperties::default(),
        )
        .await
        .unwrap()
        .await
        .unwrap();
    assert_eq!(consumer.next().await.unwrap().unwrap().data, b"hello");
    // Committed transaction publishes must be captured; rolled-back ones must not.
    let tx = conn.create_channel().await.unwrap();
    tx.queue_declare("recorded-tx", QueueDeclareOptions::default(), FieldTable::default())
        .await
        .unwrap();
    tx.tx_select().await.unwrap();
    tx.basic_publish(
        "",
        "recorded-tx",
        BasicPublishOptions::default(),
        b"rolled-back",
        BasicProperties::default(),
    )
    .await
    .unwrap();
    tx.tx_rollback().await.unwrap();
    assert!(tx
        .basic_get("recorded-tx", BasicGetOptions { no_ack: true })
        .await
        .unwrap()
        .is_none());
    tx.basic_publish(
        "",
        "recorded-tx",
        BasicPublishOptions::default(),
        b"committed",
        BasicProperties::default(),
    )
    .await
    .unwrap();
    tx.tx_commit().await.unwrap();
    assert_eq!(
        tx.basic_get("recorded-tx", BasicGetOptions { no_ack: true })
            .await
            .unwrap()
            .unwrap()
            .data,
        b"committed"
    );

    let filter = || QueryFilter {
        protocol: Some(Protocol::Amqp),
        ..Default::default()
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        let rows = execute_query(&db, filter()).await.unwrap();
        if rows.total >= 6 {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "expected six durable publish/deliver records, found {}",
            rows.total
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    // As in the existing lapin wire tests, broker-side close can race the
    // client close future. Recording assertions are independent of that race.
    let _ = conn.close(0, "done").await;
    drop(conn);
    server.abort();
    let _ = server.await;
    // Reopen storage after disconnect; polling above waits for asynchronous writes.
    let reopened = RecorderDatabase::new(&path).await.unwrap();
    let rows = execute_query(&reopened, filter()).await.unwrap();
    assert_eq!(rows.total, 6, "empty pull must not record a phantom delivery");
    for (queue, body, encoding) in [
        ("recorded-pull", "//4A", "base64"),
        ("recorded-push", "hello", "utf8"),
        ("recorded-tx", "committed", "utf8"),
    ] {
        for op in ["publish", "deliver"] {
            let matching: Vec<_> = rows
                .exchanges
                .iter()
                .filter(|e| e.request.path == queue && e.request.method == op)
                .collect();
            assert_eq!(matching.len(), 1, "one {op} event for {queue}");
            assert_eq!(matching[0].request.body.as_deref(), Some(body));
            assert_eq!(matching[0].request.body_encoding, encoding);
        }
    }
}
