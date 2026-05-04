-- Incidents (cloud-enablement task #3 / Phase 1).
--
-- Persisted incident management surface. Sources (drift, observability
-- alerts, hosted-mock health, external webhooks) raise incidents through
-- an internal `IncidentBus` trait; this schema is the durable backing.
--
-- The dedupe_key is source-scoped (e.g., contract drift uses
-- "endpoint:method"; obs alert uses "saved_query_id") so noisy sources
-- collapse repeat fires onto a single open row. The partial-unique index
-- enforces "at most one open incident per (org, source, dedupe_key)".

CREATE TABLE IF NOT EXISTS incidents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
    source TEXT NOT NULL,                       -- 'drift' | 'observability' | 'hosted_mock_health' | 'external'
    source_ref TEXT,                            -- e.g., contract_diff_run_id
    dedupe_key TEXT NOT NULL,                   -- noise-collapse handle, see docs
    severity TEXT NOT NULL,                     -- 'critical' | 'high' | 'medium' | 'low'
    status TEXT NOT NULL,                       -- 'open' | 'acknowledged' | 'resolved'
    title TEXT NOT NULL,
    description TEXT,
    postmortem_url TEXT,
    assigned_to UUID REFERENCES users(id) ON DELETE SET NULL,
    acknowledged_at TIMESTAMPTZ,
    acknowledged_by UUID REFERENCES users(id) ON DELETE SET NULL,
    resolved_at TIMESTAMPTZ,
    resolved_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
-- Most-frequent index: open-incident list per org by recency.
CREATE INDEX IF NOT EXISTS idx_incidents_org_status
    ON incidents(org_id, status, created_at DESC);
-- Workspace filter for the per-workspace view.
CREATE INDEX IF NOT EXISTS idx_incidents_workspace
    ON incidents(workspace_id)
    WHERE workspace_id IS NOT NULL;
-- Dedupe constraint: at most one open incident per (org, source, dedupe_key).
-- Partial-unique so resolved rows stay around for history.
CREATE UNIQUE INDEX IF NOT EXISTS idx_incidents_open_dedupe
    ON incidents(org_id, source, dedupe_key)
    WHERE status != 'resolved';

CREATE TABLE IF NOT EXISTS incident_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    incident_id UUID NOT NULL REFERENCES incidents(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,                   -- 'created' | 'acknowledged' | 'commented' | 'resolved' | 'reopened' | 'notification_sent'
    actor_id UUID REFERENCES users(id) ON DELETE SET NULL,
    payload JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_incident_events_incident
    ON incident_events(incident_id, created_at DESC);

CREATE TABLE IF NOT EXISTS notification_channels (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,                         -- 'email' | 'slack' | 'pagerduty' | 'webhook'
    config JSONB NOT NULL,                      -- encrypted secrets via settings::encrypt_api_key
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_notification_channels_org
    ON notification_channels(org_id)
    WHERE enabled = TRUE;

CREATE TABLE IF NOT EXISTS routing_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    priority INTEGER NOT NULL,                  -- lower = evaluated first
    match_severity TEXT[] NOT NULL DEFAULT '{}', -- empty = match all
    match_source TEXT[] NOT NULL DEFAULT '{}',
    match_workspace_id UUID REFERENCES workspaces(id) ON DELETE CASCADE,
    channel_ids UUID[] NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_routing_rules_org_priority
    ON routing_rules(org_id, priority);
