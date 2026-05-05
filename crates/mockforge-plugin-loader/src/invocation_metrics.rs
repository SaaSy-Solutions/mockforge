//! Per-invocation metrics for plugin execution.
//!
//! Each call into a plugin emits an [`InvocationMetric`] on a
//! broadcast channel ([`InvocationMetricsBus`]). Observers — the admin
//! UI's plugin status page, an OTLP exporter for cloud metering, a
//! billing aggregator — subscribe and consume.
//!
//! This is the foundation for cloud-plugins Phase 2 metering: the bus
//! shape is the same whether we're emitting locally for the OSS admin
//! UI or shipping events to a SaaS billing pipeline. Local first,
//! cloud second.
//!
//! # Why a broadcast channel
//!
//! Multiple subscribers each get every event without coordinating.
//! Slow consumers don't slow down the producer — `tokio::sync::broadcast`
//! drops messages for laggers and surfaces it as a `RecvError::Lagged`,
//! which is the right semantics for telemetry (better to miss a sample
//! than to backpressure the request path).
//!
//! # Why RAII for the timer
//!
//! The producer side is wrapped in [`InvocationTimer`], which records
//! the start instant on construction and emits the metric on `finish_*`.
//! Forgetting to finish a timer is a (warned) bug; the `Drop` impl emits
//! a "dropped" metric so we don't silently lose calls.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use mockforge_plugin_core::PluginId;
use tokio::sync::broadcast::{self, Receiver, Sender};

/// Default capacity for the broadcast channel. 256 is generous for a
/// single mockforge process — laggers up to ~256 events behind still
/// receive; beyond that the receiver gets `RecvError::Lagged`.
const DEFAULT_CHANNEL_CAPACITY: usize = 256;

/// One plugin invocation's measured cost + outcome.
#[derive(Debug, Clone)]
pub struct InvocationMetric {
    /// The plugin whose function was invoked.
    pub plugin_id: PluginId,
    /// The exported function the host called. For most plugins this is
    /// `"on_request"` / `"on_response"` / similar.
    pub function_name: String,
    /// Wall-clock start, useful for correlation with request traces.
    pub started_at: DateTime<Utc>,
    /// Wall-time spent inside the plugin call. Microseconds for fine
    /// resolution; sub-millisecond invocations are common for transform
    /// plugins.
    pub wall_time_us: u64,
    /// Peak memory usage observed during this invocation, in bytes.
    /// Reported as 0 when the underlying tracker isn't wired up
    /// (current state — see `memory_tracking.rs` for the limiter that
    /// will populate this in cloud Phase 2).
    pub memory_peak_bytes: u64,
    /// Outcome.
    pub status: InvocationStatus,
}

/// Outcome of a plugin invocation, attached to each [`InvocationMetric`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvocationStatus {
    /// The plugin function returned normally.
    Success,
    /// The plugin function returned an error or trapped.
    Failure {
        /// Error message captured from the failed invocation.
        error: String,
    },
    /// The timer was dropped without `finish_*` being called. Indicates
    /// a host-side bug — the call path didn't see the work through.
    Dropped,
}

/// Broadcast bus that fans an [`InvocationMetric`] out to subscribers.
///
/// Construct one per [`PluginSandbox`] and clone the `Arc` to pass it
/// into each [`SandboxInstance`]. External code calls [`subscribe`] to
/// get a [`Receiver`] for live events.
///
/// [`PluginSandbox`]: crate::sandbox::PluginSandbox
/// [`SandboxInstance`]: crate::sandbox::SandboxInstance
/// [`subscribe`]: InvocationMetricsBus::subscribe
#[derive(Debug, Clone)]
pub struct InvocationMetricsBus {
    tx: Sender<InvocationMetric>,
}

