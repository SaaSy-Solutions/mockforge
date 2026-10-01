-- Per-user RLS for the user-owned tables (#1087).
--
-- 20250101000086 left two tables out of org isolation because their owner is
-- a USER, not an org:
--
--   user_public_keys            publisher signing keys. `org_id` is an
--                               optional sharing tag, not the owner.
--   cloud_plugin_beta_interest  per-user waitlist signal. `org_id` is a
--                               snapshot for segmentation, not the owner.
--
-- This migration polices both with a second GUC, `app.current_user_id`, bound
-- exactly like `app.current_org_id`: the `rls_org_scope` middleware puts the
-- authenticated user in the CURRENT_USER task-local, and `with_current_user` /
-- `with_user_context` set it transaction-local (`set_config(.., true)`).
--
-- ── POLICIES ───────────────────────────────────────────────────────────────
--   user_isolation   (both tables)
--       user_id = nullif(current_setting('app.current_user_id', true), '')::uuid
--
--   org_shared_keys  (user_public_keys only, a second PERMISSIVE policy, so
--                    the two are OR-ed)
--       org_id = nullif(current_setting('app.current_org_id', true), '')::uuid
--
--   Why the second policy: a key tagged to an org is managed by that org's
--   admins too. `GET /organizations/{id}/public-keys` lists them and
--   `DELETE /users/me/public-keys/{id}` lets an org admin revoke a member's
--   org-tagged key. Those paths bind the key's org (after `require_org_admin`)
--   and would otherwise see nothing. An untagged key (`org_id IS NULL`) never
--   matches it, so it stays visible to its owner only.
--
-- The `nullif(.., '')` is load-bearing for the same reason as in 082/086: a
-- transaction-local GUC reverts to the EMPTY STRING at COMMIT on a pooled
-- connection, and a bare `::uuid` cast of '' raises 22P02.
--
-- Unbound connection -> NULL -> no row matches -> zero rows / WITH CHECK
-- failure. Fail closed.
--
-- ── OWNER-POOL PATHS (unaffected: the owner role has BYPASSRLS) ────────────
--   list_keys_for_publisher       publish-time signature check across every
--                                 org the author belongs to.
--   find_user_public_key_by_id    learns a key's org tag so the caller's admin
--                                 role on that org can be checked; the org to
--                                 bind is derived from this answer.
--   delete_user_data_cascade      GDPR erasure.
--   Pre-auth lookups              none read these tables today; any future one
--                                 must use the owner pool or bind explicitly.
--
-- Additive and idempotent: ENABLE/FORCE are no-ops when already set and each
-- policy is created only if missing.
--
-- ── ROLLBACK ───────────────────────────────────────────────────────────────
-- Run as the table owner:
--   DROP POLICY IF EXISTS user_isolation ON user_public_keys; DROP POLICY IF EXISTS org_shared_keys ON user_public_keys; ALTER TABLE user_public_keys NO FORCE ROW LEVEL SECURITY; ALTER TABLE user_public_keys DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS user_isolation ON cloud_plugin_beta_interest; ALTER TABLE cloud_plugin_beta_interest NO FORCE ROW LEVEL SECURITY; ALTER TABLE cloud_plugin_beta_interest DISABLE ROW LEVEL SECURITY;

-- Same guard as 086: the owner pool must bypass RLS, or GDPR erasure and the
-- publish-time key lookup fail closed.
DO $$
DECLARE
    r record;
BEGIN
    SELECT rolsuper, rolbypassrls INTO r FROM pg_roles WHERE rolname = current_user;
    IF NOT (r.rolsuper OR r.rolbypassrls) THEN
        RAISE EXCEPTION
            'rls user-scoped tables (#1087): migrating role "%" has neither SUPERUSER nor BYPASSRLS. '
            'The owner pool (DATABASE_URL) must bypass RLS. Run: ALTER ROLE % BYPASSRLS; then redeploy.',
            current_user, quote_ident(current_user);
    END IF;
END $$;

ALTER TABLE user_public_keys ENABLE ROW LEVEL SECURITY;
ALTER TABLE user_public_keys FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'user_public_keys' AND policyname = 'user_isolation') THEN
        CREATE POLICY user_isolation ON user_public_keys
            USING (user_id = nullif(current_setting('app.current_user_id', true), '')::uuid)
            WITH CHECK (user_id = nullif(current_setting('app.current_user_id', true), '')::uuid);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'user_public_keys' AND policyname = 'org_shared_keys') THEN
        CREATE POLICY org_shared_keys ON user_public_keys
            USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
            WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);
    END IF;
END $$;

ALTER TABLE cloud_plugin_beta_interest ENABLE ROW LEVEL SECURITY;
ALTER TABLE cloud_plugin_beta_interest FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'cloud_plugin_beta_interest' AND policyname = 'user_isolation') THEN
        CREATE POLICY user_isolation ON cloud_plugin_beta_interest
            USING (user_id = nullif(current_setting('app.current_user_id', true), '')::uuid)
            WITH CHECK (user_id = nullif(current_setting('app.current_user_id', true), '')::uuid);
    END IF;
END $$;
