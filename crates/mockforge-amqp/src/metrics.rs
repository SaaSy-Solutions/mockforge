//! Metrics and monitoring for AMQP broker operations

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Metrics collector for AMQP broker operations
#[derive(Debug)]
pub struct AmqpMetrics {
    /// Total number of connections
    pub connections_total: AtomicU64,
    /// Active connections
    pub connections_active: AtomicU64,
    /// Total channels opened
    pub channels_total: AtomicU64,
    /// Active channels
    pub channels_active: AtomicU64,
    /// Total messages published
    pub messages_published_total: AtomicU64,
    /// Total messages consumed
    pub messages_consumed_total: AtomicU64,
    /// Total messages acknowledged
    pub messages_acked_total: AtomicU64,
    /// Total messages rejected/nacked
    pub messages_rejected_total: AtomicU64,
    /// Total queues declared
    pub queues_total: AtomicU64,
    /// Active queues
    pub queues_active: AtomicU64,
    /// Total exchanges declared
    pub exchanges_total: AtomicU64,
    /// Active exchanges
    pub exchanges_active: AtomicU64,
    /// Total bindings created
    pub bindings_total: AtomicU64,
    /// Errors total
    pub errors_total: AtomicU64,
    /// Average message latency in microseconds
    pub message_latency_micros: AtomicU64,
}

