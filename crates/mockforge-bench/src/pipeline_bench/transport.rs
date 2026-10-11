//! Deadline-bounded socket workers and successful wire-byte accounting.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;

use super::response::read_response;
use super::{BodyKind, StreamBody};

#[derive(Default)]
pub(super) struct Counters {
    pub(super) requests_sent: AtomicU64,
    pub(super) responses_received: AtomicU64,
    pub(super) bytes_sent: AtomicU64,
    pub(super) bytes_received: AtomicU64,
    pub(super) closed_early: AtomicU64,
    pub(super) connection_errors: AtomicU64,
    pub(super) reconnects: AtomicU64,
    statuses: std::sync::Mutex<std::collections::BTreeMap<String, u64>>,
}

impl Counters {
    pub(super) fn status_snapshot(&self) -> std::collections::BTreeMap<String, u64> {
        self.statuses.lock().map(|m| m.clone()).unwrap_or_default()
    }
    pub(super) fn clone_snapshot(&self) -> Self {
        Self {
            requests_sent: AtomicU64::new(self.requests_sent.load(Ordering::Relaxed)),
            responses_received: AtomicU64::new(self.responses_received.load(Ordering::Relaxed)),
            bytes_sent: AtomicU64::new(self.bytes_sent.load(Ordering::Relaxed)),
            bytes_received: AtomicU64::new(self.bytes_received.load(Ordering::Relaxed)),
            closed_early: AtomicU64::new(self.closed_early.load(Ordering::Relaxed)),
            connection_errors: AtomicU64::new(self.connection_errors.load(Ordering::Relaxed)),
            reconnects: AtomicU64::new(self.reconnects.load(Ordering::Relaxed)),
            statuses: std::sync::Mutex::new(self.status_snapshot()),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn connection_loop(
    addresses: &[std::net::SocketAddr],
    host: &str,
    port: u16,
    path: &str,
    method: &str,
    kind: BodyKind,
    body_size: u64,
    file_content_type: &str,
    depth: usize,
    deadline: Instant,
    counters: &Counters,
) {
    let authority = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_owned()
    };
    let authority = if port == 80 {
        authority
    } else {
        format!("{authority}:{port}")
    };
    // Bound every connect, write and read by the same load deadline. Dropping
    // this future closes the owned socket even when the target never responds.
    let _ = tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), async {
        while Instant::now() < deadline {
            let mut stream = match TcpStream::connect(addresses).await {
                Ok(stream) => stream,
                Err(_) => {
                    counters.connection_errors.fetch_add(1, Ordering::Relaxed);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
            };
            let (mut reader, mut writer) = stream.split();
            // Send requests back-to-back without waiting for each response.
            // Drain replies concurrently so large replies cannot fill the
            // receive window and block a server from reading the next request.
            let send = async {
                for _ in 0..depth {
                    let mut body =
                        StreamBody::with_multipart_content_type(kind, body_size, file_content_type)
                            .map_err(|error| {
                                std::io::Error::new(std::io::ErrorKind::InvalidInput, error)
                            })?;
                    write_batch_request(
                        &mut writer,
                        method,
                        &authority,
                        path,
                        &mut body,
                        &counters.bytes_sent,
                    )
                    .await?;
                    counters.requests_sent.fetch_add(1, Ordering::Relaxed);
                }
                writer.flush().await
            };
            let receive = async {
                let mut buffer = Vec::with_capacity(16 * 1024);
                for _ in 0..depth {
                    match read_response(
                        &mut reader,
                        &mut buffer,
                        &counters.bytes_received,
                        method == "HEAD",
                    )
                    .await
                    {
                        Ok((status, _)) => {
                            counters.responses_received.fetch_add(1, Ordering::Relaxed);
                            if let Ok(mut map) = counters.statuses.lock() {
                                *map.entry(status.to_string()).or_insert(0) += 1;
                            }
                        }
                        Err(error) => {
                            if error.kind() == std::io::ErrorKind::UnexpectedEof {
                                counters.closed_early.fetch_add(1, Ordering::Relaxed);
                            }
                            return Err(error);
                        }
                    }
                }
                Ok(())
            };
            if tokio::try_join!(send, receive).is_err() {
                counters.connection_errors.fetch_add(1, Ordering::Relaxed);
            }
            counters.reconnects.fetch_add(1, Ordering::Relaxed);
        }
    })
    .await;
}

/// Write one complete request (head + streamed exact-sized body).
async fn write_batch_request(
    stream: &mut (impl AsyncWrite + Unpin),
    method: &str,
    host: &str,
    path: &str,
    body: &mut StreamBody,
    bytes_sent: &AtomicU64,
) -> std::io::Result<u64> {
    let ct = body.content_type();
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: mockforge-bench-pipeline\r\n\
         Content-Type: {ct}\r\nContent-Length: {}\r\n\r\n",
        body.total_len(),
    );
    let mut written = head.len() as u64;
    write_counted(stream, head.as_bytes(), bytes_sent).await?;
    while let Some(chunk) = body.next_chunk() {
        write_counted(stream, &chunk, bytes_sent).await?;
        written += chunk.len() as u64;
    }
    Ok(written)
}

// Count successful socket writes immediately, including a partial request
// interrupted by a peer error or the overall load deadline.
async fn write_counted(
    stream: &mut (impl AsyncWrite + Unpin),
    mut bytes: &[u8],
    counter: &AtomicU64,
) -> std::io::Result<()> {
    while !bytes.is_empty() {
        let written = stream.write(bytes).await?;
        if written == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "socket stopped accepting request bytes",
            ));
        }
        counter.fetch_add(written as u64, Ordering::Relaxed);
        bytes = &bytes[written..];
    }
    Ok(())
}
