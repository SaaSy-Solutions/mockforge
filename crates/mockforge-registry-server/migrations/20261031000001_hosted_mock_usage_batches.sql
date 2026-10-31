-- A receipt and both monthly counters commit together. Keep receipts for the
-- deployment lifetime: a Worker can retry a persisted batch after eviction.
CREATE TABLE hosted_mock_usage_batches (
    deployment_id UUID NOT NULL REFERENCES hosted_mocks(id) ON DELETE CASCADE,
    batch_id UUID NOT NULL,
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    period_start DATE NOT NULL CHECK (EXTRACT(DAY FROM period_start) = 1),
    requests BIGINT NOT NULL CHECK (requests >= 0),
    egress_bytes BIGINT NOT NULL CHECK (egress_bytes >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (deployment_id, batch_id)
);
ALTER TABLE hosted_mock_usage_batches ENABLE ROW LEVEL SECURITY;
ALTER TABLE hosted_mock_usage_batches FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON hosted_mock_usage_batches
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);
