-- Cloud Tunnels (cloud-enablement task #5 / Phase 1).
--
-- Persistent state for the managed tunnel relay product. Reservations are
-- subdomain claims (with optional custom-domain attachment); sessions are
-- the bandwidth/request roll-ups for each connect-disconnect cycle. The
-- relay binary (separate deployment, future slice) writes session rows
-- and bumps usage_counters.tunnel_bytes_used via internal mTLS routes.
--
-- See docs/cloud/CLOUD_TUNNELS_DESIGN.md for the full design.

CREATE TABLE IF NOT EXISTS tunnel_reservations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
    name TEXT NOT NULL,                          -- human label
    subdomain TEXT NOT NULL,                     -- e.g. "stage-api" → stage-api.t.mockforge.dev
    custom_domain TEXT,                          -- e.g. "api-stage.example.com"
    custom_domain_verified BOOLEAN NOT NULL DEFAULT FALSE,
    custom_domain_verified_at TIMESTAMPTZ,
    status TEXT NOT NULL DEFAULT 'reserved',     -- 'reserved' | 'active' | 'disabled'
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
-- Subdomains are globally unique on the relay zone.
CREATE UNIQUE INDEX IF NOT EXISTS idx_tunnel_reservations_subdomain
    ON tunnel_reservations(subdomain);
-- Custom domains are globally unique when present.
CREATE UNIQUE INDEX IF NOT EXISTS idx_tunnel_reservations_custom_domain
    ON tunnel_reservations(custom_domain)
    WHERE custom_domain IS NOT NULL;
-- Org-scoped list view.
CREATE INDEX IF NOT EXISTS idx_tunnel_reservations_org
    ON tunnel_reservations(org_id, created_at DESC);

CREATE TABLE IF NOT EXISTS tunnel_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    reservation_id UUID NOT NULL REFERENCES tunnel_reservations(id) ON DELETE CASCADE,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMPTZ,
    client_ip INET,
    bytes_in BIGINT NOT NULL DEFAULT 0,
    bytes_out BIGINT NOT NULL DEFAULT 0,
    request_count BIGINT NOT NULL DEFAULT 0
);
-- Recent-sessions view per reservation; the table can grow large so we
-- index by (reservation_id, started_at DESC) to keep the dashboard
-- query well-bounded.
CREATE INDEX IF NOT EXISTS idx_tunnel_sessions_reservation
    ON tunnel_sessions(reservation_id, started_at DESC);
-- Index on still-active sessions (NULL ended_at) — the relay reads this
-- on reconnect to avoid double-charging an interrupted cycle.
CREATE INDEX IF NOT EXISTS idx_tunnel_sessions_active
    ON tunnel_sessions(reservation_id)
    WHERE ended_at IS NULL;

-- New billing meter mirroring runner_seconds_used / ai_tokens_used.
ALTER TABLE usage_counters
    ADD COLUMN IF NOT EXISTS tunnel_bytes_used BIGINT NOT NULL DEFAULT 0;
