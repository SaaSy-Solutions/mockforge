//! HTTP/1.1 request-pipelining bench with arbitrary-size synthetic bodies
//! (#937).
//!
//! Two capabilities live here:
//!
//! 1. **Streaming synthetic body generation** for a target content type
//!    and exact byte size (`application/json`, `application/xml`,
//!    `application/x-www-form-urlencoded`, `multipart/form-data`). Bodies
//!    stream from a fixed-prefix / patterned-fill / fixed-suffix generator,
//!    never materialised fully in memory, so GB-scale sizes work.
//!
//! 2. **True HTTP/1.1 pipelining transport**: each connection writes
//!    `pipeline_depth` requests back-to-back without waiting for each
//!    response, while draining responses in order concurrently. k6/reqwest cannot do this — it is
//!    why the feature needs a raw socket path (like bench-qos, #933).
//!
//! Many servers and proxies serialise or close pipelined connections; the
//! report surfaces early closes and request/response count mismatches so
//! that behaviour is visible instead of silently skewing numbers.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;
#[cfg(test)]
use tokio::io::AsyncReadExt;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;

mod body;
mod response;
#[cfg(test)]
mod tests;

use body::StreamBody;
use response::read_response;

/// Synthetic body flavour (#937).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyKind {
    Json,
    Xml,
    UrlEncoded,
    Multipart,
}

impl BodyKind {
    /// Content-Type header value for this body kind.
    pub fn content_type(self) -> &'static str {
        match self {
            BodyKind::Json => "application/json",
            BodyKind::Xml => "application/xml",
            BodyKind::UrlEncoded => "application/x-www-form-urlencoded",
            BodyKind::Multipart => "multipart/form-data; boundary=mockforge-pipeline",
        }
    }
}

/// Parse a body kind name (`json`, `xml`, `urlencoded`, `multipart`).
pub fn parse_body_kind(name: &str) -> Result<BodyKind, String> {
    match name.to_ascii_lowercase().as_str() {
        "json" | "application/json" => Ok(BodyKind::Json),
        "xml" | "application/xml" => Ok(BodyKind::Xml),
        "urlencoded" | "form" | "application/x-www-form-urlencoded" => Ok(BodyKind::UrlEncoded),
        "multipart" | "multipart/form-data" => Ok(BodyKind::Multipart),
        other => Err(format!(
            "unsupported content type '{other}' (use json | xml | urlencoded | multipart)"
        )),
    }
}

/// Parse a human body size: bare bytes (`4096`) or KB/MB/GB /
/// KiB/MiB/GiB suffixed (`500KB`, `1.5GB`).
pub fn parse_body_size(spec: &str) -> Result<u64, String> {
    let s = spec.trim();
    let lower = s.to_ascii_lowercase();
    // Suffix match runs longest-first so "kib" wins over "kb".
    let suffix_mult: Option<(&str, u64)> = [
        ("kib", 1u64 << 10),
        ("mib", 1u64 << 20),
        ("gib", 1u64 << 30),
        ("kb", 1_000),
        ("mb", 1_000_000),
        ("gb", 1_000_000_000),
    ]
    .into_iter()
    .find_map(|(suf, mult)| lower.strip_suffix(suf).map(|num| (num, mult)));
    let (num_part, mult): (&str, u64) = suffix_mult.unwrap_or((s, 1));
    let n: f64 = num_part
        .trim()
        .parse()
        .map_err(|_| format!("invalid body size '{spec}' (examples: 500KB, 10MB, 2GB)"))?;
    if !n.is_finite() || n < 0.0 || n * mult as f64 >= u64::MAX as f64 {
        return Err(format!("body size must be finite, non-negative, and within the byte counter range, got '{spec}'"));
    }
    let bytes = (n * mult as f64) as u64;
    if bytes == 0 {
        return Err(format!("body size must be > 0, got '{spec}'"));
    }
    Ok(bytes)
}

