-- Fix TIMESTAMP to TIMESTAMPTZ for organizations table
-- This ensures compatibility with Rust's DateTime<Utc> type

ALTER TABLE organizations
    ALTER COLUMN created_at TYPE TIMESTAMPTZ USING created_at AT TIME ZONE 'UTC',
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ USING updated_at AT TIME ZONE 'UTC';
