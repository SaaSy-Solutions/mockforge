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
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
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
    if !n.is_finite() || n < 0.0 || n * mult as f64 >= u64::MAX as f64 {
        return Err(format!("body size must be finite, non-negative, and within the byte counter range, got '{spec}'"));
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
    /// Returns an error when `size` is too small for the frame.
    #[cfg(test)]
    fn new(kind: BodyKind, size: u64) -> Result<Self, String> {
        Self::with_multipart_content_type(kind, size, "application/octet-stream")
    }

    fn with_multipart_content_type(
        kind: BodyKind,
        size: u64,
        file_type: &str,
    ) -> Result<Self, String> {
        use BodyKind::*;
        let (prefix, suffix): (Vec<u8>, Vec<u8>) = match kind {
            Json => (b"{\"data\":\"".to_vec(), b"\"}".to_vec()),
            Xml => (b"<root><data>".to_vec(), b"</data></root>".to_vec()),
            UrlEncoded => (b"data=".to_vec(), Vec::new()),
            // One synthetic file part, framed by the multipart boundary
            // declared in BodyKind::content_type().
            Multipart => (
                format!("--mockforge-pipeline\r\nContent-Disposition: form-data; name=\"file\"; filename=\"synthetic.bin\"\r\nContent-Type: {file_type}\r\n\r\n").into_bytes(),
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
                self.phase = if self.phase >= b'z' {
                    b'a'
                } else {
                    self.phase + 1
                };
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

/// Read a final HTTP response, retaining bytes belonging to the next response.
async fn read_response(
    stream: &mut (impl AsyncRead + Unpin),
    buf: &mut Vec<u8>,
    bytes_received: &AtomicU64,
    head_request: bool,
) -> std::io::Result<(u16, u64)> {
    let before = bytes_received.load(Ordering::Relaxed);
    let (status, head) = loop {
        let end = loop {
            if let Some(end) = find_head_end(buf) {
                if end > 128 * 1024 {
                    return Err(invalid_response("response head exceeds 128 KiB"));
                }
                break end;
            }
            if buf.len() > 128 * 1024 {
                return Err(invalid_response("response head exceeds 128 KiB"));
            }
            read_more(stream, buf, bytes_received).await?;
        };
        let head = std::str::from_utf8(&buf[..end])
            .map_err(|_| invalid_response("invalid response head"))?
            .to_owned();
        buf.drain(..end + 4);
        let status = head
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|value| value.parse::<u16>().ok())
            .filter(|value| (100..=599).contains(value))
            .ok_or_else(|| invalid_response("malformed status line"))?;
        if status == 101 {
            return Err(invalid_response("protocol upgrades are not supported"));
        }
        if status >= 200 {
            break (status, head);
        }
        // Informational responses do not complete the pipelined request.
    };
    if head_request || status == 204 || status == 304 {
        return Ok((status, bytes_received.load(Ordering::Relaxed) - before));
    }
    let mut length = None;
    let mut transfer = Vec::new();
    for line in head.lines().skip(1) {
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| invalid_response("malformed response header"))?;
        if name.eq_ignore_ascii_case("content-length") {
            for value in value.split(',') {
                let parsed = value
                    .trim()
                    .parse::<u64>()
                    .map_err(|_| invalid_response("invalid content length"))?;
                if length.is_some_and(|previous| previous != parsed) {
                    return Err(invalid_response("conflicting content lengths"));
                }
                length = Some(parsed);
            }
        } else if name.eq_ignore_ascii_case("transfer-encoding") {
            transfer.extend(value.split(',').map(|value| value.trim().to_ascii_lowercase()));
        }
    }
    if transfer.last().is_some_and(|coding| coding == "chunked") {
        loop {
            let line = read_line(stream, buf, bytes_received).await?;
            let size = std::str::from_utf8(&line)
                .ok()
                .and_then(|line| line.split(';').next())
                .filter(|value| {
                    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_hexdigit())
                })
                .and_then(|value| u64::from_str_radix(value, 16).ok())
                .ok_or_else(|| invalid_response("invalid chunk size"))?;
            if size == 0 {
                let mut trailer_bytes = 0;
                loop {
                    let trailer = read_line(stream, buf, bytes_received).await?;
                    trailer_bytes += trailer.len() + 2;
                    if trailer_bytes > 128 * 1024 {
                        return Err(invalid_response("response trailers exceed 128 KiB"));
                    }
                    if trailer.is_empty() {
                        break;
                    }
                    if !trailer.contains(&b':') {
                        return Err(invalid_response("malformed response trailer"));
                    }
                }
                break;
            }
            drain_bytes(stream, buf, bytes_received, size).await?;
            // Validate the payload terminator without accepting arbitrary bytes.
            while buf.len() < 2 {
                read_more(stream, buf, bytes_received).await?;
            }
            if &buf[..2] != b"\r\n" {
                return Err(invalid_response("invalid chunk terminator"));
            }
            buf.drain(..2);
        }
    } else if transfer.is_empty() && length.is_some() {
        drain_bytes(stream, buf, bytes_received, length.unwrap_or_default()).await?;
    } else {
        // A close-delimited response completes only at EOF. Its bytes cannot
        // be interpreted as another pipelined status line.
        buf.clear();
        let mut chunk = [0u8; 16384];
        loop {
            let read = stream.read(&mut chunk).await?;
            if read == 0 {
                break;
            }
            bytes_received.fetch_add(read as u64, Ordering::Relaxed);
        }
    }
    Ok((status, bytes_received.load(Ordering::Relaxed) - before))
}

fn invalid_response(message: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message)
}

async fn read_more(
    stream: &mut (impl AsyncRead + Unpin),
    buf: &mut Vec<u8>,
    counter: &AtomicU64,
) -> std::io::Result<()> {
    let mut chunk = [0u8; 8192];
    let read = stream.read(&mut chunk).await?;
    if read == 0 {
        return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "incomplete response"));
    }
    counter.fetch_add(read as u64, Ordering::Relaxed);
    buf.extend_from_slice(&chunk[..read]);
    Ok(())
}

async fn read_line(
    stream: &mut (impl AsyncRead + Unpin),
    buf: &mut Vec<u8>,
    counter: &AtomicU64,
) -> std::io::Result<Vec<u8>> {
    loop {
        if let Some(end) = buf.windows(2).position(|window| window == b"\r\n") {
            if end > 128 * 1024 {
                return Err(invalid_response("response line exceeds 128 KiB"));
            }
            let line = buf.drain(..end).collect();
            buf.drain(..2);
            return Ok(line);
        }
        if buf.len() > 128 * 1024 {
            return Err(invalid_response("response line exceeds 128 KiB"));
        }
        read_more(stream, buf, counter).await?;
    }
}

async fn drain_bytes(
    stream: &mut (impl AsyncRead + Unpin),
    buf: &mut Vec<u8>,
    counter: &AtomicU64,
    mut remaining: u64,
) -> std::io::Result<()> {
    while remaining > 0 {
        if buf.is_empty() {
            read_more(stream, buf, counter).await?;
        }
        let take = remaining.min(buf.len() as u64) as usize;
        buf.drain(..take);
        remaining -= take as u64;
    }
    Ok(())
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
    let ct = body.kind.content_type();
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

#[cfg(test)]
mod tests {
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

    fn socket_config(
        port: u16,
        kind: BodyKind,
        size: u64,
        duration: Duration,
    ) -> PipelineBenchConfig {
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
}
