# Cloud Tunnel Relay Deployment

This doc covers deploying the existing `mockforge-tunnel` binary as a Fly app
that validates incoming subdomain claims against the cloud registry's
`tunnel_reservations` table.

The registry side of #5 is fully done (CRUD + DNS verification + UI).
This doc covers the operational side: standing up the relay so real
traffic can flow through reserved subdomains.

## Architecture

```
┌──────────────┐       ┌──────────────────┐       ┌────────────────────┐
│  Public      │  HTTPS│   tunnel relay   │ HTTPS │  user's local      │
│  internet    │──────▶│   (Fly app)      │──────▶│  mockforge server  │
│              │       │                  │       │  (via WebSocket)   │
└──────────────┘       └─────────┬────────┘       └────────────────────┘
                                 │
                  validates subdomain ownership
                                 │
                                 ▼
                       ┌──────────────────┐
                       │  registry        │  GET /api/v1/internal/
                       │  (mockforge-     │      tunnel-reservations/
                       │   registry)      │      by-subdomain/{slug}
                       └──────────────────┘
```

## Internal API contract

The registry exposes a single endpoint the relay needs:

```
GET /api/v1/internal/tunnel-reservations/by-subdomain/{subdomain}
Authorization: Bearer ${MOCKFORGE_INTERNAL_API_TOKEN}

200 OK
{
  "id": "uuid",
  "org_id": "uuid",
  "name": "Payment Service Dev",
  "subdomain": "payment-dev",
  "custom_domain": "api.example.com",
  "custom_domain_verified": true,
  "status": "reserved"
}

400 Invalid
"Subdomain not reserved"
```

Implementation: `mockforge-registry-server::handlers::internal_test_runs::get_tunnel_reservation_by_subdomain`.

The relay should reject incoming connections (or HTTP requests by Host
header) when this endpoint returns 400. For custom domains, the relay
should additionally check `custom_domain_verified == true` before
routing.

## Deploying the relay on Fly

```bash
# 1. Create a new Fly app for the relay.
fly apps create mockforge-tunnel-relay

# 2. Build + push the relay image.
docker build \
  -f Dockerfile.tunnel \
  -t registry.fly.io/mockforge-tunnel-relay:latest .
fly deploy -a mockforge-tunnel-relay

# 3. Wire it to the registry over Fly's internal network.
fly secrets set -a mockforge-tunnel-relay \
  REGISTRY_URL=http://mockforge-registry.internal:8080 \
  MOCKFORGE_INTERNAL_API_TOKEN=$(fly secrets list -a mockforge-registry \
    | grep MOCKFORGE_INTERNAL_API_TOKEN | awk '{print $1}')

# 4. Point a wildcard CNAME at the relay.
#    *.tunnels.mockforge.dev  CNAME  mockforge-tunnel-relay.fly.dev.
#    Then attach the cert via Fly's Let's Encrypt automation:
fly certs create -a mockforge-tunnel-relay '*.tunnels.mockforge.dev'
```

After deploy, the relay needs a small wrapper around `mockforge-tunnel`'s
existing `TunnelStoreTrait` that delegates to the registry on first
lookup of a subdomain — see "Integration sketch" below.

## Integration sketch

In `mockforge-tunnel`, add a `RegistryTunnelStore` that wraps
`InMemoryTunnelStore` and consults the registry via the internal API
the first time it sees a new subdomain:

```rust
pub struct RegistryTunnelStore {
    inner: InMemoryTunnelStore,
    registry_url: String,
    token: String,
    http: reqwest::Client,
}

#[async_trait]
impl TunnelStoreTrait for RegistryTunnelStore {
    async fn get_tunnel_by_subdomain(&self, subdomain: &str) -> Result<TunnelStatus> {
        // Cache hit: serve from memory.
        if let Ok(t) = self.inner.get_tunnel_by_subdomain(subdomain).await {
            return Ok(t);
        }

        // Cache miss: ask the registry.
        let url = format!(
            "{}/api/v1/internal/tunnel-reservations/by-subdomain/{}",
            self.registry_url.trim_end_matches('/'),
            subdomain
        );
        let resp = self.http.get(&url).bearer_auth(&self.token).send().await?;
        if !resp.status().is_success() {
            return Err(TunnelError::NotFound(subdomain.into()));
        }
        let row: serde_json::Value = resp.json().await?;
        if row["status"] != "reserved" {
            return Err(TunnelError::NotFound(subdomain.into()));
        }
        // Materialize into the in-memory store so subsequent lookups
        // skip the network round-trip. TTL the cache entry to a few
        // minutes so revoked reservations stop routing.
        // ... (cache populate)

        self.inner.get_tunnel_by_subdomain(subdomain).await
    }

    // Other trait methods delegate to inner.
}
```

Wire it from the relay binary's main:

```rust
let store = if let Ok(registry_url) = std::env::var("REGISTRY_URL") {
    let token = std::env::var("MOCKFORGE_INTERNAL_API_TOKEN")
        .expect("MOCKFORGE_INTERNAL_API_TOKEN required when REGISTRY_URL is set");
    Arc::new(RegistryTunnelStore::new(InMemoryTunnelStore::new(), registry_url, token))
        as Arc<dyn TunnelStoreTrait>
} else {
    Arc::new(InMemoryTunnelStore::new()) as Arc<dyn TunnelStoreTrait>
};
```

## What's NOT in this slice

- The `RegistryTunnelStore` impl above is the integration sketch — it's
  not in `mockforge-tunnel/src/` yet because the existing `tunnel-server`
  binary is a self-hostable product and we don't want to bake registry
  dependence into its default path. A follow-up slice should add it
  behind a `cloud-validation` cargo feature.
- mTLS between relay and registry. The bearer token is enough for the
  internal Fly network for now.
- Per-subdomain rate-limiting against the cloud's `usage_counters`.
  `tunnel_bytes_used` exists as a meter but the relay doesn't yet
  report bytes back through the registry's internal API.
