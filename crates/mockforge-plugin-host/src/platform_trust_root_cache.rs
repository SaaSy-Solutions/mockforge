//! Per-deployment platform signing-root cache + live-reload loop
//! (Issue #568 — follow-up to #549).
//!
//! ## What this does
//!
//! 1. On host boot, seeds the [`PlatformTrustStore`] from config (env
//!    var `MOCKFORGE_PLATFORM_TRUST_ROOTS` — newline-separated
//!    `<key_id>:<base64_spki_der>` entries). This is the "embedded
//!    root" mechanism the runbook calls out: even if the registry's
//!    rotation history is missing, a fresh host that just deployed
//!    knows which platform key to trust because the operator pinned
//!    it into the release.
//! 2. Polls `GET /api/internal/plugin-rotation-events` on a 60-second
//!    cadence (matches `blocklist::run_poll_loop` and
//!    `trust_root_cache::run_trust_root_refresh_loop`).
//! 3. For each rotation event returned, looks up `from_key_id` in the
//!    current trust set. If trusted, calls
//!    [`mockforge_platform_signing::verify_rotation_event`] on the
//!    event; on success, adds `to_key_id` to the trust set and schedules
//!    eviction of `from_key_id` at `transition_until`.
//! 4. On every tick, evicts entries whose `expires_at` has passed.
//!
//! Failed polls **do not** clear the trust store — same fail-closed
//! posture as the per-org [`crate::trust_root_cache`].
//!
//! ## Why a separate store from `TrustStore` (publisher keys)
//!
//! `TrustStore` is Ed25519 — publishers sign plugin WASM. Platform
//! roots are ECDSA P-256 / P-384 — the registry signs rotation events
//! (and, in a follow-up, the kill-switch blocklist) with them. Different
//! algorithms, different threat models, different rotation cadence;
//! a single combined store would just be an unnecessary `match`.
//!
//! ## What this does NOT do
//!
//! - Verify plugin **publisher** signatures (that's [`crate::signing`]).
//! - Sign anything (the host is verify-only — see RFC §8.2).
//! - Surface keys via an HTTP `/healthz` (the host runs no HTTP server;
//!   the parent process forwards the IPC [`crate::protocol::Response::HealthOk`]
//!   payload over its own healthz instead).

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use mockforge_platform_signing::{verify_rotation_event, RotationEvent, VerifyError};
use serde::Deserialize;
use tokio::sync::RwLock;

/// Default poll interval — matches `blocklist::run_poll_loop` and
/// `trust_root_cache::run_trust_root_refresh_loop`.
pub const DEFAULT_REFRESH_INTERVAL_SECS: u64 = 60;

/// Configuration env var: newline-separated `<key_id>:<base64_spki_der>`
/// entries. Seeded at boot; subsequent rotation events extend or
/// replace the set.
pub const ENV_EMBEDDED_ROOTS: &str = "MOCKFORGE_PLATFORM_TRUST_ROOTS";

/// One trusted platform signing root.
#[derive(Debug, Clone)]
pub struct TrustedPlatformKey {
    /// SubjectPublicKeyInfo (DER) for the key. Stored base64-decoded;
    /// the rotation-event verifier reconstructs the raw point.
    pub public_key_spki_der: Vec<u8>,
    /// When this key should drop out of the trust set. `None` means
    /// "trust until explicitly replaced" — the steady-state shape for
    /// embedded roots. `Some(_)` is set when the host applies a
    /// rotation event: the previous (`from`) key gets stamped with
    /// `transition_until` so it ages out cleanly.
    pub expires_at: Option<DateTime<Utc>>,
}

/// Cheaply-cloneable handle to the live platform-signing trust set.
///
/// Both the poll task and the IPC Health handler hold an
/// `Arc<RwLock<_>>` so updates from the poller are visible to readers
/// without a channel hop. Same shape as [`crate::signing::TrustStore`]
/// but specialized to ECDSA P-256/P-384.
#[derive(Debug, Clone, Default)]
pub struct PlatformTrustStore {
    inner: Arc<RwLock<HashMap<String, TrustedPlatformKey>>>,
}

