-- Fork and merge support for workspaces

-- Workspace forks table
-- Tracks fork relationships between workspaces
CREATE TABLE IF NOT EXISTS workspace_forks (
    id TEXT PRIMARY KEY NOT NULL,
    source_workspace_id TEXT NOT NULL,
    forked_workspace_id TEXT NOT NULL,
    forked_at TEXT NOT NULL,
    forked_by TEXT NOT NULL,
    fork_point_commit_id TEXT, -- Commit ID at which fork was created
    FOREIGN KEY (source_workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    FOREIGN KEY (forked_workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    FOREIGN KEY (forked_by) REFERENCES users(id),
    FOREIGN KEY (fork_point_commit_id) REFERENCES commits(id)
);

CREATE INDEX idx_forks_source ON workspace_forks(source_workspace_id);
CREATE INDEX idx_forks_forked ON workspace_forks(forked_workspace_id);
CREATE INDEX idx_forks_forked_by ON workspace_forks(forked_by);

-- Workspace merge requests table
-- Tracks merge operations and their status
CREATE TABLE IF NOT EXISTS workspace_merges (
    id TEXT PRIMARY KEY NOT NULL,
    source_workspace_id TEXT NOT NULL, -- Workspace being merged FROM
    target_workspace_id TEXT NOT NULL, -- Workspace being merged INTO
    base_commit_id TEXT NOT NULL, -- Common ancestor commit
    source_commit_id TEXT NOT NULL, -- Latest commit from source
    target_commit_id TEXT NOT NULL, -- Latest commit from target
    merge_commit_id TEXT, -- Resulting merge commit (NULL if not completed)
    status TEXT NOT NULL CHECK (status IN ('pending', 'in_progress', 'completed', 'conflict', 'cancelled')),
    conflict_data TEXT, -- JSON array of conflicts if status is 'conflict'
    merged_by TEXT,
    merged_at TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (source_workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    FOREIGN KEY (target_workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    FOREIGN KEY (base_commit_id) REFERENCES commits(id),
    FOREIGN KEY (source_commit_id) REFERENCES commits(id),
    FOREIGN KEY (target_commit_id) REFERENCES commits(id),
    FOREIGN KEY (merge_commit_id) REFERENCES commits(id),
    FOREIGN KEY (merged_by) REFERENCES users(id)
);

CREATE INDEX idx_merges_source ON workspace_merges(source_workspace_id);
CREATE INDEX idx_merges_target ON workspace_merges(target_workspace_id);
CREATE INDEX idx_merges_status ON workspace_merges(status);
