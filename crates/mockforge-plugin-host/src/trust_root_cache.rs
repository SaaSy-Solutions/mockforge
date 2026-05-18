//! Per-org trust-root cache — closes the security loop opened by
//! issue #416 (control-plane API to manage trust roots) by wiring
//! the plugin-host to actually fetch and honor that set at runtime
//! (issue #549).
//!
//! ## What this does
//!
//! On host boot the cache fetches `GET /api/v1/organizations/{org_id}
//! /trust-roots` from the registry and rebuilds the shared
//! [`TrustStore`] with the active (non-revoked) Ed25519 public keys.
//! It then refreshes on a configurable tick (default 60s, matching
//! the blocklist poller in §8.2 of the trust RFC).
//!
//! ## Why polling, not push
//!
//! Same reasoning as the blocklist (see `blocklist.rs`): trust-root
//! revocations are rare (key rotation, compromise response), the
//! latency target is minutes-to-restart, and pushing requires a
//! long-lived control-plane connection per host. Polling is
//! operationally boring and reuses the IPC the host already speaks
//! to the registry for the kill-switch.
//!
//! ## Failure semantics
//!
//! A failed poll **does not** clear the trust store — a registry
//! outage shouldn't open a security hole by silently dropping every
//! signature check. The last-known set stays in effect until the
//! next successful refresh. This mirrors the blocklist's behavior
//! and is required for the "fail-closed" property of
//! [`SignatureMode::Required`]: empty store + Required = reject all,
//! so an outage that empties the store would brick every load.
//!
//! ## Wire format
//!
//! The registry returns:
//!
//! ```json
//! {
//!   "trustRoots": [
//!     {
//!       "id": "...",
//!       "orgId": "...",
//!       "publicKeyB64": "<base64>",
//!       "name": "...",
//!       "active": true,
//!       "revokedAt": null
//!     }
//!   ]
//! }
//! ```
//!
//! Revoked entries (`active: false` / `revokedAt: Some(_)`) are
//! returned by the API for audit-history rendering but the cache
//! filters them out — the plugin-host only cares about the active
//! set.
//!
//! [`SignatureMode::Required`]: crate::signing::SignatureMode::Required

use std::collections::HashMap;

use ed25519_dalek::VerifyingKey;
use serde::Deserialize;

use crate::signing::{decode_ed25519_key, TrustStore, TrustStoreError};

/// Default poll interval — matches the blocklist's RFC §8.2 cadence.
pub const DEFAULT_REFRESH_INTERVAL_SECS: u64 = 60;

/// Configuration for the trust-root refresh task.
#[derive(Debug, Clone)]
pub struct TrustRootCacheConfig {
    /// Fully-qualified URL to poll. The path must already include
    /// `/api/v1/organizations/{org_id}/trust-roots`. We keep the
    /// org id in the URL (rather than as a separate field) so the
    /// caller — typically `main.rs` reading env vars — owns the
    /// composition; the cache itself stays org-agnostic.
    pub url: String,
    /// Poll interval. Default 60s.
    pub interval: std::time::Duration,
    /// Optional bearer token; sent as `Authorization: Bearer ...`.
    /// Cloud production uses a service-account token here so the
    /// registry can audit which host pulled the list.
    pub bearer_token: Option<String>,
}

impl TrustRootCacheConfig {
    /// Construct with the default interval and no bearer token.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            interval: std::time::Duration::from_secs(DEFAULT_REFRESH_INTERVAL_SECS),
            bearer_token: None,
        }
    }
}

