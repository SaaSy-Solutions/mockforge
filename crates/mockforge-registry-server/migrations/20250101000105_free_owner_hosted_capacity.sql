-- Updating this internal row also fences admission under repeatable-read
-- snapshots; locking an unchanged user row alone would not refresh a snapshot.
CREATE TABLE free_hosted_capacity_locks (
    owner_user_id UUID PRIMARY KEY,
    revision BIGINT NOT NULL DEFAULT 0
);
ALTER TABLE free_hosted_capacity_locks ENABLE ROW LEVEL SECURITY;
ALTER TABLE free_hosted_capacity_locks FORCE ROW LEVEL SECURITY;
CREATE POLICY owned_capacity_lock ON free_hosted_capacity_locks
USING(EXISTS(SELECT 1 FROM organizations o WHERE o.id=nullif(current_setting('app.current_org_id',true),'')::uuid AND o.owner_id=owner_user_id));

-- Return only an admission decision, never sibling-tenant deployment data.
-- The runtime pool cannot directly aggregate RLS-isolated sibling orgs.
CREATE FUNCTION free_owner_hosted_capacity(p_org UUID,p_existing UUID) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE owner UUID; occupied BIGINT; maximum BIGINT; verified BOOLEAN;
BEGIN
    IF p_org IS DISTINCT FROM nullif(current_setting('app.current_org_id',true),'')::uuid THEN
        RETURN FALSE;
    END IF;
    SELECT owner_id INTO owner FROM organizations WHERE id=p_org;
    IF owner IS NULL THEN RETURN FALSE; END IF;
    -- A write-locked revision row serializes admissions and transfers, including
    -- transactions using a repeatable-read snapshot. No user profile is changed.
    INSERT INTO free_hosted_capacity_locks(owner_user_id) VALUES(owner)
    ON CONFLICT(owner_user_id) DO UPDATE SET revision=free_hosted_capacity_locks.revision+1;
    -- Compatible with the foreign-key key-share lock taken by ownership transfer.
    SELECT is_verified INTO verified FROM users WHERE id=owner FOR KEY SHARE;
    IF verified IS DISTINCT FROM TRUE THEN RETURN FALSE; END IF;
    IF p_existing IS NOT NULL AND NOT EXISTS(SELECT 1 FROM hosted_mocks WHERE id=p_existing AND org_id=p_org) THEN
        RETURN FALSE;
    END IF;
    WITH free_orgs AS (
        SELECT o.* FROM organizations o WHERE o.owner_id=owner AND
        (o.plan NOT IN ('pro','team') OR EXISTS(SELECT 1 FROM subscriptions s WHERE s.org_id=o.id
            AND (s.status NOT IN ('active','trialing','past_due') OR
                 (s.status='past_due' AND s.updated_at<NOW()-interval '24 hours'))))
    ), configured AS (
        SELECT o.id,CASE WHEN jsonb_typeof(s.setting_value)='object' AND s.setting_value ? 'max_hosted_mocks'
            THEN s.setting_value->'max_hosted_mocks' ELSE
            CASE WHEN o.plan='free' THEN o.limits_json->'max_hosted_mocks' ELSE '1'::jsonb END END AS quota
        FROM free_orgs o LEFT JOIN org_settings s ON s.org_id=o.id AND s.setting_key='quota'
    ), grants AS (
        SELECT id,CASE WHEN jsonb_typeof(quota)='number' AND (quota #>> '{}') ~ '^-?[0-9]+$'
            THEN (quota #>> '{}')::bigint ELSE 1 END AS quota FROM configured
    )
    SELECT COALESCE((SELECT CASE WHEN BOOL_OR(quota<0) THEN -1 ELSE GREATEST(1,MAX(quota)) END FROM grants),1),
           (SELECT COUNT(*) FROM hosted_mocks h JOIN grants g ON g.id=h.org_id
             WHERE h.capacity_reserved AND (p_existing IS NULL OR h.id<>p_existing))
    INTO maximum,occupied;
    RETURN maximum<0 OR occupied<maximum;
END $$;
