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
use rumqttc::{AsyncClient, Event, EventLoop, Incoming, LastWill, MqttOptions, QoS};
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

/// Variant of `drain_until_publish` that also forwards the `retain`
/// flag — the retained-messages test needs to distinguish "delivered
/// as retained snapshot" from "delivered as live publish".
fn drain_with_retain(
    mut eventloop: EventLoop,
    tx: mpsc::UnboundedSender<(Vec<u8>, bool)>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::Publish(p))) => {
                    let _ = tx.send((p.payload.to_vec(), p.retain));
                }
                Ok(_) => {}
                Err(e) => {
                    eprintln!("rumqttc eventloop terminated: {e}");
                    break;
                }
            }
        }
    })
}

/// Retained-message delivery for late subscribers. The MQTT spec:
/// when a PUBLISH arrives with `retain=true`, the broker stores it
/// per-topic and delivers it immediately to any future subscriber —
/// with the `retain` flag set on that delivery. This test:
///   1. Publisher sets retain=true on "home/temperature".
///   2. Publisher disconnects (broker keeps the retained value).
///   3. A fresh subscriber subscribes — expects the retained PUBLISH
///      immediately, with retain=true.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mqtt_retained_message_delivered_to_new_subscriber() {
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

    // --- Publish a retained message, then disconnect --------------------
    let mut pub_opts = MqttOptions::new("retain-pub", "127.0.0.1", port);
    pub_opts.set_keep_alive(Duration::from_secs(30));
    let (pub_client, pub_eventloop) = AsyncClient::new(pub_opts, 16);
    let pub_pump = pump(pub_eventloop);
    pub_client
        .publish("home/temperature", QoS::AtLeastOnce, /*retain=*/ true, b"21.8C".to_vec())
        .await
        .unwrap();
    // Give the broker a beat to process the PUBLISH + persist the retain.
    tokio::time::sleep(Duration::from_millis(200)).await;
    pub_client.disconnect().await.ok();
    pub_pump.abort();

    // --- Fresh subscriber joins after the publisher is gone -------------
    let mut sub_opts = MqttOptions::new("retain-sub", "127.0.0.1", port);
    sub_opts.set_keep_alive(Duration::from_secs(30));
    let (sub_client, sub_eventloop) = AsyncClient::new(sub_opts, 16);
    let (tx, mut rx) = mpsc::unbounded_channel();
    let sub_pump = drain_with_retain(sub_eventloop, tx);

    sub_client.subscribe("home/temperature", QoS::AtLeastOnce).await.unwrap();

    let (payload, retain_flag) = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("subscriber should receive retained message within 5s")
        .expect("eventloop forwarded payload");
    assert_eq!(payload, b"21.8C", "retained payload must round-trip byte-for-byte");
    assert!(
        retain_flag,
        "retained messages MUST be delivered with retain=true per MQTT spec"
    );

    sub_client.disconnect().await.ok();
    sub_pump.abort();
    server.abort();
}

/// Last-Will-and-Testament delivery on *abrupt* disconnect. The MQTT
/// spec (§3.1.2.5): a client that declares a will on CONNECT and then
/// goes away without sending a DISCONNECT packet causes the broker to
/// publish that will to its declared topic on behalf of the departed
/// client. A graceful DISCONNECT, by contrast, silently discards the
/// will. This test covers the abrupt case end-to-end:
///   1. Subscriber attaches to "device/will-topic".
///   2. Second client connects with a Last Will pointing at that topic.
///   3. Second client's event loop is dropped without `disconnect()` —
///      simulating a crash / network drop.
///   4. Subscriber must receive the will payload.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mqtt_last_will_delivered_on_abrupt_disconnect() {
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

    // --- Subscriber waits on the will topic -----------------------------
    let mut sub_opts = MqttOptions::new("will-watcher", "127.0.0.1", port);
    sub_opts.set_keep_alive(Duration::from_secs(30));
    let (sub_client, sub_eventloop) = AsyncClient::new(sub_opts, 16);
    let (tx, mut rx) = mpsc::unbounded_channel();
    let sub_pump = drain_until_publish(sub_eventloop, tx);

    sub_client.subscribe("device/will-topic", QoS::AtLeastOnce).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    // --- Client with a Last Will; CONNECT, then go away ungracefully ---
    let mut will_opts = MqttOptions::new("will-maker", "127.0.0.1", port);
    will_opts.set_keep_alive(Duration::from_secs(30));
    will_opts.set_last_will(LastWill::new(
        "device/will-topic",
        b"will-maker went offline".to_vec(),
        QoS::AtLeastOnce,
        /*retain=*/ false,
    ));
    let (will_client, will_eventloop) = AsyncClient::new(will_opts, 16);

    // Pump the event loop just long enough to finish CONNECT/CONNACK,
    // then drop both halves. Dropping the event loop closes the TCP
    // socket without a DISCONNECT packet — the broker treats this as an
    // abrupt disconnect and publishes the will.
    let will_handle = tokio::spawn(async move {
        let mut ev = will_eventloop;
        // Poll once so the CONNECT/CONNACK handshake completes. If we
        // return before that, the broker never saw the will.
        for _ in 0..20 {
            if ev.poll().await.is_err() {
                return;
            }
        }
    });
    // Let the connect handshake land.
    tokio::time::sleep(Duration::from_millis(300)).await;
    drop(will_client);
    will_handle.abort();

    // --- Subscriber should see the will --------------------------------
    let payload = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("subscriber should receive will within 5s")
        .expect("eventloop forwarded payload");
    assert_eq!(payload, b"will-maker went offline");

    sub_client.disconnect().await.ok();
    sub_pump.abort();
    server.abort();
}

