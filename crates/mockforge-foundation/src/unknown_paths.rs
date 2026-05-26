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

/// Issue #79 round 14 — Srikanth's shadow-mode ask. When enabled, the
/// server returns `200` for requests that would otherwise be rejected
/// (unknown paths → 404, spec violations → 400/422) while still
/// recording them to the unknown-paths / conformance buffers. Lets a
/// proxy replay run flow through non-blocking with full violation
/// capture — a "report-only" / monitor mode.
///
/// Read once per request from `MOCKFORGE_SHADOW_MODE` (`1`/`true`).
/// Cheap enough for the hot path; no caching needed since env lookups
/// are fast and this keeps the flag dynamically toggleable in tests.
pub fn shadow_mode_enabled() -> bool {
    std::env::var("MOCKFORGE_SHADOW_MODE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

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
    /// HTTP status the server actually returned for this request.
    /// Normally `404`; in shadow mode (Issue #79 round 14) the server
    /// returns `200` instead but still records the unknown path here,
    /// so the column reflects what the client saw.
    #[serde(default = "default_unknown_status")]
    pub status: u16,
}

fn default_unknown_status() -> u16 {
    404
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

    /// The buffer tests mutate the global `UNKNOWN_PATHS` static, so
    /// they must not interleave (cargo runs tests in parallel by
    /// default). Serialize them through a shared lock.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn req(path: &str) -> UnknownPathRequest {
        UnknownPathRequest {
            timestamp: Utc::now(),
            method: "GET".into(),
            path: path.into(),
            client_ip: "127.0.0.1".into(),
            query: String::new(),
            status: 404,
        }
    }

    #[test]
    fn record_and_snapshot_lifo() {
        let _guard = TEST_LOCK.lock();
        clear();
        record(req("/first"));
        record(req("/second"));
        let snap = snapshot();
        assert_eq!(snap.len(), 2);
        assert_eq!(snap[0].path, "/second");
        assert_eq!(snap[1].path, "/first");
    }

    #[test]
    fn drops_oldest_at_capacity() {
        let _guard = TEST_LOCK.lock();
        clear();
        for i in 0..(DEFAULT_BUFFER_SIZE + 5) {
            record(req(&format!("/p/{i}")));
        }
        assert_eq!(len(), DEFAULT_BUFFER_SIZE);
        let snap = snapshot();
        assert_eq!(snap[0].path, format!("/p/{}", DEFAULT_BUFFER_SIZE + 4));
        assert_eq!(snap[DEFAULT_BUFFER_SIZE - 1].path, "/p/5");
    }

    #[test]
    fn shadow_mode_reads_env() {
        std::env::set_var("MOCKFORGE_SHADOW_MODE", "true");
        assert!(shadow_mode_enabled());
        std::env::set_var("MOCKFORGE_SHADOW_MODE", "1");
        assert!(shadow_mode_enabled());
        std::env::set_var("MOCKFORGE_SHADOW_MODE", "0");
        assert!(!shadow_mode_enabled());
        std::env::remove_var("MOCKFORGE_SHADOW_MODE");
        assert!(!shadow_mode_enabled());
    }

    #[test]
    fn status_defaults_to_404_on_legacy_payload() {
        // Old payloads (round 13) had no `status` field; serde default
        // must fill 404 so the TUI column doesn't break on older servers.
        let json = r#"{"timestamp":"2026-05-26T00:00:00Z","method":"GET","path":"/x","client_ip":"unknown","query":""}"#;
        let parsed: UnknownPathRequest = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.status, 404);
    }
}
