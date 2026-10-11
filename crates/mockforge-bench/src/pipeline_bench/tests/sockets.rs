//! Sockets regression cases.

use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn load_deadline_stops_a_target_that_never_responds() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let peer = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut received = Vec::new();
        socket.read_to_end(&mut received).await.unwrap();
        received.len() as u64
    });
    let started = Instant::now();
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        run(socket_config(port, BodyKind::Json, 1024, Duration::from_millis(150))),
    )
    .await
    .expect("load deadline must close a stalled socket")
    .unwrap();
    assert!(started.elapsed() < Duration::from_secs(1));
    assert_eq!(result.requests_sent, 2);
    assert_eq!(result.responses_received, 0);
    assert_eq!(result.bytes_sent, peer.await.unwrap());
}

#[tokio::test]
async fn connections_try_the_next_address_when_the_first_is_unavailable() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let peer = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        socket.read_to_end(&mut bytes).await.unwrap();
        bytes.len() as u64
    });
    // Port zero cannot be a listening service; the second address must
    // still be tried under the same benchmark deadline.
    let addresses = ["127.0.0.1:0".parse().unwrap(), address];
    let counters = Counters::default();
    connection_loop(
        &addresses,
        "localhost",
        address.port(),
        "/",
        "POST",
        BodyKind::Json,
        1024,
        "application/octet-stream",
        2,
        Instant::now() + Duration::from_millis(150),
        &counters,
    )
    .await;
    assert_eq!(counters.requests_sent.load(Ordering::Relaxed), 2);
    assert_eq!(counters.bytes_sent.load(Ordering::Relaxed), peer.await.unwrap());
}

#[tokio::test]
async fn large_responses_drain_while_the_next_large_request_is_writing() {
    const SIZE: u64 = 8 * 1024 * 1024;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let peer = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut sent = 0u64;
        let mut received = 0u64;
        for _ in 0..2 {
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                let mut byte = [0u8; 1];
                socket.read_exact(&mut byte).await.unwrap();
                head.push(byte[0]);
            }
            received += head.len() as u64;
            let mut remaining = SIZE;
            let mut chunk = [0u8; 65536];
            while remaining > 0 {
                let cap = remaining.min(chunk.len() as u64) as usize;
                let read = socket.read(&mut chunk[..cap]).await.unwrap();
                assert!(read > 0);
                remaining -= read as u64;
                received += read as u64;
            }
            // Finish this large response before reading the next request.
            // A client that waits to read until both requests are written
            // deadlocks when both socket windows fill.
            let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {SIZE}\r\n\r\n");
            socket.write_all(head.as_bytes()).await.unwrap();
            sent += head.len() as u64;
            for _ in 0..SIZE / chunk.len() as u64 {
                socket.write_all(&chunk).await.unwrap();
                sent += chunk.len() as u64;
            }
        }
        (received, sent)
    });
    let result = run(socket_config(port, BodyKind::Json, SIZE, Duration::from_secs(2)))
        .await
        .unwrap();
    let (sent, received) =
        tokio::time::timeout(Duration::from_secs(1), peer).await.unwrap().unwrap();
    assert_eq!(result.requests_sent, 2);
    assert_eq!(result.responses_received, 2);
    assert_eq!(result.bytes_sent, sent);
    assert_eq!(result.bytes_received, received);
}

#[tokio::test]
async fn partial_large_requests_keep_their_sent_byte_count_at_the_deadline() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let (release, hold) = tokio::sync::oneshot::channel::<()>();
    let peer = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut first = [0u8; 1];
        socket.read_exact(&mut first).await.unwrap();
        // Keep the connection open without draining a gigabyte request.
        let _ = hold.await;
    });
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        run(socket_config(port, BodyKind::Json, 1_000_000_000, Duration::from_millis(150))),
    )
    .await
    .expect("blocked writes must obey the load deadline")
    .unwrap();
    release.send(()).unwrap();
    peer.await.unwrap();
    assert_eq!(result.requests_sent, 0);
    assert!(result.bytes_sent > 0 && result.bytes_sent < 1_000_000_000);
}

#[tokio::test]
async fn true_pipeline_sends_the_whole_batch_before_any_reply_for_every_body_kind() {
    for (kind, file_type) in [
        (BodyKind::Json, "application/octet-stream"),
        (BodyKind::Xml, "application/octet-stream"),
        (BodyKind::UrlEncoded, "application/octet-stream"),
        (BodyKind::Multipart, "application/octet-stream"),
        (BodyKind::Multipart, "image/png"),
        (BodyKind::Multipart, "text/plain"),
    ] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let peer = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut wire_bytes = 0u64;
            for _ in 0..2 {
                let mut head = Vec::new();
                while !head.ends_with(b"\r\n\r\n") {
                    let mut byte = [0u8; 1];
                    socket.read_exact(&mut byte).await.unwrap();
                    head.push(byte[0]);
                    assert!(head.len() < 4096);
                }
                let header = String::from_utf8(head.clone()).unwrap();
                assert!(header.contains(kind.content_type()));
                let length: usize = header
                    .lines()
                    .find_map(|line| line.strip_prefix("Content-Length: "))
                    .unwrap()
                    .parse()
                    .unwrap();
                assert_eq!(length, 1024);
                let mut body = vec![0u8; length];
                socket.read_exact(&mut body).await.unwrap();
                if kind == BodyKind::Json {
                    serde_json::from_slice::<serde_json::Value>(&body).unwrap();
                }
                if kind == BodyKind::Multipart {
                    assert!(String::from_utf8(body.clone())
                        .unwrap()
                        .contains(&format!("Content-Type: {file_type}\r\n")));
                }
                assert!(header.contains(&format!("Host: 127.0.0.1:{port}\r\n")));
                wire_bytes += (head.len() + body.len()) as u64;
            }
            let reply = b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\nx";
            socket.write_all(reply).await.unwrap();
            socket.write_all(reply).await.unwrap();
            (wire_bytes, 2 * reply.len() as u64)
        });
        let result = tokio::time::timeout(
            Duration::from_secs(2),
            run_with_multipart_content_type(
                socket_config(port, kind, 1024, Duration::from_millis(200)),
                file_type,
            ),
        )
        .await
        .unwrap()
        .unwrap();
        let (sent, received) = peer.await.unwrap();
        assert_eq!(result.requests_sent, 2, "{kind:?}");
        assert_eq!(result.responses_received, 2, "{kind:?}");
        assert_eq!(result.bytes_sent, sent, "{kind:?}");
        assert_eq!(result.bytes_received, received, "{kind:?}");
    }
}
