//! Metrics and monitoring for MQTT broker operations

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Metrics collector for MQTT broker operations
#[derive(Debug)]
pub struct MqttMetrics {
    /// Total number of connections
    pub connections_total: AtomicU64,
    /// Active connections
    pub connections_active: AtomicU64,
    /// Total messages published
    pub messages_published_total: AtomicU64,
    /// Total messages delivered to subscribers
    pub messages_delivered_total: AtomicU64,
    /// Total subscriptions created
    pub subscriptions_total: AtomicU64,
    /// Active subscriptions
    pub subscriptions_active: AtomicU64,
    /// Total topics
    pub topics_total: AtomicU64,
    /// Total retained messages
    pub retained_messages_total: AtomicU64,
    /// QoS 0 messages
    pub qos0_messages_total: AtomicU64,
    /// QoS 1 messages
    pub qos1_messages_total: AtomicU64,
    /// QoS 2 messages
    pub qos2_messages_total: AtomicU64,
    /// Errors total
    pub errors_total: AtomicU64,
    /// Average message latency in microseconds
    pub message_latency_micros: AtomicU64,
}

impl MqttMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            connections_total: AtomicU64::new(0),
            connections_active: AtomicU64::new(0),
            messages_published_total: AtomicU64::new(0),
            messages_delivered_total: AtomicU64::new(0),
            subscriptions_total: AtomicU64::new(0),
            subscriptions_active: AtomicU64::new(0),
            topics_total: AtomicU64::new(0),
            retained_messages_total: AtomicU64::new(0),
            qos0_messages_total: AtomicU64::new(0),
            qos1_messages_total: AtomicU64::new(0),
            qos2_messages_total: AtomicU64::new(0),
            errors_total: AtomicU64::new(0),
            message_latency_micros: AtomicU64::new(0),
        }
    }

    /// Record a new connection
    pub fn record_connection(&self) {
        self.connections_total.fetch_add(1, Ordering::Relaxed);
        self.connections_active.fetch_add(1, Ordering::Relaxed);
        tracing::debug!("MQTT connection established");
    }

    /// Record a connection closed
    pub fn record_connection_closed(&self) {
        self.connections_active.fetch_sub(1, Ordering::Relaxed);
        tracing::debug!("MQTT connection closed");
    }

    /// Record a message published
    pub fn record_publish(&self, qos: u8) {
        self.messages_published_total.fetch_add(1, Ordering::Relaxed);
        match qos {
            0 => self.qos0_messages_total.fetch_add(1, Ordering::Relaxed),
            1 => self.qos1_messages_total.fetch_add(1, Ordering::Relaxed),
            2 => self.qos2_messages_total.fetch_add(1, Ordering::Relaxed),
            _ => 0,
        };
        tracing::trace!("MQTT message published with QoS {}", qos);
    }

    /// Record a message delivered to subscriber
    pub fn record_delivery(&self) {
        self.messages_delivered_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a subscription created
    pub fn record_subscription(&self) {
        self.subscriptions_total.fetch_add(1, Ordering::Relaxed);
        self.subscriptions_active.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a subscription removed
    pub fn record_unsubscription(&self) {
        self.subscriptions_active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record a topic created
    pub fn record_topic_created(&self) {
        self.topics_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a retained message
    pub fn record_retained_message(&self) {
        self.retained_messages_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an error
    pub fn record_error(&self, error: &str) {
        self.errors_total.fetch_add(1, Ordering::Relaxed);
        tracing::warn!("MQTT error: {}", error);
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
    pub fn snapshot(&self) -> MqttMetricsSnapshot {
        MqttMetricsSnapshot {
            connections_total: self.connections_total.load(Ordering::Relaxed),
            connections_active: self.connections_active.load(Ordering::Relaxed),
            messages_published_total: self.messages_published_total.load(Ordering::Relaxed),
            messages_delivered_total: self.messages_delivered_total.load(Ordering::Relaxed),
            subscriptions_total: self.subscriptions_total.load(Ordering::Relaxed),
            subscriptions_active: self.subscriptions_active.load(Ordering::Relaxed),
            topics_total: self.topics_total.load(Ordering::Relaxed),
            retained_messages_total: self.retained_messages_total.load(Ordering::Relaxed),
            qos0_messages_total: self.qos0_messages_total.load(Ordering::Relaxed),
            qos1_messages_total: self.qos1_messages_total.load(Ordering::Relaxed),
            qos2_messages_total: self.qos2_messages_total.load(Ordering::Relaxed),
            errors_total: self.errors_total.load(Ordering::Relaxed),
            avg_message_latency_micros: self.message_latency_micros.load(Ordering::Relaxed),
        }
    }
}

impl Default for MqttMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of MQTT metrics at a point in time
#[derive(Debug, Clone)]
pub struct MqttMetricsSnapshot {
    pub connections_total: u64,
    pub connections_active: u64,
    pub messages_published_total: u64,
    pub messages_delivered_total: u64,
    pub subscriptions_total: u64,
    pub subscriptions_active: u64,
    pub topics_total: u64,
    pub retained_messages_total: u64,
    pub qos0_messages_total: u64,
    pub qos1_messages_total: u64,
    pub qos2_messages_total: u64,
    pub errors_total: u64,
    pub avg_message_latency_micros: u64,
}

/// Metrics exporter for Prometheus-style metrics
pub struct MqttMetricsExporter {
    metrics: Arc<MqttMetrics>,
}

impl MqttMetricsExporter {
    /// Create a new metrics exporter
    pub fn new(metrics: Arc<MqttMetrics>) -> Self {
        Self { metrics }
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let snapshot = self.metrics.snapshot();

        format!(
            "# HELP mqtt_connections_total Total number of MQTT connections\n\
             # TYPE mqtt_connections_total counter\n\
             mqtt_connections_total {}\n\
             # HELP mqtt_connections_active Number of active MQTT connections\n\
             # TYPE mqtt_connections_active gauge\n\
             mqtt_connections_active {}\n\
             # HELP mqtt_messages_published_total Total messages published\n\
             # TYPE mqtt_messages_published_total counter\n\
             mqtt_messages_published_total {}\n\
             # HELP mqtt_messages_delivered_total Total messages delivered\n\
             # TYPE mqtt_messages_delivered_total counter\n\
             mqtt_messages_delivered_total {}\n\
             # HELP mqtt_subscriptions_total Total subscriptions created\n\
             # TYPE mqtt_subscriptions_total counter\n\
             mqtt_subscriptions_total {}\n\
             # HELP mqtt_subscriptions_active Active subscriptions\n\
             # TYPE mqtt_subscriptions_active gauge\n\
             mqtt_subscriptions_active {}\n\
             # HELP mqtt_topics_total Total topics\n\
             # TYPE mqtt_topics_total gauge\n\
             mqtt_topics_total {}\n\
             # HELP mqtt_retained_messages_total Total retained messages\n\
             # TYPE mqtt_retained_messages_total gauge\n\
             mqtt_retained_messages_total {}\n\
             # HELP mqtt_qos0_messages_total QoS 0 messages\n\
             # TYPE mqtt_qos0_messages_total counter\n\
             mqtt_qos0_messages_total {}\n\
             # HELP mqtt_qos1_messages_total QoS 1 messages\n\
             # TYPE mqtt_qos1_messages_total counter\n\
             mqtt_qos1_messages_total {}\n\
             # HELP mqtt_qos2_messages_total QoS 2 messages\n\
             # TYPE mqtt_qos2_messages_total counter\n\
             mqtt_qos2_messages_total {}\n\
             # HELP mqtt_errors_total Total errors\n\
             # TYPE mqtt_errors_total counter\n\
             mqtt_errors_total {}\n\
             # HELP mqtt_message_latency_micros_avg Average message latency\n\
             # TYPE mqtt_message_latency_micros_avg gauge\n\
             mqtt_message_latency_micros_avg {}\n",
            snapshot.connections_total,
            snapshot.connections_active,
            snapshot.messages_published_total,
            snapshot.messages_delivered_total,
            snapshot.subscriptions_total,
            snapshot.subscriptions_active,
            snapshot.topics_total,
            snapshot.retained_messages_total,
            snapshot.qos0_messages_total,
            snapshot.qos1_messages_total,
            snapshot.qos2_messages_total,
            snapshot.errors_total,
            snapshot.avg_message_latency_micros
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mqtt_metrics_new() {
        let metrics = MqttMetrics::new();
        assert_eq!(metrics.connections_total.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.connections_active.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_record_connection() {
        let metrics = MqttMetrics::new();
        metrics.record_connection();
        assert_eq!(metrics.connections_total.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.connections_active.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_record_connection_closed() {
        let metrics = MqttMetrics::new();
        metrics.record_connection();
        metrics.record_connection_closed();
        assert_eq!(metrics.connections_total.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.connections_active.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_record_publish_qos() {
        let metrics = MqttMetrics::new();
        metrics.record_publish(0);
        metrics.record_publish(1);
        metrics.record_publish(2);
        metrics.record_publish(0);

        assert_eq!(metrics.messages_published_total.load(Ordering::Relaxed), 4);
        assert_eq!(metrics.qos0_messages_total.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.qos1_messages_total.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.qos2_messages_total.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_record_subscription() {
        let metrics = MqttMetrics::new();
        metrics.record_subscription();
        metrics.record_subscription();
        metrics.record_unsubscription();

        assert_eq!(metrics.subscriptions_total.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.subscriptions_active.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_snapshot() {
        let metrics = MqttMetrics::new();
        metrics.record_connection();
        metrics.record_publish(1);
        metrics.record_subscription();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.connections_total, 1);
        assert_eq!(snapshot.messages_published_total, 1);
        assert_eq!(snapshot.subscriptions_total, 1);
    }

    #[test]
    fn test_prometheus_export() {
        let metrics = Arc::new(MqttMetrics::new());
        metrics.record_connection();
        metrics.record_publish(0);

        let exporter = MqttMetricsExporter::new(metrics);
        let output = exporter.export_prometheus();

        assert!(output.contains("mqtt_connections_total 1"));
        assert!(output.contains("mqtt_messages_published_total 1"));
        assert!(output.contains("# HELP"));
        assert!(output.contains("# TYPE"));
    }
}
