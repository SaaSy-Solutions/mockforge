-- Organization deletion must not cascade away durable provider cleanup intent.
-- DELETE locks the organization row, serializing against hosted admission's
-- FOR UPDATE lock and the foreign key's parent-row lock. Inspect as the owner
-- so request-role RLS cannot hide reservations or legacy tombstones.
CREATE FUNCTION prevent_unfinished_hosted_org_deletion() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = public, pg_temp AS $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM hosted_mocks
        WHERE org_id = OLD.id AND (capacity_reserved OR deleted_at IS NULL)
    ) THEN
        RAISE EXCEPTION 'Delete hosted deployments and wait for cleanup before deleting the organization'
            USING ERRCODE = '23514';
    END IF;
    RETURN OLD;
END;
$$;

CREATE TRIGGER preserve_hosted_cleanup_on_org_delete
BEFORE DELETE ON organizations
FOR EACH ROW EXECUTE FUNCTION prevent_unfinished_hosted_org_deletion();