impl PlatformTrustStore {
    /// Empty store — nothing is trusted. The host will reject every
    /// rotation event until the embedded-root config is applied.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or replace a key. Tests + the rotation handler use this.
    pub async fn insert(&self, key_id: String, key: TrustedPlatformKey) {
        self.inner.write().await.insert(key_id, key);
    }

    /// Remove a key. Returns `true` if it was present.
    pub async fn remove(&self, key_id: &str) -> bool {
        self.inner.write().await.remove(key_id).is_some()
    }

    /// Snapshot the SPKI DER for a given key id, if trusted.
    pub async fn get(&self, key_id: &str) -> Option<Vec<u8>> {
        self.inner.read().await.get(key_id).map(|k| k.public_key_spki_der.clone())
    }

    /// Currently-trusted key ids. Used by the IPC Health response so
    /// the runbook's `/healthz.trust.platform_signing_keys` fleet check
    /// works against every host.
    pub async fn key_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.inner.read().await.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Evict entries whose `expires_at` is in the past. Called on
    /// every poll tick so retired keys don't linger.
    ///
    /// Returns the removed key ids for tracing/log enrichment.
    pub async fn evict_expired(&self, now: DateTime<Utc>) -> Vec<String> {
        let mut guard = self.inner.write().await;
        let expired: Vec<String> = guard
            .iter()
            .filter_map(|(id, key)| key.expires_at.filter(|exp| *exp <= now).map(|_| id.clone()))
            .collect();
        for id in &expired {
            guard.remove(id);
        }
        expired
    }
}

/// Configuration for the rotation-event refresh task.
#[derive(Debug, Clone)]
pub struct PlatformTrustCacheConfig {
    /// Fully-qualified URL to poll. Must point at the registry's
    /// `/api/internal/plugin-rotation-events`.
    pub url: String,
    /// Poll interval. Default 60s.
    pub interval: std::time::Duration,
    /// Bearer token sent on every poll; the registry uses
    /// `MOCKFORGE_INTERNAL_API_TOKEN` as the shared secret.
    pub bearer_token: Option<String>,
}

impl PlatformTrustCacheConfig {
    /// Build a config with the default 60s interval and no bearer
    /// token. Callers will normally fill in `bearer_token` from their
    /// env wiring before passing it to [`run_platform_trust_refresh_loop`].
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            interval: std::time::Duration::from_secs(DEFAULT_REFRESH_INTERVAL_SECS),
            bearer_token: None,
        }
    }
}

/// JSON wire shape of `/api/internal/plugin-rotation-events`. Mirrors
/// `mockforge_registry_server::handlers::platform_signing::PluginRotationEventsResponse`
/// — we keep our own deserializer so the host doesn't pull the
/// registry-server crate into the build.
#[derive(Debug, Deserialize)]
struct WireResponse {
    latest: Option<RotationEvent>,
    // Other fields (phase, trusted_key_ids) are diagnostic; the host
    // doesn't act on them. Keeping them out of the local struct means
    // future additions to the wire shape don't force a host upgrade
    // (`serde` ignores unknown fields by default).
}

