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
//!     rps: None,
//!     no_keep_alive: false,
//!     body: None,
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
    /// Cap on request *starts* per second, shared by all workers. `None` =
    /// each worker starts its next request as soon as the previous one ends.
    /// The achieved rate can't exceed `concurrency / request_duration`.
    pub rps: Option<u32>,
    /// Open a fresh TCP/TLS connection for every request (no pooling), so the
    /// connections-per-second rate equals the request rate.
    pub no_keep_alive: bool,
    /// Pre-built request body to stream instead of `X` filler (e.g. a JSON
    /// document from [`build_json_body`]). When set, its length replaces
    /// `total_size_bytes` as the per-request body size.
    pub body: Option<Arc<Vec<u8>>>,
}

impl ChunkedBenchConfig {
    /// Bytes each request actually sends.
    fn body_len(&self) -> usize {
        self.body.as_ref().map_or(self.total_size_bytes, |b| b.len())
    }
}

/// Field name used to pad JSON bodies up to the requested size.
pub const JSON_PADDING_FIELD: &str = "_padding";

/// True when a `Content-Type` header (any casing) names a JSON media type:
/// `application/json` or any `+json` suffix type.
pub fn is_json_content_type(headers: &HashMap<String, String>) -> bool {
    headers.iter().any(|(k, v)| {
        if !k.eq_ignore_ascii_case("content-type") {
            return false;
        }
        let media = v.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
        media == "application/json" || media.ends_with("+json")
    })
}

/// Build a valid JSON document of exactly `total` bytes.
///
/// `seed` (typically the spec-generated request body) supplies the fields; a
/// string field named [`JSON_PADDING_FIELD`] is added and filled with `X`s so
/// the serialized size lands on `total`. A non-object seed is wrapped as
/// `{"data": seed}`. If `total` is smaller than the smallest valid document,
/// that smallest document is returned (and is longer than `total`).
pub fn build_json_body(seed: Option<&serde_json::Value>, total: usize) -> Vec<u8> {
    let mut obj = match seed {
        Some(serde_json::Value::Object(m)) => m.clone(),
        Some(other) => {
            let mut m = serde_json::Map::new();
            m.insert("data".to_string(), other.clone());
            m
        }
        None => serde_json::Map::new(),
    };
    obj.insert(JSON_PADDING_FIELD.to_string(), serde_json::Value::String(String::new()));
    let base = serde_json::to_vec(&obj).unwrap_or_default().len();
    // `X` serializes as one byte, so the padding length maps 1:1 to output size.
    let pad = total.saturating_sub(base);
    obj.insert(JSON_PADDING_FIELD.to_string(), serde_json::Value::String("X".repeat(pad)));
    serde_json::to_vec(&obj).unwrap_or_default()
}

/// Hands out evenly spaced request start times across all workers.
struct Pacer {
    interval: Duration,
    next: Mutex<Instant>,
}

impl Pacer {
    fn new(rps: u32) -> Self {
        Self {
            interval: Duration::from_secs_f64(1.0 / f64::from(rps)),
            next: Mutex::new(Instant::now()),
        }
    }

    /// Reserve the next start slot. Slots are never handed out in the past,
    /// so a stall doesn't cause a burst to "catch up".
    async fn reserve(&self) -> Instant {
        let mut next = self.next.lock().await;
        let slot = (*next).max(Instant::now());
        *next = slot + self.interval;
        slot
    }
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
    if cfg.rps == Some(0) {
        anyhow::bail!("rps must be >= 1");
    }

    let mut builder = reqwest::Client::builder().danger_accept_invalid_certs(cfg.skip_tls_verify);
    if cfg.no_keep_alive {
        builder = builder.pool_max_idle_per_host(0);
    }
    let client = builder.build()?;
    let pacer = cfg.rps.map(|n| Arc::new(Pacer::new(n)));

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
        let pacer = pacer.clone();

