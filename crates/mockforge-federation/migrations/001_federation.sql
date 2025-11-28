-- Federation database schema
-- Supports multi-workspace federation for composing workspaces into virtual systems

-- Federations table
-- Stores federation definitions (collections of services)
CREATE TABLE IF NOT EXISTS federations (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    org_id TEXT NOT NULL,
    description TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_federations_org ON federations(org_id);
CREATE INDEX IF NOT EXISTS idx_federations_name ON federations(name);

-- Federation services table
-- Maps services to workspaces within a federation
CREATE TABLE IF NOT EXISTS federation_services (
    id TEXT PRIMARY KEY NOT NULL,
    federation_id TEXT NOT NULL,
    service_name TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    base_path TEXT NOT NULL,
    reality_level TEXT NOT NULL,
    config TEXT, -- JSON string
    dependencies TEXT, -- JSON array of service names
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (federation_id) REFERENCES federations(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_federation_services_federation ON federation_services(federation_id);
CREATE INDEX IF NOT EXISTS idx_federation_services_workspace ON federation_services(workspace_id);
CREATE INDEX IF NOT EXISTS idx_federation_services_base_path ON federation_services(base_path);

-- System scenarios table
-- Stores system-wide scenarios that span multiple services in a federation
CREATE TABLE IF NOT EXISTS system_scenarios (
    id TEXT PRIMARY KEY NOT NULL,
    federation_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    scenario_definition TEXT NOT NULL, -- JSON string
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (federation_id) REFERENCES federations(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_system_scenarios_federation ON system_scenarios(federation_id);
CREATE INDEX IF NOT EXISTS idx_system_scenarios_name ON system_scenarios(name);
