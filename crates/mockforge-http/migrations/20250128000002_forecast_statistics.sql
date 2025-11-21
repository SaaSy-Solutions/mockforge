-- Forecast Statistics Schema
-- This migration creates tables for storing statistical models and pattern metadata used in forecasting

-- Forecast statistics table
-- Stores historical change frequencies, volatility metrics, and pattern signatures
CREATE TABLE IF NOT EXISTS forecast_statistics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID,
    service_id TEXT,
    service_name TEXT,
    endpoint TEXT,
    method TEXT,
    -- Aggregation level: 'workspace', 'service', or 'endpoint'
    aggregation_level TEXT NOT NULL CHECK (aggregation_level IN ('workspace', 'service', 'endpoint')),
    -- Time window for statistics (30, 90, 180 days)
    time_window_days INTEGER NOT NULL CHECK (time_window_days IN (30, 90, 180)),
    -- Historical change frequency (changes per day)
    change_frequency DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    -- Breaking change frequency (breaking changes per day)
    breaking_change_frequency DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    -- Volatility metrics
    volatility_score DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    -- Pattern signatures (JSONB array of detected patterns)
    pattern_signatures JSONB DEFAULT '[]'::jsonb,
    -- Pattern types detected: field_addition, field_rename, breaking_change, etc.
    detected_pattern_types TEXT[] DEFAULT ARRAY[]::TEXT[],
    -- Last change date
    last_change_date TIMESTAMPTZ,
    -- Last breaking change date
    last_breaking_change_date TIMESTAMPTZ,
    -- Total changes in window
    total_changes INTEGER NOT NULL DEFAULT 0,
    -- Total breaking changes in window
    total_breaking_changes INTEGER NOT NULL DEFAULT 0,
    -- Calculated at timestamp
    calculated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Window start and end for this statistic
    window_start TIMESTAMPTZ NOT NULL,
    window_end TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Unique constraint: one statistic per aggregation level + time window + scope
    UNIQUE(workspace_id, service_id, endpoint, method, aggregation_level, time_window_days, window_start)
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_forecast_stats_workspace ON forecast_statistics(workspace_id);
CREATE INDEX IF NOT EXISTS idx_forecast_stats_service ON forecast_statistics(service_id);
CREATE INDEX IF NOT EXISTS idx_forecast_stats_endpoint ON forecast_statistics(endpoint, method);
CREATE INDEX IF NOT EXISTS idx_forecast_stats_level ON forecast_statistics(aggregation_level);
CREATE INDEX IF NOT EXISTS idx_forecast_stats_window ON forecast_statistics(time_window_days);
CREATE INDEX IF NOT EXISTS idx_forecast_stats_calculated ON forecast_statistics(calculated_at DESC);
CREATE INDEX IF NOT EXISTS idx_forecast_stats_window_range ON forecast_statistics(window_start, window_end);

-- GIN index for pattern signatures JSONB queries
CREATE INDEX IF NOT EXISTS idx_forecast_stats_patterns ON forecast_statistics USING GIN (pattern_signatures);

-- GIN index for pattern types array queries
CREATE INDEX IF NOT EXISTS idx_forecast_stats_pattern_types ON forecast_statistics USING GIN (detected_pattern_types);

