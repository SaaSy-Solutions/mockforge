-- Cloud backup and restore metadata

-- Workspace backups table
-- Tracks cloud backups of workspaces
CREATE TABLE IF NOT EXISTS workspace_backups (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL,
    backup_url TEXT NOT NULL, -- URL or path to backup location
    storage_backend TEXT NOT NULL CHECK (storage_backend IN ('local', 's3', 'azure', 'gcs', 'custom')),
    storage_config TEXT, -- JSON config for storage backend (credentials, bucket, etc.)
    size_bytes INTEGER NOT NULL,
    backup_format TEXT NOT NULL DEFAULT 'yaml', -- 'yaml' or 'json'
    encrypted INTEGER NOT NULL DEFAULT 0, -- Whether backup is encrypted
    commit_id TEXT, -- Commit ID this backup represents
    created_at TEXT NOT NULL,
    created_by TEXT NOT NULL,
    expires_at TEXT, -- Optional expiration date for backups
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    FOREIGN KEY (commit_id) REFERENCES commits(id),
    FOREIGN KEY (created_by) REFERENCES users(id)
);

CREATE INDEX idx_backups_workspace ON workspace_backups(workspace_id);
CREATE INDEX idx_backups_created ON workspace_backups(created_at);
CREATE INDEX idx_backups_storage ON workspace_backups(storage_backend);
CREATE INDEX idx_backups_expires ON workspace_backups(expires_at);

-- Workspace state snapshots table
-- Efficient state snapshots for real-time synchronization
CREATE TABLE IF NOT EXISTS workspace_state_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL,
    state_hash TEXT NOT NULL, -- SHA256 hash of state_data for deduplication
    state_data TEXT NOT NULL, -- JSON snapshot of full workspace state
    version INTEGER NOT NULL, -- Version number matching workspace.version
    created_at TEXT NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

CREATE INDEX idx_state_snapshots_workspace ON workspace_state_snapshots(workspace_id);
CREATE INDEX idx_state_snapshots_version ON workspace_state_snapshots(workspace_id, version);
CREATE INDEX idx_state_snapshots_hash ON workspace_state_snapshots(state_hash);

-- State change log table
-- Tracks incremental state changes for efficient sync
CREATE TABLE IF NOT EXISTS workspace_state_changes (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL,
    change_type TEXT NOT NULL CHECK (change_type IN ('mock_added', 'mock_updated', 'mock_deleted', 'env_updated', 'config_updated', 'full_sync')),
    change_data TEXT NOT NULL, -- JSON describing the change
    version INTEGER NOT NULL, -- Version after this change
    created_at TEXT NOT NULL,
    created_by TEXT NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES users(id)
);

CREATE INDEX idx_state_changes_workspace ON workspace_state_changes(workspace_id);
CREATE INDEX idx_state_changes_version ON workspace_state_changes(workspace_id, version);
CREATE INDEX idx_state_changes_created ON workspace_state_changes(created_at);
