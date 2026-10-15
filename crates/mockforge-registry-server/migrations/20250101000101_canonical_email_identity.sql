-- Never combine legacy accounts: operators must resolve identity collisions
-- with verified ownership before this migration can complete.
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM users GROUP BY lower(btrim(email)) HAVING COUNT(*) > 1) THEN
        RAISE EXCEPTION 'Canonical email collisions require an ownership-reviewed repair before migration';
    END IF;
END $$;

CREATE UNIQUE INDEX users_canonical_email_unique ON users(lower(btrim(email)));
CREATE INDEX login_attempts_canonical_email_created ON login_attempts(lower(btrim(email)), created_at);

CREATE FUNCTION canonicalize_user_email() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    NEW.email := lower(btrim(NEW.email));
    IF TG_OP = 'UPDATE' AND lower(btrim(OLD.email)) IS DISTINCT FROM NEW.email THEN
        -- Verification belongs to the old address, not the new identity.
        NEW.is_verified := FALSE;
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER canonical_user_email BEFORE INSERT OR UPDATE OF email ON users
FOR EACH ROW EXECUTE FUNCTION canonicalize_user_email();
