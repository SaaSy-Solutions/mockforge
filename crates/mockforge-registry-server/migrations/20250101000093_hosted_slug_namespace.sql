-- Subdomain routing has no organization ID, so new hosted slugs must be
-- globally unique. Preserve existing URLs/data; ambiguous legacy slugs are
-- rejected by routing instead of choosing another tenant's first row.
CREATE FUNCTION reserve_global_hosted_slug() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = public, pg_temp AS $$
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended('mockforge-hosted-slug:' || lower(NEW.slug), 0));
    IF EXISTS (
        SELECT 1 FROM hosted_mocks WHERE lower(slug) = lower(NEW.slug) AND id <> NEW.id
        AND (deleted_at IS NULL OR capacity_reserved)
    ) THEN
        RAISE EXCEPTION 'Hosted slug is unavailable'
            USING ERRCODE = '23505', CONSTRAINT = 'hosted_mocks_global_slug';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER reserve_global_hosted_slug
BEFORE INSERT OR UPDATE OF slug, org_id ON hosted_mocks
FOR EACH ROW EXECUTE FUNCTION reserve_global_hosted_slug();
