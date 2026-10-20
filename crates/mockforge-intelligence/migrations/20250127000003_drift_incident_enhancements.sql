-- Drift Incident Enhancements for Contracts Control Plane v2
-- This migration adds fitness test results, consumer impact, and protocol fields to drift incidents

-- Add fitness_test_results column (stored as JSONB array)
ALTER TABLE drift_incidents
ADD COLUMN IF NOT EXISTS fitness_test_results JSONB DEFAULT '[]'::jsonb;

-- Add affected_consumers column (stored as JSONB object)
ALTER TABLE drift_incidents
ADD COLUMN IF NOT EXISTS affected_consumers JSONB;

-- Add protocol column (stored as TEXT)
ALTER TABLE drift_incidents
ADD COLUMN IF NOT EXISTS protocol TEXT CHECK (protocol IN ('http', 'graphql', 'grpc', 'websocket', 'smtp', 'mqtt', 'ftp', 'kafka', 'rabbitmq', 'amqp', 'tcp', 'udp'));

-- Create index on protocol for filtering
CREATE INDEX IF NOT EXISTS idx_drift_incidents_protocol ON drift_incidents(protocol);

-- Create index on fitness_test_results for querying (using GIN index for JSONB)
CREATE INDEX IF NOT EXISTS idx_drift_incidents_fitness_results ON drift_incidents USING GIN (fitness_test_results);