/// Pipeline bench configuration (#937).
#[derive(Debug, Clone)]
pub struct PipelineBenchConfig {
    /// Target URL (plain http:// only), e.g. `http://localhost:3000/upload`.
    pub target_url: String,
    /// HTTP method. Pipelining is only meaningful for requests with bodies,
    /// so the default is POST.
    pub method: String,
    /// Synthetic body flavour.
    pub body_kind: BodyKind,
    /// Exact body size in bytes per request.
    pub body_size: u64,
    /// Requests sent back-to-back per connection while responses drain concurrently.
    pub pipeline_depth: usize,
    /// Concurrent connections.
    pub connections: usize,
    /// Wall-clock load duration.
    pub duration: Duration,
}

/// Aggregated result across all connections.
#[derive(Debug, Clone, Default, Serialize)]
pub struct PipelineBenchResult {
    /// Requests actually written onto the wire.
    pub requests_sent: u64,
    /// Responses read to completion, in order.
    pub responses_received: u64,
    /// Status-code histogram.
    pub status_counts: std::collections::BTreeMap<String, u64>,
    /// Application payload bytes written (bodies + request heads).
    pub bytes_sent: u64,
    /// Bytes read back (status line + headers + bodies).
    pub bytes_received: u64,
    /// Batches where the server closed the connection before all
    /// pipelined responses arrived — the classic "no pipelining here"
    /// signal.
    pub connection_closed_early: u64,
    /// Connection-level failures (connect refused, reset, timeouts).
    pub connection_errors: u64,
    /// Reconnects performed after an early close or error.
    pub reconnects: u64,
}

/// Split `http://host:port/path` into its parts (plain http only — TLS
/// termination belongs to the target or a proxy, same as bench-qos).
fn parse_http_target(url: &str) -> Result<(String, u16, String), String> {
    let parsed = reqwest::Url::parse(url).map_err(|_| "invalid target URL".to_owned())?;
    if parsed.scheme() != "http" {
        return Err("bench-pipeline needs a plain http:// target".into());
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("target URLs must not contain credentials".into());
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| "target has no host".to_owned())?
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_owned();
    let mut path = parsed.path().to_owned();
    if let Some(query) = parsed.query() {
        path.push('?');
        path.push_str(query);
    }
    Ok((host, parsed.port_or_known_default().unwrap_or(80), path))
}

/// Run the pipelining bench.
pub async fn run(cfg: PipelineBenchConfig) -> anyhow::Result<PipelineBenchResult> {
    run_with_multipart_content_type(cfg, "application/octet-stream").await
}

