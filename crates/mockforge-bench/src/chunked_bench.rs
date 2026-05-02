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
}

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

        workers.push(tokio::spawn(async move {
            while Instant::now() < deadline {
                let req_started = Instant::now();
                match send_one_chunked_request(&client, &cfg).await {
                    Ok(status) => {
                        successful.fetch_add(1, Ordering::Relaxed);
                        bytes_sent.fetch_add(cfg.total_size_bytes as u64, Ordering::Relaxed);
                        let elapsed_ms = req_started.elapsed().as_millis() as u64;
                        latencies.lock().await.push(elapsed_ms);
                        *status_counts.lock().await.entry(status).or_insert(0) += 1;
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
    })
}

async fn send_one_chunked_request(
    client: &reqwest::Client,
    cfg: &ChunkedBenchConfig,
) -> anyhow::Result<u16> {
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
    Ok(resp.status().as_u16())
}

#[cfg(test)]
mod tests {
    use super::*;

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
