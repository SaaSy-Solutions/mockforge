//! Pillar usage tracking utilities
//!
//! Provides helper functions for recording pillar usage events throughout the codebase.
//! These events are used for analytics and understanding which pillars are most used.

use crate::pillars::Pillar;
use chrono::Utc;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Optional analytics database for recording pillar usage
/// This is set globally and can be None if analytics is not enabled
#[allow(clippy::type_complexity)]
static ANALYTICS_DB: Lazy<Arc<RwLock<Option<Arc<dyn PillarUsageRecorder>>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Tracks dropped/failed pillar events so we can emit a rate-limited
/// aggregate WARN instead of one WARN per event under load.
///
/// Issue #79 — Srikanth's bench at `--rps 100` for 600s flooded the log
/// with hundreds of `WARN ... Failed to record pillar usage event: pool
/// timed out` lines (one per failed event). The events themselves are
/// best-effort metrics — losing them under sustained load doesn't break
/// anything functional — so the right behaviour is to drop with low-
/// volume reporting rather than spam.
static FAILED_EVENT_COUNT: AtomicU64 = AtomicU64::new(0);
static LAST_FAILURE_WARN_AT: Lazy<RwLock<Instant>> = Lazy::new(|| RwLock::new(Instant::now()));

/// How often we emit the aggregated "X pillar events dropped" warning.
const FAILURE_WARN_INTERVAL: Duration = Duration::from_secs(60);

