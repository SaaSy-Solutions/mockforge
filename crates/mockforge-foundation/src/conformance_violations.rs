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
use std::sync::atomic::{AtomicU64, Ordering};

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

/// Round 29 — Srikanth on 0.3.172 had 10,145 violations seen but only
/// 114 unique entries in his export, because the in-memory ring buffer
/// caps at 256. For long-running runs against large specs (vCenter,
/// Microsoft Graph) that fills quickly. Override via
/// `MOCKFORGE_CONFORMANCE_BUFFER_SIZE` so users can raise it without
/// recompiling. Capped at 64k to keep peak memory bounded.
fn effective_buffer_size() -> usize {
    let cap: usize = 64 * 1024;
    std::env::var("MOCKFORGE_CONFORMANCE_BUFFER_SIZE")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .map(|n| n.min(cap))
        .unwrap_or(DEFAULT_BUFFER_SIZE)
}

static VIOLATIONS: Lazy<Mutex<VecDeque<ServerConformanceViolation>>> =
    Lazy::new(|| Mutex::new(VecDeque::with_capacity(effective_buffer_size())));

/// Lifetime count of violations recorded since process start (Issue #79
/// round 15). The ring buffer only keeps the most recent
/// `DEFAULT_BUFFER_SIZE`; this counter answers Srikanth's "I sent 656k
/// requests but only see 256" — the 256 is the buffer cap, this is the
/// true total seen.
static TOTAL_SEEN: AtomicU64 = AtomicU64::new(0);

/// Lifetime count of requests that *passed* the spec validator (round
/// 17.1). Bumped on the `Ok(())` branch of
/// `run_validation_with_recording`. Lets the TUI display
/// "X conformant / Y violations" instead of just one side.
static TOTAL_OK: AtomicU64 = AtomicU64::new(0);

/// Bump the conformant-request counter. Called from the validator's
/// success path. Bench code can call it directly when wiring its own
/// counters too.
pub fn record_ok() {
    TOTAL_OK.fetch_add(1, Ordering::Relaxed);
}

/// Lifetime total of requests that passed the spec validator.
pub fn total_ok() -> u64 {
    TOTAL_OK.load(Ordering::Relaxed)
}

/// Record a violation. Old entries are dropped when the buffer is full
/// (FIFO). Cheap enough to call from the hot path — uses a parking_lot
/// Mutex which is uncontended in steady state.
pub fn record(violation: ServerConformanceViolation) {
    TOTAL_SEEN.fetch_add(1, Ordering::Relaxed);
    let cap = effective_buffer_size();
    let mut buf = VIOLATIONS.lock();
    while buf.len() >= cap {
        buf.pop_front();
    }
    buf.push_back(violation);
}

/// Snapshot of the buffered violations, newest first.
pub fn snapshot() -> Vec<ServerConformanceViolation> {
    let buf = VIOLATIONS.lock();
    buf.iter().rev().cloned().collect()
}

/// Number of violations currently buffered (≤ `DEFAULT_BUFFER_SIZE`).
pub fn len() -> usize {
    VIOLATIONS.lock().len()
}

/// Lifetime total of violations recorded since process start, including
/// ones the ring buffer has since evicted.
pub fn total_seen() -> u64 {
    TOTAL_SEEN.load(Ordering::Relaxed)
}

/// Clear the buffer and reset both lifetime counters. Primarily for
/// tests and TUI "reset" actions.
pub fn clear() {
    VIOLATIONS.lock().clear();
    TOTAL_SEEN.store(0, Ordering::Relaxed);
    TOTAL_OK.store(0, Ordering::Relaxed);
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

    /// Round 29 — `MOCKFORGE_CONFORMANCE_BUFFER_SIZE` env var
    /// overrides the default 256 cap. Tagged `#[ignore]` because it
    /// mutates a process-wide env var that races with the other
    /// tests in this module (which call `record()` → which reads
    /// the same env var). Run explicitly with
    /// `cargo test -p mockforge-foundation -- --ignored
    /// effective_buffer_size_respects_env_var --test-threads=1`.
    #[test]
    #[ignore]
    fn effective_buffer_size_respects_env_var() {
        let original = std::env::var("MOCKFORGE_CONFORMANCE_BUFFER_SIZE").ok();

        // SAFETY: process-wide env mutation is unsound under multi-
        // threaded test runs; this test is gated with `#[ignore]` to
        // force serial execution by the developer when needed.
        unsafe {
            std::env::set_var("MOCKFORGE_CONFORMANCE_BUFFER_SIZE", "1000");
        }
        assert_eq!(effective_buffer_size(), 1000);

        unsafe {
            std::env::set_var("MOCKFORGE_CONFORMANCE_BUFFER_SIZE", "0");
        }
        assert_eq!(effective_buffer_size(), DEFAULT_BUFFER_SIZE, "zero falls back to default");

        unsafe {
            std::env::set_var("MOCKFORGE_CONFORMANCE_BUFFER_SIZE", "garbage");
        }
        assert_eq!(
            effective_buffer_size(),
            DEFAULT_BUFFER_SIZE,
            "unparsable falls back to default"
        );

        unsafe {
            std::env::set_var("MOCKFORGE_CONFORMANCE_BUFFER_SIZE", "999999");
        }
        assert_eq!(effective_buffer_size(), 64 * 1024, "clamped to 64k");

        // Restore
        unsafe {
            match original {
                Some(v) => std::env::set_var("MOCKFORGE_CONFORMANCE_BUFFER_SIZE", v),
                None => std::env::remove_var("MOCKFORGE_CONFORMANCE_BUFFER_SIZE"),
            }
        }
    }
}