impl AmqpMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            connections_total: AtomicU64::new(0),
            connections_active: AtomicU64::new(0),
            channels_total: AtomicU64::new(0),
            channels_active: AtomicU64::new(0),
            messages_published_total: AtomicU64::new(0),
            messages_consumed_total: AtomicU64::new(0),
            messages_acked_total: AtomicU64::new(0),
            messages_rejected_total: AtomicU64::new(0),
            queues_total: AtomicU64::new(0),
            queues_active: AtomicU64::new(0),
            exchanges_total: AtomicU64::new(0),
            exchanges_active: AtomicU64::new(0),
            bindings_total: AtomicU64::new(0),
            errors_total: AtomicU64::new(0),
            message_latency_micros: AtomicU64::new(0),
        }
    }

    /// Record a new connection
    pub fn record_connection(&self) {
        self.connections_total.fetch_add(1, Ordering::Relaxed);
        self.connections_active.fetch_add(1, Ordering::Relaxed);
        tracing::debug!("AMQP connection established");
    }

    /// Record a connection closed
    pub fn record_connection_closed(&self) {
        self.connections_active.fetch_sub(1, Ordering::Relaxed);
        tracing::debug!("AMQP connection closed");
    }

    /// Record a channel opened
    pub fn record_channel_opened(&self) {
        self.channels_total.fetch_add(1, Ordering::Relaxed);
        self.channels_active.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a channel closed
    pub fn record_channel_closed(&self) {
        self.channels_active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record a message published
    pub fn record_publish(&self) {
        self.messages_published_total.fetch_add(1, Ordering::Relaxed);
        tracing::trace!("AMQP message published");
    }

    /// Record a message consumed
    pub fn record_consume(&self) {
        self.messages_consumed_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a message acknowledged
    pub fn record_ack(&self) {
        self.messages_acked_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a message rejected/nacked
    pub fn record_reject(&self) {
        self.messages_rejected_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a queue declared
    pub fn record_queue_declared(&self) {
        self.queues_total.fetch_add(1, Ordering::Relaxed);
        self.queues_active.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a queue deleted
    pub fn record_queue_deleted(&self) {
        self.queues_active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record an exchange declared
    pub fn record_exchange_declared(&self) {
        self.exchanges_total.fetch_add(1, Ordering::Relaxed);
        self.exchanges_active.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an exchange deleted
    pub fn record_exchange_deleted(&self) {
        self.exchanges_active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record a binding created
    pub fn record_binding(&self) {
        self.bindings_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an error
    pub fn record_error(&self, error: &str) {
        self.errors_total.fetch_add(1, Ordering::Relaxed);
        tracing::warn!("AMQP error: {}", error);
    }

    /// Record message latency
    pub fn record_latency(&self, latency_micros: u64) {
        let current = self.message_latency_micros.load(Ordering::Relaxed);
        let new_avg = if current == 0 {
            latency_micros
        } else {
            (current + latency_micros) / 2
        };
        self.message_latency_micros.store(new_avg, Ordering::Relaxed);
    }

    /// Get metrics snapshot
    pub fn snapshot(&self) -> AmqpMetricsSnapshot {
        AmqpMetricsSnapshot {
            connections_total: self.connections_total.load(Ordering::Relaxed),
            connections_active: self.connections_active.load(Ordering::Relaxed),
            channels_total: self.channels_total.load(Ordering::Relaxed),
            channels_active: self.channels_active.load(Ordering::Relaxed),
            messages_published_total: self.messages_published_total.load(Ordering::Relaxed),
            messages_consumed_total: self.messages_consumed_total.load(Ordering::Relaxed),
            messages_acked_total: self.messages_acked_total.load(Ordering::Relaxed),
            messages_rejected_total: self.messages_rejected_total.load(Ordering::Relaxed),
            queues_total: self.queues_total.load(Ordering::Relaxed),
            queues_active: self.queues_active.load(Ordering::Relaxed),
            exchanges_total: self.exchanges_total.load(Ordering::Relaxed),
            exchanges_active: self.exchanges_active.load(Ordering::Relaxed),
            bindings_total: self.bindings_total.load(Ordering::Relaxed),
            errors_total: self.errors_total.load(Ordering::Relaxed),
            avg_message_latency_micros: self.message_latency_micros.load(Ordering::Relaxed),
        }
    }
}

impl Default for AmqpMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of AMQP metrics at a point in time
#[derive(Debug, Clone)]
pub struct AmqpMetricsSnapshot {
    pub connections_total: u64,
    pub connections_active: u64,
    pub channels_total: u64,
    pub channels_active: u64,
    pub messages_published_total: u64,
    pub messages_consumed_total: u64,
    pub messages_acked_total: u64,
    pub messages_rejected_total: u64,
    pub queues_total: u64,
    pub queues_active: u64,
    pub exchanges_total: u64,
    pub exchanges_active: u64,
    pub bindings_total: u64,
    pub errors_total: u64,
    pub avg_message_latency_micros: u64,
}

/// Metrics exporter for Prometheus-style metrics
pub struct AmqpMetricsExporter {
    metrics: Arc<AmqpMetrics>,
}

impl AmqpMetricsExporter {
    /// Create a new metrics exporter
    pub fn new(metrics: Arc<AmqpMetrics>) -> Self {
        Self { metrics }
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let snapshot = self.metrics.snapshot();

        format!(
            "# HELP amqp_connections_total Total AMQP connections\n\
             # TYPE amqp_connections_total counter\n\
             amqp_connections_total {}\n\
             # HELP amqp_connections_active Active AMQP connections\n\
             # TYPE amqp_connections_active gauge\n\
             amqp_connections_active {}\n\
             # HELP amqp_channels_total Total channels opened\n\
             # TYPE amqp_channels_total counter\n\
             amqp_channels_total {}\n\
             # HELP amqp_channels_active Active channels\n\
             # TYPE amqp_channels_active gauge\n\
             amqp_channels_active {}\n\
             # HELP amqp_messages_published_total Total messages published\n\
             # TYPE amqp_messages_published_total counter\n\
             amqp_messages_published_total {}\n\
             # HELP amqp_messages_consumed_total Total messages consumed\n\
             # TYPE amqp_messages_consumed_total counter\n\
             amqp_messages_consumed_total {}\n\
             # HELP amqp_messages_acked_total Total messages acknowledged\n\
             # TYPE amqp_messages_acked_total counter\n\
             amqp_messages_acked_total {}\n\
             # HELP amqp_messages_rejected_total Total messages rejected\n\
             # TYPE amqp_messages_rejected_total counter\n\
             amqp_messages_rejected_total {}\n\
             # HELP amqp_queues_total Total queues declared\n\
             # TYPE amqp_queues_total counter\n\
             amqp_queues_total {}\n\
             # HELP amqp_queues_active Active queues\n\
             # TYPE amqp_queues_active gauge\n\
             amqp_queues_active {}\n\
             # HELP amqp_exchanges_total Total exchanges declared\n\
             # TYPE amqp_exchanges_total counter\n\
             amqp_exchanges_total {}\n\
             # HELP amqp_exchanges_active Active exchanges\n\
             # TYPE amqp_exchanges_active gauge\n\
             amqp_exchanges_active {}\n\
             # HELP amqp_bindings_total Total bindings created\n\
             # TYPE amqp_bindings_total counter\n\
             amqp_bindings_total {}\n\
             # HELP amqp_errors_total Total errors\n\
             # TYPE amqp_errors_total counter\n\
             amqp_errors_total {}\n\
             # HELP amqp_message_latency_micros_avg Average message latency\n\
             # TYPE amqp_message_latency_micros_avg gauge\n\
             amqp_message_latency_micros_avg {}\n",
            snapshot.connections_total,
            snapshot.connections_active,
            snapshot.channels_total,
            snapshot.channels_active,
            snapshot.messages_published_total,
            snapshot.messages_consumed_total,
            snapshot.messages_acked_total,
            snapshot.messages_rejected_total,
            snapshot.queues_total,
            snapshot.queues_active,
            snapshot.exchanges_total,
            snapshot.exchanges_active,
            snapshot.bindings_total,
            snapshot.errors_total,
            snapshot.avg_message_latency_micros
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amqp_metrics_new() {
        let metrics = AmqpMetrics::new();
        assert_eq!(metrics.connections_total.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.channels_total.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_record_connection() {
        let metrics = AmqpMetrics::new();
        metrics.record_connection();
        assert_eq!(metrics.connections_total.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.connections_active.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_record_channel() {
        let metrics = AmqpMetrics::new();
        metrics.record_channel_opened();
        metrics.record_channel_opened();
        metrics.record_channel_closed();
        assert_eq!(metrics.channels_total.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.channels_active.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_record_messages() {
        let metrics = AmqpMetrics::new();
        metrics.record_publish();
        metrics.record_consume();
        metrics.record_ack();
        metrics.record_reject();

        assert_eq!(metrics.messages_published_total.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.messages_consumed_total.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.messages_acked_total.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.messages_rejected_total.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_snapshot() {
        let metrics = AmqpMetrics::new();
        metrics.record_connection();
        metrics.record_channel_opened();
        metrics.record_publish();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.connections_total, 1);
        assert_eq!(snapshot.channels_total, 1);
        assert_eq!(snapshot.messages_published_total, 1);
    }

    #[test]
    fn test_prometheus_export() {
        let metrics = Arc::new(AmqpMetrics::new());
        metrics.record_connection();
        metrics.record_publish();

        let exporter = AmqpMetricsExporter::new(metrics);
        let output = exporter.export_prometheus();

        assert!(output.contains("amqp_connections_total 1"));
        assert!(output.contains("amqp_messages_published_total 1"));
        assert!(output.contains("# HELP"));
        assert!(output.contains("# TYPE"));
    }
}
