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
//!    `pipeline_depth` requests back-to-back BEFORE reading any response,
//!    then reads the responses in order. k6/reqwest cannot do this — it is
//!    why the feature needs a raw socket path (like bench-qos, #933).
//!
//! Many servers and proxies serialise or close pipelined connections; the
//! report surfaces early closes and request/response count mismatches so
//! that behaviour is visible instead of silently skewing numbers.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

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
    if n < 0.0 {
        return Err(format!("body size must be non-negative, got '{spec}'"));
    }
    let bytes = (n * mult as f64) as u64;
    if bytes == 0 {
        return Err(format!("body size must be > 0, got '{spec}'"));
    }
    Ok(bytes)
}

/// Fixed head + patterned middle + fixed tail, totalling exactly `size`
/// bytes. The middle streams in 64 KiB patterned chunks so a 900 GB body
/// costs O(64 KiB) memory.
struct StreamBody {
    kind: BodyKind,
    prefix: Vec<u8>,
    suffix: Vec<u8>,
    fill_total: u64,
    written_fill: u64,
    phase: u8,
    done_prefix: bool,
    done_fill: bool,
    done_suffix: bool,
}

impl StreamBody {
    const CHUNK: usize = 64 * 1024;

    /// Build a well-formed body of exactly `size` bytes for `kind`.
    /// Panics via returned error when `size` is too small for the frame.
    fn new(kind: BodyKind, size: u64) -> Result<Self, String> {
        use BodyKind::*;
        let (prefix, suffix): (Vec<u8>, Vec<u8>) = match kind {
            Json => (b"{\"data\":\"".to_vec(), b"\"}".to_vec()),
            Xml => (b"<root><data>".to_vec(), b"</data></root>".to_vec()),
            UrlEncoded => (Vec::new(), Vec::new()),
            // One synthetic file part, framed by the multipart boundary
            // declared in BodyKind::content_type().
            Multipart => (
                format!(
                    "--mockforge-pipeline\r\n\
                     Content-Disposition: form-data; name=\"file\"; filename=\"synthetic.bin\"\r\n\
                     Content-Type: application/octet-stream\r\n\r\n"
                )
                .into_bytes(),
                b"\r\n--mockforge-pipeline--\r\n".to_vec(),
            ),
        };
        let overhead = (prefix.len() + suffix.len()) as u64;
        if size < overhead {
            return Err(format!(
                "body size {size} is smaller than the {kind:?} framing overhead ({overhead} bytes)"
            ));
        }
        Ok(Self {
            kind,
            prefix,
            suffix,
            fill_total: size - overhead,
            written_fill: 0,
            phase: b'a',
            done_prefix: false,
            done_fill: false,
            done_suffix: false,
        })
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn exhausted(&self) -> bool {
        self.done_prefix && self.done_fill && self.done_suffix
    }

    /// Next chunk (up to 64 KiB) of the exact-sized body.
    fn next_chunk(&mut self) -> Option<Vec<u8>> {
        if !self.done_prefix {
            self.done_prefix = true;
            return Some(std::mem::take(&mut self.prefix));
        }
        if !self.done_fill && self.fill_total > 0 {
            let remaining = self.fill_total - self.written_fill;
            let take = remaining.min(Self::CHUNK as u64) as usize;
            let mut buf = vec![0u8; take];
            for b in buf.iter_mut() {
                // Patterned filler keeps compression proxies honest and
                // makes truncation visible in captured bodies.
                *b = self.phase;
                self.phase = if self.phase >= b'z' { b'a' } else { self.phase + 1 };
            }
            self.written_fill += take as u64;
            if self.written_fill == self.fill_total {
                self.done_fill = true;
            }
            return Some(buf);
        }
        if !self.done_fill && self.fill_total == 0 {
            self.done_fill = true;
        }
        if !self.done_suffix {
            self.done_suffix = true;
            return Some(std::mem::take(&mut self.suffix));
        }
        None
    }
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
    /// Requests in flight per connection before any response is read.
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
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| format!("bench-pipeline needs a plain http:// target, got '{url}'"))?;
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => {
            let port: u16 = p.parse().map_err(|_| format!("invalid port in '{url}'"))?;
            (h.to_string(), port)
        }
        None => (authority.to_string(), 80),
    };
    if host.is_empty() {
        return Err(format!("empty host in '{url}'"));
    }
    Ok((host, port, if path.is_empty() { "/".into() } else { path.to_string() }))
}

