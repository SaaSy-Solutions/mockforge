-- Initial schema for MockForge Plugin Registry

-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(100) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    api_token VARCHAR(255) UNIQUE,
    is_verified BOOLEAN DEFAULT FALSE,
    is_admin BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Plugins table
CREATE TABLE plugins (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) UNIQUE NOT NULL,
    description TEXT NOT NULL,
    current_version VARCHAR(50) NOT NULL,
    category VARCHAR(50) NOT NULL,
    license VARCHAR(100) NOT NULL,
    repository TEXT,
    homepage TEXT,
    downloads_total BIGINT DEFAULT 0,
    rating_avg DECIMAL(3,2) DEFAULT 0,
    rating_count INT DEFAULT 0,
    author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    verified_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),

    -- Constraints
    CONSTRAINT category_values CHECK (category IN ('auth', 'template', 'response', 'datasource', 'middleware', 'testing', 'observability', 'other'))
);

-- Plugin versions table
CREATE TABLE plugin_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    plugin_id UUID NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    version VARCHAR(50) NOT NULL,
    download_url TEXT NOT NULL,
    checksum VARCHAR(64) NOT NULL,
    file_size BIGINT NOT NULL,
    min_mockforge_version VARCHAR(50),
    yanked BOOLEAN DEFAULT FALSE,
    downloads INT DEFAULT 0,
    published_at TIMESTAMP DEFAULT NOW(),

    UNIQUE(plugin_id, version)
);

-- Tags table
CREATE TABLE tags (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) UNIQUE NOT NULL
);

-- Plugin-tags junction table
CREATE TABLE plugin_tags (
    plugin_id UUID REFERENCES plugins(id) ON DELETE CASCADE,
    tag_id INT REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (plugin_id, tag_id)
);

-- Plugin dependencies table
CREATE TABLE plugin_dependencies (
    id SERIAL PRIMARY KEY,
    version_id UUID NOT NULL REFERENCES plugin_versions(id) ON DELETE CASCADE,
    depends_on_plugin VARCHAR(255) NOT NULL,
    version_requirement VARCHAR(100) NOT NULL
);

-- Reviews table
CREATE TABLE reviews (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    plugin_id UUID NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    version VARCHAR(50) NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rating SMALLINT NOT NULL CHECK (rating >= 1 AND rating <= 5),
    title VARCHAR(100),
    comment TEXT NOT NULL CHECK (length(comment) >= 10),
    helpful_count INT DEFAULT 0,
    unhelpful_count INT DEFAULT 0,
    verified BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),

    UNIQUE(plugin_id, user_id)
);

-- Create indexes for performance
CREATE INDEX idx_plugins_name ON plugins(name);
CREATE INDEX idx_plugins_category ON plugins(category);
CREATE INDEX idx_plugins_downloads ON plugins(downloads_total DESC);
CREATE INDEX idx_plugins_rating ON plugins(rating_avg DESC);
CREATE INDEX idx_plugins_author ON plugins(author_id);
CREATE INDEX idx_versions_plugin ON plugin_versions(plugin_id);
CREATE INDEX idx_versions_version ON plugin_versions(version);
CREATE INDEX idx_reviews_plugin ON reviews(plugin_id);
CREATE INDEX idx_reviews_user ON reviews(user_id);
CREATE INDEX idx_tags_name ON tags(name);

-- Full-text search setup
ALTER TABLE plugins ADD COLUMN search_vector tsvector
    GENERATED ALWAYS AS (
        setweight(to_tsvector('english', name), 'A') ||
        setweight(to_tsvector('english', description), 'B')
    ) STORED;

CREATE INDEX idx_plugins_search ON plugins USING GIN(search_vector);

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Triggers for updated_at
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_plugins_updated_at BEFORE UPDATE ON plugins
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_reviews_updated_at BEFORE UPDATE ON reviews
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
