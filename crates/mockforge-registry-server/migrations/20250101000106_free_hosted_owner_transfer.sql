-- Account deletion can transfer an organization instead of removing it. Apply
-- the same funding boundary to that transfer, retaining resources on failure.
CREATE FUNCTION enforce_free_hosted_owner_transfer() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE existing UUID; previous_context TEXT; allowed BOOLEAN;
BEGIN
    IF OLD.owner_id IS NOT DISTINCT FROM NEW.owner_id THEN RETURN NEW; END IF;
    IF NEW.plan IN ('pro','team') AND NOT EXISTS(SELECT 1 FROM subscriptions s WHERE s.org_id=NEW.id
        AND (s.status NOT IN ('active','trialing','past_due') OR
             (s.status='past_due' AND s.updated_at<NOW()-interval '24 hours'))) THEN
        RETURN NEW;
    END IF;
    SELECT id INTO existing FROM hosted_mocks WHERE org_id=NEW.id AND capacity_reserved LIMIT 1;
    IF existing IS NULL THEN RETURN NEW; END IF;
    previous_context:=current_setting('app.current_org_id',true);
    PERFORM set_config('app.current_org_id',NEW.id::text,true);
    -- Excluding exactly one incoming reservation turns the '< maximum'
    -- admission predicate into '<= maximum' for the already-transferred total.
    allowed:=free_owner_hosted_capacity(NEW.id,existing);
    PERFORM set_config('app.current_org_id',COALESCE(previous_context,''),true);
    IF NOT allowed THEN
        RAISE EXCEPTION 'Confirmed cleanup or a larger verified Free grant is required before transferring occupied hosted deployments'
            USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER enforce_free_account_capacity_on_owner_transfer
AFTER UPDATE OF owner_id ON organizations FOR EACH ROW EXECUTE FUNCTION enforce_free_hosted_owner_transfer();
