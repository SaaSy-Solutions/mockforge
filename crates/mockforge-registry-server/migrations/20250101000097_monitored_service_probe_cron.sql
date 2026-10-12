-- Cron schedules take precedence over legacy interval cadence when present.
-- Timezones use IANA names, validated by the registry API before insertion.
ALTER TABLE monitored_services
    ADD COLUMN probe_cron TEXT CHECK (probe_cron IS NULL OR length(trim(probe_cron)) > 0),
    ADD COLUMN probe_timezone TEXT NOT NULL DEFAULT 'UTC';