/// Read one HTTP/1.1 response head, drain its body (Content-Length or
/// chunked), and return `(status, bytes_consumed)`.
async fn read_response(
    stream: &mut TcpStream,
    buf: &mut Vec<u8>,
) -> std::io::Result<(u16, u64)> {
    let mut consumed: u64 = 0;
    // --- head ---
    let head_end = loop {
        if let Some(pos) = find_head_end(buf) {
            break pos;
        }
        if buf.len() > 128 * 1024 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "response head exceeds 128 KiB",
            ));
        }
        let mut chunk = [0u8; 8192];
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "connection closed during response head",
            ));
        }
        consumed += n as u64;
        buf.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    buf.drain(..head_end + 4);

    let status = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse::<u16>().ok())
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "malformed status line")
        })?;

    let lower = head.to_ascii_lowercase();
    let content_length: Option<u64> = lower
        .lines()
        .find_map(|l| l.strip_prefix("content-length:"))
        .and_then(|v| v.trim().parse().ok());
    let chunked = lower.contains("transfer-Encoding:") || lower.contains("transfer-encoding:")
        && lower.split("transfer-encoding:").nth(1).map_or(false, |v| v.contains("chunked"));

    // --- body ---
    if chunked {
        loop {
            // Chunk-size line.
            let size_line_end = loop {
                if let Some(p) = buf.windows(2).position(|w| w == b"\r\n") {
                    break p;
                }
                let mut chunk = [0u8; 1024];
                let n = stream.read(&mut chunk).await?;
                if n == 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "eof inside chunked body",
                    ));
                }
                consumed += n as u64;
                buf.extend_from_slice(&chunk[..n]);
            };
            let size_str = String::from_utf8_lossy(&buf[..size_line_end]);
            let size = u64::from_str_radix(size_str.split(';').next().unwrap_or("0").trim(), 16)
                .unwrap_or(0);
            buf.drain(..size_line_end + 2);
            if size == 0 {
                // Trailers + final CRLF: read until blank line.
                loop {
                    if let Some(p) = buf.windows(2).position(|w| w == b"\r\n") {
                        buf.drain(..p + 2);
                        if p == 0 {
                            break;
                        }
                    } else {
                        let mut chunk = [0u8; 1024];
                        let n = stream.read(&mut chunk).await?;
                        if n == 0 {
                            break;
                        }
                        consumed += n as u64;
                        buf.extend_from_slice(&chunk[..n]);
                    }
                }
                break;
            }
            let mut to_read = size + 2; // payload + CRLF
            while to_read > 0 {
                if !buf.is_empty() {
                    let take = to_read.min(buf.len() as u64) as usize;
                    buf.drain(..take);
                    to_read -= take as u64;
                } else {
                    let mut chunk = [0u8; 16384];
                    let cap = to_read.min(chunk.len() as u64) as usize;
                    let n = stream.read(&mut chunk[..cap]).await?;
                    if n == 0 {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "eof inside chunk data",
                        ));
                    }
                    consumed += n as u64;
                    to_read -= n as u64;
                }
            }
        }
    } else if let Some(len) = content_length {
        let mut to_read = len;
        while to_read > 0 {
            if !buf.is_empty() {
                let take = to_read.min(buf.len() as u64) as usize;
                buf.drain(..take);
                to_read -= take as u64;
            } else {
                let mut chunk = [0u8; 16384];
                let cap = to_read.min(chunk.len() as u64) as usize;
                let n = stream.read(&mut chunk[..cap]).await?;
                if n == 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "eof inside content-length body",
                    ));
                }
                consumed += n as u64;
                to_read -= n as u64;
            }
        }
    }
    // No length and no chunking: the body ends at connection close, which
    // terminates the whole pipeline batch — handled by the caller.

    Ok((status, consumed))
}

/// Find the `\r\n\r\n` terminator in `buf`, returning the offset of its
/// first byte.
fn find_head_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

impl StreamBody {
    fn total_len(&self) -> u64 {
        self.prefix.len() as u64 + self.fill_total + self.suffix.len() as u64
    }
}

