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
use std::collections::{HashMap, VecDeque};
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
    /// Round 30 — number of times this signature has been observed.
    /// Always `1` in FIFO mode (the default). In unique-buffer mode
    /// (`MOCKFORGE_CONFORMANCE_BUFFER_UNIQUE=true`) every duplicate
    /// hit bumps this counter on the existing entry instead of
    /// consuming a new buffer slot. Defaults to `1` when deserialising
    /// older payloads that don't carry the field.
    #[serde(default = "one")]
    pub occurrences: u32,
    /// Round 36 (#876) — mockforge version the *client* (the bench
    /// driver) was running when it sent the request, as read from the
    /// `X-Mockforge-Client-Version` header. `None` when the inbound
    /// request didn't carry the header (older client, real proxy
    /// traffic, etc.). Lets users cross-correlate a client-side
    /// `CaseCapture` JSONL line with the matching server-side
    /// violation when both sides log against the same code base.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_mockforge_version: Option<String>,
    /// Round 36 (#876) — wall-clock timestamp the *client* stamped on
    /// its `CaseCapture`, as read from the `X-Mockforge-Client-Sent-At`
    /// header (RFC3339). Server-side `timestamp` is when the
    /// violation was *received*; this is when the probe was *sent*.
    /// Grep both for the same value to line up client + server
    /// records of the same probe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_sent_at: Option<DateTime<Utc>>,
}

/// Header set by the bench client (round 36, #876) carrying the
/// mockforge version that sent the request.
pub const CLIENT_VERSION_HEADER: &str = "x-mockforge-client-version";

/// Header set by the bench client (round 36, #876) carrying the
/// RFC3339 timestamp the request was sent at.
pub const CLIENT_SENT_AT_HEADER: &str = "x-mockforge-client-sent-at";

/// Parse the client-stamp headers off a raw `(name, value)` lookup
/// function. Accepts a closure so the same helper can read from
/// `axum::http::HeaderMap`, `reqwest::header::HeaderMap`, or a plain
/// `HashMap<String, String>` without forcing a particular type on
/// the caller. Header names are looked up case-insensitively.
pub fn read_client_stamps<F>(get: F) -> (Option<String>, Option<DateTime<Utc>>)
where
    F: Fn(&str) -> Option<String>,
{
    let version = get(CLIENT_VERSION_HEADER).filter(|s| !s.is_empty());
    let sent_at = get(CLIENT_SENT_AT_HEADER)
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|d| d.with_timezone(&Utc));
    (version, sent_at)
}

