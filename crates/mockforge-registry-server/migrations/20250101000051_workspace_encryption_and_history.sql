-- Workspace-level application-layer encryption flag + config.
-- At-rest infra encryption (Fly volumes + Postgres) is already on; this tracks the
-- per-workspace "encrypt sensitive fields" toggle the UI exposes. Actual cryptographic
-- operations reuse the BYOK infrastructure in handlers::settings.
ALTER TABLE workspaces
    ADD COLUMN IF NOT EXISTS encryption_enabled       BOOLEAN     NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS encryption_algorithm     VARCHAR(64) NOT NULL DEFAULT 'aes-256-gcm',
    ADD COLUMN IF NOT EXISTS encryption_config        JSONB       NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN IF NOT EXISTS encryption_key_rotated_at TIMESTAMPTZ;

-- Every workspace request execution appends one row here. Used by the request "history"
-- modal and for simple audit purposes. Oldest rows can be purged by a periodic job.
CREATE TABLE workspace_request_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    request_id UUID NOT NULL REFERENCES workspace_requests(id) ON DELETE CASCADE,
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    executed_by UUID REFERENCES users(id) ON DELETE SET NULL,
    executed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    request_method VARCHAR(16) NOT NULL,
    request_path TEXT NOT NULL,
    request_headers JSONB NOT NULL DEFAULT '{}'::jsonb,
    request_body TEXT,
    response_status_code INTEGER NOT NULL,
    response_headers JSONB NOT NULL DEFAULT '{}'::jsonb,
    response_body TEXT,
    response_time_ms INTEGER NOT NULL DEFAULT 0,
    response_size_bytes INTEGER NOT NULL DEFAULT 0,
    error_message TEXT
);
CREATE INDEX idx_workspace_request_history_request
    ON workspace_request_history(request_id, executed_at DESC);
CREATE INDEX idx_workspace_request_history_workspace
    ON workspace_request_history(workspace_id, executed_at DESC);
