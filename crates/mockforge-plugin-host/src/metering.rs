//! Per-invocation metric exporter — wires the
//! [`InvocationMetricsBus`] from `mockforge-plugin-loader` to a
//! cloud aggregator over HTTP.
//!
//! Closes the metering loop the bus was designed for in PR #388:
//! every plugin invocation produces an [`InvocationMetric`], the
//! exporter batches them, and every `flush_interval` POSTs the
//! batch as JSON to a configured endpoint.
//!
//! ## Why JSON, not OTLP-protobuf
//!
//! The aggregator is in a sibling MockForge codebase — we control
//! both sides, so a stable JSON shape is enough for v1 and avoids
//! pulling the full OpenTelemetry SDK (heavy dep tree, slow build).
//! When we want to feed third-party collectors (Tempo, Honeycomb,
//! Datadog) that accept OTLP natively, swap this module to emit
//! OTLP/HTTP without changing the bus contract.
//!
//! ## Backpressure
//!
//! The exporter subscribes to a `tokio::sync::broadcast`. If the
//! exporter falls behind (slow upstream, network partition), the
//! broadcast drops oldest events and surfaces `RecvError::Lagged`.
//! That's correct: a hosted-mock should never block its request
//! path because the metric pipeline is slow. We log lag events
//! so operators can size the channel up if they recur.

use std::time::Duration;

use mockforge_plugin_loader::{InvocationMetric, InvocationMetricsBus, InvocationStatus};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Configuration for the exporter.
#[derive(Debug, Clone)]
pub struct ExporterConfig {
    /// HTTP endpoint that accepts the JSON batch. Cloud production
    /// points this at the registry's `/api/internal/plugin-metrics`
    /// receiver.
    pub url: String,
    /// How long to buffer events before flushing. Default 10 s
    /// balances per-invocation accuracy against HTTP overhead.
    pub flush_interval: Duration,
    /// Hard cap on the in-memory queue. If we accumulate more than
    /// this many events between flushes, we flush immediately to
    /// avoid an unbounded memory growth (e.g. during a downstream
    /// outage that recovered partially).
    pub max_queue_size: usize,
    /// Optional bearer token; sent as `Authorization: Bearer ...`.
    /// Cloud production sets this so the registry can rate-limit
    /// per host.
    pub bearer_token: Option<String>,
}

impl ExporterConfig {
    /// Construct with sensible defaults.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            flush_interval: Duration::from_secs(10),
            max_queue_size: 1024,
            bearer_token: None,
        }
    }
}

/// JSON shape posted to the aggregator. Wire-format-stable —
/// adding fields is fine, renaming or removing is breaking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedMetric {
    /// Plugin id (matches `PluginId::to_string()`).
    pub plugin_id: String,
    /// Exported function the host called.
    pub function_name: String,
    /// Wall-clock start time in RFC 3339.
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Wall-time spent inside the plugin call, microseconds.
    pub wall_time_us: u64,
    /// Peak memory usage observed during this invocation, bytes.
    pub memory_peak_bytes: u64,
    /// Outcome: `success`, `failure`, or `dropped`.
    pub status: &'static str,
    /// Failure error message, if `status = "failure"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl From<InvocationMetric> for ExportedMetric {
    fn from(m: InvocationMetric) -> Self {
        let (status, error) = match &m.status {
            InvocationStatus::Success => ("success", None),
            InvocationStatus::Failure { error } => ("failure", Some(error.clone())),
            InvocationStatus::Dropped => ("dropped", None),
        };
        Self {
            plugin_id: m.plugin_id.to_string(),
            function_name: m.function_name,
            started_at: m.started_at,
            wall_time_us: m.wall_time_us,
            memory_peak_bytes: m.memory_peak_bytes,
            status,
            error,
        }
    }
}