/// Trait for recording pillar usage events
/// This allows different implementations (analytics DB, API endpoint, etc.)
#[async_trait::async_trait]
pub trait PillarUsageRecorder: Send + Sync {
    /// Record a pillar usage event
    async fn record(
        &self,
        event: PillarUsageEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Pillar usage event (simplified version for internal use)
#[derive(Debug, Clone)]
pub struct PillarUsageEvent {
    /// Workspace ID where the event occurred
    pub workspace_id: Option<String>,
    /// Organization ID (if applicable)
    pub org_id: Option<String>,
    /// The pillar this event relates to
    pub pillar: Pillar,
    /// Name of the metric being recorded
    pub metric_name: String,
    /// Value of the metric (JSON)
    pub metric_value: Value,
    /// Timestamp when the event occurred
    pub timestamp: chrono::DateTime<Utc>,
}

/// Initialize the pillar usage tracker with a recorder
pub async fn init(recorder: Arc<dyn PillarUsageRecorder>) {
    let mut db = ANALYTICS_DB.write().await;
    *db = Some(recorder);
}

/// Record a reality pillar usage event
///
/// This should be called when:
/// - Reality continuum blend ratio is used
/// - Smart personas are activated
/// - Chaos is enabled/used
/// - Reality level changes
pub async fn record_reality_usage(
    workspace_id: Option<String>,
    org_id: Option<String>,
    metric_name: &str,
    metric_value: Value,
) {
    record_pillar_usage(workspace_id, org_id, Pillar::Reality, metric_name, metric_value).await;
}

/// Record a contracts pillar usage event
///
/// This should be called when:
/// - Contract validation is performed
/// - Drift detection occurs
/// - Contract sync happens
/// - Validation mode changes
pub async fn record_contracts_usage(
    workspace_id: Option<String>,
    org_id: Option<String>,
    metric_name: &str,
    metric_value: Value,
) {
    record_pillar_usage(workspace_id, org_id, Pillar::Contracts, metric_name, metric_value).await;
}

/// Record a DevX pillar usage event
///
/// This should be called when:
/// - SDK is installed/used
/// - Client code is generated
/// - Playground session starts
/// - CLI command is executed
pub async fn record_devx_usage(
    workspace_id: Option<String>,
    org_id: Option<String>,
    metric_name: &str,
    metric_value: Value,
) {
    record_pillar_usage(workspace_id, org_id, Pillar::DevX, metric_name, metric_value).await;
}

/// Record a cloud pillar usage event
///
/// This should be called when:
/// - Scenario is shared
/// - Marketplace download occurs
/// - Workspace is created/shared
/// - Organization template is used
pub async fn record_cloud_usage(
    workspace_id: Option<String>,
    org_id: Option<String>,
    metric_name: &str,
    metric_value: Value,
) {
    record_pillar_usage(workspace_id, org_id, Pillar::Cloud, metric_name, metric_value).await;
}

/// Record an AI pillar usage event
///
/// This should be called when:
/// - AI mock generation occurs
/// - AI contract diff is performed
/// - Voice command is executed
/// - LLM-assisted operation happens
pub async fn record_ai_usage(
    workspace_id: Option<String>,
    org_id: Option<String>,
    metric_name: &str,
    metric_value: Value,
) {
    record_pillar_usage(workspace_id, org_id, Pillar::Ai, metric_name, metric_value).await;
}

/// Record a pillar usage event (internal helper)
async fn record_pillar_usage(
    workspace_id: Option<String>,
    org_id: Option<String>,
    pillar: Pillar,
    metric_name: &str,
    metric_value: Value,
) {
    let db = ANALYTICS_DB.read().await;
    if let Some(recorder) = db.as_ref() {
        let event = PillarUsageEvent {
            workspace_id,
            org_id,
            pillar,
            metric_name: metric_name.to_string(),
            metric_value,
            timestamp: Utc::now(),
        };

        // Record asynchronously without blocking
        let recorder = recorder.clone();
        tokio::spawn(async move {
            if let Err(e) = recorder.record(event).await {
                // Issue #79 — under high load (Srikanth's `--rps 100`
                // for 600s) the analytics DB pool gets saturated and
                // every event spawns a task that times out and logs a
                // WARN. Pillar tracking is best-effort metrics; losing
                // events under load is acceptable, but spamming the log
                // with one WARN per dropped event is not. Demote per-
                // event failures to DEBUG and emit one aggregated WARN
                // at most every FAILURE_WARN_INTERVAL.
                tracing::debug!("Failed to record pillar usage event: {}", e);
                FAILED_EVENT_COUNT.fetch_add(1, Ordering::Relaxed);
                maybe_flush_dropped_warning().await;
            }
        });
    }
}

/// Emit a single aggregated WARN summarising dropped pillar events when
/// at least `FAILURE_WARN_INTERVAL` has elapsed since the last summary.
/// The check is racy by design — under contention we'd rather skip a
/// summary than serialize on a mutex. Counts not surfaced by one race
/// roll into the next interval's summary.
async fn maybe_flush_dropped_warning() {
    let last = *LAST_FAILURE_WARN_AT.read().await;
    if last.elapsed() < FAILURE_WARN_INTERVAL {
        return;
    }
    // Race-aware swap: take the count we'll report, leave the rest for
    // the next interval. Another task may have already flushed — we
    // double-check the timestamp under the write lock and bail if so.
    let mut last_w = LAST_FAILURE_WARN_AT.write().await;
    if last_w.elapsed() < FAILURE_WARN_INTERVAL {
        return;
    }
    let dropped = FAILED_EVENT_COUNT.swap(0, Ordering::Relaxed);
    if dropped > 0 {
        tracing::warn!(
            dropped_events = dropped,
            interval_secs = FAILURE_WARN_INTERVAL.as_secs(),
            "pillar_tracking: dropped events in the last {}s due to analytics-DB pressure \
             (analytics is best-effort; bench / serve behaviour is unaffected)",
            FAILURE_WARN_INTERVAL.as_secs(),
        );
    }
    *last_w = Instant::now();
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct TestRecorder {
        events: Arc<RwLock<Vec<PillarUsageEvent>>>,
    }

    #[async_trait::async_trait]
    impl PillarUsageRecorder for TestRecorder {
        async fn record(
            &self,
            event: PillarUsageEvent,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let mut events = self.events.write().await;
            events.push(event);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_record_reality_usage() {
        let events = Arc::new(RwLock::new(Vec::new()));
        let recorder = Arc::new(TestRecorder {
            events: events.clone(),
        });
        init(recorder).await;

        record_reality_usage(
            Some("workspace-1".to_string()),
            None,
            "blended_reality_ratio",
            json!({"ratio": 0.5}),
        )
        .await;

        // Give async task time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let recorded = events.read().await;
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].pillar, Pillar::Reality);
        assert_eq!(recorded[0].metric_name, "blended_reality_ratio");
    }
}
