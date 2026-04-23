//! End-to-end regression: real `tokio::net::TcpStream` clients send bytes
//! to the mock TCP server and get the expected response back. Exercises
//! both echo mode (no fixture) and delimiter-framed mode.
//!
//! Pre-existing TCP tests cover fixture matching and config serialization
//! at the Rust level but none of them binds a listener and drives a real
//! socket. A regression in the accept loop, read/write framing, or echo
//! path would ship silently. This locks in the on-the-wire contract.

use mockforge_tcp::{TcpConfig, TcpServer, TcpSpecRegistry};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

async fn free_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

async fn wait_for_port(port: u16, max: Duration) {
    let deadline = tokio::time::Instant::now() + max;
    loop {
        if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("tcp server never started listening on 127.0.0.1:{port}");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn spawn_server(config: TcpConfig) -> (u16, tokio::task::JoinHandle<()>) {
    let port = config.port;
    let spec_registry = Arc::new(TcpSpecRegistry::new());
    let server = TcpServer::new(config, spec_registry).expect("server builds");
    let handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });
    wait_for_port(port, Duration::from_secs(5)).await;
    (port, handle)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tcp_echo_mode_returns_bytes_verbatim() {
    let port = free_port().await;
    let config = TcpConfig {
        port,
        host: "127.0.0.1".into(),
        fixtures_dir: None,
        echo_mode: true,
        // Short timeout so the accumulator flush triggers on first read
        // and a lingering client doesn't hold the server after the test.
        timeout_secs: 5,
        ..TcpConfig::default()
    };
    let (_port, server) = spawn_server(config).await;

    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.expect("client connect");

    let payload = b"roundtrip-check-123";
    stream.write_all(payload).await.expect("write");
    stream.flush().await.expect("flush");

    let mut buf = vec![0u8; payload.len()];
    tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut buf))
        .await
        .expect("server must echo within 5s")
        .expect("read");
    assert_eq!(&buf, payload);

    drop(stream);
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tcp_delimiter_mode_frames_multiple_messages() {
    // With `delimiter = b"\n"`, the server should treat each line as a
    // complete message, echo it, and reset the accumulator so subsequent
    // lines from the same connection also round-trip independently.
    let port = free_port().await;
    let config = TcpConfig {
        port,
        host: "127.0.0.1".into(),
        fixtures_dir: None,
        echo_mode: true,
        timeout_secs: 5,
        delimiter: Some(b"\n".to_vec()),
        ..TcpConfig::default()
    };
    let (_port, server) = spawn_server(config).await;

    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.expect("connect");

    // Two line-framed messages
    stream.write_all(b"first\n").await.unwrap();
    stream.flush().await.unwrap();
    let mut buf = vec![0u8; 6];
    tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut buf))
        .await
        .expect("first echo within 5s")
        .expect("read");
    assert_eq!(&buf, b"first\n");

    stream.write_all(b"second\n").await.unwrap();
    stream.flush().await.unwrap();
    let mut buf2 = vec![0u8; 7];
    tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut buf2))
        .await
        .expect("second echo within 5s")
        .expect("read");
    assert_eq!(&buf2, b"second\n");

    drop(stream);
    server.abort();
}
