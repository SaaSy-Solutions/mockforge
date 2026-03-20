//! Generic protocol metrics collector for all protocol crates.
//!
//! Provides a shared [`ProtocolMetrics`] struct with the common counters that every
//! protocol implementation needs (connections, messages, errors, bytes, latency).
//! Protocol crates embed this struct and add protocol-specific counters alongside it.
//!
//! # Example
//!
//! ```rust
//! use mockforge_observability::protocol_metrics::ProtocolMetrics;
//!
//! let metrics = ProtocolMetrics::new();
//! metrics.record_connection();
//! metrics.record_message();
//! metrics.record_bytes_sent(1024);
//! metrics.record_error();
//! metrics.record_disconnection();
//!
//! let snapshot = metrics.snapshot();
//! assert_eq!(snapshot.connections_total, 1);
//! assert_eq!(snapshot.connections_active, 0);
//! assert_eq!(snapshot.messages_total, 1);
//! assert_eq!(snapshot.errors_total, 1);
//! assert_eq!(snapshot.bytes_sent, 1024);
//! ```

use std::sync::atomic::{AtomicU64, Ordering};

/// Generic metrics collector shared by all protocol crates.
///
/// Contains the common counters that every protocol needs. Protocol crates can
/// embed this struct and add protocol-specific fields alongside it.
#[derive(Debug)]
pub struct ProtocolMetrics {
    /// Total number of connections ever established
    pub connections_total: AtomicU64,
    /// Currently active connections
    pub connections_active: AtomicU64,
    /// Total messages processed (sent or received, depending on protocol)
    pub messages_total: AtomicU64,
    /// Total errors encountered
    pub errors_total: AtomicU64,
    /// Total bytes sent
    pub bytes_sent: AtomicU64,
    /// Total bytes received
    pub bytes_received: AtomicU64,
    /// Average latency in microseconds (simple moving average)
    pub latency_micros: AtomicU64,
}

impl ProtocolMetrics {
    /// Create a new metrics collector with all counters at zero.
    pub fn new() -> Self {
        Self {
            connections_total: AtomicU64::new(0),
            connections_active: AtomicU64::new(0),
            messages_total: AtomicU64::new(0),
            errors_total: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            latency_micros: AtomicU64::new(0),
        }
    }

