-- GIN index on federations.services.
--
-- Supports the workspace poll query in
-- `find_active_federation_scenarios_for_workspace`, which uses JSONB
-- containment (`@>`) to find federations whose services list contains a
-- given workspace_id. Without this index the query degenerates to a
-- sequential scan over every federation row per poll tick.
--
-- SQLite has no analog — it stores services as TEXT and the SQLite store
-- impl filters in-memory.

CREATE INDEX IF NOT EXISTS idx_federations_services_gin
    ON federations
    USING GIN (services);