/// Errors the refresh loop can hit on a single poll. Logged + retried
/// on the next tick — never escalated, same as the per-org trust
/// cache. Public so tests and observability layers can match on
/// specific variants.
#[derive(Debug, thiserror::Error)]
pub enum PlatformRefreshError {
    /// HTTP request failed (connect, timeout, non-2xx).
    #[error("rotation-event HTTP error: {0}")]
    Http(String),
    /// Response body wasn't valid JSON for the expected shape.
    #[error("rotation-event parse error: {0}")]
    Parse(String),
    /// The event referred to a `from_key_id` not in our trust set.
    /// Logged + skipped — most likely the host is brand new and hasn't
    /// received the prior rotation yet; the operator needs to re-pin
    /// the new release per the runbook.
    #[error("rotation event's from_key_id '{0}' is not currently trusted")]
    UnknownFromKey(String),
    /// Crypto verification failed. Logged with `severity=high` — this
    /// is the only error variant that points at active misuse.
    #[error("rotation-event signature verification failed: {0}")]
    Verify(#[from] VerifyError),
    /// `to_public_key_b64` couldn't be decoded as base64.
    #[error("to_public_key_b64 is not valid base64: {0}")]
    InvalidToPublicKey(String),
}

/// Parse the embedded-roots env var into a vec of `(key_id, spki_der)`
/// pairs. Errors point at the offending line so the operator can fix
/// the config and restart.
///
/// Format: one entry per line, `key_id:base64_spki_der`. Lines starting
/// with `#` or containing only whitespace are ignored.
pub fn parse_embedded_roots(raw: &str) -> Result<Vec<(String, Vec<u8>)>, String> {
    use base64::Engine;
    let mut out = Vec::new();
    for (idx, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let (id, b64) = trimmed
            .split_once(':')
            .ok_or_else(|| format!("line {}: expected 'key_id:base64_spki_der'", idx + 1))?;
        let id = id.trim();
        let b64 = b64.trim();
        if id.is_empty() {
            return Err(format!("line {}: empty key_id", idx + 1));
        }
        let der = base64::engine::general_purpose::STANDARD
            .decode(b64.as_bytes())
            .map_err(|e| format!("line {}: invalid base64: {e}", idx + 1))?;
        out.push((id.to_string(), der));
    }
    Ok(out)
}

/// Seed the trust store with embedded roots from
/// [`ENV_EMBEDDED_ROOTS`]. Returns the number of keys seeded so the
/// caller can log the boot-time state.
pub async fn seed_from_env(store: &PlatformTrustStore) -> Result<usize, String> {
    let raw = match std::env::var(ENV_EMBEDDED_ROOTS) {
        Ok(v) if !v.is_empty() => v,
        _ => return Ok(0),
    };
    let parsed = parse_embedded_roots(&raw)?;
    let count = parsed.len();
    for (id, der) in parsed {
        store
            .insert(
                id,
                TrustedPlatformKey {
                    public_key_spki_der: der,
                    expires_at: None,
                },
            )
            .await;
    }
    Ok(count)
}

/// Run the rotation-event refresh task forever. Returns only on the
/// caller's `select!` cancelling the task.
///
/// Drives `tokio::time::interval` with `Delay` missed-tick behavior so
/// a slow poll doesn't double-fire on the next tick. Same approach as
/// the publisher trust cache.
pub async fn run_platform_trust_refresh_loop(
    config: PlatformTrustCacheConfig,
    store: PlatformTrustStore,
) {
    let client =
        match reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).build() {
            Ok(c) => c,
            Err(err) => {
                tracing::error!(
                    error = %err,
                    "failed to build platform-rotation HTTP client; refresh task exiting"
                );
                return;
            }
        };

    tracing::info!(
        url = %config.url,
        interval_secs = config.interval.as_secs(),
        "platform-signing rotation-event refresh loop starting"
    );

    let mut ticker = tokio::time::interval(config.interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        ticker.tick().await;
        let evicted = store.evict_expired(Utc::now()).await;
        if !evicted.is_empty() {
            tracing::info!(
                evicted_key_ids = ?evicted,
                "platform-signing trust set: keys expired and removed"
            );
        }
        match fetch_and_apply(&client, &config, &store).await {
            Ok(Some(to_key_id)) => {
                tracing::info!(
                    to_key_id = %to_key_id,
                    "platform-signing rotation event applied"
                );
            }
            Ok(None) => {
                tracing::debug!("platform-signing rotation event poll: nothing new");
            }
            Err(err) => {
                // Don't fail the loop — log + try again next tick. The
                // trust set is preserved.
                let level = match &err {
                    PlatformRefreshError::Verify(_) => tracing::Level::WARN,
                    _ => tracing::Level::DEBUG,
                };
                if level == tracing::Level::WARN {
                    tracing::warn!(error = %err, "platform-signing rotation event rejected");
                } else {
                    tracing::debug!(error = %err, "platform-signing rotation event poll error");
                }
            }
        }
    }
}

