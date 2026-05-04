-- One report per (campaign, run) (cloud-enablement task #7 / Phase 2).
--
-- The dispatcher path inserts the report row from the test_runs.finish
-- callback. Without this constraint a worker retry could write two rows
-- for the same run; with it, the upsert in
-- ChaosCampaignReport::create is idempotent.

CREATE UNIQUE INDEX IF NOT EXISTS idx_chaos_reports_campaign_run
    ON chaos_campaign_reports(campaign_id, run_id);
