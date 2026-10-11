//! Framing regression cases.

use super::super::response::read_response;
use super::*;

#[tokio::test]
async fn opaque_header_values_do_not_reject_valid_response_framing() {
    let wire = b"HTTP/1.1 200 OK\r\nX-Label: \xff\r\nContent-Length: 1\r\n\r\nx";
    let mut reader = &wire[..];
    let counter = AtomicU64::new(0);
    assert_eq!(
        read_response(&mut reader, &mut Vec::new(), &counter, false).await.unwrap().0,
        200
    );
    assert_eq!(counter.load(Ordering::Relaxed), wire.len() as u64);
}
use std::sync::atomic::AtomicU64;

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