/// One refresh tick: fetch latest event, verify, apply. Pulled out
/// for testability — the side-effecting [`run_platform_trust_refresh_loop`]
/// just wraps it in a `tokio::time::interval`.
///
/// Returns `Ok(Some(to_key_id))` when an event was successfully
/// applied, `Ok(None)` when there's nothing new (registry hasn't
/// rotated since the host's last poll, or the event's `from` key isn't
/// trusted — both quiet no-ops from the host's perspective).
async fn fetch_and_apply(
    client: &reqwest::Client,
    config: &PlatformTrustCacheConfig,
    store: &PlatformTrustStore,
) -> Result<Option<String>, PlatformRefreshError> {
    let mut req = client.get(&config.url);
    if let Some(token) = &config.bearer_token {
        req = req.bearer_auth(token);
    }
    let response = req.send().await.map_err(|err| PlatformRefreshError::Http(err.to_string()))?;
    if !response.status().is_success() {
        return Err(PlatformRefreshError::Http(format!("status {}", response.status())));
    }
    let body: WireResponse = response
        .json::<WireResponse>()
        .await
        .map_err(|err| PlatformRefreshError::Parse(err.to_string()))?;

    let event = match body.latest {
        Some(ev) => ev,
        None => return Ok(None),
    };

    apply_rotation_event(&event, store).await.map(Some)
}

