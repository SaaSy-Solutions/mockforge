//! Request frame size handling over raw TCP.
//!
//! Requests larger than 10 MB used to be left unread, so their body was parsed as the next size
//! header and the connection desynchronised (`CorrelationIdMismatchException` on Java clients,
//! whose producers batch several partitions into one large Produce request). The limit is now
//! `KafkaConfig::max_request_bytes`; requests above it are drained and dropped, not answered.

use mockforge_core::config::KafkaConfig;
use mockforge_kafka::KafkaMockBroker;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

async fn start_broker(max_request_bytes: Option<usize>) -> (u16, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let mut config = KafkaConfig {
        port,
        host: "127.0.0.1".into(),
        ..KafkaConfig::default()
    };
    if let Some(limit) = max_request_bytes {
        config.max_request_bytes = limit;
    }
    let broker = Arc::new(KafkaMockBroker::new(config).await.expect("broker init"));
    let handle = tokio::spawn(async move {
        let _ = broker.start().await;
    });
    tokio::time::sleep(Duration::from_millis(250)).await;
    (port, handle)
}

/// An ApiVersions v0 request (request header v1, empty body) padded with `padding` zero bytes.
/// The body is ignored by the broker, so the padding only affects the frame size.
fn api_versions_request(correlation_id: i32, padding: usize) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&18i16.to_be_bytes()); // api_key: ApiVersions
    payload.extend_from_slice(&0i16.to_be_bytes()); // api_version
    payload.extend_from_slice(&correlation_id.to_be_bytes());
    payload.extend_from_slice(&4i16.to_be_bytes());
    payload.extend_from_slice(b"test");
    payload.resize(payload.len() + padding, 0);

    let mut packet = Vec::with_capacity(4 + payload.len());
    packet.extend_from_slice(&(payload.len() as i32).to_be_bytes());
    packet.extend_from_slice(&payload);
    packet
}

/// Reads one response and returns its correlation id.
async fn read_correlation_id(stream: &mut TcpStream) -> i32 {
    let mut len = [0u8; 4];
    stream.read_exact(&mut len).await.expect("read response length");
    let mut body = vec![0u8; i32::from_be_bytes(len) as usize];
    stream.read_exact(&mut body).await.expect("read response body");
    i32::from_be_bytes(body[0..4].try_into().unwrap())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn default_limit_serves_requests_larger_than_10mb() {
    let (port, server) = start_broker(None).await;
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.expect("connect");

    stream.write_all(&api_versions_request(21, 12 * 1024 * 1024)).await.unwrap();
    assert_eq!(read_correlation_id(&mut stream).await, 21, "large request must be answered");

    // The next request on the same connection must still be framed correctly.
    stream.write_all(&api_versions_request(22, 0)).await.unwrap();
    assert_eq!(read_correlation_id(&mut stream).await, 22, "connection desynchronised");

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_above_configured_limit_is_discarded_and_connection_stays_in_sync() {
    let (port, server) = start_broker(Some(1024 * 1024)).await;
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.expect("connect");

    // 2 MiB request against a 1 MiB limit: drained and dropped without a response ...
    stream.write_all(&api_versions_request(31, 2 * 1024 * 1024)).await.unwrap();
    // ... so the first response on this connection belongs to the next request.
    stream.write_all(&api_versions_request(32, 0)).await.unwrap();
    assert_eq!(
        read_correlation_id(&mut stream).await,
        32,
        "oversized request must be dropped and the stream must stay in sync"
    );

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_within_configured_limit_is_served() {
    let (port, server) = start_broker(Some(1024 * 1024)).await;
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.expect("connect");

    stream.write_all(&api_versions_request(41, 512 * 1024)).await.unwrap();
    assert_eq!(read_correlation_id(&mut stream).await, 41);

    server.abort();
}
