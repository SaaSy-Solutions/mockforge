-- Cloud MockAI rule explanations.
--
-- Backs the cloud-mode replacement for the local mockai rule
-- explanations API (`/__mockforge/api/mockai/rules/*`). Each row is an
-- LLM-generated explanation of a rule learned from example traffic. The
-- explanation text is the surfacable artifact; pattern_matches records
-- which input examples the rule was derived from so the UI can show
-- provenance.
--
-- Closes #353 (cloud-enable MockAI suite).

CREATE TABLE cloud_mockai_rule_explanations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    rule_id TEXT NOT NULL,
    rule_type TEXT NOT NULL,
    confidence REAL NOT NULL DEFAULT 0.0,
    source_examples JSONB NOT NULL DEFAULT '[]'::jsonb,
    reasoning TEXT NOT NULL,
    pattern_matches JSONB NOT NULL DEFAULT '[]'::jsonb,
    generated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(workspace_id, rule_id)
);

CREATE INDEX idx_cloud_mockai_rules_workspace
    ON cloud_mockai_rule_explanations(workspace_id, generated_at DESC);
CREATE INDEX idx_cloud_mockai_rules_type
    ON cloud_mockai_rule_explanations(rule_type);
