-- Observability stack — saved queries + dashboards + storage meter
-- (cloud-enablement task #2 / Phase 1).
--
-- The runtime_logs / runtime_traces / runtime_captures tables already
-- exist (migrations 20250101000051..53). This migration adds the
-- *org-scoped query infrastructure*:
-- - `usage_counters.log_bytes_ingested` — billing meter for ingest volume
-- - `observability_saved_queries` — per-org named filters
-- - `observability_dashboards` — per-org named layouts
--
-- Cross-deployment query handlers + retention worker + local-source
-- ingest land in follow-up slices. See docs/cloud/CLOUD_OBSERVABILITY_DESIGN.md.

-- New billing meter mirroring the others. log_bytes_ingested is a counter
-- (monthly throughput), unlike snapshot_bytes_stored which is a gauge.
ALTER TABLE usage_counters
    ADD COLUMN IF NOT EXISTS log_bytes_ingested BIGINT NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS observability_saved_queries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    description TEXT,
    -- 'logs' | 'traces' | 'metrics' — which surface this filter is for.
    -- Open string so future signal types don't need a schema change.
    kind TEXT NOT NULL,
    filters JSONB NOT NULL,                     -- kind-specific filter spec
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_obs_saved_queries_org_kind
    ON observability_saved_queries(org_id, kind, updated_at DESC);

CREATE TABLE IF NOT EXISTS observability_dashboards (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    description TEXT,
    layout JSONB NOT NULL,                      -- panel positions/sizes
    queries JSONB NOT NULL,                     -- saved-query refs + inline overrides
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_obs_dashboards_org
    ON observability_dashboards(org_id, updated_at DESC);