/// Errors the refresh task can hit. Mirrors `blocklist::PollError`
/// in shape — each variant is logged + retried on the next tick;
/// we never give up because a stale trust store is preferable to no
/// store, but a fresh one is preferable still.
#[derive(Debug, thiserror::Error)]
pub enum RefreshError {
    /// HTTP request failed.
    #[error("trust-root HTTP error: {0}")]
    Http(String),
    /// Response body wasn't valid JSON or didn't match the expected
    /// shape.
    #[error("trust-root parse error: {0}")]
    Parse(String),
    /// A returned entry couldn't be decoded into a valid Ed25519
    /// key. Logged but the rest of the response is still applied —
    /// one bad key shouldn't black-hole the whole refresh.
    #[error("trust-root key decode error: {0}")]
    KeyDecode(#[from] TrustStoreError),
}

/// JSON shape returned by `GET /api/v1/organizations/{org_id}/trust-roots`.
/// Mirrors `mockforge_registry_server::handlers::trust_roots::ListTrustRootsResponse`
/// but we keep our own deserializer so we don't have to pull the
/// registry-server crate into the host (it owns the whole HTTP
/// stack — pulling it in for a single type would blow up the
/// build).
#[derive(Debug, Deserialize)]
struct ListTrustRootsResponse {
    #[serde(rename = "trustRoots")]
    trust_roots: Vec<TrustRootEntry>,
}

#[derive(Debug, Deserialize)]
struct TrustRootEntry {
    /// Registry-assigned UUID. Used as the publisher key id the
    /// LoadPlugin request carries — keeps the wire identity stable
    /// even if the human-readable `name` changes.
    id: String,
    /// Human-readable label; surfaced in logs for operators.
    name: String,
    #[serde(rename = "publicKeyB64")]
    public_key_b64: String,
    /// Convenience flag from the API; equivalent to
    /// `revoked_at.is_none()`. We trust the server's computation
    /// rather than recomputing locally.
    active: bool,
}

/// Run the refresh task forever. Returns only on shutdown signal —
/// the `select!` in `main.rs` is responsible for cancelling.
///
/// Drives `tokio::time::interval` with `Delay` missed-tick behavior
/// so a slow poll doesn't double-fire on the next tick; same
/// approach as `blocklist::run_poll_loop`.
pub async fn run_trust_root_refresh_loop(
    config: TrustRootCacheConfig,
    store: TrustStore,
    on_refresh: impl Fn(Vec<String>) + Send + 'static,
) {
    let client =
        match reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).build() {
            Ok(c) => c,
            Err(err) => {
                tracing::error!(
                    error = %err,
                    "failed to build trust-root HTTP client; refresh task exiting"
                );
                return;
            }
        };

    tracing::info!(
        url = %config.url,
        interval_secs = config.interval.as_secs(),
        "trust-root refresh loop starting"
    );

    let mut ticker = tokio::time::interval(config.interval);
    // Skip-the-first-tick semantics so we poll *immediately*, not
    // after one full interval — boot-time fetch is the headline
    // feature of issue #549.
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        ticker.tick().await;
        match fetch_trust_roots(&client, &config).await {
            Ok(new_keys) => {
                let active_count = new_keys.len();
                let removed = store.replace(new_keys);
                if !removed.is_empty() {
                    tracing::warn!(
                        removed_key_ids = ?removed,
                        active_keys = active_count,
                        "trust roots removed on refresh — invoking on_refresh hook"
                    );
                    on_refresh(removed);
                } else {
                    tracing::debug!(
                        active_keys = active_count,
                        "trust-root refresh applied (no removals)"
                    );
                }
            }
            Err(err) => {
                // Don't update on error — preserve the last-known
                // store. See module docs for why this is the
                // security-correct behavior.
                tracing::warn!(
                    error = %err,
                    "trust-root poll failed; keeping last-known set"
                );
            }
        }
    }
}

/// Fetch + parse + decode the trust-root list. Pure data plumbing —
/// the side-effecting [`run_trust_root_refresh_loop`] uses this for
/// the actual GET on each tick.
async fn fetch_trust_roots(
    client: &reqwest::Client,
    config: &TrustRootCacheConfig,
) -> Result<HashMap<String, VerifyingKey>, RefreshError> {
    let mut req = client.get(&config.url);
    if let Some(token) = &config.bearer_token {
        req = req.bearer_auth(token);
    }
    let response = req.send().await.map_err(|err| RefreshError::Http(err.to_string()))?;
    if !response.status().is_success() {
        return Err(RefreshError::Http(format!("status {}", response.status())));
    }
    let body: ListTrustRootsResponse = response
        .json::<ListTrustRootsResponse>()
        .await
        .map_err(|err| RefreshError::Parse(err.to_string()))?;
    Ok(build_active_key_map(body.trust_roots))
}

/// Project the API response into the `(key_id → VerifyingKey)` map
/// the [`TrustStore`] expects. Skips revoked entries and logs (but
/// does not propagate) per-entry decode errors so one malformed row
/// can't black-hole the whole refresh.
fn build_active_key_map(entries: Vec<TrustRootEntry>) -> HashMap<String, VerifyingKey> {
    let mut keys = HashMap::with_capacity(entries.len());
    for entry in entries {
        if !entry.active {
            tracing::debug!(
                key_id = %entry.id,
                name = %entry.name,
                "skipping revoked trust root"
            );
            continue;
        }
        match decode_ed25519_key(&entry.id, &entry.public_key_b64) {
            Ok(key) => {
                keys.insert(entry.id, key);
            }
            Err(err) => {
                // Log + skip — one malformed key shouldn't poison
                // the whole refresh. The control plane should
                // never produce an invalid key (it validates on
                // create), so this surfacing in a real deployment
                // is a registry bug worth surfacing in metrics.
                tracing::warn!(
                    key_id = %entry.id,
                    name = %entry.name,
                    error = %err,
                    "skipping trust root with invalid key"
                );
            }
        }
    }
    keys
}

