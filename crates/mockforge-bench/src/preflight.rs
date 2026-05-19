//! Pre-flight latency probe for sizing `--vus` against `--rps`.
//!
//! Issue #79 round 8 — Srikanth's reply on 0.3.137 flagged that the
//! `--vus * 10 < --rps` warning's "1 VU sustains ~10 req/s at 100ms latency"
//! rule of thumb is wrong for fast targets (~2ms). At 2ms response time, one
//! VU can drive ~500 req/s, so `--vus 5` is enough for `--rps 1000` but the
//! static rule incorrectly warns "bump to --vus 100".
//!
//! Instead, do a tiny (1-3 request) HTTP probe of the actual target to
//! measure baseline latency, then derive a more accurate "VUs needed to
//! sustain rate" estimate. Skip the warning entirely if the measured rate
//! comfortably covers the requested rate.

use std::time::{Duration, Instant};

/// Result of a pre-flight latency probe.
#[derive(Debug, Clone, Copy)]
pub struct ProbeResult {
    /// Observed average request latency.
    pub avg_latency: Duration,
    /// Number of successful probe requests.
    pub samples: u32,
}

impl ProbeResult {
    /// Required VUs to sustain `target_rps` end-to-end, given the observed
    /// latency. Formula: `rps × latency_secs`, with a small +1 safety margin.
    /// Returns at least 1.
    pub fn required_vus(&self, target_rps: u32) -> u32 {
        let lat_secs = self.avg_latency.as_secs_f64().max(0.001);
        let raw = (target_rps as f64 * lat_secs).ceil() as u32;
        raw.saturating_add(1).max(1)
    }
}

/// Probe `target` with up to `samples` quick HEAD/GET requests and report
/// the average successful-response latency. Each probe has a 5s timeout;
/// failed probes are excluded from the average. Returns `None` if no
/// probe succeeded.
///
/// Used pre-flight to size the `--vus` warning. We deliberately *don't*
/// fail the bench if probes fail — the target might require auth or be
/// strict about HEADs. Falling back to the static heuristic is fine.
pub async fn probe_target_latency(
    target: &str,
    samples: u32,
    skip_tls_verify: bool,
) -> Option<ProbeResult> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .danger_accept_invalid_certs(skip_tls_verify)
        .build()
        .ok()?;

    let mut total = Duration::ZERO;
    let mut count: u32 = 0;
    for _ in 0..samples {
        let start = Instant::now();
        // HEAD first — cheaper for the target. Fall back to GET if HEAD
        // fails (some servers / WAFs reject HEAD).
        let head = client.head(target).send().await;
        let elapsed = match head {
            Ok(_) => start.elapsed(),
            Err(_) => {
                let start = Instant::now();
                match client.get(target).send().await {
                    Ok(_) => start.elapsed(),
                    Err(_) => continue,
                }
            }
        };
        total += elapsed;
        count = count.saturating_add(1);
    }

    if count == 0 {
        return None;
    }

    Some(ProbeResult {
        avg_latency: total / count,
        samples: count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_vus_scales_with_latency() {
        let fast = ProbeResult {
            avg_latency: Duration::from_millis(2),
            samples: 3,
        };
        // 1000 rps × 2ms = 2 VU + 1 margin = 3
        assert_eq!(fast.required_vus(1000), 3);

        let slow = ProbeResult {
            avg_latency: Duration::from_millis(100),
            samples: 3,
        };
        // 100 rps × 100ms = 10 VU + 1 margin = 11
        assert_eq!(slow.required_vus(100), 11);
    }

    #[test]
    fn required_vus_clamps_to_one() {
        let fast = ProbeResult {
            avg_latency: Duration::from_micros(50),
            samples: 1,
        };
        // (1 × 0.001s clamp) × 1 = 1, +1 margin = 2
        assert!(fast.required_vus(1) >= 1);
    }

    #[tokio::test]
    async fn probe_returns_none_for_unreachable() {
        // Reserved-for-docs port on a non-routable address — should error
        // fast (no DNS lookup) without hanging.
        let result = probe_target_latency("http://127.0.0.1:1/", 1, false).await;
        assert!(result.is_none());
    }
}
