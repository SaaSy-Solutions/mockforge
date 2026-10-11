//! Socket and body-generator regressions for the pipeline benchmark.

use super::*;

#[test]
fn targets_keep_query_ports_and_ipv6_but_omit_fragments() {
    assert_eq!(
        parse_http_target("http://[::1]:8123/upload?x=1#fragment").unwrap(),
        ("::1".into(), 8123, "/upload?x=1".into())
    );
    assert_eq!(
        parse_http_target("http://localhost?x=1").unwrap(),
        ("localhost".into(), 80, "/?x=1".into())
    );
    assert!(parse_http_target("http://user:password@localhost/").is_err());
}

#[tokio::test]
async fn multipart_mime_and_methods_reject_header_injection_before_connecting() {
    for file_type in [
        "text/plain\r\nX-Injected: yes",
        "not-a-mime",
        "image/png/extra",
        "",
    ] {
        assert!(run_with_multipart_content_type(
            socket_config(1, BodyKind::Multipart, 1024, Duration::from_millis(1)),
            file_type,
        )
        .await
        .is_err());
    }
    let mut cfg = socket_config(1, BodyKind::Json, 1024, Duration::from_millis(1));
    cfg.method = "POST\r\nX-Injected: yes".into();
    assert!(run(cfg).await.is_err());
    for size in ["NaN", "inf", "1e100GB", "-1"] {
        assert!(parse_body_size(size).is_err());
    }
}

#[tokio::test]
async fn response_framing_preserves_the_next_response_and_counts_wire_bytes() {
    for first in [
        "HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc",
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip, Chunked\r\n\r\n3;extension=yes\r\nabc\r\n0\r\nX-Test: yes\r\n\r\n",
        "HTTP/1.1 204 No Content\r\nContent-Length: 99\r\n\r\n",
        "HTTP/1.1 304 Not Modified\r\nContent-Length: 99\r\n\r\n",
    ] {
        let wire = format!("{first}HTTP/1.1 201 Created\r\nContent-Length: 1\r\n\r\nz");
        let mut reader = wire.as_bytes();
        let mut buffer = Vec::new();
        let counter = AtomicU64::new(0);
        let (status, _) = read_response(&mut reader, &mut buffer, &counter, false).await.unwrap();
        assert!(matches!(status, 200 | 204 | 304));
        assert_eq!(read_response(&mut reader, &mut buffer, &counter, false).await.unwrap().0, 201);
        assert_eq!(counter.load(Ordering::Relaxed), wire.len() as u64);
    }
}

#[tokio::test]
async fn head_and_close_delimited_responses_obey_their_body_rules() {
    let counter = AtomicU64::new(0);
    let mut reader =
        &b"HTTP/1.1 200 OK\r\nContent-Length: 999\r\n\r\nHTTP/1.1 200 OK\r\n\r\nclose body"[..];
    let mut buffer = Vec::new();
    assert_eq!(read_response(&mut reader, &mut buffer, &counter, true).await.unwrap().0, 200);
    assert_eq!(read_response(&mut reader, &mut buffer, &counter, false).await.unwrap().0, 200);
    assert!(buffer.is_empty());
    assert!(reader.is_empty());
}

#[tokio::test]
async fn malformed_or_truncated_response_bodies_are_not_completed() {
    for wire in [
        "HTTP/1.1 200 OK\r\nContent-Length: bad\r\n\r\n",
        "HTTP/1.1 200 OK\r\nContent-Length: 1\r\nContent-Length: 2\r\n\r\nxx",
        "HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nx",
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nwat\r\n\r\n",
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1\r\nx!!0\r\n\r\n",
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nX-Test: unfinished",
        "HTTP/1.1 101 Switching Protocols\r\n\r\n",
    ] {
        let mut reader = wire.as_bytes();
        let counter = AtomicU64::new(0);
        assert!(read_response(&mut reader, &mut Vec::new(), &counter, false).await.is_err());
        assert_eq!(counter.load(Ordering::Relaxed), wire.len() as u64);
    }
}

fn socket_config(port: u16, kind: BodyKind, size: u64, duration: Duration) -> PipelineBenchConfig {
    PipelineBenchConfig {
        target_url: format!("http://127.0.0.1:{port}/owned-test"),
        method: "POST".into(),
        body_kind: kind,
        body_size: size,
        pipeline_depth: 2,
        connections: 1,
        duration,
    }
}

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

#[test]
fn early_responses_are_reported_without_unsigned_underflow() {
    let result = PipelineBenchResult {
        responses_received: 1,
        ..Default::default()
    };
    assert!(render_report(&result).contains("before their request bodies were fully written"));
}

