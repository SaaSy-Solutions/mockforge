-- Drift Budget and Incident Enhancements
-- This migration adds new fields for enhanced drift tracking

-- Add new fields to drift_budgets table
ALTER TABLE drift_budgets
ADD COLUMN IF NOT EXISTS max_field_churn_percent DOUBLE PRECISION,
ADD COLUMN IF NOT EXISTS time_window_days INTEGER,
ADD COLUMN IF NOT EXISTS severity_threshold TEXT DEFAULT 'high' CHECK (severity_threshold IN ('critical', 'high', 'medium', 'low', 'info')),
ADD COLUMN IF NOT EXISTS budget_type TEXT DEFAULT 'endpoint' CHECK (budget_type IN ('workspace', 'service', 'tag', 'endpoint', 'default')),
ADD COLUMN IF NOT EXISTS service_name TEXT,
ADD COLUMN IF NOT EXISTS tag_name TEXT;

-- Add new fields to drift_incidents table
ALTER TABLE drift_incidents
ADD COLUMN IF NOT EXISTS sync_cycle_id TEXT,
ADD COLUMN IF NOT EXISTS contract_diff_id TEXT,
ADD COLUMN IF NOT EXISTS before_sample JSONB,
ADD COLUMN IF NOT EXISTS after_sample JSONB,
ADD COLUMN IF NOT EXISTS detected_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Create indexes for new fields
CREATE INDEX IF NOT EXISTS idx_drift_budgets_service ON drift_budgets(service_name);
CREATE INDEX IF NOT EXISTS idx_drift_budgets_tag ON drift_budgets(tag_name);
CREATE INDEX IF NOT EXISTS idx_drift_budgets_type ON drift_budgets(budget_type);
CREATE INDEX IF NOT EXISTS idx_drift_incidents_sync_cycle ON drift_incidents(sync_cycle_id);
CREATE INDEX IF NOT EXISTS idx_drift_incidents_contract_diff ON drift_incidents(contract_diff_id);

-- Create table for tracking field counts for percentage-based budgets
CREATE TABLE IF NOT EXISTS drift_field_counts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID,
    endpoint TEXT NOT NULL,
    method TEXT NOT NULL,
    field_count INTEGER NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(workspace_id, endpoint, method, recorded_at)
);

CREATE INDEX IF NOT EXISTS idx_drift_field_counts_endpoint ON drift_field_counts(endpoint, method);
CREATE INDEX IF NOT EXISTS idx_drift_field_counts_recorded ON drift_field_counts(recorded_at DESC);
CREATE INDEX IF NOT EXISTS idx_drift_field_counts_workspace ON drift_field_counts(workspace_id);