/// Run with a custom MIME type for the synthetic multipart file part.
/// Other body kinds retain their usual outer Content-Type and framing.
pub async fn run_with_multipart_content_type(
    cfg: PipelineBenchConfig,
    file_content_type: &str,
) -> anyhow::Result<PipelineBenchResult> {
    // The part header must never permit newlines or delimiter injection.
    let valid_token = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"!#$&^_.+-".contains(&byte))
    };
    if file_content_type.len() > 128
        || !file_content_type
            .split_once('/')
            .is_some_and(|(kind, subtype)| valid_token(kind) && valid_token(subtype))
    {
        anyhow::bail!(
            "multipart file content type must be a MIME type such as image/png or text/plain"
        );
    }

    let (host, port, path) = parse_http_target(&cfg.target_url).map_err(anyhow::Error::msg)?;
    if reqwest::Method::from_bytes(cfg.method.as_bytes()).is_err() {
        anyhow::bail!("invalid HTTP method");
    }
    if cfg.pipeline_depth == 0 {
        anyhow::bail!("--pipeline-depth must be >= 1");
    }
    if cfg.connections == 0 {
        anyhow::bail!("--connections must be >= 1");
    }
    // Validate framing up front so a bad size fails fast.
    StreamBody::with_multipart_content_type(cfg.body_kind, cfg.body_size, file_content_type)
        .map_err(anyhow::Error::msg)?;

    let addresses: Vec<_> =
        tokio::net::lookup_host((host.as_str(), port)).await.anyhow_err()?.collect();
    if addresses.is_empty() {
        anyhow::bail!("could not resolve {host}");
    }
    let addresses = Arc::new(addresses);

    let deadline = Instant::now() + cfg.duration;
    let counters = Arc::new(Counters::default());

    let mut handles = Vec::with_capacity(cfg.connections);
    for _ in 0..cfg.connections {
        let counters = counters.clone();
        let host = host.clone();
        let path = path.clone();
        let method = cfg.method.clone();
        let file_content_type = file_content_type.to_owned();
        let addresses = addresses.clone();
        handles.push(tokio::spawn(async move {
            connection_loop(
                &addresses,
                &host,
                port,
                &path,
                &method,
                cfg.body_kind,
                cfg.body_size,
                &file_content_type,
                cfg.pipeline_depth,
                deadline,
                &counters,
            )
            .await;
        }));
    }
    for h in handles {
        let _ = h.await;
    }

    let c = Arc::try_unwrap(counters).unwrap_or_else(|c| (*c).clone_snapshot());
    Ok(PipelineBenchResult {
        requests_sent: c.requests_sent.load(Ordering::Relaxed),
        responses_received: c.responses_received.load(Ordering::Relaxed),
        status_counts: c.status_snapshot(),
        bytes_sent: c.bytes_sent.load(Ordering::Relaxed),
        bytes_received: c.bytes_received.load(Ordering::Relaxed),
        connection_closed_early: c.closed_early.load(Ordering::Relaxed),
        connection_errors: c.connection_errors.load(Ordering::Relaxed),
        reconnects: c.reconnects.load(Ordering::Relaxed),
    })
}

trait AnyhowExt<T> {
    fn anyhow_err(self) -> anyhow::Result<T>;
}
impl<T> AnyhowExt<T> for std::io::Result<T> {
    fn anyhow_err(self) -> anyhow::Result<T> {
        self.map_err(|e| anyhow::anyhow!(e))
    }
}

#[derive(Default)]
struct Counters {
    requests_sent: AtomicU64,
    responses_received: AtomicU64,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
    closed_early: AtomicU64,
    connection_errors: AtomicU64,
    reconnects: AtomicU64,
    statuses: std::sync::Mutex<std::collections::BTreeMap<String, u64>>,
}

impl Counters {
    fn status_snapshot(&self) -> std::collections::BTreeMap<String, u64> {
        self.statuses.lock().map(|m| m.clone()).unwrap_or_default()
    }
    fn clone_snapshot(&self) -> Self {
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
async fn connection_loop(
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

/// Human-readable report.
pub fn render_report(res: &PipelineBenchResult) -> String {
    let mut out = String::new();
    out.push_str("\n=== bench-pipeline results ===\n");
    out.push_str(&format!("requests sent          : {}\n", res.requests_sent));
    out.push_str(&format!("responses received     : {}\n", res.responses_received));
    if res.requests_sent > res.responses_received {
        out.push_str(&format!(
            "  NOTE: {} fully written request(s) had no completed response before \
             the connection ended or the benchmark duration expired.\n",
            res.requests_sent - res.responses_received
        ));
    } else if res.responses_received > res.requests_sent {
        out.push_str("  NOTE: the target completed responses before their request bodies were fully written.\n");
    }
    out.push_str(&format!("connection closed early: {}\n", res.connection_closed_early));
    out.push_str(&format!("connection errors      : {}\n", res.connection_errors));
    out.push_str(&format!("reconnects             : {}\n", res.reconnects));
    out.push_str(&format!(
        "bytes sent / received  : {} / {}\n",
        res.bytes_sent, res.bytes_received
    ));
    out.push_str("status histogram       :\n");
    for (code, n) in &res.status_counts {
        out.push_str(&format!("  {code}: {n}\n"));
    }
    out
}
