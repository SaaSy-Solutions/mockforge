# Cloud Tunnels — Design

Cloud-enablement plan for the `tunnels` nav item. Tracks task #5 in the cloud-enablement plan.

## Goal

Move tunnels to a managed-relay product. Today `mockforge-tunnel` already includes both client and server halves; the cloud play is to operate the relay ourselves and bill on (concurrent tunnels × custom-domain × bandwidth). Direct ngrok-shaped monetization. Tier 1 because the pieces are mostly built — the missing work is operating the relay and integrating with billing.

## What exists

- **`mockforge-tunnel` crate** has client (`client.rs`), server (`server.rs`, gated by `feature = "server"`), manager, provider trait, audit, rate-limit, persistent storage.
- **`mockforge-ui` `TunnelsPage`** — local UI for managing tunnels.
- The crate already supports multiple `TunnelProvider`s (cloudflare, ngrok-style, MockForge native).

## What's missing

1. **Relay deployment.** The `feature = "server"` half of `mockforge-tunnel` isn't actually running anywhere. Cloud needs:
   - A `tunnel-relay` binary deployed on Fly.io (or wherever).
   - A wildcard DNS record (e.g., `*.t.mockforge.dev` → relay) so tunnels get auto-assigned subdomains.
   - TLS via Let's Encrypt wildcard (already supported by the server crate? — verify).
   - Custom-domain support: customer points `api-stage.example.com` → relay CNAME, we serve their tunnel.
2. **Registry integration.** No tunnel routes exist in `mockforge-registry-server`. We need:
   - CRUD for tunnel reservations (subdomain, custom domain, owner workspace).
   - Auth handshake when a local client connects to the relay (validate API token, look up reservation).
   - Per-tunnel metering (bandwidth in/out, request count).
3. **Bandwidth metering.** New `usage_counters.tunnel_bytes_used`. Increment in the relay request path; ship to registry every N seconds.
4. **Plan tiers and concurrency caps.** Free = 1 ephemeral tunnel, Pro = 3 with reserved subdomains, Team = 10 with custom domains, Enterprise = unlimited.
5. **CLI ergonomics.** `mockforge tunnel start` should auto-pick the cloud provider when the user is logged into a cloud account, otherwise fall back to local-only providers.
6. **UI cloud-mode wiring.** TunnelsPage needs to talk to `/api/v1/organizations/{org_id}/tunnels` instead of the local admin endpoints.

## Cloud architecture

```
[ Local mockforge-cli ]──tunnel:auth(api_token) ──┐
                                                   │
            +──────────────────────────────────────┘
            │   establish persistent connection
            ▼
[ Fly.io: tunnel-relay binary ] ◀── public traffic on *.t.mockforge.dev / custom domains
            │
            │   ingress request → look up tunnel by Host header
            │   forward via persistent connection
            ▼
[ Local mockforge ]
            │
            ▲
            │   response → relay → public client
            │
            └─ periodic usage report ──▶ Registry (POST /api/v1/usage/tunnel-bytes)
```

### Proposed routes

```
GET    /api/v1/organizations/{org_id}/tunnels                 # list reservations
POST   /api/v1/organizations/{org_id}/tunnels                 # create reservation (subdomain, optional custom domain)
GET    /api/v1/organizations/{org_id}/tunnels/{id}
PATCH  /api/v1/organizations/{org_id}/tunnels/{id}            # rename, attach custom domain
DELETE /api/v1/organizations/{org_id}/tunnels/{id}

POST   /api/v1/organizations/{org_id}/tunnels/{id}/test       # send synthetic ping through tunnel

# Internal (relay → registry, mTLS)
POST   /api/v1/internal/tunnels/auth                          # validate token + reservation
POST   /api/v1/internal/tunnels/{id}/usage                    # bandwidth ticks
```

## Data model

```sql
CREATE TABLE tunnel_reservations (
    id UUID PRIMARY KEY,
    org_id UUID NOT NULL REFERENCES organizations(id),
    workspace_id UUID REFERENCES workspaces(id),
    name TEXT NOT NULL,
    subdomain TEXT NOT NULL,                  -- e.g., "stage-api" → stage-api.t.mockforge.dev
    custom_domain TEXT,                       -- e.g., "api-stage.example.com"
    custom_domain_verified BOOLEAN NOT NULL DEFAULT FALSE,
    custom_domain_verified_at TIMESTAMPTZ,
    status TEXT NOT NULL,                     -- 'reserved' | 'active' | 'disabled'
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE UNIQUE INDEX tunnel_reservations_subdomain_idx ON tunnel_reservations (subdomain);
CREATE UNIQUE INDEX tunnel_reservations_custom_domain_idx ON tunnel_reservations (custom_domain) WHERE custom_domain IS NOT NULL;

CREATE TABLE tunnel_sessions (
    id UUID PRIMARY KEY,
    reservation_id UUID NOT NULL REFERENCES tunnel_reservations(id) ON DELETE CASCADE,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMPTZ,
    client_ip INET,
    bytes_in BIGINT NOT NULL DEFAULT 0,
    bytes_out BIGINT NOT NULL DEFAULT 0,
    request_count BIGINT NOT NULL DEFAULT 0
);
CREATE INDEX tunnel_sessions_reservation_idx ON tunnel_sessions (reservation_id, started_at DESC);
```

