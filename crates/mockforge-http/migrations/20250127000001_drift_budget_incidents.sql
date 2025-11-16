-- Drift Budget and Incident Management Schema
-- This migration creates tables for drift budget tracking and incident management

-- Drift budgets table
CREATE TABLE IF NOT EXISTS drift_budgets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID,
    endpoint TEXT NOT NULL,
    method TEXT NOT NULL,
    max_breaking_changes INTEGER NOT NULL DEFAULT 0,
    max_non_breaking_changes INTEGER NOT NULL DEFAULT 10,
    breaking_change_rules JSONB DEFAULT '[]'::jsonb,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(workspace_id, endpoint, method)
);

CREATE INDEX idx_drift_budgets_workspace ON drift_budgets(workspace_id);
CREATE INDEX idx_drift_budgets_endpoint ON drift_budgets(endpoint, method);

-- Drift incidents table
CREATE TABLE IF NOT EXISTS drift_incidents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID,
    budget_id UUID REFERENCES drift_budgets(id) ON DELETE SET NULL,
    endpoint TEXT NOT NULL,
    method TEXT NOT NULL,
    incident_type TEXT NOT NULL CHECK (incident_type IN ('breaking_change', 'threshold_exceeded')),
    severity TEXT NOT NULL CHECK (severity IN ('low', 'medium', 'high', 'critical')),
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'acknowledged', 'resolved', 'closed')),
    details JSONB NOT NULL DEFAULT '{}'::jsonb,
    external_ticket_id TEXT,
    external_ticket_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    acknowledged_at TIMESTAMPTZ,
    resolved_at TIMESTAMPTZ,
    closed_at TIMESTAMPTZ
);

CREATE INDEX idx_drift_incidents_workspace ON drift_incidents(workspace_id);
CREATE INDEX idx_drift_incidents_status ON drift_incidents(status);
CREATE INDEX idx_drift_incidents_severity ON drift_incidents(severity);
CREATE INDEX idx_drift_incidents_endpoint ON drift_incidents(endpoint, method);
CREATE INDEX idx_drift_incidents_created ON drift_incidents(created_at DESC);

-- Consumer contracts table
CREATE TABLE IF NOT EXISTS consumers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID,
    identifier TEXT NOT NULL,
    identifier_type TEXT NOT NULL CHECK (identifier_type IN ('workspace', 'custom', 'api_key', 'auth_token')),
    name TEXT,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(workspace_id, identifier, identifier_type)
);

CREATE INDEX idx_consumers_workspace ON consumers(workspace_id);
CREATE INDEX idx_consumers_identifier ON consumers(identifier, identifier_type);

-- Consumer usage tracking table
CREATE TABLE IF NOT EXISTS consumer_usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    consumer_id UUID NOT NULL REFERENCES consumers(id) ON DELETE CASCADE,
    endpoint TEXT NOT NULL,
    method TEXT NOT NULL,
    fields_used JSONB DEFAULT '[]'::jsonb,
    last_used_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    usage_count INTEGER NOT NULL DEFAULT 1,
    UNIQUE(consumer_id, endpoint, method)
);

CREATE INDEX idx_consumer_usage_consumer ON consumer_usage(consumer_id);
CREATE INDEX idx_consumer_usage_endpoint ON consumer_usage(endpoint, method);
CREATE INDEX idx_consumer_usage_last_used ON consumer_usage(last_used_at DESC);

-- Consumer violations table
CREATE TABLE IF NOT EXISTS consumer_violations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    consumer_id UUID NOT NULL REFERENCES consumers(id) ON DELETE CASCADE,
    endpoint TEXT NOT NULL,
    method TEXT NOT NULL,
    violation_type TEXT NOT NULL CHECK (violation_type IN ('field_removed', 'field_type_changed', 'required_field_added')),
    field_path TEXT NOT NULL,
    details JSONB DEFAULT '{}'::jsonb,
    detected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ
);

CREATE INDEX idx_consumer_violations_consumer ON consumer_violations(consumer_id);
CREATE INDEX idx_consumer_violations_endpoint ON consumer_violations(endpoint, method);
CREATE INDEX idx_consumer_violations_detected ON consumer_violations(detected_at DESC);

-- Webhook configurations table
CREATE TABLE IF NOT EXISTS webhook_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    secret TEXT,
    events TEXT[] DEFAULT ARRAY[]::TEXT[],
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_webhook_configs_workspace ON webhook_configs(workspace_id);
CREATE INDEX idx_webhook_configs_enabled ON webhook_configs(enabled);

-- Webhook delivery logs table
CREATE TABLE IF NOT EXISTS webhook_deliveries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    webhook_config_id UUID NOT NULL REFERENCES webhook_configs(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    status_code INTEGER,
    response_body TEXT,
    delivered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    retry_count INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_webhook_deliveries_webhook ON webhook_deliveries(webhook_config_id);
CREATE INDEX idx_webhook_deliveries_delivered ON webhook_deliveries(delivered_at DESC);
