//! Real MQTT clients drive live, retained and last-will delivery into SQLite.
use mockforge_mqtt::{broker::MqttConfig, recording, start_mqtt_server};
use mockforge_recorder::{
    models::Protocol,
    query::{execute_query, QueryFilter},
    Recorder, RecorderDatabase,
};
use rumqttc::{AsyncClient, Event, EventLoop, Incoming, LastWill, MqttOptions, QoS};
use std::{sync::Arc, time::Duration};

async fn received(events: &mut EventLoop, topic: &str, payload: &[u8], retained: bool) {
    loop {
        if let Event::Incoming(Incoming::Publish(message)) = events.poll().await.unwrap() {
            assert_eq!(message.topic, topic);
            assert_eq!(message.payload.as_ref(), payload);
            assert_eq!(message.retain, retained);
            return;
        }
    }
}

async fn publish(
    client: &AsyncClient,
    events: &mut EventLoop,
    topic: &str,
    payload: &[u8],
    retain: bool,
) {
    client.publish(topic, QoS::AtLeastOnce, retain, payload.to_vec()).await.unwrap();
    while !matches!(events.poll().await.unwrap(), Event::Incoming(Incoming::PubAck(_))) {}
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mqtt_wire_publish_delivery_persist_payload_and_metadata() {
    tokio::time::timeout(Duration::from_secs(20), exercise_recording())
        .await
        .expect("MQTT wire deadline");
}

async fn exercise_recording() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("traffic.db");
    let db = RecorderDatabase::new(&path).await.unwrap();
    recording::set_recorder(Arc::new(Recorder::new(db.clone())));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let config = MqttConfig {
        host: "127.0.0.1".into(),
        port,
        ..Default::default()
    };
    let server = tokio::spawn(async move {
        start_mqtt_server(config).await.unwrap();
    });
    while tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_err() {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let (subscriber, mut events) =
        AsyncClient::new(MqttOptions::new("recording-subscriber", "127.0.0.1", port), 16);
    subscriber.subscribe("recorded/#", QoS::AtLeastOnce).await.unwrap();
    while !matches!(events.poll().await.unwrap(), Event::Incoming(Incoming::SubAck(_))) {}
    let (publisher, mut publisher_events) =
        AsyncClient::new(MqttOptions::new("recording-publisher", "127.0.0.1", port), 16);
    publish(&publisher, &mut publisher_events, "recorded/topic", b"payload", false).await;
    received(&mut events, "recorded/topic", b"payload", false).await;
    publish(&publisher, &mut publisher_events, "retained/topic", b"snapshot", true).await;
    let (late, mut late_events) =
        AsyncClient::new(MqttOptions::new("recording-late", "127.0.0.1", port), 16);
    late.subscribe("retained/topic", QoS::AtLeastOnce).await.unwrap();
    received(&mut late_events, "retained/topic", b"snapshot", true).await;
    drop(late_events);
    drop(late);
    let mut will_options = MqttOptions::new("recording-will", "127.0.0.1", port);
    will_options.set_last_will(LastWill::new(
        "recorded/will",
        b"gone".to_vec(),
        QoS::AtLeastOnce,
        false,
    ));
    let (will_client, mut will_events) = AsyncClient::new(will_options, 4);
    while !matches!(will_events.poll().await.unwrap(), Event::Incoming(Incoming::ConnAck(_))) {}
    // Drop the transport without DISCONNECT so the broker emits the will.
    drop(will_events);
    drop(will_client);
    received(&mut events, "recorded/will", b"gone", false).await;
    let filter = || QueryFilter {
        protocol: Some(Protocol::Mqtt),
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
            "expected six live/retained/will records; found {}",
            rows.total
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    drop(events);
    drop(subscriber);
    drop(publisher_events);
    drop(publisher);
    server.abort();
    let _ = server.await;
    let reopened = RecorderDatabase::new(&path).await.unwrap();
    let rows = execute_query(&reopened, filter()).await.unwrap();
    assert_eq!(rows.total, 6);
    for (topic, body, origin, retained) in [
        ("recorded/topic", "payload", "recording-publisher", false),
        ("retained/topic", "snapshot", "recording-publisher", true),
        ("recorded/will", "gone", "recording-will", false),
    ] {
        for op in ["publish", "deliver"] {
            let matching: Vec<_> = rows
                .exchanges
                .iter()
                .filter(|e| e.request.method == op && e.request.path == topic)
                .collect();
            assert_eq!(matching.len(), 1);
            let request = &matching[0].request;
            assert_eq!(request.body.as_deref(), Some(body));
            assert_eq!(request.body_encoding, "utf8");
            assert_eq!(
                request.client_ip.as_deref(),
                Some(if op == "publish" {
                    origin
                } else if retained {
                    "recording-late"
                } else {
                    "recording-subscriber"
                })
            );
            let metadata: serde_json::Value =
                serde_json::from_str(request.query_params.as_deref().unwrap()).unwrap();
            assert_eq!(metadata["qos"], 1);
            assert_eq!(metadata["retain"], retained);
        }
    }
}
