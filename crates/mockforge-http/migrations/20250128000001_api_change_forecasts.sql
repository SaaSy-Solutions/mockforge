-- API Change Forecasting Schema
-- This migration creates tables for storing API change predictions based on historical patterns

-- API change forecasts table
-- Stores predictions for service-level and endpoint-level contract changes
CREATE TABLE IF NOT EXISTS api_change_forecasts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID,
    service_id TEXT,
    service_name TEXT,
    endpoint TEXT NOT NULL,
    method TEXT NOT NULL,
    forecast_window_days INTEGER NOT NULL CHECK (forecast_window_days IN (30, 90, 180)),
    predicted_change_probability DOUBLE PRECISION NOT NULL CHECK (predicted_change_probability >= 0.0 AND predicted_change_probability <= 1.0),
    predicted_break_probability DOUBLE PRECISION NOT NULL CHECK (predicted_break_probability >= 0.0 AND predicted_break_probability <= 1.0),
    next_expected_change_date TIMESTAMPTZ,
    next_expected_break_date TIMESTAMPTZ,
    volatility_score DOUBLE PRECISION NOT NULL CHECK (volatility_score >= 0.0 AND volatility_score <= 1.0),
    confidence DOUBLE PRECISION NOT NULL CHECK (confidence >= 0.0 AND confidence <= 1.0),
    seasonal_patterns JSONB DEFAULT '[]'::jsonb,
    predicted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_forecasts_workspace ON api_change_forecasts(workspace_id);
CREATE INDEX IF NOT EXISTS idx_forecasts_service ON api_change_forecasts(service_id);
CREATE INDEX IF NOT EXISTS idx_forecasts_endpoint ON api_change_forecasts(endpoint, method);
CREATE INDEX IF NOT EXISTS idx_forecasts_window ON api_change_forecasts(forecast_window_days);
CREATE INDEX IF NOT EXISTS idx_forecasts_expires ON api_change_forecasts(expires_at);
CREATE INDEX IF NOT EXISTS idx_forecasts_predicted ON api_change_forecasts(predicted_at DESC);
CREATE INDEX IF NOT EXISTS idx_forecasts_volatility ON api_change_forecasts(volatility_score DESC);

-- Composite index for common query pattern: workspace + service + window
CREATE INDEX IF NOT EXISTS idx_forecasts_workspace_service_window ON api_change_forecasts(workspace_id, service_id, forecast_window_days);

-- Composite index for endpoint-level queries
CREATE INDEX IF NOT EXISTS idx_forecasts_endpoint_window ON api_change_forecasts(endpoint, method, forecast_window_days);

