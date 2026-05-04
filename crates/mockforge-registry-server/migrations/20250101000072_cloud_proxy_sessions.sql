-- Cloud recorder proxy — Phase 5 of the cloud-runs roadmap.
--
-- Adds a public-facing recording proxy: users create a session pinned
-- to an upstream URL, point their clients at
-- `/api/v1/cloud-runs/recorder-proxy/sess/{session_token}/...`, and the
-- registry-server forwards each request to the upstream while
-- persisting the request/response pair for later inspection.
--
-- This is a third capture source alongside the existing 'hosted' and
-- 'local' sources in `runtime_captures`. Rather than relax the
-- `deployment_id NOT NULL` FK on that table, cloud-proxy captures get
-- their own table — they don't share the hosted-mock lifecycle and
-- have a different ownership model (session, not deployment).

CREATE TABLE IF NOT EXISTS cloud_proxy_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(id) ON DELETE CASCADE,

    -- The opaque token that goes in the proxy URL. Long-random so the
    -- session ID itself authenticates incoming traffic — we cannot ask
    -- the user's clients to attach Mockforge JWTs.
    session_token TEXT NOT NULL UNIQUE,

    -- The upstream the proxy forwards to, e.g.
    -- "https://api.example.com". Must be publicly reachable; SSRF guard
    -- runs at create time.
    upstream_url TEXT NOT NULL,

    -- User-supplied label so the dashboard can list "staging API",
    -- "prod readonly", etc.
    name TEXT,

    -- Lifecycle.
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    -- nulled on DELETE; lets us hide expired/destroyed sessions from
    -- the active listing without losing capture history.
    revoked_at TIMESTAMPTZ,

    -- Cached counters refreshed on capture ingest. Gives the listing
    -- "12,345 requests captured" without a JOIN+COUNT each time.
    capture_count BIGINT NOT NULL DEFAULT 0,
    total_bytes BIGINT NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_cloud_proxy_sessions_org_active
    ON cloud_proxy_sessions(org_id, created_at DESC)
    WHERE revoked_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_cloud_proxy_sessions_token
    ON cloud_proxy_sessions(session_token)
    WHERE revoked_at IS NULL;

-- One row per request/response exchange the proxy handles.
CREATE TABLE IF NOT EXISTS cloud_proxy_captures (
    id BIGSERIAL PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES cloud_proxy_sessions(id) ON DELETE CASCADE,
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,

    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    method TEXT NOT NULL,
    path TEXT NOT NULL,
    query_string TEXT,
    request_headers TEXT NOT NULL,    -- JSON-encoded HashMap
    request_body TEXT,                -- truncated at 1 MB; encoding flagged
    request_body_encoding TEXT NOT NULL DEFAULT 'utf8',
    request_body_truncated BOOLEAN NOT NULL DEFAULT FALSE,
    request_size_bytes BIGINT NOT NULL,

    response_status INTEGER,
    response_headers TEXT,
    response_body TEXT,
    response_body_encoding TEXT,
    response_body_truncated BOOLEAN NOT NULL DEFAULT FALSE,
    response_size_bytes BIGINT,

    duration_ms BIGINT NOT NULL,
    upstream_error TEXT,              -- non-null when forward failed (network, timeout, etc.)
    client_ip TEXT
);

CREATE INDEX IF NOT EXISTS idx_cloud_proxy_captures_session_time
    ON cloud_proxy_captures(session_id, occurred_at DESC);

CREATE INDEX IF NOT EXISTS idx_cloud_proxy_captures_org_time
    ON cloud_proxy_captures(org_id, occurred_at DESC);

-- "Show me the last N 5xxs through this proxy".
CREATE INDEX IF NOT EXISTS idx_cloud_proxy_captures_session_status
    ON cloud_proxy_captures(session_id, response_status, occurred_at DESC)
    WHERE response_status >= 500;
