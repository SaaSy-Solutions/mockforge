-- Audit-log tamper-evidence + append-only immutability + forensic retention (#872).
--
-- This migration hardens `audit_logs` for SOC2 / ISO-27001 / GDPR posture:
--
--   1. New auth + GDPR audit event types (#871 / #872). The Rust mirror lives
--      in `crates/mockforge-registry-core/src/models/audit_log.rs`
--      (`AuditEventType::{LoginSucceeded,LoginFailed,Logout,DataExported}`).
--      The round-trip test there guarantees every snake_case literal below
--      also exists on the Rust side.
--
--   2. A per-org hash chain (`prev_hash` / `entry_hash`) so any row mutation,
--      deletion, or reordering is detectable by recomputing the chain. The
--      chain is computed in `AuditLog::create` (Rust) inside a transaction
--      that takes a `FOR UPDATE` lock on the org's latest row.
--
--   3. A DB-level append-only trigger that raises on UPDATE or DELETE of any
--      `audit_logs` row. This is role-independent: even the table owner /
--      superuser hits it unless they explicitly DROP/DISABLE the trigger,
--      which is itself an auditable DDL act. This makes `cleanup_old()`'s mass
--      DELETE impossible — that method is now a logged no-op (retain forever).
--
--   4. Breaking the `ON DELETE CASCADE` from `organizations`: deleting an org
--      must NOT wipe its audit trail. `org_id` stays a plain NOT NULL UUID
--      (no FK), so rows outlive the org for forensic value.

-- ---------------------------------------------------------------------------
-- 1. New audit event types (auth + GDPR export).
--    ADD VALUE is safe inside this migration's transaction because none of
--    these values are *used* (referenced as a literal cast to the enum) here.
-- ---------------------------------------------------------------------------
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'login_succeeded';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'login_failed';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'logout';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'data_exported';

-- ---------------------------------------------------------------------------
-- 2. Hash-chain columns. NULL on the first row of each org's chain and on any
--    pre-existing rows (back-fill is intentionally skipped — historical rows
--    predate the chain and are flagged as "chain start" by verify_chain).
-- ---------------------------------------------------------------------------
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS prev_hash  TEXT;
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS entry_hash TEXT;

-- ---------------------------------------------------------------------------
-- 4. Break the org CASCADE so deleting an org keeps its audit trail.
--    The FK constraint name from migration ...012 is `audit_logs_org_id_fkey`
--    (Postgres default: <table>_<column>_fkey). Drop it; keep the column.
-- ---------------------------------------------------------------------------
ALTER TABLE audit_logs DROP CONSTRAINT IF EXISTS audit_logs_org_id_fkey;

-- ---------------------------------------------------------------------------
-- 3. Append-only immutability trigger. Any UPDATE or DELETE on audit_logs
--    raises an exception, regardless of the DB role performing it.
-- ---------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION audit_logs_block_mutation()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION
        'audit_logs is append-only (#872): % is not permitted', TG_OP
        USING ERRCODE = 'check_violation';
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS audit_logs_append_only ON audit_logs;
CREATE TRIGGER audit_logs_append_only
    BEFORE UPDATE OR DELETE ON audit_logs
    FOR EACH ROW
    EXECUTE FUNCTION audit_logs_block_mutation();
