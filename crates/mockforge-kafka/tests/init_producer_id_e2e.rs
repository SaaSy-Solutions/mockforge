//! End-to-end checks for InitProducerId (API key 22) over raw TCP.
//!
//! The Java client's idempotent producer calls InitProducerId before its first Produce; before
//! this API existed the broker answered with an ApiVersions response and the client failed with
//! `UnsupportedVersionException: The node does not support INIT_PRODUCER_ID`.

use mockforge_core::config::KafkaConfig;
use mockforge_kafka::KafkaMockBroker;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

async fn start_broker() -> (u16, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let config = KafkaConfig {
        port,
        host: "127.0.0.1".into(),
        ..KafkaConfig::default()
    };
    let broker = Arc::new(KafkaMockBroker::new(config).await.expect("broker init"));
    let handle = tokio::spawn(async move {
        let _ = broker.start().await;
    });
    tokio::time::sleep(Duration::from_millis(250)).await;
    (port, handle)
}

/// Request header v1 (classic) or v2 (flexible: trailing empty tag buffer).
fn header(api_version: i16, correlation_id: i32, flexible: bool) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&22i16.to_be_bytes());
    out.extend_from_slice(&api_version.to_be_bytes());
    out.extend_from_slice(&correlation_id.to_be_bytes());
    out.extend_from_slice(&4i16.to_be_bytes());
    out.extend_from_slice(b"test");
    if flexible {
        out.push(0);
    }
    out
}

fn frame(header: Vec<u8>, body: Vec<u8>) -> Vec<u8> {
    let mut packet = Vec::with_capacity(4 + header.len() + body.len());
    packet.extend_from_slice(&((header.len() + body.len()) as i32).to_be_bytes());
    packet.extend_from_slice(&header);
    packet.extend_from_slice(&body);
    packet
}

/// v1 body: null transactional_id, timeout.
fn body_v1(transactional_id: Option<&str>) -> Vec<u8> {
    let mut b = Vec::new();
    match transactional_id {
        None => b.extend_from_slice(&(-1i16).to_be_bytes()),
        Some(t) => {
            b.extend_from_slice(&(t.len() as i16).to_be_bytes());
            b.extend_from_slice(t.as_bytes());
        }
    }
    b.extend_from_slice(&60_000i32.to_be_bytes());
    b
}

/// v4 body: compact null transactional_id, timeout, producer_id -1, epoch -1, tag buffer.
fn body_v4() -> Vec<u8> {
    let mut b = vec![0u8];
    b.extend_from_slice(&60_000i32.to_be_bytes());
    b.extend_from_slice(&(-1i64).to_be_bytes());
    b.extend_from_slice(&(-1i16).to_be_bytes());
    b.push(0);
    b
}

async fn round_trip(stream: &mut TcpStream, packet: &[u8]) -> Vec<u8> {
    stream.write_all(packet).await.expect("write request");
    let mut len = [0u8; 4];
    stream.read_exact(&mut len).await.expect("read response length");
    let mut body = vec![0u8; i32::from_be_bytes(len) as usize];
    stream.read_exact(&mut body).await.expect("read response body");
    body
}

/// Decodes `(correlation_id, error_code, producer_id, producer_epoch)`.
fn decode(resp: &[u8], flexible: bool) -> (i32, i16, i64, i16) {
    let correlation = i32::from_be_bytes(resp[0..4].try_into().unwrap());
    let mut at = 4;
    if flexible {
        assert_eq!(resp[at], 0, "response header tag buffer");
        at += 1;
    }
    at += 4; // throttle_time_ms
    let error = i16::from_be_bytes(resp[at..at + 2].try_into().unwrap());
    let id = i64::from_be_bytes(resp[at + 2..at + 10].try_into().unwrap());
    let epoch = i16::from_be_bytes(resp[at + 10..at + 12].try_into().unwrap());
    (correlation, error, id, epoch)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn init_producer_id_v1_and_v4_hand_out_distinct_ids() {
    let (port, server) = start_broker().await;
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.expect("connect");

    let v1 = round_trip(&mut stream, &frame(header(1, 11, false), body_v1(None))).await;
    let (corr, err, id1, epoch1) = decode(&v1, false);
    assert_eq!((corr, err, epoch1), (11, 0, 0));
    assert!(id1 >= 1000, "unexpected producer id {id1}");

    let v4 = round_trip(&mut stream, &frame(header(4, 12, true), body_v4())).await;
    let (corr, err, id2, epoch2) = decode(&v4, true);
    assert_eq!((corr, err, epoch2), (12, 0, 0));
    assert_ne!(id1, id2, "two producers must not share an id");

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn init_producer_id_bumps_epoch_for_same_transactional_id() {
    let (port, server) = start_broker().await;
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.expect("connect");

    let first = round_trip(&mut stream, &frame(header(1, 1, false), body_v1(Some("t1")))).await;
    let second = round_trip(&mut stream, &frame(header(1, 2, false), body_v1(Some("t1")))).await;
    let (_, _, id_a, epoch_a) = decode(&first, false);
    let (_, _, id_b, epoch_b) = decode(&second, false);
    assert_eq!(id_a, id_b);
    assert_eq!(epoch_b, epoch_a + 1);

    server.abort();
}
