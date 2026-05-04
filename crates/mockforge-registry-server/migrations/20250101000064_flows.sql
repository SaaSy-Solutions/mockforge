-- Scenario Studio + Orchestration unified flows resource
-- (cloud-enablement task #9 / Phase 1).
--
-- One table, four kinds: scenario | orchestration | state_machine | chain.
-- The four UIs differ but the persistence/lifecycle is identical, so they
-- share `flows` + `flow_versions`. Runs reuse the #4 test_runs table with
-- the matching `kind` value, so no separate run table is needed here.
--
-- Versioning is mandatory: every save creates a new flow_version row,
-- and `flows.current_version_id` points at the latest. Old versions
-- stay around for rollback (the design doc explicitly calls this out).
--
-- See docs/cloud/CLOUD_SCENARIO_ORCHESTRATION_DESIGN.md.

CREATE TABLE IF NOT EXISTS flows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,                         -- 'scenario' | 'orchestration' | 'state_machine' | 'chain'
    name TEXT NOT NULL,
    description TEXT,
    -- Nullable + deferred FK because flow_versions.flow_id REFERENCES flows.id —
    -- we need to insert a flow row before the first version exists. Set to the
    -- new version's id immediately after that first INSERT in the same tx.
    current_version_id UUID,
    is_published_to_marketplace BOOLEAN NOT NULL DEFAULT FALSE,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
-- Workspace + kind list view.
CREATE INDEX IF NOT EXISTS idx_flows_workspace_kind
    ON flows(workspace_id, kind, updated_at DESC);

CREATE TABLE IF NOT EXISTS flow_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    flow_id UUID NOT NULL REFERENCES flows(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,
    config JSONB NOT NULL,                      -- kind-specific definition
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (flow_id, version_number)
);
-- Latest-versions index for the rollback view.
CREATE INDEX IF NOT EXISTS idx_flow_versions_flow_recent
    ON flow_versions(flow_id, version_number DESC);

-- Wire flows.current_version_id → flow_versions.id now that both tables exist.
-- Deferred initially so the bootstrap INSERT works inside a transaction.
ALTER TABLE flows
    ADD CONSTRAINT flows_current_version_fk
    FOREIGN KEY (current_version_id) REFERENCES flow_versions(id) ON DELETE SET NULL
    DEFERRABLE INITIALLY DEFERRED;
