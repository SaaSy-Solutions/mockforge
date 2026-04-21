-- Federation-wide scenario activations.
--
-- A federation can have at most one active scenario at a time. The activation
-- row snapshots the scenario manifest + per-service overrides so that
-- deactivation/rollback can proceed even if the scenario row is later edited
-- or deleted.
--
-- Workspace runtimes poll for their active scenarios by joining via the
-- federation's `services` JSONB (each service row has a `workspace_id`).

ALTER TYPE feature_type ADD VALUE IF NOT EXISTS 'federation_scenario_activate';
ALTER TYPE feature_type ADD VALUE IF NOT EXISTS 'federation_scenario_deactivate';

ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'federation_scenario_activated';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'federation_scenario_deactivated';

CREATE TABLE IF NOT EXISTS federation_scenario_activations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    federation_id UUID NOT NULL REFERENCES federations(id) ON DELETE CASCADE,
    scenario_id UUID REFERENCES scenarios(id) ON DELETE SET NULL,
    scenario_name VARCHAR(255) NOT NULL,
    manifest_snapshot JSONB NOT NULL,
    service_overrides JSONB NOT NULL DEFAULT '{}'::jsonb,
    status VARCHAR(32) NOT NULL DEFAULT 'active',
    per_service_state JSONB NOT NULL DEFAULT '[]'::jsonb,
    activated_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    activated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deactivated_at TIMESTAMPTZ,

    CHECK (status IN ('active', 'deactivated', 'failed'))
);

-- At most one active row per federation.
CREATE UNIQUE INDEX IF NOT EXISTS idx_fed_scenario_activations_one_active
    ON federation_scenario_activations(federation_id)
    WHERE status = 'active';

CREATE INDEX IF NOT EXISTS idx_fed_scenario_activations_federation
    ON federation_scenario_activations(federation_id, activated_at DESC);

CREATE INDEX IF NOT EXISTS idx_fed_scenario_activations_scenario
    ON federation_scenario_activations(scenario_id)
    WHERE scenario_id IS NOT NULL;