impl InvocationMetricsBus {
    /// Construct a bus with the default channel capacity
    /// ([`DEFAULT_CHANNEL_CAPACITY`]).
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CHANNEL_CAPACITY)
    }

    /// Construct a bus with a custom channel capacity. Larger values
    /// tolerate slower subscribers without dropping events; smaller
    /// values use less memory per bus.
    pub fn with_capacity(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Subscribe to live metrics. Each subscriber sees every event sent
    /// after the call to `subscribe`; events emitted before are not
    /// replayed.
    pub fn subscribe(&self) -> Receiver<InvocationMetric> {
        self.tx.subscribe()
    }

    /// Send a metric. Non-blocking. If the channel is full, the oldest
    /// undelivered event is dropped (broadcast semantics) — that
    /// subscriber gets `RecvError::Lagged` on the next `recv`. If
    /// nobody is subscribed, the metric is silently dropped.
    pub fn record(&self, metric: InvocationMetric) {
        // `send` returns `Err` if there are no subscribers. That's
        // expected (e.g. self-hosted user without admin UI open) — not
        // an error, so we ignore it.
        let _ = self.tx.send(metric);
    }

    /// Current number of subscribers. Useful for tests and for skipping
    /// metric construction when nobody's listening.
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for InvocationMetricsBus {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard that times a plugin invocation and emits one
/// [`InvocationMetric`] when finished.
///
/// Construct with [`start`], then call exactly one of
/// [`finish_success`] or [`finish_failure`]. Dropping the timer
/// without calling `finish_*` emits a [`InvocationStatus::Dropped`]
/// metric — this is a bug, but a visible one rather than a silent
/// omission.
///
/// [`start`]: InvocationTimer::start
/// [`finish_success`]: InvocationTimer::finish_success
/// [`finish_failure`]: InvocationTimer::finish_failure
pub struct InvocationTimer {
    bus: Arc<InvocationMetricsBus>,
    plugin_id: PluginId,
    function_name: String,
    started_at: DateTime<Utc>,
    started_instant: std::time::Instant,
    /// Set to `true` when `finish_*` consumes the timer, so the `Drop`
    /// impl knows to skip the dropped-metric emit.
    finished: bool,
}

impl InvocationTimer {
    /// Start timing an invocation. The wall-clock and `Instant` are
    /// captured here; nothing is sent on the bus until `finish_*`.
    pub fn start(
        bus: Arc<InvocationMetricsBus>,
        plugin_id: PluginId,
        function_name: impl Into<String>,
    ) -> Self {
        Self {
            bus,
            plugin_id,
            function_name: function_name.into(),
            started_at: Utc::now(),
            started_instant: std::time::Instant::now(),
            finished: false,
        }
    }

    /// Emit a successful invocation metric.
    pub fn finish_success(mut self, memory_peak_bytes: u64) {
        self.emit(InvocationStatus::Success, memory_peak_bytes);
    }

    /// Emit a failed invocation metric.
    pub fn finish_failure(mut self, error: impl Into<String>, memory_peak_bytes: u64) {
        self.emit(
            InvocationStatus::Failure {
                error: error.into(),
            },
            memory_peak_bytes,
        );
    }

    fn emit(&mut self, status: InvocationStatus, memory_peak_bytes: u64) {
        self.finished = true;
        let wall_time_us = self.started_instant.elapsed().as_micros().min(u64::MAX as u128) as u64;
        let metric = InvocationMetric {
            plugin_id: self.plugin_id.clone(),
            function_name: std::mem::take(&mut self.function_name),
            started_at: self.started_at,
            wall_time_us,
            memory_peak_bytes,
            status,
        };
        self.bus.record(metric);
    }
}

impl Drop for InvocationTimer {
    fn drop(&mut self) {
        if !self.finished {
            tracing::warn!(
                plugin_id = %self.plugin_id,
                function_name = %self.function_name,
                "InvocationTimer dropped without finish_* — emitting Dropped metric"
            );
            // Re-use `emit`. We have to provide a memory_peak; we don't
            // know it at drop time, so 0 is the honest value.
            self.emit(InvocationStatus::Dropped, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn test_plugin_id() -> PluginId {
        PluginId::new("test-plugin")
    }

    #[tokio::test]
    async fn record_with_no_subscribers_is_silent() {
        let bus = InvocationMetricsBus::new();
        assert_eq!(bus.subscriber_count(), 0);

        // Should not panic, should not error.
        bus.record(InvocationMetric {
            plugin_id: test_plugin_id(),
            function_name: "fn1".into(),
            started_at: Utc::now(),
            wall_time_us: 100,
            memory_peak_bytes: 0,
            status: InvocationStatus::Success,
        });
    }

    #[tokio::test]
    async fn subscribe_then_receive() {
        let bus = InvocationMetricsBus::new();
        let mut rx = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 1);

        bus.record(InvocationMetric {
            plugin_id: test_plugin_id(),
            function_name: "fn1".into(),
            started_at: Utc::now(),
            wall_time_us: 42,
            memory_peak_bytes: 0,
            status: InvocationStatus::Success,
        });

        let received = rx.recv().await.unwrap();
        assert_eq!(received.function_name, "fn1");
        assert_eq!(received.wall_time_us, 42);
        assert_eq!(received.status, InvocationStatus::Success);
    }

    #[tokio::test]
    async fn multiple_subscribers_each_get_every_event() {
        let bus = InvocationMetricsBus::new();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 2);

        bus.record(InvocationMetric {
            plugin_id: test_plugin_id(),
            function_name: "fn-broadcast".into(),
            started_at: Utc::now(),
            wall_time_us: 7,
            memory_peak_bytes: 0,
            status: InvocationStatus::Success,
        });

        let m1 = rx1.recv().await.unwrap();
        let m2 = rx2.recv().await.unwrap();
        assert_eq!(m1.function_name, "fn-broadcast");
        assert_eq!(m2.function_name, "fn-broadcast");
    }

    #[tokio::test]
    async fn timer_finish_success_emits_metric() {
        let bus = Arc::new(InvocationMetricsBus::new());
        let mut rx = bus.subscribe();

        let timer = InvocationTimer::start(bus.clone(), test_plugin_id(), "do_thing");
        // Sleep a tiny amount so wall_time_us is meaningfully > 0.
        tokio::time::sleep(Duration::from_millis(2)).await;
        timer.finish_success(1024);

        let metric = rx.recv().await.unwrap();
        assert_eq!(metric.function_name, "do_thing");
        assert_eq!(metric.status, InvocationStatus::Success);
        assert_eq!(metric.memory_peak_bytes, 1024);
        assert!(metric.wall_time_us >= 1_000, "expected ≥1ms, got {}us", metric.wall_time_us);
    }

    #[tokio::test]
    async fn timer_finish_failure_includes_error() {
        let bus = Arc::new(InvocationMetricsBus::new());
        let mut rx = bus.subscribe();

        let timer = InvocationTimer::start(bus.clone(), test_plugin_id(), "do_thing");
        timer.finish_failure("boom", 0);

        let metric = rx.recv().await.unwrap();
        match metric.status {
            InvocationStatus::Failure { error } => assert_eq!(error, "boom"),
            other => panic!("expected Failure, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn timer_dropped_without_finish_emits_dropped_metric() {
        let bus = Arc::new(InvocationMetricsBus::new());
        let mut rx = bus.subscribe();

        {
            let _timer = InvocationTimer::start(bus.clone(), test_plugin_id(), "leaked");
            // Dropped here without calling finish_*.
        }

        let metric = rx.recv().await.unwrap();
        assert_eq!(metric.function_name, "leaked");
        assert_eq!(metric.status, InvocationStatus::Dropped);
    }

    #[tokio::test]
    async fn started_at_is_set_at_start_not_finish() {
        let bus = Arc::new(InvocationMetricsBus::new());
        let mut rx = bus.subscribe();

        let timer = InvocationTimer::start(bus.clone(), test_plugin_id(), "fn");
        let started = timer.started_at;

        tokio::time::sleep(Duration::from_millis(5)).await;
        timer.finish_success(0);

        let metric = rx.recv().await.unwrap();
        assert_eq!(metric.started_at, started);
        let elapsed_via_metric = Utc::now()
            .signed_duration_since(metric.started_at)
            .num_microseconds()
            .unwrap_or(i64::MAX);
        assert!(elapsed_via_metric >= 5_000);
    }
}