        workers.push(tokio::spawn(async move {
            while Instant::now() < deadline {
                if let Some(pacer) = &pacer {
                    let slot = pacer.reserve().await;
                    if slot >= deadline {
                        break;
                    }
                    tokio::time::sleep_until(slot.into()).await;
                }
                let req_started = Instant::now();
                match send_one_chunked_request(&client, &cfg).await {
                    Ok(SendResult { status, sample }) => {
                        successful.fetch_add(1, Ordering::Relaxed);
                        bytes_sent.fetch_add(cfg.body_len() as u64, Ordering::Relaxed);
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
    let total = cfg.body_len();
    let interval_ms = cfg.chunk_interval_ms;
    let prebuilt = cfg.body.clone();

    // Build a stream that yields fixed-size chunks until `total` bytes are
    // emitted, slicing the pre-built body when there is one and `X` filler
    // otherwise. No Content-Length is set on the request, so hyper transports
    // the body as Transfer-Encoding: chunked.
    let body_stream = stream! {
        let mut sent: usize = 0;
        let filler = vec![b'X'; chunk_size];
        while sent < total {
            // Wait *between* chunks: the first chunk goes out immediately.
            if interval_ms > 0 && sent > 0 {
                tokio::time::sleep(Duration::from_millis(interval_ms)).await;
            }
            let next = std::cmp::min(chunk_size, total - sent);
            let chunk = match &prebuilt {
                Some(b) => b[sent..sent + next].to_vec(),
                None => filler[..next].to_vec(),
            };
            sent += next;
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
            rps: None,
            no_keep_alive: false,
            body: None,
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
            rps: None,
            no_keep_alive: false,
            body: None,
        };
        assert!(run(cfg).await.is_err());
    }

    #[tokio::test]
    async fn pacer_spaces_slots_evenly() {
        let pacer = Pacer::new(10);
        let a = pacer.reserve().await;
        let b = pacer.reserve().await;
        let c = pacer.reserve().await;
        assert_eq!(b - a, Duration::from_millis(100));
        assert_eq!(c - b, Duration::from_millis(100));
    }

    #[tokio::test]
    async fn rps_caps_request_starts_across_workers() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/upload")
            .with_status(200)
            .expect_at_least(1)
            .create_async()
            .await;
        let cfg = ChunkedBenchConfig {
            target_url: format!("{}/upload", server.url()),
            method: reqwest::Method::POST,
            concurrency: 8,
            duration: Duration::from_millis(1000),
            chunk_size_bytes: 64,
            total_size_bytes: 256,
            chunk_interval_ms: 0,
            headers: HashMap::new(),
            skip_tls_verify: false,
            rps: Some(5),
            no_keep_alive: true,
            body: None,
        };
        let r = run(cfg).await.unwrap();
        // 1s at 5 rps = slots at 0, 200, 400, 600, 800ms. Unpaced, 8 workers
        // against a local server would do hundreds.
        assert!((4..=6).contains(&r.total_requests), "got {} requests", r.total_requests);
        assert_eq!(r.failed, 0);
    }

    #[tokio::test]
    async fn rejects_zero_rps() {
        let cfg = ChunkedBenchConfig {
            target_url: "http://127.0.0.1:1".into(),
            method: reqwest::Method::POST,
            concurrency: 1,
            duration: Duration::from_millis(10),
            chunk_size_bytes: 1024,
            total_size_bytes: 4096,
            chunk_interval_ms: 0,
            headers: HashMap::new(),
            skip_tls_verify: false,
            rps: Some(0),
            no_keep_alive: false,
            body: None,
        };
        assert!(run(cfg).await.is_err());
    }

    #[test]
    fn json_body_is_valid_and_exact_size() {
        let seed = serde_json::json!({"name": "widget", "count": 3});
        for total in [64usize, 4096, 1_048_576] {
            let body = build_json_body(Some(&seed), total);
            assert_eq!(body.len(), total);
            let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(v["name"], "widget");
            assert_eq!(v["count"], 3);
        }
    }

    #[test]
    fn json_body_without_seed_or_non_object_seed() {
        let body = build_json_body(None, 100);
        assert_eq!(body.len(), 100);
        assert!(serde_json::from_slice::<serde_json::Value>(&body).unwrap().is_object());

        let body = build_json_body(Some(&serde_json::json!([1, 2])), 100);
        assert_eq!(body.len(), 100);
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["data"], serde_json::json!([1, 2]));
    }

    #[test]
    fn json_body_smaller_than_minimum_stays_valid() {
        let body = build_json_body(None, 1);
        assert_eq!(body, br#"{"_padding":""}"#);
    }

    #[test]
    fn detects_json_content_types() {
        let h = |v: &str| HashMap::from([("content-type".to_string(), v.to_string())]);
        assert!(is_json_content_type(&h("application/json")));
        assert!(is_json_content_type(&h("Application/JSON; charset=utf-8")));
        assert!(is_json_content_type(&h("application/vnd.api+json")));
        assert!(!is_json_content_type(&h("application/octet-stream")));
        assert!(!is_json_content_type(&HashMap::new()));
        let upper = HashMap::from([("Content-Type".to_string(), "application/json".to_string())]);
        assert!(is_json_content_type(&upper));
    }

    #[tokio::test]
    async fn streams_prebuilt_json_body_intact() {
        let mut server = mockito::Server::new_async().await;
        let m = server
            .mock("POST", "/items")
            .match_body(mockito::Matcher::PartialJson(serde_json::json!({"name": "widget"})))
            .with_status(201)
            .expect_at_least(1)
            .create_async()
            .await;
        let body = build_json_body(Some(&serde_json::json!({"name": "widget"})), 10_000);
        let cfg = ChunkedBenchConfig {
            target_url: format!("{}/items", server.url()),
            method: reqwest::Method::POST,
            concurrency: 1,
            duration: Duration::from_millis(300),
            chunk_size_bytes: 777,
            total_size_bytes: 10_000,
            chunk_interval_ms: 0,
            headers: HashMap::from([("Content-Type".into(), "application/json".into())]),
            skip_tls_verify: false,
            rps: None,
            no_keep_alive: false,
            body: Some(Arc::new(body)),
        };
        let r = run(cfg).await.unwrap();
        assert_eq!(r.failed, 0);
        assert!(r.status_counts.contains_key(&201), "statuses: {:?}", r.status_counts);
        assert_eq!(r.bytes_sent, r.successful * 10_000);
        m.assert_async().await;
    }
}
