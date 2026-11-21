//! Request frame size handling over raw TCP.
//!
//! Requests larger than 10 MB used to be left unread, so their body was parsed as the next size
//! header and the connection desynchronised (`CorrelationIdMismatchException` on Java clients,
//! whose producers batch several partitions into one large Produce request). The limit is now
//! `KafkaConfig::max_request_bytes` (default 100 MiB). A frame above the limit — or a negative
//! one — closes the connection, as real Kafka does, so a client that declares a frame and then
//! sends nothing cannot pin it (the old drain had no deadline). Within the limit the body is
//! read incrementally under a deadline that scales with the declared size (10s base + 1s per
//! started MiB, capped at 300s).

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

/// The ApiVersions v0 request header (api_key, api_version, correlation_id,
/// client_id length + client_id), plus `padding` zero bytes of ignored body. The broker
/// ignores the body, so the padding only affects the frame size.
fn api_versions_payload(correlation_id: i32, padding: usize) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&18i16.to_be_bytes()); // api_key: ApiVersions
    payload.extend_from_slice(&0i16.to_be_bytes()); // api_version
    payload.extend_from_slice(&correlation_id.to_be_bytes());
    payload.extend_from_slice(&4i16.to_be_bytes());
    payload.extend_from_slice(b"test");
    payload.resize(payload.len() + padding, 0);

    payload
}

/// Wraps a payload in the 4-byte big-endian size header the Kafka wire format prefixes
/// to every request.
fn frame(payload: &[u8]) -> Vec<u8> {
    let mut packet = Vec::with_capacity(4 + payload.len());
    packet.extend_from_slice(&(payload.len() as i32).to_be_bytes());
    packet.extend_from_slice(payload);
    packet
}

/// Builds the full ApiVersions request frame from `api_versions_payload`.
fn api_versions_request(correlation_id: i32, padding: usize) -> Vec<u8> {
    frame(&api_versions_payload(correlation_id, padding))
}

/// An ApiVersions payload padded so the whole request frame is exactly `frame_size` bytes
/// (the frame size prefix equals the payload length). Derives the padding from the real
/// header length instead of assuming it, so a change to the header does not silently shift
/// the boundary these tests probe.
fn payload_of_frame_size(correlation_id: i32, frame_size: usize) -> Vec<u8> {
    let header_len = api_versions_payload(correlation_id, 0).len();
    assert!(frame_size >= header_len, "frame_size must be at least the header length");
    api_versions_payload(correlation_id, frame_size - header_len)
}

/// Writes just the 4-byte size header declaring `declared` bytes, without a body —
/// the shape of a client that goes silent after announcing a frame.
async fn send_size_header(stream: &mut TcpStream, declared: i32) {
    stream.write_all(&declared.to_be_bytes()).await.expect("write size header");
}

/// Waits up to `limit` for the server to close. Returns `true` on EOF or any read error
/// within the window; `false` when the connection is still open (or answered).
async fn closes_within(stream: &mut TcpStream, limit: Duration) -> bool {
    let mut byte = [0u8; 1];
    match tokio::time::timeout(limit, stream.read(&mut byte)).await {
        Ok(Ok(0)) | Ok(Err(_)) => true,
        Ok(Ok(_)) => false,
        Err(_) => false,
    }
}

/// Connects to the broker and sends one ApiVersions request, asserting it is answered.
async fn expect_served(port: u16, correlation_id: i32) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.expect("connect");
    stream
        .write_all(&api_versions_request(correlation_id, 0))
        .await
        .expect("write request");
    assert_eq!(
        read_correlation_id(&mut stream).await,
        correlation_id,
        "broker must serve a fresh connection"
    );
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

/// Oversize no longer drains: the server closes the connection, as real Kafka does.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_above_configured_limit_closes_the_connection() {
    let (port, server) = start_broker(Some(1024 * 1024)).await;
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.expect("connect");

    // 2 MiB request against a 1 MiB limit: the server closes on the size header, possibly
    // while the client is still writing the body — tolerate a write error, then require a
    // close within 2s (no drain, no stall), and a serving broker for new connections after.
    let _ = stream.write_all(&frame(&api_versions_payload(31, 2 * 1024 * 1024))).await;
    assert!(
        closes_within(&mut stream, Duration::from_secs(2)).await,
        "server must close an oversize request's connection within 2s"
    );
    expect_served(port, 33).await;
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

/// Exactly `max_request_bytes` is inside the limit, so the broker answers it — the boundary
/// is inclusive.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_at_exactly_the_limit_is_served() {
    let limit = 1024 * 1024;
    let (port, server) = start_broker(Some(limit)).await;
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.expect("connect");

    // Pad so the frame size lands exactly on the limit.
    let payload = payload_of_frame_size(51, limit);
    stream.write_all(&frame(&payload)).await.unwrap();
    assert_eq!(
        read_correlation_id(&mut stream).await,
        51,
        "a frame of exactly max_request_bytes must be served"
    );

    // And the connection stays in sync for the next request.
    stream.write_all(&api_versions_request(52, 0)).await.unwrap();
    assert_eq!(read_correlation_id(&mut stream).await, 52);

    server.abort();
}

/// One byte over `max_request_bytes` closes the connection.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_one_byte_over_the_limit_closes_the_connection() {
    let limit = 1024 * 1024;
    let (port, server) = start_broker(Some(limit)).await;
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.expect("connect");

    // Pad so the frame size lands exactly one byte past the limit.
    let payload = payload_of_frame_size(61, limit + 1);
    stream.write_all(&frame(&payload)).await.unwrap();
    assert!(
        closes_within(&mut stream, Duration::from_secs(2)).await,
        "a frame of max_request_bytes + 1 must close the connection within 2s"
    );
    expect_served(port, 62).await;

    server.abort();
}

/// `FF FF FF FF` (as i32, −1) must close the connection promptly — parsing it as usize used
/// to read it as ~4 GiB — and the broker keeps serving new connections afterwards.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn negative_frame_size_closes_the_connection() {
    let (port, server) = start_broker(None).await;
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.expect("connect");

    stream.write_all(&(-1i32).to_be_bytes()).await.unwrap();
    assert!(
        closes_within(&mut stream, Duration::from_secs(2)).await,
        "a negative frame size must close the connection within 2s"
    );
    expect_served(port, 71).await;

    server.abort();
}

/// `0x7F FF FF FF` (`i32::MAX`) is positive but far above the limit — the oversize path, and
/// the largest length a hostile client can declare.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn i32_max_frame_size_closes_the_connection() {
    let (port, server) = start_broker(None).await;
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.expect("connect");

    stream.write_all(&i32::MAX.to_be_bytes()).await.unwrap();
    assert!(
        closes_within(&mut stream, Duration::from_secs(2)).await,
        "an i32::MAX frame size must close the connection within 2s"
    );
    expect_served(port, 81).await;

    server.abort();
}

/// A client that declares a small oversize frame and then sends nothing is closed, not held:
/// the drain loop this test guards against had no deadline, so a silent client could pin a
/// connection forever.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn oversize_frame_declared_but_never_sent_is_closed() {
    let limit = 1024 * 1024;
    let (port, server) = start_broker(Some(limit)).await;
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.expect("connect");

    // Declare limit + 1 bytes and go silent — never send the body, never half-close.
    send_size_header(&mut stream, (limit + 1) as i32).await;

    assert!(
        closes_within(&mut stream, Duration::from_secs(2)).await,
        "a silent oversize client must be closed within 2s, not held"
    );
    expect_served(port, 91).await;

    server.abort();
}
