-- Fix region default from AWS-style 'us-east-1' to Fly.io region code 'iad'
ALTER TABLE hosted_mocks ALTER COLUMN region SET DEFAULT 'iad';

-- Update any existing rows that still have the old default
UPDATE hosted_mocks SET region = 'iad' WHERE region = 'us-east-1';
