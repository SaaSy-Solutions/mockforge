-- Contract Threat Modeling Schema
-- This migration creates tables for storing API security threat assessments and findings

-- Contract threat assessments table
-- Stores security posture analysis at workspace, service, and endpoint levels
CREATE TABLE IF NOT EXISTS contract_threat_assessments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID,
    service_id TEXT,
    service_name TEXT,
    endpoint TEXT,
    method TEXT,
    -- Aggregation level: 'workspace', 'service', or 'endpoint'
    aggregation_level TEXT NOT NULL CHECK (aggregation_level IN ('workspace', 'service', 'endpoint')),
    -- Overall threat level
    threat_level TEXT NOT NULL CHECK (threat_level IN ('low', 'medium', 'high', 'critical')),
    -- Threat score (0.0-1.0)
    threat_score DOUBLE PRECISION NOT NULL CHECK (threat_score >= 0.0 AND threat_score <= 1.0),
    -- Threat categories (JSONB array): PII exposure, DoS risk, error leakage, etc.
    threat_categories JSONB DEFAULT '[]'::jsonb,
    -- Detailed findings (JSONB array of threat findings)
    findings JSONB DEFAULT '[]'::jsonb,
    -- AI-generated remediation suggestions (JSONB array)
    remediation_suggestions JSONB DEFAULT '[]'::jsonb,
    -- Timestamps
    assessed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Unique constraint: one assessment per aggregation level + scope
    UNIQUE(workspace_id, service_id, endpoint, method, aggregation_level)
);

-- Threat findings table
-- Individual security findings linked to assessments
CREATE TABLE IF NOT EXISTS threat_findings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    assessment_id UUID NOT NULL REFERENCES contract_threat_assessments(id) ON DELETE CASCADE,
    -- Finding type: pii_exposure, dos_risk, error_leakage, schema_inconsistency, etc.
    finding_type TEXT NOT NULL CHECK (finding_type IN (
        'pii_exposure',
        'dos_risk',
        'error_leakage',
        'schema_inconsistency',
        'unbounded_arrays',
        'missing_rate_limits',
        'stack_trace_leakage',
        'sensitive_data_exposure',
        'insecure_schema_design',
        'missing_validation',
        'excessive_optional_fields'
    )),
    -- Severity of this specific finding
    severity TEXT NOT NULL CHECK (severity IN ('low', 'medium', 'high', 'critical')),
    -- Finding description
    description TEXT NOT NULL,
    -- Field path or location where finding was detected
    field_path TEXT,
    -- Additional context (JSONB)
    context JSONB DEFAULT '{}'::jsonb,
    -- Remediation suggestion for this finding
    remediation_suggestion TEXT,
    -- Remediation code example (if applicable)
    remediation_code_example TEXT,
    -- Confidence in this finding (0.0-1.0)
    confidence DOUBLE PRECISION NOT NULL DEFAULT 1.0 CHECK (confidence >= 0.0 AND confidence <= 1.0),
    -- Whether remediation is AI-generated
    ai_generated_remediation BOOLEAN NOT NULL DEFAULT false,
    -- Timestamps
    detected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for threat assessments
CREATE INDEX IF NOT EXISTS idx_threat_assessments_workspace ON contract_threat_assessments(workspace_id);
CREATE INDEX IF NOT EXISTS idx_threat_assessments_service ON contract_threat_assessments(service_id);
CREATE INDEX IF NOT EXISTS idx_threat_assessments_endpoint ON contract_threat_assessments(endpoint, method);
CREATE INDEX IF NOT EXISTS idx_threat_assessments_level ON contract_threat_assessments(aggregation_level);
CREATE INDEX IF NOT EXISTS idx_threat_assessments_threat_level ON contract_threat_assessments(threat_level);
CREATE INDEX IF NOT EXISTS idx_threat_assessments_score ON contract_threat_assessments(threat_score DESC);
CREATE INDEX IF NOT EXISTS idx_threat_assessments_assessed ON contract_threat_assessments(assessed_at DESC);

-- GIN indexes for JSONB columns
CREATE INDEX IF NOT EXISTS idx_threat_assessments_categories ON contract_threat_assessments USING GIN (threat_categories);
CREATE INDEX IF NOT EXISTS idx_threat_assessments_findings ON contract_threat_assessments USING GIN (findings);
CREATE INDEX IF NOT EXISTS idx_threat_assessments_remediations ON contract_threat_assessments USING GIN (remediation_suggestions);

-- Indexes for threat findings
CREATE INDEX IF NOT EXISTS idx_threat_findings_assessment ON threat_findings(assessment_id);
CREATE INDEX IF NOT EXISTS idx_threat_findings_type ON threat_findings(finding_type);
CREATE INDEX IF NOT EXISTS idx_threat_findings_severity ON threat_findings(severity);
CREATE INDEX IF NOT EXISTS idx_threat_findings_detected ON threat_findings(detected_at DESC);
CREATE INDEX IF NOT EXISTS idx_threat_findings_field_path ON threat_findings(field_path);

-- GIN index for context JSONB
CREATE INDEX IF NOT EXISTS idx_threat_findings_context ON threat_findings USING GIN (context);
