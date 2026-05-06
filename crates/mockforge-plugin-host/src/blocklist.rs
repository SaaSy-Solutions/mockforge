//! Kill-switch blocklist (RFC §8.3).
//!
//! Plugin-host polls a registry endpoint for revoked
//! `(plugin_name, version)` entries on a configurable interval.
//! When the blocklist updates, the actor unloads any matched
//! plugins and rejects subsequent loads with `revoked` so the
//! caller (main mockforge → API client) gets a stable error
//! they can surface.
//!
//! ## Why polling, not push
//!
//! Push requires a long-lived control-plane connection from each
//! plugin-host back to the registry — that's extra ops surface
//! (TLS rotation, reconnects, partitions) for a path that fires
//! every few weeks at most. Polling is operationally boring and
//! the latency target is already minutes per RFC §8.1.
//!
//! ## Wire format
//!
//! The registry endpoint returns a JSON array:
//!
//! ```json
//! [
//!   { "plugin_name": "stripe-rewriter", "version": "1.2.3",
//!     "reason": "CVE-2026-1234", "revoked_at": "2026-05-06T01:00:00Z" }
//! ]
//! ```
//!
//! Match is exact `(name, version)` — semver ranges are out of
//! scope for v1. Operators publishing a range simply enumerate
//! the affected versions on the registry side.

use std::collections::HashSet;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// One revoked plugin pin. The registry returns an array of
/// these; the host stores them in a [`Blocklist`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct BlocklistEntry {
    /// Plugin name from the registry.
    pub plugin_name: String,
    /// Exact version revoked. Wildcard semantics (e.g. revoke all
    /// versions of a plugin) are out of scope for v1; the
    /// registry should enumerate.
    pub version: String,
    /// Stable reason string. Surfaced in the `revoked` error code
    /// message and the audit log so operators can correlate
    /// against advisories.
    pub reason: String,
    /// When the registry recorded the revocation.
    pub revoked_at: DateTime<Utc>,
}

/// Cheaply-cloneable handle to the live blocklist. Both the poll
/// task and the actor hold an `Arc<RwLock<...>>` so updates from
/// the poller are observed by the actor without a channel hop.
#[derive(Debug, Clone, Default)]
pub struct Blocklist {
    inner: Arc<RwLock<Vec<BlocklistEntry>>>,
}

impl Blocklist {
    /// Empty blocklist — nothing is revoked.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the entire entry set. Called by the poll task on
    /// each successful refresh; the actor observes the new set on
    /// its next read without coordinating.
    pub async fn replace(&self, entries: Vec<BlocklistEntry>) {
        let mut guard = self.inner.write().await;
        *guard = entries;
    }

    /// Take a cheap snapshot of the current entries. Returned as
    /// `Vec` rather than holding the read lock, so callers can
    /// iterate without blocking the writer.
    pub async fn snapshot(&self) -> Vec<BlocklistEntry> {
        self.inner.read().await.clone()
    }

    /// Whether the named `(plugin, version)` is on the blocklist.
    /// Returns the matching reason for audit logging.
    pub async fn matches(&self, plugin_name: &str, version: &str) -> Option<String> {
        let guard = self.inner.read().await;
        for entry in guard.iter() {
            if entry.plugin_name == plugin_name && entry.version == version {
                return Some(entry.reason.clone());
            }
        }
        None
    }

    /// Return the names of every loaded plugin that's on the
    /// blocklist. Used by the actor's periodic sweep to decide
    /// what to unload.
    pub async fn matches_in<I, S>(&self, loaded: I) -> HashSet<String>
    where
        I: IntoIterator<Item = (S, S)>,
        S: AsRef<str>,
    {
        let guard = self.inner.read().await;
        let mut hits = HashSet::new();
        for (name, version) in loaded {
            for entry in guard.iter() {
                if entry.plugin_name == name.as_ref() && entry.version == version.as_ref() {
                    hits.insert(name.as_ref().to_string());
                    break;
                }
            }
        }
        hits
    }
}

/// Errors the poll task can hit. Each one is logged + retried on
/// the next interval; we never give up on the blocklist because a
/// stale blocklist is a security risk.
#[derive(Debug, thiserror::Error)]
pub enum PollError {
    /// HTTP request failed.
    #[error("blocklist HTTP error: {0}")]
    Http(String),
    /// Response body wasn't valid JSON / shape.
    #[error("blocklist parse error: {0}")]
    Parse(String),
}

/// Configuration for the poll task.
#[derive(Debug, Clone)]
pub struct BlocklistConfig {
    /// Registry endpoint to poll. Returns the JSON array
    /// described in the module docs.
    pub url: String,
    /// Poll interval. Default 60 seconds per RFC §8.2.
    pub interval: std::time::Duration,
    /// Optional bearer token; sent as `Authorization: Bearer ...`.
    /// Cloud production sets this so the registry can rate-limit
    /// per host.
    pub bearer_token: Option<String>,
}

impl BlocklistConfig {
    /// Construct with the default 60s interval and no bearer
    /// token.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            interval: std::time::Duration::from_secs(60),
            bearer_token: None,
        }
    }
}

