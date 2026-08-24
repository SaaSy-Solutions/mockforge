-- #720 — per-service probe cadence for the contract-verification probe
-- worker. `probe_interval_secs` overrides the global
-- MOCKFORGE_CONTRACT_PROBE_INTERVAL_SECS default per service (NULL =
-- use the global default). `last_probed_at` lets the worker's tick be a
-- due-filter instead of "enqueue everything every global tick", so a
-- 5-minute service and a 6-hour service coexist under one worker.

ALTER TABLE monitored_services
    ADD COLUMN probe_interval_secs INTEGER
        CHECK (probe_interval_secs IS NULL OR probe_interval_secs > 0),
    ADD COLUMN last_probed_at TIMESTAMPTZ;
