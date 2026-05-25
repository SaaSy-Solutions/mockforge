//! Unmatched-path request tracking.
//!
//! Issue #79 round 13 — Srikanth's question (a): when a client sends
//! requests to paths that aren't in the server's loaded OpenAPI spec,
//! the request never reaches the validator (router returns 404 from
//! lookup), so `conformance_violations` never picks it up. This module
//! captures those unmatched 404s into a separate bounded ring buffer
//! so the TUI's Conformance tab can surface them.
//!
//! Use case: cross-checking a proxy's path coverage against the
//! server's. If the proxy reports a path the server doesn't know,
//! it'll show up here.

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// One unmatched-path request — captured by the HTTP server's fallback
/// when no registered route matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnknownPathRequest {
    /// When the request was rejected.
    pub timestamp: DateTime<Utc>,
    /// HTTP method (uppercase).
    pub method: String,
    /// Raw request path (not normalised to any spec template).
    pub path: String,
    /// Client IP if available, else `"unknown"`.
    pub client_ip: String,
    /// Query string portion, if any.
    pub query: String,
}

const DEFAULT_BUFFER_SIZE: usize = 256;

static UNKNOWN_PATHS: Lazy<Mutex<VecDeque<UnknownPathRequest>>> =
    Lazy::new(|| Mutex::new(VecDeque::with_capacity(DEFAULT_BUFFER_SIZE)));

/// Record an unmatched-path 404. FIFO when the buffer is full.
pub fn record(req: UnknownPathRequest) {
    let mut buf = UNKNOWN_PATHS.lock();
    if buf.len() == DEFAULT_BUFFER_SIZE {
        buf.pop_front();
    }
    buf.push_back(req);
}

/// Snapshot of buffered entries, newest first.
pub fn snapshot() -> Vec<UnknownPathRequest> {
    let buf = UNKNOWN_PATHS.lock();
    buf.iter().rev().cloned().collect()
}

/// Current buffer length.
pub fn len() -> usize {
    UNKNOWN_PATHS.lock().len()
}

/// Clear the buffer.
pub fn clear() {
    UNKNOWN_PATHS.lock().clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_snapshot_lifo() {
        clear();
        record(UnknownPathRequest {
            timestamp: Utc::now(),
            method: "GET".into(),
            path: "/first".into(),
            client_ip: "127.0.0.1".into(),
            query: String::new(),
        });
        record(UnknownPathRequest {
            timestamp: Utc::now(),
            method: "GET".into(),
            path: "/second".into(),
            client_ip: "127.0.0.1".into(),
            query: String::new(),
        });
        let snap = snapshot();
        assert_eq!(snap.len(), 2);
        assert_eq!(snap[0].path, "/second");
        assert_eq!(snap[1].path, "/first");
    }

    #[test]
    fn drops_oldest_at_capacity() {
        clear();
        for i in 0..(DEFAULT_BUFFER_SIZE + 5) {
            record(UnknownPathRequest {
                timestamp: Utc::now(),
                method: "GET".into(),
                path: format!("/p/{i}"),
                client_ip: "127.0.0.1".into(),
                query: String::new(),
            });
        }
        assert_eq!(len(), DEFAULT_BUFFER_SIZE);
        let snap = snapshot();
        assert_eq!(snap[0].path, format!("/p/{}", DEFAULT_BUFFER_SIZE + 4));
        assert_eq!(snap[DEFAULT_BUFFER_SIZE - 1].path, "/p/5");
    }
}
