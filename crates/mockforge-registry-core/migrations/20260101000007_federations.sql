-- SQLite mirror of the Postgres federations table.
--
-- Federations group multiple workspaces for cross-service composition.
-- The Postgres version also extends two enums (feature_type and
-- audit_event_type) — those don't exist in SQLite (we store event types
-- as TEXT), so this migration is schema-only.
--
-- Store methods for federations are still Postgres-only; this migration
-- exists so the tables can be queried by SQLite-backed trait impls as
-- those are ported over.

CREATE TABLE IF NOT EXISTS federations (
    id TEXT PRIMARY KEY NOT NULL,
    org_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    services TEXT NOT NULL DEFAULT '[]',
    created_by TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    UNIQUE(org_id, name)
);

CREATE INDEX IF NOT EXISTS idx_federations_org ON federations(org_id);
CREATE INDEX IF NOT EXISTS idx_federations_created_by ON federations(created_by);
CREATE INDEX IF NOT EXISTS idx_federations_org_created
    ON federations(org_id, created_at DESC);
