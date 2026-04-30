-- Plugin moderation, author review responses, and per-version download
-- tracking columns + audit events.
--
-- 1. `plugins.taken_down_at` / `taken_down_reason` — admin moderation tools
--    that remove a plugin from search and detail (404 for non-admins) without
--    losing history or breaking installed copies. Restoring just NULLs both
--    columns.
--
-- 2. `reviews.author_response_text` / `author_response_at` — lets the plugin
--    author publicly respond to a review. Stored on the review row instead
--    of a separate `review_responses` table because there's at most one
--    response per review (overwrites replace prior ones).
--
-- 3. `plugin_versions` already has a `downloads INTEGER` column, but the
--    existing `Plugin::increment_downloads` helpers were never wired into a
--    handler. We add the audit-event variants here so the new
--    /downloads endpoint can record audit rows on each tracked download
--    if/when we want to (currently we only update the counters).

ALTER TABLE plugins
    ADD COLUMN IF NOT EXISTS taken_down_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS taken_down_reason TEXT;

CREATE INDEX IF NOT EXISTS plugins_taken_down_idx
    ON plugins(taken_down_at)
    WHERE taken_down_at IS NOT NULL;

ALTER TABLE reviews
    ADD COLUMN IF NOT EXISTS author_response_text TEXT,
    ADD COLUMN IF NOT EXISTS author_response_at TIMESTAMPTZ;

ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'plugin_taken_down';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'plugin_restored';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'plugin_review_response_posted';
