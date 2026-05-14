-- Virtual entities table (#461) — workspace-scoped persistent state for
-- entities materialized when a lifecycle preset is applied to a persona.
--
-- The local mockforge-core engine keeps this in an in-process HashMap
-- (`UnifiedState.entity_state`). Cloud needs durable per-workspace storage
-- so the Virtual Backends UI shows the same shape users see locally.
--
-- Schema mirrors the fields the UI's `consistencyApi.listEntities()`
-- consumes — `entity_type`, `entity_id`, `data`, `current_state`,
-- `seen_in_protocols`. Time-driven transition execution lives elsewhere
-- (or doesn't yet) — this table is the persisted state machine snapshot.

CREATE TABLE IF NOT EXISTS virtual_entities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    -- Logical entity classification. Matches the lifecycle preset id
    -- (e.g. 'subscription', 'order_fulfillment') when seeded by preset
    -- application; free-form when populated by other writers later.
    entity_type TEXT NOT NULL,
    -- Stable key within (workspace_id, entity_type). For preset-applied
    -- rows the apply handler defaults this to `{persona_id}:{entity_type}`
    -- so re-applying is idempotent.
    entity_id TEXT NOT NULL,
    -- Optional persona binding — non-null for rows seeded by preset apply.
    persona_id TEXT,
    -- Current state name from the preset's state machine (e.g. 'active',
    -- 'past_due'). Free-form text so future presets / custom state
    -- machines don't need schema migrations.
    current_state TEXT,
    -- Provenance + arbitrary user metadata.
    data JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- Which protocols have observed traffic against this entity (HTTP /
    -- gRPC / etc.). Populated by future request-side writers; the apply
    -- handler leaves this empty.
    seen_in_protocols JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (workspace_id, entity_type, entity_id)
);

-- Workspace + type list view.
CREATE INDEX IF NOT EXISTS idx_virtual_entities_workspace_type
    ON virtual_entities(workspace_id, entity_type, updated_at DESC);

-- Persona-scoped queries (e.g. "what's this persona's subscription state").
CREATE INDEX IF NOT EXISTS idx_virtual_entities_persona
    ON virtual_entities(workspace_id, persona_id)
    WHERE persona_id IS NOT NULL;