/// Run the pipelining bench.
pub async fn run(cfg: PipelineBenchConfig) -> anyhow::Result<PipelineBenchResult> {
    let (host, port, path) =
        parse_http_target(&cfg.target_url).map_err(anyhow::Error::msg)?;
    if cfg.pipeline_depth == 0 {
        anyhow::bail!("--pipeline-depth must be >= 1");
    }
    if cfg.connections == 0 {
        anyhow::bail!("--connections must be >= 1");
    }
    // Validate framing up front so a bad size fails fast.
    StreamBody::new(cfg.body_kind, cfg.body_size).map_err(anyhow::Error::msg)?;

    let addr = tokio::net::lookup_host((host.as_str(), port))
        .await
        .anyhow_err()?
        .next()
        .ok_or_else(|| anyhow::anyhow!("could not resolve {host}"))?;

    let deadline = Instant::now() + cfg.duration;
    let counters = Arc::new(Counters::default());

    let mut handles = Vec::with_capacity(cfg.connections);
    for _ in 0..cfg.connections {
        let counters = counters.clone();
        let host = host.clone();
        let path = path.clone();
        let method = cfg.method.clone();
        handles.push(tokio::spawn(async move {
            connection_loop(
                addr, &host, port, &path, &method, cfg.body_kind, cfg.body_size,
                cfg.pipeline_depth, deadline, &counters,
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
    addr: std::net::SocketAddr,
    host: &str,
    _port: u16,
    path: &str,
    method: &str,
    kind: BodyKind,
    body_size: u64,
    depth: usize,
    deadline: Instant,
    counters: &Counters,
) {
    let mut stream: Option<TcpStream> = None;
    while Instant::now() < deadline {
        if stream.is_none() {
            match TcpStream::connect(addr).await {
                Ok(s) => stream = Some(s),
                Err(_) => {
                    counters.connection_errors.fetch_add(1, Ordering::Relaxed);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
            }
        }
        let s = stream.as_mut().expect("stream just set");

        // Write the whole pipeline batch back-to-back.
        let mut batch_sent = 0u64;
        let mut ok = true;
        for _ in 0..depth {
            // A FRESH generator per request: StreamBody is single-shot, so
            // reusing it would send a body on request 1 and starve the rest
            // (the exact deadlock seen in the first live smoke run).
            let mut body = match StreamBody::new(kind, body_size) {
                Ok(b) => b,
                Err(_) => return, // validated up front; unreachable
            };
            match write_batch_request(s, method, host, path, &mut body).await {
                Ok(bytes) => {
                    batch_sent += bytes;
                    counters.requests_sent.fetch_add(1, Ordering::Relaxed);
                }
                Err(_) => {
                    counters.connection_errors.fetch_add(1, Ordering::Relaxed);
                    ok = false;
                    break;
                }
            }
        }
        if ok {
            if let Err(e) = s.flush().await {
                let _ = e;
                counters.connection_errors.fetch_add(1, Ordering::Relaxed);
                ok = false;
            }
        }
        counters.bytes_sent.fetch_add(batch_sent, Ordering::Relaxed);
        if !ok {
            stream = None;
            continue;
        }

        // Read the responses strictly in order.
        let mut recv_buf: Vec<u8> = Vec::with_capacity(16 * 1024);
        for _ in 0..depth {
            match read_response(s, &mut recv_buf).await {
                Ok((status, consumed)) => {
                    counters.responses_received.fetch_add(1, Ordering::Relaxed);
                    counters
                        .bytes_received
                        .fetch_add(consumed, Ordering::Relaxed);
                    if let Ok(mut map) = counters.statuses.lock() {
                        *map.entry(status.to_string()).or_insert(0) += 1;
                    }
                }
                Err(e) => {
                    // Server hung up before answering the whole pipeline —
                    // the classic "pipelining not supported" tell (#937).
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        counters.closed_early.fetch_add(1, Ordering::Relaxed);
                    } else {
                        counters.connection_errors.fetch_add(1, Ordering::Relaxed);
                    }
                    break;
                }
            }
        }

        stream = None;
        counters.reconnects.fetch_add(1, Ordering::Relaxed);
    }
}

/// Write one complete request (head + streamed exact-sized body).
async fn write_batch_request(
    stream: &mut TcpStream,
    method: &str,
    host: &str,
    path: &str,
    body: &mut StreamBody,
) -> std::io::Result<u64> {
    let ct = body.kind.content_type();
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: mockforge-bench-pipeline\r\n\
         Content-Type: {ct}\r\nContent-Length: {}\r\n\r\n",
        body.total_len(),
    );
    let mut written = head.len() as u64;
    stream.write_all(head.as_bytes()).await?;
    while let Some(chunk) = body.next_chunk() {
        stream.write_all(&chunk).await?;
        written += chunk.len() as u64;
    }
    Ok(written)
}

/// Human-readable report.
pub fn render_report(res: &PipelineBenchResult) -> String {
    let mut out = String::new();
    out.push_str("\n=== bench-pipeline results ===\n");
    out.push_str(&format!("requests sent          : {}\n", res.requests_sent));
    out.push_str(&format!(
        "responses received     : {}\n",
        res.responses_received
    ));
    if res.requests_sent != res.responses_received {
        out.push_str(&format!(
            "  NOTE: {} request(s) never got a response — the target likely \
             does not support pipelining (serialized or closed early).\n",
            res.requests_sent - res.responses_received
        ));
    }
    out.push_str(&format!(
        "connection closed early: {}\n",
        res.connection_closed_early
    ));
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
