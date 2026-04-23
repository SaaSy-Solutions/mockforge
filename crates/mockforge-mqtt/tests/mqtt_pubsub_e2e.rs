//! End-to-end regression: a real `rumqttc` publisher and subscriber
//! exchange a message through the mock broker.
//!
//! The existing integration tests cover topic-tree matching, registry
//! operations, and broker struct construction — but none of them binds
//! the TCP listener and none of them drives a real MQTT client. A
//! regression in the CONNECT / SUBSCRIBE / PUBLISH wire protocol could
//! ship silently. This locks in the end-to-end pub/sub contract.

use mockforge_mqtt::broker::MqttConfig;
use mockforge_mqtt::start_mqtt_server;
use rumqttc::{AsyncClient, Event, EventLoop, Incoming, MqttOptions, QoS};
use std::time::Duration;
use tokio::sync::mpsc;

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
            panic!("mqtt broker never started listening on 127.0.0.1:{port}");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Drain an MQTT client's event loop, forwarding the first PUBLISH packet
/// we see to a channel. Pings/CONNACKs/SUBACKs are allowed through.
fn drain_until_publish(
    mut eventloop: EventLoop,
    tx: mpsc::UnboundedSender<Vec<u8>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::Publish(p))) => {
                    let _ = tx.send(p.payload.to_vec());
                }
                Ok(_) => { /* CONNACK / SUBACK / ping / etc. */ }
                Err(e) => {
                    eprintln!("rumqttc eventloop terminated: {e}");
                    break;
                }
            }
        }
    })
}

/// Pump a publisher client's event loop so operations aren't blocked on
/// backpressure. We don't need to observe its incoming traffic.
fn pump(mut eventloop: EventLoop) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move { while let Ok(_event) = eventloop.poll().await {} })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mqtt_publish_subscribe_round_trip() {
    let port = free_port().await;
    let config = MqttConfig {
        port,
        host: "127.0.0.1".into(),
        ..MqttConfig::default()
    };

    let server = tokio::spawn(async move {
        start_mqtt_server(config).await.unwrap();
    });
    wait_for_port(port, Duration::from_secs(5)).await;

    // --- Subscriber -----------------------------------------------------
    let mut sub_opts = MqttOptions::new("test-subscriber", "127.0.0.1", port);
    sub_opts.set_keep_alive(Duration::from_secs(30));
    let (sub_client, sub_eventloop) = AsyncClient::new(sub_opts, 16);
    let (received_tx, mut received_rx) = mpsc::unbounded_channel();
    let sub_pump = drain_until_publish(sub_eventloop, received_tx);

    // Subscribe and wait a tick so the SUBSCRIBE is on the wire before we
    // publish. Without this gap the broker would forward the PUBLISH to
    // nobody.
    sub_client.subscribe("sensors/temperature", QoS::AtLeastOnce).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    // --- Publisher ------------------------------------------------------
    let mut pub_opts = MqttOptions::new("test-publisher", "127.0.0.1", port);
    pub_opts.set_keep_alive(Duration::from_secs(30));
    let (pub_client, pub_eventloop) = AsyncClient::new(pub_opts, 16);
    let pub_pump = pump(pub_eventloop);

    pub_client
        .publish("sensors/temperature", QoS::AtLeastOnce, false, b"22.4C".to_vec())
        .await
        .unwrap();

    // --- Observe --------------------------------------------------------
    let payload = tokio::time::timeout(Duration::from_secs(5), received_rx.recv())
        .await
        .expect("subscriber should receive published message within 5s")
        .expect("eventloop should have forwarded the payload");
    assert_eq!(payload, b"22.4C");

    // Tidy up so sibling tests aren't hogging the broker task.
    sub_client.disconnect().await.ok();
    pub_client.disconnect().await.ok();
    sub_pump.abort();
    pub_pump.abort();
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mqtt_wildcard_subscription_matches_published_topic() {
    // A `+` wildcard subscription must receive messages from any single-
    // segment topic that matches. This validates the topic tree is
    // actually consulted on PUBLISH (not just on SUBSCRIBE registration).
    let port = free_port().await;
    let config = MqttConfig {
        port,
        host: "127.0.0.1".into(),
        ..MqttConfig::default()
    };
    let server = tokio::spawn(async move {
        start_mqtt_server(config).await.unwrap();
    });
    wait_for_port(port, Duration::from_secs(5)).await;

    let mut sub_opts = MqttOptions::new("wildcard-sub", "127.0.0.1", port);
    sub_opts.set_keep_alive(Duration::from_secs(30));
    let (sub_client, sub_eventloop) = AsyncClient::new(sub_opts, 16);
    let (tx, mut rx) = mpsc::unbounded_channel();
    let sub_pump = drain_until_publish(sub_eventloop, tx);

    sub_client.subscribe("devices/+/status", QoS::AtMostOnce).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let mut pub_opts = MqttOptions::new("wildcard-pub", "127.0.0.1", port);
    pub_opts.set_keep_alive(Duration::from_secs(30));
    let (pub_client, pub_eventloop) = AsyncClient::new(pub_opts, 16);
    let pub_pump = pump(pub_eventloop);

    pub_client
        .publish("devices/sensor-1/status", QoS::AtMostOnce, false, b"online".to_vec())
        .await
        .unwrap();

    let payload = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("wildcard subscriber should receive within 5s")
        .expect("eventloop forwarded payload");
    assert_eq!(payload, b"online");

    sub_client.disconnect().await.ok();
    pub_client.disconnect().await.ok();
    sub_pump.abort();
    pub_pump.abort();
    server.abort();
}
