-- Pipeline definitions table
CREATE TABLE IF NOT EXISTS pipelines (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    workspace_id UUID,
    org_id UUID,
    definition JSONB NOT NULL,
    enabled BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Indexes for efficient querying
    CONSTRAINT pipelines_workspace_fk FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    CONSTRAINT pipelines_org_fk FOREIGN KEY (org_id) REFERENCES organizations(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_pipelines_workspace ON pipelines(workspace_id);
CREATE INDEX IF NOT EXISTS idx_pipelines_org ON pipelines(org_id);
CREATE INDEX IF NOT EXISTS idx_pipelines_enabled ON pipelines(enabled);

-- Pipeline executions table
CREATE TABLE IF NOT EXISTS pipeline_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pipeline_id UUID NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,
    trigger_event JSONB NOT NULL,
    status VARCHAR(50) NOT NULL CHECK (status IN ('started', 'running', 'completed', 'failed', 'cancelled')),
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    execution_log JSONB,

    -- Indexes
    CONSTRAINT pipeline_executions_pipeline_fk FOREIGN KEY (pipeline_id) REFERENCES pipelines(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_pipeline_executions_pipeline ON pipeline_executions(pipeline_id);
CREATE INDEX IF NOT EXISTS idx_pipeline_executions_status ON pipeline_executions(status);
CREATE INDEX IF NOT EXISTS idx_pipeline_executions_started ON pipeline_executions(started_at);

-- Pipeline step executions table (for detailed step-level tracking)
CREATE TABLE IF NOT EXISTS pipeline_step_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    execution_id UUID NOT NULL REFERENCES pipeline_executions(id) ON DELETE CASCADE,
    step_name VARCHAR(255) NOT NULL,
    step_type VARCHAR(100) NOT NULL,
    status VARCHAR(50) NOT NULL CHECK (status IN ('started', 'running', 'completed', 'failed', 'cancelled')),
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    output JSONB,
    error_message TEXT,

    -- Indexes
    CONSTRAINT pipeline_step_executions_execution_fk FOREIGN KEY (execution_id) REFERENCES pipeline_executions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_pipeline_step_executions_execution ON pipeline_step_executions(execution_id);
CREATE INDEX IF NOT EXISTS idx_pipeline_step_executions_status ON pipeline_step_executions(status);
