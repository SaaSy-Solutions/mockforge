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
    /// Required VUs to sustain `target_rps` end-to-end with `num_operations`
    /// per iteration.
    ///
    /// Why `num_operations` matters: k6's `constant-arrival-rate` executor
    /// (set by `--rps`) targets ITERATIONS per second, not requests. Each
    /// iteration runs every operation in the generated script sequentially.
    /// So sustaining `--rps R` with a spec of `N` operations actually needs
    /// `R × N × latency_secs` VUs, not `R × latency_secs`.
    ///
    /// Issue #79 round 9 — Srikanth saw "Pre-flight probe: --vus 5 is
    /// sufficient" followed by k6 emitting "Insufficient VUs" mid-run.
    /// Cause: he had a ~12-operation spec, so the real iteration time
    /// was ~12 × measured latency, not 1 ×.
    ///
    /// Formula: `rps × num_operations × latency_secs + 1` (safety margin).
    /// Returns at least 1.
    pub fn required_vus(&self, target_rps: u32, num_operations: u32) -> u32 {
        let lat_secs = self.avg_latency.as_secs_f64().max(0.001);
        let ops = num_operations.max(1) as f64;
        let raw = (target_rps as f64 * ops * lat_secs).ceil() as u32;
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
        // Single-op spec: 1000 rps × 1 op × 2ms = 2 VU + 1 margin = 3
        assert_eq!(fast.required_vus(1000, 1), 3);

        let slow = ProbeResult {
            avg_latency: Duration::from_millis(100),
            samples: 3,
        };
        // Single-op: 100 rps × 1 op × 100ms = 10 VU + 1 margin = 11
        assert_eq!(slow.required_vus(100, 1), 11);
    }

    #[test]
    fn required_vus_scales_with_operation_count() {
        // Issue #79 round 9 — Srikanth saw "vus 5 is sufficient" then k6
        // hit Insufficient VUs because his spec had ~12 operations per
        // iteration. With 15ms baseline × 12 ops × 100 rps = 18 VUs.
        let probe = ProbeResult {
            avg_latency: Duration::from_millis(15),
            samples: 3,
        };
        assert_eq!(probe.required_vus(100, 1), 3); // single op
        assert_eq!(probe.required_vus(100, 12), 19); // 12 ops + 1 margin
    }

    #[test]
    fn required_vus_clamps_to_one() {
        let fast = ProbeResult {
            avg_latency: Duration::from_micros(50),
            samples: 1,
        };
        // (1 × 0.001s clamp) × 1 op × 1 rps = 1, +1 margin = 2
        assert!(fast.required_vus(1, 1) >= 1);
    }

    #[test]
    fn required_vus_treats_zero_operations_as_one() {
        let probe = ProbeResult {
            avg_latency: Duration::from_millis(10),
            samples: 1,
        };
        // num_operations=0 should clamp to 1 so we never divide-by-zero
        // upstream or report an impossible "0 VUs" recommendation.
        assert_eq!(probe.required_vus(100, 0), probe.required_vus(100, 1));
    }

    #[tokio::test]
    async fn probe_returns_none_for_unreachable() {
        // Reserved-for-docs port on a non-routable address — should error
        // fast (no DNS lookup) without hanging.
        let result = probe_target_latency("http://127.0.0.1:1/", 1, false).await;
        assert!(result.is_none());
    }
}