Add `usage_counters.tunnel_bytes_used BIGINT NOT NULL DEFAULT 0`.

## Custom domain verification

DNS-based verification:
1. User adds `api-stage.example.com` to a tunnel.
2. UI shows: "Add `CNAME api-stage.example.com → t.mockforge.dev` and click Verify."
3. On Verify: registry resolves the CNAME, checks it points to our zone, marks `custom_domain_verified = true`.
4. Relay only accepts traffic on a custom domain once verified.
5. TLS cert minted on-demand via Let's Encrypt HTTP-01 (relay handles ACME challenge for verified domains).

## Plan tiers

- **Free**: 1 ephemeral tunnel (random subdomain, no reservation), 1 GB/month bandwidth, no custom domain. Resets monthly.
- **Pro**: 3 reserved subdomains, 50 GB/month, 1 custom domain.
- **Team**: 10 reserved subdomains, 500 GB/month, 5 custom domains.
- **Enterprise**: unlimited subdomains, custom bandwidth quota, unlimited custom domains, optional dedicated relay region.

## CLI ergonomics

```
$ mockforge tunnel start --port 3000
✓ Authenticated as ray@example.com (org: acme-inc)
✓ Reservation: stage-api (stage-api.t.mockforge.dev)
✓ Connected · forwarding stage-api.t.mockforge.dev → localhost:3000

[ctrl-c to stop]
```

Auto-flow:
1. `mockforge tunnel start` checks `~/.mockforge/credentials.json`.
2. If logged in: auto-pick the cloud provider; if no reservation, create an ephemeral one.
3. If logged out: fall back to local providers (cloudflare/ngrok-style as before).

Reservations let `mockforge tunnel start` re-attach to the same subdomain across runs — critical for setting up callbacks/webhooks during development.

## UI changes

1. `AppShell.tsx:217` — add `'tunnels'` to `cloudNavItemIds`.
2. **TunnelsPage rewrite for cloud mode**: list reservations (not active sessions), with a "Live now / Idle" indicator per reservation.
3. **Reservation editor**: name, subdomain, custom domain field with Verify-DNS step.
4. **Bandwidth indicator** in page header: "32 GB / 50 GB used this month."
5. **Session history table**: per reservation, last 30 sessions with bytes/requests/duration.
6. **Test-ping button**: round-trips a synthetic request to verify the tunnel is wired up end-to-end.

## Effort estimate

| Phase | Scope | Estimate |
|-------|-------|----------|
| 1 | Schema migration (reservations, sessions, tunnel_bytes_used) | ~1 day |
| 2 | CRUD handlers + reservation-claim auth | ~1.5 days |
| 3 | Internal auth/usage endpoints (relay → registry mTLS) | ~1 day |
| 4 | Deploy `tunnel-relay` to Fly.io with wildcard DNS + TLS | ~2.5 days |
| 5 | Bandwidth metering loop in relay + ship-to-registry batching | ~1 day |
| 6 | Custom domain verification + on-demand cert minting | ~2 days |
| 7 | CLI auto-flow (auth → reservation → connect) | ~1.5 days |
| 8 | UI rewrite for cloud mode | ~2 days |
| 9 | E2E (CLI start → relay route → custom domain → metering → quota) | ~1.5 days |

Total: ~14 working days for v1.

## Decisions

### Subdomain reservation lifetime

**Decision: subdomains are reserved for the org's lifetime once paid; expire after 30 days of inactivity on Free.** Prevents subdomain squatting on Free; gives paid customers stable URLs they can hardcode in webhook configs.

### Bandwidth as the primary meter, not session count

**Decision: bytes in + bytes out, summed.** Concurrent-session caps handle abuse (someone running 1000 idle tunnels); bandwidth handles cost. Don't double-meter.

### Use ports/routing built into Fly Anycast vs. a custom relay

**Decision: custom relay binary.** We need control of the wire protocol (auth handshake, multiplexing) and we already have it in `mockforge-tunnel::server`. Fly is just compute; the relay logic stays ours.

## Out of scope for v1

- TCP/UDP tunnels (HTTP only).
- Per-region relay selection (single region in v1, expand later).
- Tunnel-level access control (IP allowlist, basic auth) — defer to v2.
- WebSocket-over-tunnel guarantees (should work but not officially tested).

## Open questions

1. Wildcard cert for `*.t.mockforge.dev` (single-cert simple) vs. per-tunnel certs (clean revocation but ACME volume): wildcard is fine for v1, customer custom domains use per-domain certs.
2. Free-tier ephemeral subdomains: random words (`brave-otter-1234`) or short hashes (`a1b2c3`)? Random words are friendlier; hashes are shorter for sharing.
3. Should we expose tunnel logs in the Logs page (#2 Observability)? Probably yes — it's the same `runtime_logs` shape with `source = 'tunnel'`.
