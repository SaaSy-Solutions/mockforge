-- Hosted Mocks Deployment schema
-- Tracks cloud-hosted mock service deployments

CREATE TABLE IF NOT EXISTS hosted_mocks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    project_id UUID REFERENCES projects(id) ON DELETE SET NULL,
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(255) NOT NULL,
    description TEXT,
    config_json JSONB NOT NULL, -- Full MockForge config
    openapi_spec_url TEXT, -- URL to OpenAPI spec (stored in object storage)
    status VARCHAR(50) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'deploying', 'active', 'stopped', 'failed', 'deleting')),
    deployment_url TEXT, -- Public URL for the deployed mock (e.g., https://mock-{slug}.mockforge.dev)
    internal_url TEXT, -- Internal service URL
    region VARCHAR(50) DEFAULT 'us-east-1', -- Deployment region
    instance_type VARCHAR(50) DEFAULT 'small', -- Instance size
    health_check_url TEXT, -- Health check endpoint
    last_health_check TIMESTAMPTZ,
    health_status VARCHAR(50) DEFAULT 'unknown' CHECK (health_status IN ('healthy', 'unhealthy', 'unknown')),
    error_message TEXT, -- Error message if deployment failed
    metadata_json JSONB DEFAULT '{}'::jsonb, -- Additional metadata (ports, env vars, etc.)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ, -- Soft delete
    UNIQUE(org_id, slug)
);

-- Deployment logs table (for debugging and monitoring)
CREATE TABLE IF NOT EXISTS deployment_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    hosted_mock_id UUID NOT NULL REFERENCES hosted_mocks(id) ON DELETE CASCADE,
    level VARCHAR(20) NOT NULL CHECK (level IN ('info', 'warning', 'error', 'debug')),
    message TEXT NOT NULL,
    metadata_json JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Deployment metrics (for usage tracking)
CREATE TABLE IF NOT EXISTS deployment_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    hosted_mock_id UUID NOT NULL REFERENCES hosted_mocks(id) ON DELETE CASCADE,
    period_start DATE NOT NULL, -- First day of month
    requests BIGINT NOT NULL DEFAULT 0,
    requests_2xx BIGINT NOT NULL DEFAULT 0,
    requests_4xx BIGINT NOT NULL DEFAULT 0,
    requests_5xx BIGINT NOT NULL DEFAULT 0,
    egress_bytes BIGINT NOT NULL DEFAULT 0,
    avg_response_time_ms BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(hosted_mock_id, period_start)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_hosted_mocks_org ON hosted_mocks(org_id);
CREATE INDEX IF NOT EXISTS idx_hosted_mocks_project ON hosted_mocks(project_id);
CREATE INDEX IF NOT EXISTS idx_hosted_mocks_slug ON hosted_mocks(slug);
CREATE INDEX IF NOT EXISTS idx_hosted_mocks_status ON hosted_mocks(status);
CREATE INDEX IF NOT EXISTS idx_hosted_mocks_deleted ON hosted_mocks(deleted_at) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_deployment_logs_mock ON deployment_logs(hosted_mock_id);
CREATE INDEX IF NOT EXISTS idx_deployment_logs_created ON deployment_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_deployment_metrics_mock ON deployment_metrics(hosted_mock_id);
CREATE INDEX IF NOT EXISTS idx_deployment_metrics_period ON deployment_metrics(hosted_mock_id, period_start);

-- Add triggers for updated_at
CREATE TRIGGER update_hosted_mocks_updated_at BEFORE UPDATE ON hosted_mocks
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_deployment_metrics_updated_at BEFORE UPDATE ON deployment_metrics
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