/// Run the exporter forever. Returns only on cancellation by the
/// outer `select!`. Subscribes to the bus, batches events, posts
/// every `flush_interval` (or sooner if the queue fills).
pub async fn run_exporter(config: ExporterConfig, bus: InvocationMetricsBus) {
    let client = match reqwest::Client::builder().timeout(Duration::from_secs(5)).build() {
        Ok(c) => c,
        Err(err) => {
            tracing::error!(error = %err, "failed to build metric-exporter HTTP client; exporter exiting");
            return;
        }
    };

    let mut rx = bus.subscribe();
    let mut buffer: Vec<ExportedMetric> = Vec::with_capacity(64);
    let mut ticker = tokio::time::interval(config.flush_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    tracing::info!(
        url = %config.url,
        flush_interval_secs = config.flush_interval.as_secs(),
        max_queue_size = config.max_queue_size,
        "metric exporter starting"
    );

    loop {
        tokio::select! {
            recv_result = rx.recv() => {
                match recv_result {
                    Ok(metric) => {
                        buffer.push(metric.into());
                        if buffer.len() >= config.max_queue_size {
                            flush(&client, &config, &mut buffer).await;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        // Slow exporter, fast producers. The broadcast
                        // dropped `skipped` events; log so operators can
                        // size up the channel if this recurs, but keep
                        // running.
                        tracing::warn!(skipped, "metric exporter lagged; events dropped");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::info!("metric bus closed; exporter exiting");
                        // Final flush so we don't drop in-buffer events.
                        if !buffer.is_empty() {
                            flush(&client, &config, &mut buffer).await;
                        }
                        return;
                    }
                }
            }
            _ = ticker.tick() => {
                if !buffer.is_empty() {
                    flush(&client, &config, &mut buffer).await;
                }
            }
        }
    }
}

/// POST the buffered batch and clear it. On HTTP failure we **drop
/// the batch** rather than retry — retrying would create unbounded
/// memory pressure during a downstream outage. The aggregator is
/// best-effort; a missed batch is a metering gap, not a security
/// issue. Operators see the gap as a flat stretch on dashboards.
async fn flush(
    client: &reqwest::Client,
    config: &ExporterConfig,
    buffer: &mut Vec<ExportedMetric>,
) {
    let batch = std::mem::take(buffer);
    let count = batch.len();

    let mut req = client.post(&config.url).json(&batch);
    if let Some(token) = &config.bearer_token {
        req = req.bearer_auth(token);
    }

    match req.send().await {
        Ok(response) if response.status().is_success() => {
            tracing::debug!(count, status = %response.status(), "metric batch flushed");
        }
        Ok(response) => {
            tracing::warn!(
                count,
                status = %response.status(),
                "metric exporter got non-success response; batch dropped"
            );
        }
        Err(err) => {
            tracing::warn!(count, error = %err, "metric exporter HTTP send failed; batch dropped");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use mockforge_plugin_core::PluginId;

    fn make_metric(status: InvocationStatus) -> InvocationMetric {
        InvocationMetric {
            plugin_id: PluginId::new("test-plugin"),
            function_name: "on_request".into(),
            started_at: Utc::now(),
            wall_time_us: 1234,
            memory_peak_bytes: 5678,
            status,
        }
    }

    #[test]
    fn invocation_metric_maps_to_exported_metric_success() {
        let exported: ExportedMetric = make_metric(InvocationStatus::Success).into();
        assert_eq!(exported.plugin_id, "test-plugin");
        assert_eq!(exported.function_name, "on_request");
        assert_eq!(exported.wall_time_us, 1234);
        assert_eq!(exported.memory_peak_bytes, 5678);
        assert_eq!(exported.status, "success");
        assert!(exported.error.is_none());
    }

    #[test]
    fn invocation_metric_maps_failure_to_error_field() {
        let exported: ExportedMetric = make_metric(InvocationStatus::Failure {
            error: "boom".into(),
        })
        .into();
        assert_eq!(exported.status, "failure");
        assert_eq!(exported.error.as_deref(), Some("boom"));
    }

    #[test]
    fn invocation_metric_maps_dropped_to_status_only() {
        let exported: ExportedMetric = make_metric(InvocationStatus::Dropped).into();
        assert_eq!(exported.status, "dropped");
        assert!(exported.error.is_none());
    }

    #[test]
    fn exported_metric_serializes_with_omitted_error_on_success() {
        let exported: ExportedMetric = make_metric(InvocationStatus::Success).into();
        let json = serde_json::to_string(&exported).unwrap();
        assert!(json.contains("\"status\":\"success\""));
        assert!(!json.contains("\"error\""), "success should omit error field, got {}", json);
    }

    #[test]
    fn config_defaults_to_10s_flush() {
        let cfg = ExporterConfig::new("http://example/metrics");
        assert_eq!(cfg.flush_interval.as_secs(), 10);
        assert_eq!(cfg.max_queue_size, 1024);
        assert!(cfg.bearer_token.is_none());
    }

    // NOTE: a previous version of this test tried to drop the bus
    // and assert the exporter exits. That doesn't work: the
    // exporter holds its own `InvocationMetricsBus` clone (which
    // owns a `broadcast::Sender`), so dropping the test's clone
    // never closes the channel. The exporter's actual shutdown
    // path is the outer `tokio::select!` in `main.rs` cancelling
    // the future, not bus closure. We test the bus-close path
    // implicitly via the production wiring.
}
