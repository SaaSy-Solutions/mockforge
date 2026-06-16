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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tcp_delimiter_buffer_over_cap_closes_connection() {
    // In delimiter mode the accumulator only clears on a delimiter match. A
    // client that streams data without ever sending the delimiter must not be
    // able to grow the buffer past `max_message_bytes`; the server caps it and
    // drops the connection (#755).
    let port = free_port().await;
    let config = TcpConfig {
        port,
        host: "127.0.0.1".into(),
        fixtures_dir: None,
        echo_mode: true,
        timeout_secs: 30,
        // Never appears in our payload, so the accumulator never resets.
        delimiter: Some(b"\n".to_vec()),
        max_message_bytes: 64 * 1024,
        ..TcpConfig::default()
    };
    let (_port, server) = spawn_server(config).await;

    let stream = TcpStream::connect(("127.0.0.1", port)).await.expect("connect");
    let (mut rd, mut wr) = stream.into_split();

    // Writer task: stream 4 MiB of delimiter-free bytes, well past the cap.
    // In echo mode the server echoes the (growing) accumulator back, so the
    // writer must run concurrently with a draining reader or the pipe stalls.
    let writer = tokio::spawn(async move {
        let chunk = vec![b'Z'; 16 * 1024];
        for _ in 0..256 {
            if wr.write_all(&chunk).await.is_err() {
                break;
            }
            let _ = wr.flush().await;
        }
        // Keep the write half alive briefly so the close is server-driven.
        tokio::time::sleep(Duration::from_millis(200)).await;
    });

    // Reader: drain echoes until the server closes the connection (EOF/error).
    // The whole exchange must terminate; if the cap weren't enforced the
    // server would buffer forever and the read loop would never see EOF.
    let mut buf = [0u8; 32 * 1024];
    let closed = loop {
        match tokio::time::timeout(Duration::from_secs(10), rd.read(&mut buf)).await {
            Ok(Ok(0)) => break true,  // clean EOF — server closed
            Ok(Err(_)) => break true, // reset — server closed
            Ok(Ok(_)) => continue,    // drained an echo chunk, keep going
            Err(_) => break false,    // overall read stalled without closing
        }
    };

    assert!(
        closed,
        "server must close the connection once the delimiter buffer exceeds the cap"
    );

    writer.abort();
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tcp_max_connections_limits_concurrent_sessions() {
    // Best-effort: with max_connections = 1, a second client should not be
    // serviced while the first holds its slot. We verify the first connection
    // gets an echo and that the limiter is in effect (the second connect may
    // succeed at the TCP layer but won't be handled until the first frees).
    let port = free_port().await;
    let config = TcpConfig {
        port,
        host: "127.0.0.1".into(),
        fixtures_dir: None,
        echo_mode: true,
        timeout_secs: 30,
        max_connections: 1,
        ..TcpConfig::default()
    };
    let (_port, server) = spawn_server(config).await;

    // First client connects and is serviced.
    let mut first = TcpStream::connect(("127.0.0.1", port)).await.expect("first connect");
    first.write_all(b"hello").await.unwrap();
    first.flush().await.unwrap();
    let mut buf = vec![0u8; 5];
    tokio::time::timeout(Duration::from_secs(5), first.read_exact(&mut buf))
        .await
        .expect("first client serviced within 5s")
        .expect("read");
    assert_eq!(&buf, b"hello");

    // Second client: with the single permit held by `first`, its session task
    // can't start. Its echo must NOT arrive while `first` is alive.
    let mut second = TcpStream::connect(("127.0.0.1", port)).await.expect("second connect");
    second.write_all(b"world").await.unwrap();
    second.flush().await.unwrap();
    let mut buf2 = vec![0u8; 5];
    let early =
        tokio::time::timeout(Duration::from_millis(800), second.read_exact(&mut buf2)).await;
    assert!(
        early.is_err(),
        "second client should be blocked while the first holds the only permit"
    );

    // Free the permit; the second client should now be serviced.
    drop(first);
    tokio::time::timeout(Duration::from_secs(5), second.read_exact(&mut buf2))
        .await
        .expect("second client serviced after first releases permit")
        .expect("read");
    assert_eq!(&buf2, b"world");

    drop(second);
    server.abort();
}