fn one() -> u32 {
    1
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

/// Round 30 — Srikanth on 0.3.173: "Can we have this buffer for unique
/// violation as opposed to duplicate violation. If this buffer size
/// doesn't discount duplicates then again we will run out of buffer
/// easily when more and more requests come to the server."
///
/// `MOCKFORGE_CONFORMANCE_BUFFER_UNIQUE=true` switches storage from
/// FIFO to dedup-by-signature: every duplicate of an already-buffered
/// (method, path, status, category, reason) hits its existing entry
/// and bumps `occurrences` instead of consuming a new slot. The buffer
/// fills only as fast as unique signatures arrive — so at 256 entries
/// a vCenter spec with ~150 unique violation kinds will hold every
/// kind even under 10M+ requests, instead of being clobbered by the
/// most common offender.
fn unique_mode_enabled() -> bool {
    std::env::var("MOCKFORGE_CONFORMANCE_BUFFER_UNIQUE")
        .ok()
        .map(|s| matches!(s.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

fn signature(v: &ServerConformanceViolation) -> String {
    format!("{}|{}|{}|{}|{}", v.method, v.path, v.status, v.category, v.reason)
}

/// FIFO buffer (default mode). Each violation consumes one slot,
/// oldest evicted when full.
static VIOLATIONS: Lazy<Mutex<VecDeque<ServerConformanceViolation>>> =
    Lazy::new(|| Mutex::new(VecDeque::with_capacity(effective_buffer_size())));

/// Unique-mode buffer: signature → entry (with bumped `occurrences`)
/// plus a `VecDeque<signature>` for insertion-order eviction. Only
/// touched when `MOCKFORGE_CONFORMANCE_BUFFER_UNIQUE` is enabled.
struct UniqueBuffer {
    by_sig: HashMap<String, ServerConformanceViolation>,
    order: VecDeque<String>,
}

impl UniqueBuffer {
    fn new() -> Self {
        Self {
            by_sig: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    fn record(&mut self, mut v: ServerConformanceViolation, cap: usize) {
        let sig = signature(&v);
        if let Some(existing) = self.by_sig.get_mut(&sig) {
            existing.occurrences = existing.occurrences.saturating_add(1);
            existing.timestamp = v.timestamp;
            return;
        }
        v.occurrences = 1;
        while self.order.len() >= cap {
            if let Some(old) = self.order.pop_front() {
                self.by_sig.remove(&old);
            } else {
                break;
            }
        }
        self.order.push_back(sig.clone());
        self.by_sig.insert(sig, v);
    }

    fn snapshot(&self) -> Vec<ServerConformanceViolation> {
        self.order.iter().rev().filter_map(|s| self.by_sig.get(s).cloned()).collect()
    }

    fn len(&self) -> usize {
        self.order.len()
    }

    fn clear(&mut self) {
        self.by_sig.clear();
        self.order.clear();
    }
}

static UNIQUE_VIOLATIONS: Lazy<Mutex<UniqueBuffer>> = Lazy::new(|| Mutex::new(UniqueBuffer::new()));

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
/// (FIFO by default; signature-deduped under
/// `MOCKFORGE_CONFORMANCE_BUFFER_UNIQUE=true`). Cheap enough to call
/// from the hot path — uses a parking_lot Mutex which is uncontended in
/// steady state.
pub fn record(mut violation: ServerConformanceViolation) {
    TOTAL_SEEN.fetch_add(1, Ordering::Relaxed);
    let cap = effective_buffer_size();
    if unique_mode_enabled() {
        UNIQUE_VIOLATIONS.lock().record(violation, cap);
        return;
    }
    if violation.occurrences == 0 {
        violation.occurrences = 1;
    }
    let mut buf = VIOLATIONS.lock();
    while buf.len() >= cap {
        buf.pop_front();
    }
    buf.push_back(violation);
}

/// Snapshot of the buffered violations, newest first.
pub fn snapshot() -> Vec<ServerConformanceViolation> {
    if unique_mode_enabled() {
        UNIQUE_VIOLATIONS.lock().snapshot()
    } else {
        let buf = VIOLATIONS.lock();
        buf.iter().rev().cloned().collect()
    }
}

/// Number of violations currently buffered (≤ `effective_buffer_size`).
pub fn len() -> usize {
    if unique_mode_enabled() {
        UNIQUE_VIOLATIONS.lock().len()
    } else {
        VIOLATIONS.lock().len()
    }
}

/// Lifetime total of violations recorded since process start, including
/// ones the ring buffer has since evicted.
pub fn total_seen() -> u64 {
    TOTAL_SEEN.load(Ordering::Relaxed)
}

/// Clear both buffers and reset lifetime counters. Primarily for
/// tests and TUI "reset" actions.
pub fn clear() {
    VIOLATIONS.lock().clear();
    UNIQUE_VIOLATIONS.lock().clear();
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
            occurrences: 1,
            client_mockforge_version: None,
            client_sent_at: None,
        }
    }

    /// Round 36 (#876) — `read_client_stamps` returns both fields
    /// when the headers are present and RFC3339-parsable.
    #[test]
    fn read_client_stamps_roundtrips_when_headers_present() {
        let stamped_at = "2026-06-17T12:34:56Z";
        let (version, sent_at) = read_client_stamps(|name| match name {
            CLIENT_VERSION_HEADER => Some("0.3.183".to_string()),
            CLIENT_SENT_AT_HEADER => Some(stamped_at.to_string()),
            _ => None,
        });
        assert_eq!(version.as_deref(), Some("0.3.183"));
        let sent_at = sent_at.expect("should parse RFC3339 timestamp");
        assert_eq!(sent_at.to_rfc3339(), "2026-06-17T12:34:56+00:00");
    }

    /// Missing or malformed headers should yield `None`, not panic
    /// or fall back to "now" (we don't want to fabricate timestamps).
    #[test]
    fn read_client_stamps_returns_none_when_headers_absent_or_garbage() {
        let (v, s) = read_client_stamps(|_| None);
        assert!(v.is_none());
        assert!(s.is_none());

        let (v, s) = read_client_stamps(|name| {
            if name == CLIENT_SENT_AT_HEADER {
                Some("not-a-timestamp".to_string())
            } else {
                None
            }
        });
        assert!(v.is_none());
        assert!(s.is_none(), "garbage timestamp must not be invented");

        // Empty version string treated as absent.
        let (v, _) = read_client_stamps(|name| {
            if name == CLIENT_VERSION_HEADER {
                Some(String::new())
            } else {
                None
            }
        });
        assert!(v.is_none());
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

    /// Round 30 — unique-mode buffer dedups duplicate signatures and
    /// bumps `occurrences` instead of consuming new slots. Direct
    /// `UniqueBuffer::record` call avoids the global env-var read,
    /// so this test stays threadsafe without `#[ignore]`.
    #[test]
    fn unique_buffer_dedups_by_signature_and_counts_occurrences() {
        let mut buf = UniqueBuffer::new();
        for _ in 0..10_000 {
            buf.record(v("GET", 400), 256);
        }
        // 10k identical violations → 1 slot used, occurrences == 10000.
        assert_eq!(buf.len(), 1);
        let snap = buf.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].occurrences, 10_000);
        assert_eq!(snap[0].method, "GET");
    }

    /// Round 30 — different (method, path, status, category, reason)
    /// tuples occupy distinct slots; identical tuples coalesce.
    #[test]
    fn unique_buffer_distinguishes_distinct_signatures() {
        let mut buf = UniqueBuffer::new();
        // 3 distinct signatures × 100 hits each
        for _ in 0..100 {
            buf.record(v("GET", 400), 256);
            buf.record(v("POST", 422), 256);
            let mut other = v("GET", 400);
            other.reason = "different".into();
            buf.record(other, 256);
        }
        assert_eq!(buf.len(), 3);
        let snap = buf.snapshot();
        assert_eq!(snap.len(), 3);
        for entry in &snap {
            assert_eq!(entry.occurrences, 100, "each signature seen 100×");
        }
    }

    /// Round 30 — unique mode still evicts when distinct-signature
    /// count exceeds the cap. Eviction is FIFO over insertion order
    /// (NOT recency-of-hit), matching how the regular ring buffer
    /// reads.
    #[test]
    fn unique_buffer_evicts_oldest_signature_at_capacity() {
        let mut buf = UniqueBuffer::new();
        let cap = 4;
        for i in 0..(cap + 3) {
            let mut entry = v("GET", 400);
            entry.reason = format!("kind-{i}");
            buf.record(entry, cap);
        }
        assert_eq!(buf.len(), cap);
        let snap = buf.snapshot();
        // newest first; signatures 0..2 evicted, 3..6 retained
        let kinds: Vec<&str> = snap.iter().map(|e| e.reason.as_str()).collect();
        assert_eq!(kinds, vec!["kind-6", "kind-5", "kind-4", "kind-3"]);
    }
}
