//! Server-side conformance violation tracking.
//!
//! Issue #79 round 12 — Srikanth's ask: "It would be good if mockforge tui
//! have a separate section for conformance failures on the incoming
//! requests to the mockforge server which has spec violation from the
//! Server Side point of view, that way I can cross check Server Side
//! Info with our proxy and understand the diff."
//!
//! The OpenAPI router already rejects requests that violate the loaded
//! spec (status 400/422). This module captures every such rejection into
//! a bounded ring buffer so the TUI / admin API can surface them
//! without scraping logs.
//!
//! Storage is best-effort, in-memory, and bounded — under sustained
//! WAF / load-test traffic we keep only the most recent N violations.

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// A single server-side conformance violation captured at the OpenAPI
/// router. Mirrors `ConformanceViolation` semantics from the bench-side
/// client validator so consumers can use the same dashboards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConformanceViolation {
    /// When the request was rejected.
    pub timestamp: DateTime<Utc>,
    /// HTTP method (uppercase).
    pub method: String,
    /// Spec-template path the request matched (e.g. `/users/{id}`).
    pub path: String,
    /// Client IP if available, else `"unknown"`.
    pub client_ip: String,
    /// HTTP status the server replied with (typically 400 or 422).
    pub status: u16,
    /// Short, human-readable reason — derived from the validator error.
    pub reason: String,
    /// Spec category the violation falls into (`"parameters"`,
    /// `"request-body"`, `"headers"`, etc.). Empty if the validator
    /// couldn't classify.
    pub category: String,
}

const DEFAULT_BUFFER_SIZE: usize = 256;

static VIOLATIONS: Lazy<Mutex<VecDeque<ServerConformanceViolation>>> =
    Lazy::new(|| Mutex::new(VecDeque::with_capacity(DEFAULT_BUFFER_SIZE)));

/// Record a violation. Old entries are dropped when the buffer is full
/// (FIFO). Cheap enough to call from the hot path — uses a parking_lot
/// Mutex which is uncontended in steady state.
pub fn record(violation: ServerConformanceViolation) {
    let mut buf = VIOLATIONS.lock();
    if buf.len() == DEFAULT_BUFFER_SIZE {
        buf.pop_front();
    }
    buf.push_back(violation);
}

/// Snapshot of the buffered violations, newest first.
pub fn snapshot() -> Vec<ServerConformanceViolation> {
    let buf = VIOLATIONS.lock();
    buf.iter().rev().cloned().collect()
}

/// Number of violations currently buffered.
pub fn len() -> usize {
    VIOLATIONS.lock().len()
}

/// Clear the buffer. Primarily for tests and TUI "reset" actions.
pub fn clear() {
    VIOLATIONS.lock().clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(method: &str, status: u16) -> ServerConformanceViolation {
        ServerConformanceViolation {
            timestamp: Utc::now(),
            method: method.to_string(),
            path: "/test".into(),
            client_ip: "127.0.0.1".into(),
            status,
            reason: "test".into(),
            category: "parameters".into(),
        }
    }

    #[test]
    fn record_and_snapshot_in_lifo_order() {
        clear();
        record(v("GET", 400));
        record(v("POST", 422));
        let snap = snapshot();
        assert_eq!(snap.len(), 2);
        // newest first
        assert_eq!(snap[0].method, "POST");
        assert_eq!(snap[1].method, "GET");
    }

    #[test]
    fn buffer_drops_oldest_at_capacity() {
        clear();
        for i in 0..(DEFAULT_BUFFER_SIZE + 50) {
            let mut entry = v("GET", 400);
            entry.reason = format!("{i}");
            record(entry);
        }
        assert_eq!(len(), DEFAULT_BUFFER_SIZE);
        let snap = snapshot();
        // newest is the last one we pushed
        assert_eq!(snap[0].reason, format!("{}", DEFAULT_BUFFER_SIZE + 50 - 1));
        // oldest still present is index 50 (the first 50 got dropped)
        assert_eq!(snap[DEFAULT_BUFFER_SIZE - 1].reason, format!("{}", 50));
    }
}
