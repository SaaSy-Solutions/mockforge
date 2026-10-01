-- Bounded audit-log retention (400 days) without giving up append-only (#872).
--
-- Migration ...080 made `audit_logs` strictly append-only, which in practice
-- meant "retain forever", while the privacy policy promised a bounded period.
-- This replaces the trigger function so that exactly one kind of DELETE is
-- allowed: the retention purge (`AuditLog::purge_expired`), and only for rows
-- older than 400 days.
--
--   * UPDATE is still rejected unconditionally.
--   * DELETE is allowed only when the transaction has opted in with
--     `SET LOCAL mockforge.audit_retention_purge = 'on'` AND the row is older
--     than 400 days. The opt-in stops an ordinary `DELETE FROM audit_logs`
--     (or a cascade) from silently thinning the log; the age floor is the real
--     guarantee: no role can delete a row inside the retention window without
--     disabling the trigger, which is itself an auditable DDL act.
--
-- Rows are purged oldest-first, so each org's hash chain loses a prefix. The
-- oldest surviving row then has a `prev_hash` pointing at a purged row, and
-- `AuditLog::verify_chain` treats it as the chain start (it accepts whatever
-- `prev_hash` the first surviving row carries, then verifies every link after
-- it). Deleting or altering any row after the first is still detected.
--
-- The 400-day window is mirrored by `AUDIT_LOG_RETENTION_DAYS` in
-- `mockforge-registry-core/src/models/audit_log.rs`. If the Rust constant were
-- ever lower than this floor the purge would hit this exception and roll back
-- (fail-safe: nothing is deleted early).
--
-- `current_setting(..., true)` returns NULL when the GUC was never set and ''
-- after a `SET LOCAL` reverts on the same pooled connection; neither equals
-- 'on', so both fall through to the exception.

CREATE OR REPLACE FUNCTION audit_logs_block_mutation()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'DELETE'
       AND current_setting('mockforge.audit_retention_purge', true) = 'on'
       AND OLD.created_at < now() - interval '400 days' THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION
        'audit_logs is append-only (#872): % is not permitted', TG_OP
        USING ERRCODE = 'check_violation';
END;
$$ LANGUAGE plpgsql;