/// Validate that the URL the operator configured at least matches
/// the shape we expect (`.../trust-roots`). Cheap defense against a
/// typo pointing the host at an unrelated endpoint that happens to
/// return JSON. Called once at boot from `main.rs`.
///
/// Returns `Ok(())` if the path ends in `/trust-roots`, otherwise an
/// error describing the mismatch. Not security-critical (a wrong
/// URL fails at parse time anyway) but the upfront error is much
/// nicer to debug than "all my refreshes silently empty the store".
pub fn validate_trust_roots_url(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|err| format!("invalid URL: {err}"))?;
    if !parsed.path().contains("/trust-roots") {
        return Err(format!(
            "trust-root URL path '{}' does not contain '/trust-roots'; expected something like \
             '/api/v1/organizations/{{org_id}}/trust-roots'",
            parsed.path()
        ));
    }
    Ok(())
}

/// Test-only base64 encoder. Lives in the module rather than each
/// test fn so the integration test (`tests/trust_root_lifecycle.rs`)
/// can reuse it without re-importing the base64 engine.
#[cfg(test)]
fn b64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    fn fixture_vk(seed: u8) -> VerifyingKey {
        let mut bytes = [0u8; 32];
        bytes[0] = seed;
        SigningKey::from_bytes(&bytes).verifying_key()
    }

    #[test]
    fn config_defaults_to_60s_interval() {
        let cfg = TrustRootCacheConfig::new("https://example/api/v1/organizations/x/trust-roots");
        assert_eq!(cfg.interval.as_secs(), DEFAULT_REFRESH_INTERVAL_SECS);
        assert!(cfg.bearer_token.is_none());
    }

    #[test]
    fn build_active_key_map_skips_revoked_entries() {
        let vk = fixture_vk(1);
        let entries = vec![
            TrustRootEntry {
                id: "active".to_string(),
                name: "good".to_string(),
                public_key_b64: b64_encode(vk.as_bytes()),
                active: true,
            },
            TrustRootEntry {
                id: "revoked".to_string(),
                name: "stale".to_string(),
                public_key_b64: b64_encode(vk.as_bytes()),
                active: false,
            },
        ];
        let map = build_active_key_map(entries);
        assert_eq!(map.len(), 1);
        assert!(map.contains_key("active"));
        assert!(!map.contains_key("revoked"));
    }

    #[test]
    fn build_active_key_map_skips_unparsable_keys_without_failing() {
        let entries = vec![
            TrustRootEntry {
                id: "ok".to_string(),
                name: "good".to_string(),
                public_key_b64: b64_encode(fixture_vk(2).as_bytes()),
                active: true,
            },
            TrustRootEntry {
                id: "garbage".to_string(),
                name: "bad".to_string(),
                public_key_b64: "not-real-base64-!!!".to_string(),
                active: true,
            },
        ];
        // Garbage entry is silently skipped — one bad row shouldn't
        // black-hole the whole refresh.
        let map = build_active_key_map(entries);
        assert_eq!(map.len(), 1);
        assert!(map.contains_key("ok"));
    }

    #[test]
    fn list_trust_roots_response_parses_camelcase_payload() {
        let raw = r#"{
            "trustRoots": [
                {
                    "id": "11111111-1111-1111-1111-111111111111",
                    "orgId": "22222222-2222-2222-2222-222222222222",
                    "publicKeyB64": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
                    "name": "CI signing key",
                    "active": true,
                    "createdAt": "2026-01-01T00:00:00Z",
                    "createdBy": null,
                    "revokedAt": null,
                    "revokedReason": null,
                    "revokedBy": null
                }
            ]
        }"#;
        let parsed: ListTrustRootsResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.trust_roots.len(), 1);
        assert!(parsed.trust_roots[0].active);
        assert_eq!(parsed.trust_roots[0].name, "CI signing key");
    }

    #[test]
    fn validate_trust_roots_url_accepts_canonical_path() {
        let result = validate_trust_roots_url(
            "https://registry.example.com/api/v1/organizations/abc/trust-roots",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn validate_trust_roots_url_rejects_obviously_wrong_path() {
        let result = validate_trust_roots_url("https://example.com/health");
        assert!(result.is_err());
    }

    #[test]
    fn validate_trust_roots_url_rejects_garbage() {
        let result = validate_trust_roots_url("not-a-url");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn refresh_loop_applies_first_fetch_via_wiremock() {
        // End-to-end: stand up a wiremock server returning the
        // canonical response, point the cache at it, observe the
        // store get populated on the first tick.
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let vk = fixture_vk(7);
        let body = serde_json::json!({
            "trustRoots": [{
                "id": "publisher-1",
                "orgId": "00000000-0000-0000-0000-000000000000",
                "publicKeyB64": b64_encode(vk.as_bytes()),
                "name": "test key",
                "active": true,
                "createdAt": "2026-01-01T00:00:00Z",
                "createdBy": null,
                "revokedAt": null,
                "revokedReason": null,
                "revokedBy": null
            }]
        });
        Mock::given(method("GET"))
            .and(path("/api/v1/organizations/x/trust-roots"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&server)
            .await;

        let store = TrustStore::new();
        let cfg = TrustRootCacheConfig {
            url: format!("{}/api/v1/organizations/x/trust-roots", server.uri()),
            // Short interval so the test doesn't sleep for a minute
            // before the first refresh fires (the immediate-first-tick
            // semantics already cover the boot case, but the test
            // also exercises the loop after the body completes).
            interval: std::time::Duration::from_millis(50),
            bearer_token: None,
        };
        let store_clone = store.clone();
        let handle =
            tokio::spawn(
                async move { run_trust_root_refresh_loop(cfg, store_clone, |_| {}).await },
            );

        // Poll until populated — wiremock + tokio::time::interval
        // wakeups have enough jitter that a single sleep can be
        // flaky on CI.
        let mut attempts = 0;
        loop {
            if store.get("publisher-1").is_some() {
                break;
            }
            attempts += 1;
            assert!(attempts < 50, "store never populated");
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        handle.abort();
    }

    #[tokio::test]
    async fn refresh_loop_preserves_last_known_set_on_http_error() {
        // Pre-populate the store, then point at an endpoint that
        // 500s — the store should keep the old key, NOT empty out.
        // This is the "fail-closed for Required mode" guarantee.
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/organizations/x/trust-roots"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let vk = fixture_vk(9);
        let store = TrustStore::new();
        store.insert("preexisting".to_string(), vk);

        let cfg = TrustRootCacheConfig {
            url: format!("{}/api/v1/organizations/x/trust-roots", server.uri()),
            interval: std::time::Duration::from_millis(50),
            bearer_token: None,
        };
        let store_clone = store.clone();
        let handle =
            tokio::spawn(
                async move { run_trust_root_refresh_loop(cfg, store_clone, |_| {}).await },
            );

        // Give the loop a few ticks to hit the 500 — far more than
        // one interval so we know it actually polled and failed.
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        assert!(
            store.get("preexisting").is_some(),
            "failed refresh should not have cleared the store"
        );
        handle.abort();
    }

    #[tokio::test]
    async fn refresh_loop_invokes_on_refresh_when_keys_are_removed() {
        // Seed the store with an extra key, then have the server
        // return only one — the removal should trigger on_refresh
        // with the dropped key id.
        use std::sync::Mutex;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let vk = fixture_vk(3);
        let body = serde_json::json!({
            "trustRoots": [{
                "id": "kept",
                "orgId": "00000000-0000-0000-0000-000000000000",
                "publicKeyB64": b64_encode(vk.as_bytes()),
                "name": "keeper",
                "active": true,
                "createdAt": "2026-01-01T00:00:00Z",
                "createdBy": null,
                "revokedAt": null,
                "revokedReason": null,
                "revokedBy": null
            }]
        });
        Mock::given(method("GET"))
            .and(path("/api/v1/organizations/x/trust-roots"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&server)
            .await;

        let store = TrustStore::new();
        store.insert("kept".to_string(), vk);
        store.insert("revoked-out-of-band".to_string(), vk);

        let removed_log: std::sync::Arc<Mutex<Vec<Vec<String>>>> =
            std::sync::Arc::new(Mutex::new(Vec::new()));
        let removed_log_clone = removed_log.clone();

        let cfg = TrustRootCacheConfig {
            url: format!("{}/api/v1/organizations/x/trust-roots", server.uri()),
            interval: std::time::Duration::from_millis(50),
            bearer_token: None,
        };
        let store_clone = store.clone();
        let handle = tokio::spawn(async move {
            run_trust_root_refresh_loop(cfg, store_clone, move |removed| {
                removed_log_clone.lock().unwrap().push(removed);
            })
            .await
        });

        // Wait for the cache to fire on_refresh at least once.
        let mut attempts = 0;
        loop {
            if !removed_log.lock().unwrap().is_empty() {
                break;
            }
            attempts += 1;
            assert!(attempts < 50, "on_refresh hook never fired");
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let log = removed_log.lock().unwrap();
        assert!(log[0].contains(&"revoked-out-of-band".to_string()));
        assert!(store.get("revoked-out-of-band").is_none());
        assert!(store.get("kept").is_some());
        handle.abort();
    }
}
