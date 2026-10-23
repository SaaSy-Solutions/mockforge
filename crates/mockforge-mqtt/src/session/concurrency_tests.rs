use super::*;
use std::time::Duration;

async fn fixture() -> (SessionManager, mpsc::Receiver<Packet>) {
    let manager = SessionManager::new(10, None);
    let (sender, receiver) = mpsc::channel(1);
    manager.connect("subscriber".into(), true, 60, sender, None).await.unwrap();
    manager
        .subscribe("subscriber", vec![("wire/topic".into(), QoS::AtLeastOnce)])
        .await
        .unwrap();
    (manager, receiver)
}

async fn deliver(manager: &SessionManager, will: bool) {
    if will {
        manager
            .publish_will(
                "publisher",
                crate::protocol::Will {
                    topic: "wire/topic".into(),
                    message: b"body".to_vec(),
                    qos: QoS::AtLeastOnce,
                    retain: false,
                },
            )
            .await;
    } else {
        manager.publish("publisher", &message(b"body")).await;
    }
}

fn message(payload: &[u8]) -> PublishPacket {
    PublishPacket {
        dup: false,
        qos: QoS::AtLeastOnce,
        retain: false,
        topic: "wire/topic".into(),
        packet_id: None,
        payload: payload.to_vec(),
    }
}

fn assert_payload(packet: Option<Packet>, payload: &[u8]) {
    let Some(Packet::Publish(packet)) = packet else {
        panic!("expected publish")
    };
    assert_eq!(packet.topic, "wire/topic");
    assert_eq!(packet.payload, payload);
    assert_eq!(packet.qos, QoS::AtLeastOnce);
}

async fn connect_race(will: bool) {
    let (manager, mut receiver) = fixture().await;
    let topics = manager.topics.write().await;
    let delivery = deliver(&manager, will);
    tokio::pin!(delivery);
    // Stop delivery at the topic read. CONNECT takes the client write lock
    // next, then waits for the topic write lock behind this queued reader.
    assert!(futures::poll!(delivery.as_mut()).is_pending());
    let (sender, _receiver) = mpsc::channel(1);
    let connection = manager.connect("new-client".into(), true, 60, sender, None);
    tokio::pin!(connection);
    assert!(futures::poll!(connection.as_mut()).is_pending());
    drop(topics);
    let (_, connected) = tokio::time::timeout(Duration::from_secs(1), async {
        tokio::join!(delivery.as_mut(), connection.as_mut())
    })
    .await
    .expect("CONNECT and publish deadlocked across client/topic locks");
    assert!(connected.is_ok());
    assert_payload(receiver.recv().await, b"body");
}

async fn backpressure(will: bool) {
    let (manager, mut receiver) = fixture().await;
    manager.publish("publisher", &message(b"seed")).await;
    let delivery = deliver(&manager, will);
    tokio::pin!(delivery);
    assert!(futures::poll!(delivery.as_mut()).is_pending());
    let (sender, _receiver) = mpsc::channel(1);
    let connected = tokio::time::timeout(
        Duration::from_secs(1),
        manager.connect("new-client".into(), true, 60, sender, None),
    )
    .await
    .expect("a full delivery queue held broker-wide locks");
    assert!(connected.is_ok());
    assert_payload(receiver.recv().await, b"seed");
    tokio::time::timeout(Duration::from_secs(1), delivery).await.unwrap();
    assert_payload(receiver.recv().await, b"body");
}

#[tokio::test]
async fn publish_and_connect_do_not_invert_locks() {
    connect_race(false).await;
}

#[tokio::test]
async fn will_and_connect_do_not_invert_locks() {
    connect_race(true).await;
}

#[tokio::test]
async fn publish_backpressure_does_not_block_connections() {
    backpressure(false).await;
}

#[tokio::test]
async fn will_backpressure_does_not_block_connections() {
    backpressure(true).await;
}
