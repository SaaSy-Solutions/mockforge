-- Preserve the Free allowance at the owning-account level across organizations
-- and organization deletion. Paid and BYOK accounting are unchanged.
CREATE TABLE free_ai_credit_periods (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Opaque historical attribution survives a settled account deletion.
    owner_user_id UUID NOT NULL,
    period_start DATE NOT NULL,
    granted_tokens BIGINT NOT NULL CHECK (granted_tokens >= -1),
    used_tokens BIGINT NOT NULL DEFAULT 0 CHECK (used_tokens >= 0),
    reserved_tokens BIGINT NOT NULL DEFAULT 0 CHECK (reserved_tokens >= 0),
    UNIQUE(owner_user_id, period_start)
);
ALTER TABLE ai_token_reservations ADD COLUMN free_credit_period_id UUID
    REFERENCES free_ai_credit_periods(id);

-- Existing Free accounting is conservative: neither known usage nor held
-- reservations may disappear when the owner first adopts the shared pool.
WITH configured_grants AS (
    SELECT o.owner_id,
           CASE WHEN jsonb_typeof(s.setting_value)='object' AND s.setting_value ? 'ai_tokens_per_month'
                THEN s.setting_value->'ai_tokens_per_month'
                ELSE o.limits_json->'ai_tokens_per_month' END AS configured
    FROM organizations o LEFT JOIN org_settings s ON s.org_id=o.id AND s.setting_key='quota'
    WHERE o.plan='free'
), numeric_grants AS (
    SELECT owner_id,
           CASE WHEN jsonb_typeof(configured)='number' AND (configured #>> '{}') ~ '^-?[0-9]+$'
                THEN (configured #>> '{}')::bigint ELSE 0 END AS tokens
    FROM configured_grants
), owner_grants AS (
    SELECT owner_id,CASE WHEN BOOL_OR(tokens<0) THEN -1
                       ELSE GREATEST(5000,MAX(tokens)) END AS granted
    FROM numeric_grants GROUP BY owner_id
)
INSERT INTO free_ai_credit_periods(owner_user_id,period_start,granted_tokens,used_tokens,reserved_tokens)
SELECT o.owner_id,u.period_start,g.granted,SUM(u.ai_tokens_used),
       COALESCE(SUM((SELECT COALESCE(SUM(r.budget),0) FROM ai_token_reservations r
                      WHERE r.usage_counter_id=u.id AND r.state <> 'completed')),0)
FROM usage_counters u JOIN organizations o ON o.id=u.org_id
JOIN owner_grants g ON g.owner_id=o.owner_id
WHERE o.plan='free' GROUP BY o.owner_id,u.period_start,g.granted;
UPDATE ai_token_reservations r SET free_credit_period_id=f.id
FROM usage_counters u,organizations o,free_ai_credit_periods f
WHERE r.usage_counter_id=u.id AND o.id=r.org_id AND o.plan='free'
  AND f.owner_user_id=o.owner_id AND f.period_start=u.period_start;

ALTER TABLE free_ai_credit_periods ENABLE ROW LEVEL SECURITY;
ALTER TABLE free_ai_credit_periods FORCE ROW LEVEL SECURITY;
CREATE POLICY owned_account_allowance ON free_ai_credit_periods
USING (EXISTS(SELECT 1 FROM organizations o
    WHERE o.id=nullif(current_setting('app.current_org_id',true),'')::uuid
      AND o.owner_id=owner_user_id));

-- Account deletion may transfer organizations before deleting the owner. Keep
-- that entire transaction from completing while this owner's calls are held.
CREATE FUNCTION preserve_account_free_ai_credits() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$ BEGIN
    IF EXISTS(SELECT 1 FROM free_ai_credit_periods WHERE owner_user_id=OLD.id AND reserved_tokens>0) THEN
        RAISE EXCEPTION 'Resolve outstanding platform AI calls before deleting the account'
            USING ERRCODE='23514';
    END IF;
    RETURN OLD;
END $$;
CREATE TRIGGER preserve_free_ai_accounting_on_user_delete
BEFORE DELETE ON users FOR EACH ROW EXECUTE FUNCTION preserve_account_free_ai_credits();
