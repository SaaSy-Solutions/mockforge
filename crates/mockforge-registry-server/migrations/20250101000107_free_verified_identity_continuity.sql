-- Retain a private, salted identity binding so deleting/recreating a verified
-- account cannot reset its Free allowance. No raw email is stored here.
CREATE TABLE free_identity_secret (
    singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK(singleton),
    salt TEXT NOT NULL DEFAULT (gen_random_uuid()::text || gen_random_uuid()::text)
);
INSERT INTO free_identity_secret(singleton) VALUES(TRUE);
CREATE TABLE free_verified_identities (
    identity_hash BYTEA PRIMARY KEY,
    owner_user_id UUID NOT NULL,
    account_deleted_at TIMESTAMPTZ
);
-- Request roles must never inspect the salt or enumerate retained identities.
-- RLS with no policy denies every non-owner role. These tables are not tenant
-- data, so they get no org policy and no FORCE: only the owner (the
-- SECURITY DEFINER functions below and the owner-pool cleanup worker) reads them.
REVOKE ALL ON free_identity_secret,free_verified_identities FROM PUBLIC;
ALTER TABLE free_identity_secret ENABLE ROW LEVEL SECURITY;
ALTER TABLE free_verified_identities ENABLE ROW LEVEL SECURITY;

CREATE FUNCTION preserve_verified_free_identity() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE fingerprint BYTEA; previous_owner UUID; old_credit RECORD; current_credit UUID;
BEGIN
    IF NOT NEW.is_verified THEN RETURN NEW; END IF;
    SELECT sha256(convert_to(salt || lower(btrim(NEW.email)), 'UTF8'))
        INTO fingerprint FROM free_identity_secret WHERE singleton;
    INSERT INTO free_verified_identities(identity_hash,owner_user_id)
        VALUES(fingerprint,NEW.id) ON CONFLICT(identity_hash) DO NOTHING;
    SELECT owner_user_id INTO previous_owner FROM free_verified_identities
        WHERE identity_hash=fingerprint;
    -- All historical aliases of one owner use a common lock before any alias
    -- row lock. Concurrent recreation via two aliases must not split the pool
    -- or acquire alias rows in opposite orders.
    PERFORM pg_advisory_xact_lock(hashtextextended(previous_owner::text,106));
    SELECT owner_user_id INTO previous_owner FROM free_verified_identities
        WHERE identity_hash=fingerprint FOR UPDATE;
    IF previous_owner<>NEW.id THEN
        IF EXISTS(SELECT 1 FROM users WHERE id=previous_owner) THEN
            RAISE EXCEPTION 'Verified identity allowance is already attached to an existing account'
                USING ERRCODE='23514';
        END IF;
        -- Deletion with outstanding calls is prohibited by migration 103. Keep
        -- ledger IDs and reservation attribution intact, changing only ownership.
        FOR old_credit IN SELECT * FROM free_ai_credit_periods
            WHERE owner_user_id=previous_owner ORDER BY period_start FOR UPDATE LOOP
            SELECT id INTO current_credit FROM free_ai_credit_periods
                WHERE owner_user_id=NEW.id AND period_start=old_credit.period_start FOR UPDATE;
            IF current_credit IS NOT NULL THEN
                UPDATE free_ai_credit_periods SET
                    used_tokens=used_tokens+old_credit.used_tokens,
                    reserved_tokens=reserved_tokens+old_credit.reserved_tokens,
                    granted_tokens=CASE WHEN granted_tokens=-1 OR old_credit.granted_tokens=-1
                        THEN -1 ELSE GREATEST(granted_tokens,old_credit.granted_tokens) END
                    WHERE id=current_credit;
                UPDATE ai_token_reservations SET free_credit_period_id=current_credit
                    WHERE free_credit_period_id=old_credit.id;
                DELETE FROM free_ai_credit_periods WHERE id=old_credit.id;
            ELSE
                UPDATE free_ai_credit_periods SET owner_user_id=NEW.id WHERE id=old_credit.id;
            END IF;
        END LOOP;
        INSERT INTO free_hosted_request_periods(owner_user_id,period_start,granted_requests,requests)
            SELECT NEW.id,period_start,granted_requests,requests FROM free_hosted_request_periods
            WHERE owner_user_id=previous_owner
            ON CONFLICT(owner_user_id,period_start) DO UPDATE SET
                requests=free_hosted_request_periods.requests+EXCLUDED.requests,
                granted_requests=CASE WHEN free_hosted_request_periods.granted_requests=-1
                    OR EXCLUDED.granted_requests=-1 THEN -1 ELSE
                    GREATEST(free_hosted_request_periods.granted_requests,EXCLUDED.granted_requests) END;
        DELETE FROM free_hosted_request_periods WHERE owner_user_id=previous_owner;
        UPDATE free_verified_identities SET owner_user_id=NEW.id,account_deleted_at=NULL
            WHERE owner_user_id=previous_owner;
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER free_verified_identity_continuity
AFTER INSERT OR UPDATE OF is_verified,email ON users
FOR EACH ROW EXECUTE FUNCTION preserve_verified_free_identity();
-- Capture existing verified identities before the first deletion after upgrade.
INSERT INTO free_verified_identities(identity_hash,owner_user_id)
SELECT sha256(convert_to(s.salt || lower(btrim(u.email)), 'UTF8')),u.id
FROM users u CROSS JOIN free_identity_secret s WHERE u.is_verified;

-- Only retain deleted-account identity bindings through the monthly allowance
-- window plus a small operational margin. The daily owner-pool worker purges.
CREATE FUNCTION mark_deleted_free_identity() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$ BEGIN
    UPDATE free_verified_identities SET account_deleted_at=NOW() WHERE owner_user_id=OLD.id;
    RETURN OLD;
END $$;
CREATE TRIGGER mark_deleted_free_identity AFTER DELETE ON users
FOR EACH ROW EXECUTE FUNCTION mark_deleted_free_identity();