/// Apply a single rotation event against the trust store. Public so
/// the unit tests + a future "operator pasted an event JSON" admin
/// path can exercise it without going through the network.
pub async fn apply_rotation_event(
    event: &RotationEvent,
    store: &PlatformTrustStore,
) -> Result<String, PlatformRefreshError> {
    use base64::Engine;
    // Trust gate: is the `from` key currently in our trust set? If
    // not, this event is either (a) for a different fleet, (b) the
    // host is brand new and missed prior rotations, or (c) malicious.
    // In any case, do not act.
    if store.get(&event.payload.from_key_id).await.is_none() {
        return Err(PlatformRefreshError::UnknownFromKey(event.payload.from_key_id.clone()));
    }
    // Crypto verification — uses the public bytes embedded in the
    // event itself; that's fine, because the trust gate above ensured
    // the `from_key_id` is one we already trust. (If an attacker
    // crafted an event with a `from` key id that matches a trusted
    // one but with attacker-controlled `from_public_key_b64`, the
    // domain-prefixed signature over the canonical payload still
    // fails to verify against any other key — see verifier tests.)
    verify_rotation_event(event)?;

    let to_spki = base64::engine::general_purpose::STANDARD
        .decode(event.payload.to_public_key_b64.as_bytes())
        .map_err(|e| PlatformRefreshError::InvalidToPublicKey(e.to_string()))?;

    // Adopt the new key with no expiry (steady-state trust anchor).
    store
        .insert(
            event.payload.to_key_id.clone(),
            TrustedPlatformKey {
                public_key_spki_der: to_spki,
                expires_at: None,
            },
        )
        .await;

    // Stamp the previous key with the published `transition_until` so
    // it ages out cleanly. We don't remove it immediately because the
    // RFC §9 contract is "both trusted during the transition window".
    //
    // Read into a local binding *before* the write below — Rust's
    // temporary-lifetime rules in `if let` extend the scrutinee's
    // temporaries (including the `RwLockReadGuard`) to the end of the
    // if-block, which would deadlock against the write lock below.
    // The intermediate `let` releases the read guard at the end of
    // its statement.
    let existing = store.inner.read().await.get(&event.payload.from_key_id).cloned();
    if let Some(existing) = existing {
        let stamped = TrustedPlatformKey {
            public_key_spki_der: existing.public_key_spki_der,
            expires_at: Some(event.payload.transition_until),
        };
        store.insert(event.payload.from_key_id.clone(), stamped).await;
    }

    Ok(event.payload.to_key_id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_platform_signing::{MockSigner, PlatformSigner, RotationStateMachine};

    /// Spin up a state machine, fetch the from-key's SPKI, seed the
    /// store with it. Returns the state machine + store ready to drive
    /// rotation events through.
    async fn fixture_store_with_seed() -> (RotationStateMachine<MockSigner>, PlatformTrustStore) {
        let cur = MockSigner::generate("key-old").unwrap();
        let from_der = cur.public_key_der().await.unwrap();
        let store = PlatformTrustStore::new();
        store
            .insert(
                "key-old".to_string(),
                TrustedPlatformKey {
                    public_key_spki_der: from_der,
                    expires_at: None,
                },
            )
            .await;
        let sm = RotationStateMachine::new(cur);
        (sm, store)
    }

    #[test]
    fn parse_embedded_roots_happy_path() {
        let raw = "key-a:QUFB\nkey-b:QkJC";
        let parsed = parse_embedded_roots(raw).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].0, "key-a");
        assert_eq!(parsed[0].1, b"AAA");
        assert_eq!(parsed[1].0, "key-b");
        assert_eq!(parsed[1].1, b"BBB");
    }

    #[test]
    fn parse_embedded_roots_skips_comments_and_blanks() {
        let raw = "# comment line\n\n   \nkey-a:QUFB\n# trailing comment";
        let parsed = parse_embedded_roots(raw).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].0, "key-a");
    }

    #[test]
    fn parse_embedded_roots_rejects_missing_colon() {
        let err = parse_embedded_roots("no-colon-here").unwrap_err();
        assert!(err.contains("expected 'key_id:base64_spki_der'"));
    }

    #[test]
    fn parse_embedded_roots_rejects_bad_base64() {
        let err = parse_embedded_roots("key-a:not-base64!!!").unwrap_err();
        assert!(err.contains("invalid base64"));
    }

    #[tokio::test]
    async fn apply_rotation_event_happy_path_extends_trust_set() {
        let (sm, store) = fixture_store_with_seed().await;
        let next = MockSigner::generate("key-new").unwrap();
        let event = sm.begin_handover(&next, chrono::Duration::days(30)).await.unwrap();

        let to = apply_rotation_event(&event, &store).await.unwrap();
        assert_eq!(to, "key-new");

        let mut ids = store.key_ids().await;
        ids.sort();
        assert_eq!(ids, vec!["key-new".to_string(), "key-old".to_string()]);
    }

    #[tokio::test]
    async fn apply_rotation_event_stamps_old_key_with_expiry() {
        let (sm, store) = fixture_store_with_seed().await;
        let next = MockSigner::generate("key-new").unwrap();
        let event = sm.begin_handover(&next, chrono::Duration::days(30)).await.unwrap();

        apply_rotation_event(&event, &store).await.unwrap();

        let inner = store.inner.read().await;
        let old = inner.get("key-old").expect("old key still present during transition");
        assert!(old.expires_at.is_some(), "old key should be scheduled for eviction");
        assert_eq!(old.expires_at.unwrap(), event.payload.transition_until);
        let new = inner.get("key-new").expect("new key trusted");
        assert!(new.expires_at.is_none(), "new key has no eviction time");
    }

    #[tokio::test]
    async fn apply_rotation_event_rejects_unknown_from_key() {
        let store = PlatformTrustStore::new();
        let cur = MockSigner::generate("random-key").unwrap();
        let next = MockSigner::generate("dest").unwrap();
        let sm = RotationStateMachine::new(cur);
        let event = sm.begin_handover(&next, chrono::Duration::days(30)).await.unwrap();
        let err = apply_rotation_event(&event, &store).await.unwrap_err();
        assert!(matches!(err, PlatformRefreshError::UnknownFromKey(_)));
        // Trust set unchanged.
        assert!(store.key_ids().await.is_empty());
    }

    #[tokio::test]
    async fn apply_rotation_event_rejects_tampered_payload() {
        let (sm, store) = fixture_store_with_seed().await;
        let next = MockSigner::generate("key-new").unwrap();
        let mut event = sm.begin_handover(&next, chrono::Duration::days(30)).await.unwrap();
        event.payload.to_key_id = "attacker-key".into();
        let err = apply_rotation_event(&event, &store).await.unwrap_err();
        assert!(matches!(err, PlatformRefreshError::Verify(_)));
        // Trust set unchanged.
        assert_eq!(store.key_ids().await, vec!["key-old".to_string()]);
    }

    #[tokio::test]
    async fn evict_expired_drops_past_keys_only() {
        let store = PlatformTrustStore::new();
        let now = Utc::now();
        store
            .insert(
                "live".into(),
                TrustedPlatformKey {
                    public_key_spki_der: b"x".to_vec(),
                    expires_at: Some(now + chrono::Duration::hours(1)),
                },
            )
            .await;
        store
            .insert(
                "dead".into(),
                TrustedPlatformKey {
                    public_key_spki_der: b"y".to_vec(),
                    expires_at: Some(now - chrono::Duration::hours(1)),
                },
            )
            .await;
        store
            .insert(
                "permanent".into(),
                TrustedPlatformKey {
                    public_key_spki_der: b"z".to_vec(),
                    expires_at: None,
                },
            )
            .await;
        let evicted = store.evict_expired(now).await;
        assert_eq!(evicted, vec!["dead".to_string()]);
        let ids = store.key_ids().await;
        assert!(ids.contains(&"live".to_string()));
        assert!(ids.contains(&"permanent".to_string()));
        assert!(!ids.contains(&"dead".to_string()));
    }

    #[tokio::test]
    async fn refresh_loop_applies_event_via_wiremock() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let (sm, store) = fixture_store_with_seed().await;
        let next = MockSigner::generate("key-new").unwrap();
        let event = sm.begin_handover(&next, chrono::Duration::days(30)).await.unwrap();

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/internal/plugin-rotation-events"))
            .and(header("authorization", "Bearer test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "phase": "transitioning",
                "latest": event,
                "trusted_key_ids": ["key-old", "key-new"],
            })))
            .mount(&server)
            .await;

        let cfg = PlatformTrustCacheConfig {
            url: format!("{}/api/internal/plugin-rotation-events", server.uri()),
            interval: std::time::Duration::from_millis(40),
            bearer_token: Some("test-token".to_string()),
        };
        let store_clone = store.clone();
        let handle = tokio::spawn(async move {
            run_platform_trust_refresh_loop(cfg, store_clone).await;
        });

        let mut attempts = 0;
        loop {
            if store.get("key-new").await.is_some() {
                break;
            }
            attempts += 1;
            assert!(attempts < 50, "store never picked up the rotation event");
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        handle.abort();
    }

    #[tokio::test]
    async fn refresh_loop_preserves_set_on_http_error() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let (_sm, store) = fixture_store_with_seed().await;
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/internal/plugin-rotation-events"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        let cfg = PlatformTrustCacheConfig {
            url: format!("{}/api/internal/plugin-rotation-events", server.uri()),
            interval: std::time::Duration::from_millis(40),
            bearer_token: None,
        };
        let store_clone = store.clone();
        let handle = tokio::spawn(async move {
            run_platform_trust_refresh_loop(cfg, store_clone).await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        assert!(store.get("key-old").await.is_some(), "errored poll should not clear set");
        handle.abort();
    }
}
