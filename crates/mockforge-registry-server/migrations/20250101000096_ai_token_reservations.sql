-- A durable reservation serializes platform AI spend before provider calls.
-- Unknown provider spend remains reserved until authoritative reconciliation;
-- age alone must never release credits that might already have been consumed.
ALTER TABLE usage_counters ADD CONSTRAINT usage_counters_id_org_unique UNIQUE (id, org_id);
CREATE TABLE ai_token_reservations (
    id UUID PRIMARY KEY,
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    usage_counter_id UUID NOT NULL,
    budget BIGINT NOT NULL CHECK (budget > 0),
    actual_tokens BIGINT CHECK (actual_tokens >= 0),
    state TEXT NOT NULL DEFAULT 'pending' CHECK (state IN ('pending','completed','uncertain')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (usage_counter_id, org_id) REFERENCES usage_counters(id, org_id),
    CHECK ((state = 'completed') = (actual_tokens IS NOT NULL))
);
CREATE INDEX ai_token_reservations_held ON ai_token_reservations(usage_counter_id)
    WHERE state <> 'completed';
ALTER TABLE ai_token_reservations ENABLE ROW LEVEL SECURITY;
ALTER TABLE ai_token_reservations FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON ai_token_reservations
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

CREATE FUNCTION preserve_unsettled_ai_credits() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = public, pg_temp AS $$
BEGIN
    IF EXISTS (SELECT 1 FROM ai_token_reservations WHERE org_id=OLD.id AND state <> 'completed') THEN
        RAISE EXCEPTION 'Resolve outstanding AI calls before deleting the organization'
            USING ERRCODE = '23514';
    END IF;
    RETURN OLD;
END;
$$;
CREATE TRIGGER preserve_ai_accounting_on_org_delete
BEFORE DELETE ON organizations FOR EACH ROW EXECUTE FUNCTION preserve_unsettled_ai_credits();
