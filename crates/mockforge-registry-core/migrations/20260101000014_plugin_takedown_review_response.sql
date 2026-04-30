-- SQLite mirror of 20250101000057_plugin_takedown_review_response.sql.
--
-- SQLite stores `audit_logs.event_type` as TEXT, so the new audit-event
-- variants need no schema change here — they're added to the Rust enum
-- in `audit_log.rs`. We do still add the new columns on `plugins` and
-- `reviews` so handlers compiled against either backend behave the same.

ALTER TABLE plugins ADD COLUMN taken_down_at TEXT;
ALTER TABLE plugins ADD COLUMN taken_down_reason TEXT;

CREATE INDEX IF NOT EXISTS idx_plugins_taken_down ON plugins(taken_down_at)
    WHERE taken_down_at IS NOT NULL;

ALTER TABLE reviews ADD COLUMN author_response_text TEXT;
ALTER TABLE reviews ADD COLUMN author_response_at TEXT;