/// Run the poll task forever. Returns only on shutdown signal —
/// the `select!` in main.rs is responsible for cancelling.
pub async fn run_poll_loop(config: BlocklistConfig, blocklist: Blocklist) {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(err) => {
            tracing::error!(error = %err, "failed to build blocklist HTTP client; poll task exiting");
            return;
        }
    };

    tracing::info!(url = %config.url, interval_secs = config.interval.as_secs(), "blocklist poll loop starting");

    let mut ticker = tokio::time::interval(config.interval);
    // Skip-the-first-tick semantics so we poll *immediately*, not
    // after one full interval. Operators see initial blocklist
    // state on startup.
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        ticker.tick().await;
        match fetch_blocklist(&client, &config).await {
            Ok(entries) => {
                let count = entries.len();
                blocklist.replace(entries).await;
                tracing::debug!(entries = count, "blocklist refreshed");
            }
            Err(err) => {
                // Don't update on error — preserve the last-known
                // blocklist. A registry outage shouldn't open a
                // security hole by silently emptying the list.
                tracing::warn!(error = %err, "blocklist poll failed; keeping last-known list");
            }
        }
    }
}

async fn fetch_blocklist(
    client: &reqwest::Client,
    config: &BlocklistConfig,
) -> Result<Vec<BlocklistEntry>, PollError> {
    let mut req = client.get(&config.url);
    if let Some(token) = &config.bearer_token {
        req = req.bearer_auth(token);
    }
    let response = req.send().await.map_err(|err| PollError::Http(err.to_string()))?;
    if !response.status().is_success() {
        return Err(PollError::Http(format!("status {}", response.status())));
    }
    response
        .json::<Vec<BlocklistEntry>>()
        .await
        .map_err(|err| PollError::Parse(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn entry(name: &str, version: &str) -> BlocklistEntry {
        BlocklistEntry {
            plugin_name: name.to_string(),
            version: version.to_string(),
            reason: format!("test-revocation:{name}:{version}"),
            revoked_at: Utc.with_ymd_and_hms(2026, 5, 6, 1, 0, 0).unwrap(),
        }
    }

    #[tokio::test]
    async fn empty_blocklist_matches_nothing() {
        let bl = Blocklist::new();
        assert!(bl.matches("any", "1.0.0").await.is_none());
    }

    #[tokio::test]
    async fn populated_blocklist_matches_exact_pair() {
        let bl = Blocklist::new();
        bl.replace(vec![entry("evil-plugin", "1.0.0")]).await;
        assert!(bl.matches("evil-plugin", "1.0.0").await.is_some());
        // Different version of same plugin not blocked.
        assert!(bl.matches("evil-plugin", "1.0.1").await.is_none());
        // Different plugin same version not blocked.
        assert!(bl.matches("good-plugin", "1.0.0").await.is_none());
    }

    #[tokio::test]
    async fn replace_swaps_the_entire_set() {
        let bl = Blocklist::new();
        bl.replace(vec![entry("p1", "1.0.0")]).await;
        bl.replace(vec![entry("p2", "2.0.0")]).await;
        assert!(bl.matches("p1", "1.0.0").await.is_none());
        assert!(bl.matches("p2", "2.0.0").await.is_some());
    }

    #[tokio::test]
    async fn matches_in_finds_loaded_hits() {
        let bl = Blocklist::new();
        bl.replace(vec![entry("p1", "1.0.0"), entry("p3", "3.0.0")]).await;
        let loaded = vec![
            ("p1".to_string(), "1.0.0".to_string()), // hit
            ("p2".to_string(), "2.0.0".to_string()), // miss
            ("p3".to_string(), "3.0.0".to_string()), // hit
        ];
        let hits = bl.matches_in(loaded).await;
        assert_eq!(hits.len(), 2);
        assert!(hits.contains("p1"));
        assert!(hits.contains("p3"));
    }

    #[tokio::test]
    async fn snapshot_round_trips_cloneable() {
        let bl = Blocklist::new();
        bl.replace(vec![entry("p1", "1.0.0")]).await;
        let snap = bl.snapshot().await;
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].plugin_name, "p1");
    }

    #[tokio::test]
    async fn blocklist_entry_round_trips_through_json() {
        // Confirms the wire format the registry endpoint produces
        // parses correctly. Any drift would surface here.
        let raw = r#"[
            {
                "plugin_name": "evil",
                "version": "1.0.0",
                "reason": "CVE-2026-1234",
                "revoked_at": "2026-05-06T01:00:00Z"
            }
        ]"#;
        let entries: Vec<BlocklistEntry> = serde_json::from_str(raw).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].plugin_name, "evil");
        assert_eq!(entries[0].reason, "CVE-2026-1234");
    }

    #[tokio::test]
    async fn config_defaults_to_60s_interval() {
        let cfg = BlocklistConfig::new("http://example/blocklist");
        assert_eq!(cfg.interval.as_secs(), 60);
        assert!(cfg.bearer_token.is_none());
    }
}
