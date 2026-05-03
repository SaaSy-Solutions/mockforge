//! Per-second rate counters for HTTP traffic.
//!
//! Simple monotonic atomic counters. The HTTP metrics middleware bumps
//! the response counters; the TCP-accept counter is bumped at the listener
//! level (see `mockforge-http`'s `CountingTcpListener` and `mockforge-chaos`'s
//! `ChaosTcpListener`).
//!
//! These are sampled at fixed intervals by the admin dashboard collector
//! to derive per-second rates:
//!
//! * **TPS** — successful (2xx/3xx) responses per second
//! * **RPS** — 200-OK responses per second
//! * **CPS** — accepted TCP connections per second  *(plain HTTP only;
//!   the TLS path uses `axum_server`'s own accept loop and is not yet
//!   instrumented)*
//!
//! The "successful API transaction" definition for TPS is `200..=399`,
//! matching how load-testing tools (k6, JMeter, etc.) classify a successful
//! request — anything that wasn't a 4xx/5xx error.

use std::sync::atomic::{AtomicU64, Ordering};

/// Total successful HTTP responses (status `200..=399`).
pub static SUCCESSFUL_RESPONSES_TOTAL: AtomicU64 = AtomicU64::new(0);

/// Total HTTP `200 OK` responses.
pub static OK_RESPONSES_TOTAL: AtomicU64 = AtomicU64::new(0);

/// Total accepted TCP connections (plain HTTP path only — see module docs).
pub static HTTP_ACCEPTS_TOTAL: AtomicU64 = AtomicU64::new(0);

/// Bump the response counters according to the response's status code.
#[inline]
pub fn record_response(status_code: u16) {
    if (200..=399).contains(&status_code) {
        SUCCESSFUL_RESPONSES_TOTAL.fetch_add(1, Ordering::Relaxed);
    }
    if status_code == 200 {
        OK_RESPONSES_TOTAL.fetch_add(1, Ordering::Relaxed);
    }
}

/// Bump the TCP-accept counter. Call once per accepted connection.
#[inline]
pub fn record_accept() {
    HTTP_ACCEPTS_TOTAL.fetch_add(1, Ordering::Relaxed);
}

/// Point-in-time snapshot of the rate counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CounterSnapshot {
    pub successful: u64,
    pub ok: u64,
    pub accepts: u64,
}

/// Read all counters atomically (each load is independent — a snapshot
/// is not transactional, but for sampling rates this is fine).
pub fn snapshot() -> CounterSnapshot {
    CounterSnapshot {
        successful: SUCCESSFUL_RESPONSES_TOTAL.load(Ordering::Relaxed),
        ok: OK_RESPONSES_TOTAL.load(Ordering::Relaxed),
        accepts: HTTP_ACCEPTS_TOTAL.load(Ordering::Relaxed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // The counters are global; serialize tests to avoid cross-test interference.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn reset_counters() {
        SUCCESSFUL_RESPONSES_TOTAL.store(0, Ordering::Relaxed);
        OK_RESPONSES_TOTAL.store(0, Ordering::Relaxed);
        HTTP_ACCEPTS_TOTAL.store(0, Ordering::Relaxed);
    }

    #[test]
    fn record_response_classifies_2xx_as_successful() {
        let _g = TEST_LOCK.lock().unwrap();
        reset_counters();
        record_response(200);
        record_response(204);
        record_response(301);
        let s = snapshot();
        assert_eq!(s.successful, 3, "200, 204, 301 are all successful");
        assert_eq!(s.ok, 1, "only one 200");
    }

    #[test]
    fn record_response_excludes_4xx_5xx_from_successful() {
        let _g = TEST_LOCK.lock().unwrap();
        reset_counters();
        record_response(404);
        record_response(429);
        record_response(500);
        record_response(503);
        let s = snapshot();
        assert_eq!(s.successful, 0);
        assert_eq!(s.ok, 0);
    }

    #[test]
    fn record_accept_increments() {
        let _g = TEST_LOCK.lock().unwrap();
        reset_counters();
        record_accept();
        record_accept();
        record_accept();
        let s = snapshot();
        assert_eq!(s.accepts, 3);
    }

    #[test]
    fn snapshot_returns_current_values() {
        let _g = TEST_LOCK.lock().unwrap();
        reset_counters();
        record_response(200);
        record_response(200);
        record_accept();
        let s = snapshot();
        assert_eq!(s.successful, 2);
        assert_eq!(s.ok, 2);
        assert_eq!(s.accepts, 1);
    }

    #[test]
    fn ok_counter_only_for_status_200() {
        let _g = TEST_LOCK.lock().unwrap();
        reset_counters();
        record_response(200);
        record_response(201);
        record_response(204);
        let s = snapshot();
        assert_eq!(s.ok, 1, "only 200 increments ok counter");
        assert_eq!(s.successful, 3, "all three are successful");
    }
}
