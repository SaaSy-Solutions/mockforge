//! Cloud-validation wrapper for `TunnelStoreTrait` implementations
//! (cloud-enablement task #5).
//!
//! When the relay is deployed as the cloud tunnel service, subdomain
//! claims must come from the registry's `tunnel_reservations` table —
//! not the relay's own in-memory or SQLite store. Reservations alone do
//! not establish a connected client. Until authenticated client transport is
//! implemented, cloud management and forwarding refuse requests; legacy local
//! destinations are never treated as authenticated cloud connections.
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
use std::sync::Arc;
use std::time::Duration;
use tracing::warn;

/// Wraps any inner `TunnelStoreTrait` (as an `Arc<dyn ...>`) and
/// validates new subdomains against the cloud registry's
/// `tunnel_reservations` table.
///
/// Every subdomain lookup checks registry authorization. A reservation is not
/// cached as a destination, and legacy local-store entries cannot bypass that
/// check or stand in for the missing authenticated client transport.
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

    /// Ask the registry whether a subdomain is reserved.
    /// Returns Ok(()) on an active name claim, Err(NotFound) on a miss or any
    /// transport / parse error.
    async fn fetch_from_registry(&self, subdomain: &str) -> Result<()> {
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
        let reservation = resp.json::<RegistryReservation>().await.map_err(|e| {
            warn!(subdomain, error = %e, "registry tunnel response parse error");
            TunnelError::NotFound(subdomain.to_string())
        })?;
        if reservation.status != "reserved" {
            return Err(TunnelError::NotFound(subdomain.to_string()));
        }
        Ok(())
    }
}

#[async_trait]
impl TunnelStoreTrait for RegistryTunnelStore {
    fn allows_management(&self) -> bool {
        false
    }

    async fn create_tunnel(&self, _config: &TunnelConfig) -> Result<TunnelStatus> {
        Err(TunnelError::ConfigError(
            "Cloud tunnel client registration is not available".to_string(),
        ))
    }

    async fn get_tunnel(&self, tunnel_id: &str) -> Result<TunnelStatus> {
        Err(TunnelError::NotFound(tunnel_id.to_string()))
    }

    async fn delete_tunnel(&self, _tunnel_id: &str) -> Result<()> {
        Err(TunnelError::ConfigError(
            "Cloud tunnel client registration is not available".to_string(),
        ))
    }

    async fn list_tunnels(&self) -> Vec<TunnelStatus> {
        Vec::new()
    }

    async fn get_tunnel_by_subdomain(&self, subdomain: &str) -> Result<TunnelStatus> {
        // A valid reservation is a name claim, not a connected client. Neither
        // it nor a legacy unvalidated local destination authorizes forwarding.
        self.fetch_from_registry(subdomain).await?;
        Err(TunnelError::NotFound(subdomain.to_string()))
    }

    async fn get_tunnel_by_id(&self, tunnel_id: &str) -> Result<TunnelStatus> {
        Err(TunnelError::NotFound(tunnel_id.to_string()))
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
    status: String,
}
