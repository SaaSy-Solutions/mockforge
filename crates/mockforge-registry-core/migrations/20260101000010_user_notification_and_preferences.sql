-- Per-user notification toggles and free-form UI preferences (SQLite).
--
-- SQLite does not support ADD COLUMN IF NOT EXISTS, but this migration
-- is only applied once by sqlx. `preferences` is stored as TEXT containing
-- serialized JSON — the Rust layer uses serde_json::Value for round-trip.

ALTER TABLE users ADD COLUMN email_notifications BOOLEAN NOT NULL DEFAULT 1;
ALTER TABLE users ADD COLUMN security_alerts BOOLEAN NOT NULL DEFAULT 1;
ALTER TABLE users ADD COLUMN preferences TEXT NOT NULL DEFAULT '{}';
