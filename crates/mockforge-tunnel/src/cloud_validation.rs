//! Cloud-validation wrapper for `TunnelStoreTrait` implementations
//! (cloud-enablement task #5).
//!
//! When the relay is deployed as the cloud tunnel service, subdomain
//! claims must come from the registry's `tunnel_reservations` table —
//! not the relay's own in-memory or SQLite store. This wrapper consults
//! the registry's internal API on lookups for subdomains the inner
//! store doesn't already know about, then materializes the result so
//! subsequent hits skip the network round-trip.
//!
//! Wire it from a relay binary's `main`:
//!
//! ```ignore
//! let inner = InMemoryTunnelStore::new();
//! let store = if let Ok(registry_url) = std::env::var("REGISTRY_URL") {
//!     let token = std::env::var("MOCKFORGE_INTERNAL_API_TOKEN")
//!         .expect("MOCKFORGE_INTERNAL_API_TOKEN required when REGISTRY_URL is set");
//!     Arc::new(RegistryTunnelStore::new(inner, registry_url, token))
//!         as Arc<dyn TunnelStoreTrait>
//! } else {
//!     Arc::new(inner) as Arc<dyn TunnelStoreTrait>
//! };
//! ```
//!
//! See docs/cloud/CLOUD_TUNNEL_RELAY_DEPLOYMENT.md for the deployment
//! story (Fly app + wildcard DNS + cert).

use crate::server::TunnelStoreTrait;
use crate::{Result, TunnelConfig, TunnelError, TunnelStatus};
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

/// Wraps any inner `TunnelStoreTrait` (as an `Arc<dyn ...>`) and
/// validates new subdomains against the cloud registry's
/// `tunnel_reservations` table.
///
/// Lookups that hit the inner store (warm cache) skip the registry
/// entirely. Misses fall through to the registry; a 200 response
/// materializes the entry into the inner store for the cache TTL.
pub struct RegistryTunnelStore {
    inner: Arc<dyn TunnelStoreTrait>,
    registry_url: String,
    token: String,
    http: reqwest::Client,
}

impl RegistryTunnelStore {
    /// Construct from an inner store + the registry's base URL +
    /// the shared internal-API bearer token.
    pub fn new(inner: Arc<dyn TunnelStoreTrait>, registry_url: String, token: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .user_agent("mockforge-tunnel-relay/1.0")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            inner,
            registry_url,
            token,
            http,
        }
    }

    /// Ask the registry whether a subdomain is reserved + active.
    /// Returns Ok(reservation) on hit, Err(NotFound) on miss or any
    /// transport / parse error.
    async fn fetch_from_registry(&self, subdomain: &str) -> Result<RegistryReservation> {
        let url = format!(
            "{}/api/v1/internal/tunnel-reservations/by-subdomain/{}",
            self.registry_url.trim_end_matches('/'),
            urlencoding::encode(subdomain)
        );
        let resp = self.http.get(&url).bearer_auth(&self.token).send().await.map_err(|e| {
            warn!(subdomain, error = %e, "registry tunnel lookup network error");
            TunnelError::NotFound(subdomain.to_string())
        })?;
        if !resp.status().is_success() {
            return Err(TunnelError::NotFound(subdomain.to_string()));
        }
        resp.json::<RegistryReservation>().await.map_err(|e| {
            warn!(subdomain, error = %e, "registry tunnel response parse error");
            TunnelError::NotFound(subdomain.to_string())
        })
    }
}

#[async_trait]
impl TunnelStoreTrait for RegistryTunnelStore {
    async fn create_tunnel(&self, config: &TunnelConfig) -> Result<TunnelStatus> {
        // Pre-flight: if the config requests a specific subdomain, the
        // registry has to know about it first. Anonymous random
        // subdomains can't go through this store path.
        if let Some(sub) = config.subdomain.as_deref() {
            self.fetch_from_registry(sub).await?;
        }
        self.inner.create_tunnel(config).await
    }

    async fn get_tunnel(&self, tunnel_id: &str) -> Result<TunnelStatus> {
        self.inner.get_tunnel(tunnel_id).await
    }

    async fn delete_tunnel(&self, tunnel_id: &str) -> Result<()> {
        self.inner.delete_tunnel(tunnel_id).await
    }

    async fn list_tunnels(&self) -> Vec<TunnelStatus> {
        self.inner.list_tunnels().await
    }

    async fn get_tunnel_by_subdomain(&self, subdomain: &str) -> Result<TunnelStatus> {
        // 1. Cache hit: serve from the inner store.
        if let Ok(t) = self.inner.get_tunnel_by_subdomain(subdomain).await {
            return Ok(t);
        }

        // 2. Cache miss: ask the registry.
        let reservation = self.fetch_from_registry(subdomain).await?;
        debug!(
            subdomain,
            reservation_id = %reservation.id,
            "subdomain validated against registry"
        );

        // 3. Materialize a TunnelStatus into the inner store so the
        // hot path skips the network on next request. The relay's
        // existing flow can backfill richer fields when an actual
        // client connects; for now we provide enough metadata for
        // routing.
        let status = TunnelStatus {
            tunnel_id: reservation.id.clone(),
            public_url: format!("https://{subdomain}.tunnels.mockforge.dev"),
            local_url: None,
            active: reservation.status == "reserved",
            created_at: Some(Utc::now()),
            expires_at: None,
            request_count: 0,
            bytes_transferred: 0,
        };

        // Best-effort insert. If the inner store rejects the create
        // (e.g. duplicate id), we still return the status we built.
        let cfg = TunnelConfig {
            subdomain: Some(subdomain.to_string()),
            local_url: "http://127.0.0.1:0".to_string(),
            ..Default::default()
        };
        let _ = self.inner.create_tunnel(&cfg).await;
        Ok(status)
    }

    async fn get_tunnel_by_id(&self, tunnel_id: &str) -> Result<TunnelStatus> {
        self.inner.get_tunnel_by_id(tunnel_id).await
    }

    async fn record_request(&self, tunnel_id: &str, bytes: u64) {
        self.inner.record_request(tunnel_id, bytes).await
    }

    async fn cleanup_expired(&self) -> Result<u64> {
        self.inner.cleanup_expired().await
    }
}

/// Wire shape returned by the registry's
/// `/api/v1/internal/tunnel-reservations/by-subdomain/{subdomain}`
/// endpoint. Mirrors the JSON the handler hand-builds; we don't
/// import the registry-core types here to keep this crate
/// self-contained.
#[derive(Debug, serde::Deserialize)]
struct RegistryReservation {
    id: String,
    #[serde(default)]
    #[allow(dead_code)]
    org_id: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    name: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    subdomain: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    custom_domain: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    custom_domain_verified: bool,
    status: String,
}
