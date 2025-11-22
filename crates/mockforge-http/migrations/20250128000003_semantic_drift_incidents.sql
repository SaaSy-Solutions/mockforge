-- Semantic Drift Incidents Schema
-- This migration creates tables for storing semantic drift incidents detected through AI-powered analysis

-- Semantic drift incidents table
-- Similar structure to drift_incidents but focused on semantic/meaning changes
CREATE TABLE IF NOT EXISTS semantic_drift_incidents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID,
    endpoint TEXT NOT NULL,
    method TEXT NOT NULL,
    -- Semantic change type: description_change, enum_narrowing, nullable_change, error_code_removed, etc.
    semantic_change_type TEXT NOT NULL CHECK (semantic_change_type IN (
        'description_change',
        'enum_narrowing',
        'nullable_change',
        'error_code_removed',
        'semantic_constraint_change',
        'meaning_shift',
        'soft_breaking_change'
    )),
    -- Severity levels
    severity TEXT NOT NULL CHECK (severity IN ('low', 'medium', 'high', 'critical')),
    -- Status tracking
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'acknowledged', 'resolved', 'closed')),
    -- Semantic confidence score (0.0-1.0) from LLM analysis
    semantic_confidence DOUBLE PRECISION NOT NULL CHECK (semantic_confidence >= 0.0 AND semantic_confidence <= 1.0),
    -- Soft-breaking score (0.0-1.0) - likelihood this is a soft-breaking change
    soft_breaking_score DOUBLE PRECISION NOT NULL CHECK (soft_breaking_score >= 0.0 AND soft_breaking_score <= 1.0),
    -- Full LLM analysis and reasoning (JSONB)
    llm_analysis JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- Before and after semantic states (JSONB)
    before_semantic_state JSONB,
    after_semantic_state JSONB,
    -- Additional details
    details JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- Link to related structural drift incident (if any)
    related_drift_incident_id UUID REFERENCES drift_incidents(id) ON DELETE SET NULL,
    -- Link to contract diff that triggered this
    contract_diff_id TEXT,
    -- External ticket tracking
    external_ticket_id TEXT,
    external_ticket_url TEXT,
    -- Timestamps
    detected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    acknowledged_at TIMESTAMPTZ,
    resolved_at TIMESTAMPTZ,
    closed_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_semantic_incidents_workspace ON semantic_drift_incidents(workspace_id);
CREATE INDEX IF NOT EXISTS idx_semantic_incidents_endpoint ON semantic_drift_incidents(endpoint, method);
CREATE INDEX IF NOT EXISTS idx_semantic_incidents_type ON semantic_drift_incidents(semantic_change_type);
CREATE INDEX IF NOT EXISTS idx_semantic_incidents_status ON semantic_drift_incidents(status);
CREATE INDEX IF NOT EXISTS idx_semantic_incidents_severity ON semantic_drift_incidents(severity);
CREATE INDEX IF NOT EXISTS idx_semantic_incidents_confidence ON semantic_drift_incidents(semantic_confidence DESC);
CREATE INDEX IF NOT EXISTS idx_semantic_incidents_soft_breaking ON semantic_drift_incidents(soft_breaking_score DESC);
CREATE INDEX IF NOT EXISTS idx_semantic_incidents_detected ON semantic_drift_incidents(detected_at DESC);
CREATE INDEX IF NOT EXISTS idx_semantic_incidents_related ON semantic_drift_incidents(related_drift_incident_id);

-- GIN index for LLM analysis JSONB queries
CREATE INDEX IF NOT EXISTS idx_semantic_incidents_llm_analysis ON semantic_drift_incidents USING GIN (llm_analysis);

-- GIN index for details JSONB queries
CREATE INDEX IF NOT EXISTS idx_semantic_incidents_details ON semantic_drift_incidents USING GIN (details);
