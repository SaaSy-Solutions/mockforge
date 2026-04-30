-- SQLite mirror of 20250101000056_invitation_audit_events.sql.
--
-- SQLite stores `audit_logs.event_type` as TEXT, so the new variants
-- need no schema change here — adding them to the Rust enum is enough.
-- This file exists so migration counts stay aligned across backends.
SELECT 1;
