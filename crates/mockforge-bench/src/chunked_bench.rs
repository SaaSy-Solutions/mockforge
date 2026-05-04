//! Native Rust chunked-encoding traffic generator.
//!
//! `mockforge bench --native-chunked` bypasses k6 entirely. Each worker opens
//! its own HTTP connection and sends POST/PUT/PATCH requests with bodies
//! streamed via `reqwest::Body::wrap_stream`. Because the body has no known
//! `Content-Length`, hyper transports it as `Transfer-Encoding: chunked` —
//! guaranteed, unlike the k6/Go path where the runtime decides based on body
//! type.
//!
//! This is a small benchmark intended to exercise the *server's* chunked
//! handling (slow consumers, max body size, partial-response chaos against
//! chunked uploads). Not a k6 replacement for general load testing.
//!
//! ```no_run
//! # use mockforge_bench::chunked_bench::{ChunkedBenchConfig, run};
//! # use std::time::Duration;
//! # use std::collections::HashMap;
//! # async fn x() -> anyhow::Result<()> {
//! let result = run(ChunkedBenchConfig {
//!     target_url: "http://localhost:3000/upload".into(),
//!     method: reqwest::Method::POST,
//!     concurrency: 10,
//!     duration: Duration::from_secs(60),
//!     chunk_size_bytes: 1024,
//!     total_size_bytes: 1024 * 1024,
//!     chunk_interval_ms: 0,
//!     headers: HashMap::new(),
//!     skip_tls_verify: false,
//! }).await?;
//! println!("{} req/s", result.req_per_sec);
//! # Ok(()) }
//! ```

use async_stream::stream;
use futures::StreamExt;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

/// Configuration for the native chunked-encoding bench.
#[derive(Debug, Clone)]
pub struct ChunkedBenchConfig {
    /// Target URL (e.g. `http://localhost:3000/upload`).
    pub target_url: String,
    /// HTTP method. POST/PUT/PATCH make sense; GET/HEAD don't take a body.
    pub method: reqwest::Method,
    /// Number of concurrent workers (each holds its own connection / future).
    pub concurrency: u32,
    /// Total run duration.
    pub duration: Duration,
    /// Bytes per chunk emitted into the request body stream.
    pub chunk_size_bytes: usize,
    /// Total body size per request, in bytes.
    pub total_size_bytes: usize,
    /// Sleep between chunks, in milliseconds. 0 = back-to-back.
    pub chunk_interval_ms: u64,
    /// Extra headers to attach to every request. `Transfer-Encoding: chunked`
    /// is set automatically by hyper because the body has no Content-Length.
    pub headers: HashMap<String, String>,
    /// Skip TLS certificate verification (useful for test self-signed certs).
    pub skip_tls_verify: bool,
}

/// One captured non-2xx response — used to surface *who* sent the error
/// (mockforge? a proxy in front?) and *why*. Critical for diagnosing
/// "I see 503 from the bench but the server log shows 200" — almost
/// always an upstream proxy timing out on a slow chunked upload.
#[derive(Debug, Clone)]
pub struct ErrorSample {
    pub status: u16,
    /// `Server` response header, when present. Often reveals the
    /// proxy: `nginx/1.21.0`, `cloudflare`, `envoy`, `awselb/2.0`, etc.
    pub server_header: Option<String>,
    /// First N bytes of the response body, lossy-UTF8'd. Trimmed.
    pub body_excerpt: String,
}

/// Aggregate result from a chunked bench run.
#[derive(Debug, Clone)]
pub struct ChunkedBenchResult {
    pub total_requests: u64,
    pub successful: u64,
    pub failed: u64,
    pub bytes_sent: u64,
    pub elapsed: Duration,
    pub req_per_sec: f64,
    pub latencies_ms: Vec<u64>,
    pub avg_latency_ms: f64,
    pub p50_ms: u64,
    pub p95_ms: u64,
    pub p99_ms: u64,
    pub status_counts: HashMap<u16, u64>,
    /// First N captured non-2xx responses (status, body excerpt, Server
    /// header). Empty when every request succeeded.
    pub error_samples: Vec<ErrorSample>,
}

