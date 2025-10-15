//! Metrics and monitoring for Kafka broker operations

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Metrics collector for Kafka broker operations
#[derive(Debug)]
pub struct KafkaMetrics {
    /// Total number of connections
    pub connections_total: AtomicU64,
    /// Active connections
    pub connections_active: AtomicU64,
    /// Total requests received
    pub requests_total: AtomicU64,
    /// Requests by API key
    pub requests_by_api: HashMap<i16, AtomicU64>,
    /// Total responses sent
    pub responses_total: AtomicU64,
    /// Total messages produced
    pub messages_produced_total: AtomicU64,
    /// Total messages consumed
    pub messages_consumed_total: AtomicU64,
    /// Total topics created
    pub topics_created_total: AtomicU64,
    /// Total topics deleted
    pub topics_deleted_total: AtomicU64,
    /// Total consumer groups
    pub consumer_groups_total: AtomicU64,
    /// Total partitions
    pub partitions_total: AtomicU64,
    /// Request latency (in microseconds)
    pub request_latency_micros: AtomicU64,
    /// Error responses
    pub errors_total: AtomicU64,
}

impl KafkaMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        let mut requests_by_api = HashMap::new();
        // Initialize counters for common API keys
        for api_key in &[0, 1, 3, 9, 15, 16, 18, 19, 20, 32, 49] {
            requests_by_api.insert(*api_key, AtomicU64::new(0));
        }

        Self {
            connections_total: AtomicU64::new(0),
            connections_active: AtomicU64::new(0),
            requests_total: AtomicU64::new(0),
            requests_by_api,
            responses_total: AtomicU64::new(0),
            messages_produced_total: AtomicU64::new(0),
            messages_consumed_total: AtomicU64::new(0),
            topics_created_total: AtomicU64::new(0),
            topics_deleted_total: AtomicU64::new(0),
            consumer_groups_total: AtomicU64::new(0),
            partitions_total: AtomicU64::new(0),
            request_latency_micros: AtomicU64::new(0),
            errors_total: AtomicU64::new(0),
        }
    }

    /// Record a new connection
    pub fn record_connection(&self) {
        self.connections_total.fetch_add(1, Ordering::Relaxed);
        self.connections_active.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a connection closed
    pub fn record_connection_closed(&self) {
        self.connections_active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record a request
    pub fn record_request(&self, api_key: i16) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
        if let Some(counter) = self.requests_by_api.get(&api_key) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record a response
    pub fn record_response(&self) {
        self.responses_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record messages produced
    pub fn record_messages_produced(&self, count: u64) {
        self.messages_produced_total.fetch_add(count, Ordering::Relaxed);
    }

    /// Record messages consumed
    pub fn record_messages_consumed(&self, count: u64) {
        self.messages_consumed_total.fetch_add(count, Ordering::Relaxed);
    }

    /// Record topic created
    pub fn record_topic_created(&self) {
        self.topics_created_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record topic deleted
    pub fn record_topic_deleted(&self) {
        self.topics_deleted_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record consumer group created
    pub fn record_consumer_group_created(&self) {
        self.consumer_groups_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record partition created
    pub fn record_partition_created(&self) {
        self.partitions_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record request latency
    pub fn record_request_latency(&self, latency_micros: u64) {
        // Simple moving average - in production, you'd want more sophisticated tracking
        let current = self.request_latency_micros.load(Ordering::Relaxed);
        let new_avg = (current + latency_micros) / 2;
        self.request_latency_micros.store(new_avg, Ordering::Relaxed);
    }

    /// Record an error
    pub fn record_error(&self) {
        self.errors_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Get metrics snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            connections_total: self.connections_total.load(Ordering::Relaxed),
            connections_active: self.connections_active.load(Ordering::Relaxed),
            requests_total: self.requests_total.load(Ordering::Relaxed),
            responses_total: self.responses_total.load(Ordering::Relaxed),
            messages_produced_total: self.messages_produced_total.load(Ordering::Relaxed),
            messages_consumed_total: self.messages_consumed_total.load(Ordering::Relaxed),
            topics_created_total: self.topics_created_total.load(Ordering::Relaxed),
            topics_deleted_total: self.topics_deleted_total.load(Ordering::Relaxed),
            consumer_groups_total: self.consumer_groups_total.load(Ordering::Relaxed),
            partitions_total: self.partitions_total.load(Ordering::Relaxed),
            avg_request_latency_micros: self.request_latency_micros.load(Ordering::Relaxed),
            errors_total: self.errors_total.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of metrics at a point in time
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub connections_total: u64,
    pub connections_active: u64,
    pub requests_total: u64,
    pub responses_total: u64,
    pub messages_produced_total: u64,
    pub messages_consumed_total: u64,
    pub topics_created_total: u64,
    pub topics_deleted_total: u64,
    pub consumer_groups_total: u64,
    pub partitions_total: u64,
    pub avg_request_latency_micros: u64,
    pub errors_total: u64,
}

impl Default for KafkaMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics exporter for Prometheus-style metrics
pub struct MetricsExporter {
    metrics: Arc<KafkaMetrics>,
}

impl MetricsExporter {
    /// Create a new metrics exporter
    pub fn new(metrics: Arc<KafkaMetrics>) -> Self {
        Self { metrics }
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let snapshot = self.metrics.snapshot();

        format!(
            "# HELP kafka_connections_total Total number of connections\n\
             # TYPE kafka_connections_total counter\n\
             kafka_connections_total {}\n\
             # HELP kafka_connections_active Number of active connections\n\
             # TYPE kafka_connections_active gauge\n\
             kafka_connections_active {}\n\
             # HELP kafka_requests_total Total number of requests\n\
             # TYPE kafka_requests_total counter\n\
             kafka_requests_total {}\n\
             # HELP kafka_responses_total Total number of responses\n\
             # TYPE kafka_responses_total counter\n\
             kafka_responses_total {}\n\
             # HELP kafka_messages_produced_total Total messages produced\n\
             # TYPE kafka_messages_produced_total counter\n\
             kafka_messages_produced_total {}\n\
             # HELP kafka_messages_consumed_total Total messages consumed\n\
             # TYPE kafka_messages_consumed_total counter\n\
             kafka_messages_consumed_total {}\n\
             # HELP kafka_topics_created_total Total topics created\n\
             # TYPE kafka_topics_created_total counter\n\
             kafka_topics_created_total {}\n\
             # HELP kafka_topics_deleted_total Total topics deleted\n\
             # TYPE kafka_topics_deleted_total counter\n\
             kafka_topics_deleted_total {}\n\
             # HELP kafka_consumer_groups_total Total consumer groups\n\
             # TYPE kafka_consumer_groups_total gauge\n\
             kafka_consumer_groups_total {}\n\
             # HELP kafka_partitions_total Total partitions\n\
             # TYPE kafka_partitions_total gauge\n\
             kafka_partitions_total {}\n\
             # HELP kafka_request_latency_micros_avg Average request latency in microseconds\n\
             # TYPE kafka_request_latency_micros_avg gauge\n\
             kafka_request_latency_micros_avg {}\n\
             # HELP kafka_errors_total Total errors\n\
             # TYPE kafka_errors_total counter\n\
             kafka_errors_total {}\n",
            snapshot.connections_total,
            snapshot.connections_active,
            snapshot.requests_total,
            snapshot.responses_total,
            snapshot.messages_produced_total,
            snapshot.messages_consumed_total,
            snapshot.topics_created_total,
            snapshot.topics_deleted_total,
            snapshot.consumer_groups_total,
            snapshot.partitions_total,
            snapshot.avg_request_latency_micros,
            snapshot.errors_total
        )
    }
}
