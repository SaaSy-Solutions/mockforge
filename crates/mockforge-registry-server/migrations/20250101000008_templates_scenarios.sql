-- Template and Scenario Marketplace schema
-- Enables cloud hosting of orchestration templates and data scenarios

-- Templates table (orchestration templates for chaos testing)
CREATE TABLE IF NOT EXISTS templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID REFERENCES organizations(id) ON DELETE SET NULL, -- Nullable for public templates
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    version VARCHAR(50) NOT NULL,
    category VARCHAR(50) NOT NULL CHECK (category IN ('network-chaos', 'service-failure', 'load-testing', 'resilience-testing', 'security-testing', 'data-corruption', 'multi-protocol', 'custom-scenario')),
    tags TEXT[] DEFAULT ARRAY[]::TEXT[],
    content_json JSONB NOT NULL, -- Template content/configuration
    readme TEXT,
    example_usage TEXT,
    requirements TEXT[] DEFAULT ARRAY[]::TEXT[],
    compatibility_json JSONB DEFAULT '{}'::jsonb, -- min_version, max_version, required_features, protocols
    stats_json JSONB DEFAULT '{"downloads": 0, "stars": 0, "forks": 0, "rating": 0.0, "rating_count": 0}'::jsonb,
    published BOOLEAN DEFAULT FALSE,
    verified_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(name, version)
);

-- Template versions table (for version history)
CREATE TABLE IF NOT EXISTS template_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    template_id UUID NOT NULL REFERENCES templates(id) ON DELETE CASCADE,
    version VARCHAR(50) NOT NULL,
    content_json JSONB NOT NULL,
    download_url TEXT, -- URL to template package (stored in object storage)
    checksum VARCHAR(64),
    file_size BIGINT DEFAULT 0,
    yanked BOOLEAN DEFAULT FALSE,
    published_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(template_id, version)
);

-- Template reviews table
CREATE TABLE IF NOT EXISTS template_reviews (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    template_id UUID NOT NULL REFERENCES templates(id) ON DELETE CASCADE,
    reviewer_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rating INT NOT NULL CHECK (rating >= 1 AND rating <= 5),
    title TEXT,
    comment TEXT NOT NULL,
    helpful_count INT DEFAULT 0,
    verified_use BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(template_id, reviewer_id) -- One review per user per template
);

-- Scenarios table (data scenarios for mock systems)
CREATE TABLE IF NOT EXISTS scenarios (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID REFERENCES organizations(id) ON DELETE SET NULL, -- Nullable for public scenarios
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    current_version VARCHAR(50) NOT NULL,
    category VARCHAR(50) NOT NULL,
    tags TEXT[] DEFAULT ARRAY[]::TEXT[],
    license VARCHAR(100) NOT NULL,
    repository TEXT,
    homepage TEXT,
    manifest_json JSONB NOT NULL, -- Scenario manifest
    downloads_total BIGINT DEFAULT 0,
    rating_avg DECIMAL(3,2) DEFAULT 0,
    rating_count INT DEFAULT 0,
    verified_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(name)
);

-- Scenario versions table
CREATE TABLE IF NOT EXISTS scenario_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scenario_id UUID NOT NULL REFERENCES scenarios(id) ON DELETE CASCADE,
    version VARCHAR(50) NOT NULL,
    manifest_json JSONB NOT NULL,
    download_url TEXT NOT NULL, -- URL to scenario package (stored in object storage)
    checksum VARCHAR(64) NOT NULL,
    file_size BIGINT NOT NULL,
    min_mockforge_version VARCHAR(50),
    yanked BOOLEAN DEFAULT FALSE,
    downloads INT DEFAULT 0,
    published_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(scenario_id, version)
);

-- Scenario reviews table
CREATE TABLE IF NOT EXISTS scenario_reviews (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scenario_id UUID NOT NULL REFERENCES scenarios(id) ON DELETE CASCADE,
    reviewer_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rating INT NOT NULL CHECK (rating >= 1 AND rating <= 5),
    title TEXT,
    comment TEXT NOT NULL,
    helpful_count INT DEFAULT 0,
    verified_purchase BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(scenario_id, reviewer_id) -- One review per user per scenario
);

-- Template-tags junction table (if needed for better search)
CREATE TABLE IF NOT EXISTS template_tags (
    template_id UUID REFERENCES templates(id) ON DELETE CASCADE,
    tag_name VARCHAR(50) NOT NULL,
    PRIMARY KEY (template_id, tag_name)
);

-- Scenario-tags junction table
CREATE TABLE IF NOT EXISTS scenario_tags (
    scenario_id UUID REFERENCES scenarios(id) ON DELETE CASCADE,
    tag_name VARCHAR(50) NOT NULL,
    PRIMARY KEY (scenario_id, tag_name)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_templates_org ON templates(org_id);
CREATE INDEX IF NOT EXISTS idx_templates_author ON templates(author_id);
CREATE INDEX IF NOT EXISTS idx_templates_category ON templates(category);
CREATE INDEX IF NOT EXISTS idx_templates_published ON templates(published);
CREATE INDEX IF NOT EXISTS idx_templates_slug ON templates(slug);
CREATE INDEX IF NOT EXISTS idx_template_versions_template ON template_versions(template_id);
CREATE INDEX IF NOT EXISTS idx_template_reviews_template ON template_reviews(template_id);
CREATE INDEX IF NOT EXISTS idx_template_reviews_reviewer ON template_reviews(reviewer_id);

CREATE INDEX IF NOT EXISTS idx_scenarios_org ON scenarios(org_id);
CREATE INDEX IF NOT EXISTS idx_scenarios_author ON scenarios(author_id);
CREATE INDEX IF NOT EXISTS idx_scenarios_category ON scenarios(category);
CREATE INDEX IF NOT EXISTS idx_scenarios_slug ON scenarios(slug);
CREATE INDEX IF NOT EXISTS idx_scenario_versions_scenario ON scenario_versions(scenario_id);
CREATE INDEX IF NOT EXISTS idx_scenario_reviews_scenario ON scenario_reviews(scenario_id);
CREATE INDEX IF NOT EXISTS idx_scenario_reviews_reviewer ON scenario_reviews(reviewer_id);

-- Full-text search indexes (using GIN for JSONB)
CREATE INDEX IF NOT EXISTS idx_templates_search ON templates USING GIN(to_tsvector('english', name || ' ' || COALESCE(description, '')));
CREATE INDEX IF NOT EXISTS idx_scenarios_search ON scenarios USING GIN(to_tsvector('english', name || ' ' || COALESCE(description, '')));

-- Add triggers for updated_at
CREATE TRIGGER update_templates_updated_at BEFORE UPDATE ON templates
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_template_reviews_updated_at BEFORE UPDATE ON template_reviews
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_scenarios_updated_at BEFORE UPDATE ON scenarios
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_scenario_reviews_updated_at BEFORE UPDATE ON scenario_reviews
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