#[test]
fn nine_hundred_gigabyte_bodies_allocate_only_a_bounded_chunk() {
    for kind in [
        BodyKind::Json,
        BodyKind::Xml,
        BodyKind::UrlEncoded,
        BodyKind::Multipart,
    ] {
        let mut body = StreamBody::new(kind, 900_000_000_000).unwrap();
        assert_eq!(body.total_len(), 900_000_000_000);
        for _ in 0..3 {
            assert!(body.next_chunk().unwrap().len() <= 64 * 1024);
        }
    }
}

#[test]
fn body_size_parses_suffixes() {
    assert_eq!(parse_body_size("4096").unwrap(), 4096);
    assert_eq!(parse_body_size("500KB").unwrap(), 500_000);
    assert_eq!(parse_body_size("1MB").unwrap(), 1_000_000);
    assert_eq!(parse_body_size("2GB").unwrap(), 2_000_000_000);
    assert_eq!(parse_body_size("1KiB").unwrap(), 1024);
    assert_eq!(parse_body_size("1.5KB").unwrap(), 1500);
    assert!(parse_body_size("abc").is_err());
    assert!(parse_body_size("0").is_err());
}

#[test]
fn body_kinds_parse() {
    assert_eq!(parse_body_kind("json").unwrap(), BodyKind::Json);
    assert_eq!(
        parse_body_kind("application/x-www-form-urlencoded").unwrap(),
        BodyKind::UrlEncoded
    );
    assert!(parse_body_kind("grpc").is_err());
}

#[test]
fn stream_body_is_exact_size_and_well_formed() {
    for (kind, size) in [
        (BodyKind::Json, 1000u64),
        (BodyKind::Xml, 2048u64),
        (BodyKind::UrlEncoded, 777u64),
        (BodyKind::Multipart, 50_000u64),
    ] {
        let mut body = StreamBody::new(kind, size).unwrap();
        let mut total = 0usize;
        let mut assembled: Vec<u8> = Vec::new();
        while let Some(chunk) = body.next_chunk() {
            total += chunk.len();
            if assembled.len() < 512 {
                assembled.extend_from_slice(&chunk);
            }
        }
        assert_eq!(total as u64, size, "{kind:?} must produce exactly {size}");
        assert!(body.exhausted());
        match kind {
            BodyKind::Json => {
                assert!(assembled.starts_with(b"{\""));
            }
            BodyKind::Xml => assert!(assembled.starts_with(b"<root>")),
            BodyKind::UrlEncoded => {}
            BodyKind::Multipart => assert!(assembled.starts_with(b"--mockforge-pipeline")),
        }
    }
}

#[test]
fn json_body_roundtrips_through_parser() {
    let size = 4096u64;
    let mut body = StreamBody::new(BodyKind::Json, size).unwrap();
    let mut full: Vec<u8> = Vec::with_capacity(size as usize);
    while let Some(chunk) = body.next_chunk() {
        full.extend_from_slice(&chunk);
    }
    assert_eq!(full.len() as u64, size);
    let v: serde_json::Value = serde_json::from_slice(&full).expect("well-formed JSON");
    assert!(v.get("data").and_then(|d| d.as_str()).is_some());
}

#[test]
fn form_body_roundtrips_as_a_nonempty_value() {
    let mut body = StreamBody::new(BodyKind::UrlEncoded, 1024).unwrap();
    let mut full = Vec::new();
    while let Some(chunk) = body.next_chunk() {
        full.extend(chunk);
    }
    assert_eq!(full.len(), 1024);
    let query = std::str::from_utf8(&full).unwrap();
    let url = reqwest::Url::parse(&format!("http://localhost/?{query}")).unwrap();
    let pairs: Vec<_> = url.query_pairs().collect();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].0, "data");
    assert_eq!(pairs[0].1.len(), 1019);
}

#[test]
fn too_small_body_rejected() {
    assert!(StreamBody::new(BodyKind::Json, 4).is_err());
}

#[test]
fn target_parses() {
    let (h, p, path) = parse_http_target("http://localhost:3000/upload").unwrap();
    assert_eq!((h.as_str(), p, path.as_str()), ("localhost", 3000, "/upload"));
    let (h, p, path) = parse_http_target("http://example.com").unwrap();
    assert_eq!((h.as_str(), p, path.as_str()), ("example.com", 80, "/"));
    assert!(parse_http_target("https://x/").is_err());
}