    /// Record a new connection (increments both total and active).
    pub fn record_connection(&self) {
        self.connections_total.fetch_add(1, Ordering::Relaxed);
        self.connections_active.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a disconnection (decrements active connections).
    pub fn record_disconnection(&self) {
        self.connections_active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record a message processed.
    pub fn record_message(&self) {
        self.messages_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record multiple messages processed at once.
    pub fn record_messages(&self, count: u64) {
        self.messages_total.fetch_add(count, Ordering::Relaxed);
    }

    /// Record an error.
    pub fn record_error(&self) {
        self.errors_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record bytes sent.
    pub fn record_bytes_sent(&self, bytes: u64) {
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record bytes received.
    pub fn record_bytes_received(&self, bytes: u64) {
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record latency in microseconds (simple moving average).
    pub fn record_latency(&self, latency_micros: u64) {
        let current = self.latency_micros.load(Ordering::Relaxed);
        let new_avg = if current == 0 {
            latency_micros
        } else {
            (current + latency_micros) / 2
        };
        self.latency_micros.store(new_avg, Ordering::Relaxed);
    }

    /// Take a point-in-time snapshot of all counters.
    pub fn snapshot(&self) -> ProtocolMetricsSnapshot {
        ProtocolMetricsSnapshot {
            connections_total: self.connections_total.load(Ordering::Relaxed),
            connections_active: self.connections_active.load(Ordering::Relaxed),
            messages_total: self.messages_total.load(Ordering::Relaxed),
            errors_total: self.errors_total.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            avg_latency_micros: self.latency_micros.load(Ordering::Relaxed),
        }
    }

    /// Export common metrics in Prometheus format with a given protocol prefix.
    ///
    /// # Arguments
    /// * `prefix` - Protocol name for metric labels (e.g., "kafka", "mqtt")
    pub fn export_prometheus(&self, prefix: &str) -> String {
        let snap = self.snapshot();
        format!(
            "# HELP {prefix}_connections_total Total number of connections\n\
             # TYPE {prefix}_connections_total counter\n\
             {prefix}_connections_total {}\n\
             # HELP {prefix}_connections_active Number of active connections\n\
             # TYPE {prefix}_connections_active gauge\n\
             {prefix}_connections_active {}\n\
             # HELP {prefix}_messages_total Total messages processed\n\
             # TYPE {prefix}_messages_total counter\n\
             {prefix}_messages_total {}\n\
             # HELP {prefix}_errors_total Total errors\n\
             # TYPE {prefix}_errors_total counter\n\
             {prefix}_errors_total {}\n\
             # HELP {prefix}_bytes_sent Total bytes sent\n\
             # TYPE {prefix}_bytes_sent counter\n\
             {prefix}_bytes_sent {}\n\
             # HELP {prefix}_bytes_received Total bytes received\n\
             # TYPE {prefix}_bytes_received counter\n\
             {prefix}_bytes_received {}\n\
             # HELP {prefix}_latency_micros_avg Average latency in microseconds\n\
             # TYPE {prefix}_latency_micros_avg gauge\n\
             {prefix}_latency_micros_avg {}\n",
            snap.connections_total,
            snap.connections_active,
            snap.messages_total,
            snap.errors_total,
            snap.bytes_sent,
            snap.bytes_received,
            snap.avg_latency_micros,
        )
    }
}

impl Default for ProtocolMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Point-in-time snapshot of protocol metrics with plain `u64` values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolMetricsSnapshot {
    /// Total connections ever established
    pub connections_total: u64,
    /// Currently active connections
    pub connections_active: u64,
    /// Total messages processed
    pub messages_total: u64,
    /// Total errors encountered
    pub errors_total: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Average latency in microseconds
    pub avg_latency_micros: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_metrics_are_zero() {
        let m = ProtocolMetrics::new();
        let s = m.snapshot();
        assert_eq!(s.connections_total, 0);
        assert_eq!(s.connections_active, 0);
        assert_eq!(s.messages_total, 0);
        assert_eq!(s.errors_total, 0);
        assert_eq!(s.bytes_sent, 0);
        assert_eq!(s.bytes_received, 0);
        assert_eq!(s.avg_latency_micros, 0);
    }

    #[test]
    fn test_default_is_new() {
        let m = ProtocolMetrics::default();
        assert_eq!(m.connections_total.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_record_connection_and_disconnection() {
        let m = ProtocolMetrics::new();
        m.record_connection();
        m.record_connection();
        assert_eq!(m.connections_total.load(Ordering::Relaxed), 2);
        assert_eq!(m.connections_active.load(Ordering::Relaxed), 2);

        m.record_disconnection();
        assert_eq!(m.connections_total.load(Ordering::Relaxed), 2);
        assert_eq!(m.connections_active.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_record_message() {
        let m = ProtocolMetrics::new();
        m.record_message();
        m.record_message();
        assert_eq!(m.messages_total.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_record_messages_batch() {
        let m = ProtocolMetrics::new();
        m.record_messages(10);
        m.record_messages(5);
        assert_eq!(m.messages_total.load(Ordering::Relaxed), 15);
    }

    #[test]
    fn test_record_error() {
        let m = ProtocolMetrics::new();
        m.record_error();
        m.record_error();
        assert_eq!(m.errors_total.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_record_bytes() {
        let m = ProtocolMetrics::new();
        m.record_bytes_sent(100);
        m.record_bytes_received(200);
        m.record_bytes_sent(50);

        let s = m.snapshot();
        assert_eq!(s.bytes_sent, 150);
        assert_eq!(s.bytes_received, 200);
    }

    #[test]
    fn test_record_latency() {
        let m = ProtocolMetrics::new();
        m.record_latency(100);
        assert_eq!(m.latency_micros.load(Ordering::Relaxed), 100);

        m.record_latency(200);
        // Moving average: (100 + 200) / 2 = 150
        assert_eq!(m.latency_micros.load(Ordering::Relaxed), 150);
    }

    #[test]
    fn test_snapshot_is_independent() {
        let m = ProtocolMetrics::new();
        m.record_connection();
        let s1 = m.snapshot();

        m.record_connection();
        let s2 = m.snapshot();

        assert_eq!(s1.connections_total, 1);
        assert_eq!(s2.connections_total, 2);
    }

    #[test]
    fn test_snapshot_clone() {
        let m = ProtocolMetrics::new();
        m.record_connection();
        let s = m.snapshot();
        let cloned = s.clone();
        assert_eq!(s, cloned);
    }

    #[test]
    fn test_debug_formatting() {
        let m = ProtocolMetrics::new();
        let debug = format!("{:?}", m);
        assert!(debug.contains("ProtocolMetrics"));

        let s = m.snapshot();
        let debug = format!("{:?}", s);
        assert!(debug.contains("ProtocolMetricsSnapshot"));
    }

    #[test]
    fn test_export_prometheus() {
        let m = ProtocolMetrics::new();
        m.record_connection();
        m.record_message();
        m.record_error();
        m.record_bytes_sent(1024);

        let output = m.export_prometheus("test_proto");
        assert!(output.contains("test_proto_connections_total 1"));
        assert!(output.contains("test_proto_messages_total 1"));
        assert!(output.contains("test_proto_errors_total 1"));
        assert!(output.contains("test_proto_bytes_sent 1024"));
        assert!(output.contains("# HELP"));
        assert!(output.contains("# TYPE"));
    }
}
