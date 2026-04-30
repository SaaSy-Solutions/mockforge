-- Usage threshold alerts for MockForge Cloud.
--
-- A row is inserted by the threshold-checker worker the first time an org's
-- usage of a metric crosses a band (e.g. 75%, 90%) within a billing period.
-- The unique index makes inserts idempotent so the worker is safe to run on
-- any cadence.

CREATE TABLE usage_alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    -- Metric whose threshold was crossed: 'requests', 'storage', 'ai_tokens'.
    metric VARCHAR(64) NOT NULL,
    -- First-of-month, matches usage_counters.period_start.
    period_start DATE NOT NULL,
    -- Crossed band: 75 or 90.
    threshold_pct SMALLINT NOT NULL CHECK (threshold_pct BETWEEN 1 AND 100),
    -- When the worker observed the crossing (and emailed if applicable).
    notified_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Set when the user dismisses the alert in the UI.
    dismissed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (org_id, metric, period_start, threshold_pct)
);

CREATE INDEX idx_usage_alerts_org_period ON usage_alerts(org_id, period_start);
CREATE INDEX idx_usage_alerts_active ON usage_alerts(org_id) WHERE dismissed_at IS NULL;