/// Counterpart: a graceful DISCONNECT must *not* trigger the will.
/// Without this, the broker would wrongly publish wills on every clean exit.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mqtt_last_will_suppressed_on_graceful_disconnect() {
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

    let mut sub_opts = MqttOptions::new("will-watcher-graceful", "127.0.0.1", port);
    sub_opts.set_keep_alive(Duration::from_secs(30));
    let (sub_client, sub_eventloop) = AsyncClient::new(sub_opts, 16);
    let (tx, mut rx) = mpsc::unbounded_channel();
    let sub_pump = drain_until_publish(sub_eventloop, tx);

    sub_client.subscribe("device/graceful-will", QoS::AtLeastOnce).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let mut will_opts = MqttOptions::new("graceful-will-maker", "127.0.0.1", port);
    will_opts.set_keep_alive(Duration::from_secs(30));
    will_opts.set_last_will(LastWill::new(
        "device/graceful-will",
        b"should NOT be delivered".to_vec(),
        QoS::AtLeastOnce,
        false,
    ));
    let (will_client, will_eventloop) = AsyncClient::new(will_opts, 16);
    let will_pump = pump(will_eventloop);

    // Give the handshake a beat, then disconnect gracefully.
    tokio::time::sleep(Duration::from_millis(200)).await;
    will_client.disconnect().await.ok();
    // Allow the broker to process the DISCONNECT + the client's close.
    tokio::time::sleep(Duration::from_millis(300)).await;
    will_pump.abort();

    // The subscriber should *not* see anything. Using a short timeout
    // here is the whole assertion — if the will fires, rx.recv() hands
    // us the payload.
    let got = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await;
    assert!(got.is_err(), "graceful disconnect must NOT publish the will; received: {got:?}");

    sub_client.disconnect().await.ok();
    sub_pump.abort();
    server.abort();
}

/// QoS 2 (exactly-once) round-trip. The broker runs two separate
/// four-step handshakes per message:
///   inbound:  publisher → PUBLISH; broker → PUBREC; publisher → PUBREL;
///             broker → PUBCOMP
///   outbound: broker → PUBLISH; subscriber → PUBREC; broker → PUBREL;
///             subscriber → PUBCOMP
/// rumqttc's event loop drives both sides transparently — we just set
/// `QoS::ExactlyOnce` and assert the payload arrives exactly once. The
/// real check is "does the outbound PUBREL ever go out". Before this
/// fix the broker assigned an outbound packet id but never registered
/// it in `pending_qos2_out`, so the subscriber's PUBREC had no matching
/// entry and the flow stalled.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mqtt_qos2_exactly_once_round_trip() {
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

    let mut sub_opts = MqttOptions::new("qos2-sub", "127.0.0.1", port);
    sub_opts.set_keep_alive(Duration::from_secs(30));
    let (sub_client, sub_eventloop) = AsyncClient::new(sub_opts, 16);
    let (tx, mut rx) = mpsc::unbounded_channel();
    let sub_pump = drain_until_publish(sub_eventloop, tx);

    sub_client.subscribe("qos2/topic", QoS::ExactlyOnce).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let mut pub_opts = MqttOptions::new("qos2-pub", "127.0.0.1", port);
    pub_opts.set_keep_alive(Duration::from_secs(30));
    let (pub_client, pub_eventloop) = AsyncClient::new(pub_opts, 16);
    let pub_pump = pump(pub_eventloop);

    pub_client
        .publish("qos2/topic", QoS::ExactlyOnce, false, b"exactly-once-payload".to_vec())
        .await
        .unwrap();

    let payload = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("qos2 subscriber should receive payload within 5s")
        .expect("eventloop forwarded payload");
    assert_eq!(payload, b"exactly-once-payload");

    // Exactly-once: no *additional* payload should arrive on the same
    // topic. Poll for a short window and assert silence — catches any
    // duplicate-delivery bug.
    let extra = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await;
    assert!(extra.is_err(), "QoS 2 must deliver exactly once; got a duplicate: {extra:?}");

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
