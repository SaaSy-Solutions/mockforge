-- Promotion history schema
-- Tracks promotions of scenarios, personas, and configs between environments

-- Promotion history table
CREATE TABLE IF NOT EXISTS promotion_history (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL,
    entity_type TEXT NOT NULL CHECK (entity_type IN ('scenario', 'persona', 'config')),
    entity_id TEXT NOT NULL,
    entity_version TEXT,
    from_environment TEXT NOT NULL CHECK (from_environment IN ('dev', 'test', 'prod')),
    to_environment TEXT NOT NULL CHECK (to_environment IN ('dev', 'test', 'prod')),
    promoted_by TEXT NOT NULL,
    approved_by TEXT,
    status TEXT NOT NULL CHECK (status IN ('pending', 'approved', 'rejected', 'completed', 'failed')),
    comments TEXT,
    pr_url TEXT, -- GitOps PR URL if created
    metadata TEXT, -- JSON metadata (config diffs, etc.)
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    FOREIGN KEY (promoted_by) REFERENCES users(id) ON DELETE RESTRICT,
    FOREIGN KEY (approved_by) REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX idx_promotion_workspace ON promotion_history(workspace_id);
CREATE INDEX idx_promotion_entity ON promotion_history(entity_type, entity_id);
CREATE INDEX idx_promotion_status ON promotion_history(status);
CREATE INDEX idx_promotion_created ON promotion_history(created_at);
CREATE INDEX idx_promotion_environments ON promotion_history(from_environment, to_environment);