/// How many distinct error responses to capture body+headers for.
const MAX_ERROR_SAMPLES: usize = 5;
/// How many bytes of error response body to keep per sample.
const ERROR_BODY_EXCERPT_BYTES: usize = 256;

/// Run the chunked-traffic bench. Spawns `concurrency` worker tasks that send
/// chunked POSTs back-to-back until `duration` elapses, then aggregates stats.
pub async fn run(cfg: ChunkedBenchConfig) -> anyhow::Result<ChunkedBenchResult> {
    if cfg.chunk_size_bytes == 0 {
        anyhow::bail!("chunk_size_bytes must be > 0");
    }
    if cfg.total_size_bytes == 0 {
        anyhow::bail!("total_size_bytes must be > 0");
    }
    if cfg.concurrency == 0 {
        anyhow::bail!("concurrency must be >= 1");
    }

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(cfg.skip_tls_verify)
        .build()?;

    let total_requests = Arc::new(AtomicU64::new(0));
    let successful = Arc::new(AtomicU64::new(0));
    let failed = Arc::new(AtomicU64::new(0));
    let bytes_sent = Arc::new(AtomicU64::new(0));
    let latencies: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(Vec::with_capacity(8192)));
    let status_counts: Arc<Mutex<HashMap<u16, u64>>> = Arc::new(Mutex::new(HashMap::new()));
    let error_samples: Arc<Mutex<Vec<ErrorSample>>> = Arc::new(Mutex::new(Vec::new()));

    let deadline = Instant::now() + cfg.duration;
    let started_at = Instant::now();

    let mut workers = Vec::with_capacity(cfg.concurrency as usize);
    for _ in 0..cfg.concurrency {
        let cfg = cfg.clone();
        let client = client.clone();
        let total_requests = total_requests.clone();
        let successful = successful.clone();
        let failed = failed.clone();
        let bytes_sent = bytes_sent.clone();
        let latencies = latencies.clone();
        let status_counts = status_counts.clone();
        let error_samples = error_samples.clone();

        workers.push(tokio::spawn(async move {
            while Instant::now() < deadline {
                let req_started = Instant::now();
                match send_one_chunked_request(&client, &cfg).await {
                    Ok(SendResult { status, sample }) => {
                        successful.fetch_add(1, Ordering::Relaxed);
                        bytes_sent.fetch_add(cfg.total_size_bytes as u64, Ordering::Relaxed);
                        let elapsed_ms = req_started.elapsed().as_millis() as u64;
                        latencies.lock().await.push(elapsed_ms);
                        *status_counts.lock().await.entry(status).or_insert(0) += 1;
                        if let Some(s) = sample {
                            let mut g = error_samples.lock().await;
                            if g.len() < MAX_ERROR_SAMPLES {
                                g.push(s);
                            }
                        }
                    }
                    Err(_e) => {
                        failed.fetch_add(1, Ordering::Relaxed);
                    }
                }
                total_requests.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    for w in workers {
        let _ = w.await;
    }

    let elapsed = started_at.elapsed();
    let total = total_requests.load(Ordering::Relaxed);
    let mut samples: Vec<u64> = {
        let mut g = latencies.lock().await;
        std::mem::take(&mut *g)
    };
    let final_status_counts: HashMap<u16, u64> = {
        let mut g = status_counts.lock().await;
        std::mem::take(&mut *g)
    };
    let final_error_samples: Vec<ErrorSample> = {
        let mut g = error_samples.lock().await;
        std::mem::take(&mut *g)
    };
    samples.sort_unstable();
    let avg = if samples.is_empty() {
        0.0
    } else {
        samples.iter().copied().sum::<u64>() as f64 / samples.len() as f64
    };
    let p = |q: f64| -> u64 {
        if samples.is_empty() {
            return 0;
        }
        let idx = ((samples.len() as f64 - 1.0) * q).round() as usize;
        samples[idx]
    };

    Ok(ChunkedBenchResult {
        total_requests: total,
        successful: successful.load(Ordering::Relaxed),
        failed: failed.load(Ordering::Relaxed),
        bytes_sent: bytes_sent.load(Ordering::Relaxed),
        elapsed,
        req_per_sec: if elapsed.as_secs_f64() > 0.0 {
            total as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        },
        avg_latency_ms: avg,
        p50_ms: p(0.50),
        p95_ms: p(0.95),
        p99_ms: p(0.99),
        latencies_ms: samples,
        status_counts: final_status_counts,
        error_samples: final_error_samples,
    })
}

/// Per-request outcome from `send_one_chunked_request`. Carries an
/// `ErrorSample` only for non-2xx responses (and only until the caller
/// has accumulated `MAX_ERROR_SAMPLES`).
struct SendResult {
    status: u16,
    sample: Option<ErrorSample>,
}

async fn send_one_chunked_request(
    client: &reqwest::Client,
    cfg: &ChunkedBenchConfig,
) -> anyhow::Result<SendResult> {
    let chunk_size = cfg.chunk_size_bytes;
    let total = cfg.total_size_bytes;
    let interval_ms = cfg.chunk_interval_ms;

    // Build a stream that yields fixed-size chunks until `total` bytes are
    // emitted. No Content-Length is set on the request, so hyper transports
    // the body as Transfer-Encoding: chunked.
    let body_stream = stream! {
        let mut sent: usize = 0;
        let payload = vec![b'X'; chunk_size];
        while sent < total {
            let next = std::cmp::min(chunk_size, total - sent);
            let chunk = payload[..next].to_vec();
            sent += next;
            if interval_ms > 0 && sent < total {
                tokio::time::sleep(Duration::from_millis(interval_ms)).await;
            }
            yield Ok::<_, std::io::Error>(chunk);
        }
    };

    let body = reqwest::Body::wrap_stream(body_stream.boxed());

    let mut req = client.request(cfg.method.clone(), &cfg.target_url).body(body);
    for (k, v) in &cfg.headers {
        req = req.header(k, v);
    }
    let resp = req.send().await?;
    let status = resp.status().as_u16();

    // For non-2xx responses, capture a small excerpt + the Server header so
    // the user can tell at a glance whether the error came from MockForge,
    // an upstream proxy, a CDN, etc. This is the most useful diagnostic for
    // the "503 from bench, 200 in TUI" pattern (proxy upstream timeout).
    let sample = if !(200..300).contains(&status) {
        let server_header = resp
            .headers()
            .get(reqwest::header::SERVER)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let bytes = resp.bytes().await.unwrap_or_default();
        let take = std::cmp::min(bytes.len(), ERROR_BODY_EXCERPT_BYTES);
        let body_excerpt = String::from_utf8_lossy(&bytes[..take]).trim().to_owned();
        Some(ErrorSample {
            status,
            server_header,
            body_excerpt,
        })
    } else {
        None
    };

    Ok(SendResult { status, sample })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_sample_struct_holds_diagnostic_fields() {
        // Schema sanity: ErrorSample must carry the three pieces a user
        // needs to diagnose "where did this 503 come from?".
        let s = ErrorSample {
            status: 503,
            server_header: Some("nginx/1.21.0".into()),
            body_excerpt: "upstream timed out".into(),
        };
        assert_eq!(s.status, 503);
        assert_eq!(s.server_header.as_deref(), Some("nginx/1.21.0"));
        assert_eq!(s.body_excerpt, "upstream timed out");
    }

    #[tokio::test]
    async fn rejects_zero_concurrency() {
        let cfg = ChunkedBenchConfig {
            target_url: "http://127.0.0.1:1".into(),
            method: reqwest::Method::POST,
            concurrency: 0,
            duration: Duration::from_millis(10),
            chunk_size_bytes: 1024,
            total_size_bytes: 4096,
            chunk_interval_ms: 0,
            headers: HashMap::new(),
            skip_tls_verify: false,
        };
        assert!(run(cfg).await.is_err());
    }

    #[tokio::test]
    async fn rejects_zero_chunk_size() {
        let cfg = ChunkedBenchConfig {
            target_url: "http://127.0.0.1:1".into(),
            method: reqwest::Method::POST,
            concurrency: 1,
            duration: Duration::from_millis(10),
            chunk_size_bytes: 0,
            total_size_bytes: 4096,
            chunk_interval_ms: 0,
            headers: HashMap::new(),
            skip_tls_verify: false,
        };
        assert!(run(cfg).await.is_err());
    }
}
