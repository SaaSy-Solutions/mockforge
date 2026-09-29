-- Tenant-isolation RLS coverage for every remaining org-scoped table (#1087).
--
-- Migration 20250101000082 put the `org_isolation` policy on five tables
-- (projects, audit_logs, hosted_mocks, templates, scenarios). This one
-- extends the same policy to every other `public` table that carries an
-- `org_id` column, except four tables that are deliberately excluded
-- (listed below, with reasons).
--
-- Purely additive: no column, index, constraint or data changes, and the
-- five existing policies are untouched.
--
-- ── POLICY SHAPE ───────────────────────────────────────────────────────────
-- Identical to 20250101000082's strict shape:
--
--   USING      (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
--   WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
--
-- The `nullif(.., '')` is load-bearing. A tx-local `set_config` reverts to
-- the EMPTY STRING at COMMIT, not to unset, so a bare `::uuid` cast raises
-- 22P02 on any pooled connection that has ever served an org-bound
-- transaction. See 20250101000082 for the full write-up.
--
-- Unbound connection  -> NULL -> no row matches -> zero rows / WITH CHECK
-- failure. Fail closed.
--
-- ── WHO IS AFFECTED ────────────────────────────────────────────────────────
-- Only the NOBYPASSRLS runtime role (`APP_DATABASE_URL`). The owner role
-- (`DATABASE_URL`: migrations, workers, deployment orchestrator, admin
-- cross-org, webhooks, pre-auth lookups) MUST have BYPASSRLS, which the
-- guard below enforces at migration time. Request-path code binds the GUC
-- through `with_current_org` / `with_optional_org` / `with_org_context`;
-- `scripts/check_rls_coverage.py` audits that every statement on a forced
-- table is either bound or deliberately elevated.
--
-- ── TABLES COVERED HERE (strict org_isolation) ─────────────────────────────
--   api_tokens, clone_models, cloud_test_generation_jobs, feature_usage,
--   federations, fixtures, incidents, notification_channels,
--   observability_dashboards, observability_saved_queries,
--   org_ai_budgets, org_ai_feature_toggles, org_ai_rate_limits,
--   org_ai_usage_logs, org_invitations, org_members, org_settings,
--   org_templates, organization_trust_roots, routing_rules,
--   saml_assertion_ids, services, sso_configurations, sso_sessions,
--   subscriptions, suspicious_activities, test_runs, tunnel_reservations,
--   usage_alerts, usage_counters, workspaces
--
-- Notes on the less obvious ones:
--   org_members          The membership lookups that PRODUCE the org binding
--                        (`resolve_org_context`, `verify_org_access`) run on
--                        the owner pool. Store methods that check membership
--                        in a named org bind that org. Creating an org and
--                        listing a user's orgs are user-scoped / pre-tenant
--                        and run on the owner pool.
--   api_tokens           Token verification is pre-auth (the token names the
--                        org) and runs on the owner pool. Token management
--                        endpoints are covered.
--   sso_configurations,  Pre-auth SSO flows bind the org resolved from the
--   sso_sessions,        path slug. Discovery by email domain is cross-org by
--   saml_assertion_ids   definition and runs on the owner pool.
--   suspicious_activities  Rows with a NULL org_id (pre-auth events with no
--                        tenant) are visible only to the owner role. Writes
--                        are append-only telemetry on the owner pool.
--   subscriptions,       Stripe webhooks and billing workers have no user org
--   usage_counters,      context and run on the owner pool. Request-path
--   usage_alerts         billing/quota checks bind the resolved org explicitly.
--   org_invitations,     No request-path code touches these today
--   org_ai_*             (invitations are stored in org_settings; the org_ai_*
--                        tables belong to the not-yet-wired AI studio). Forcing
--                        them now means any future code starts fail-closed.
--
-- ── DELIBERATELY NOT COVERED ───────────────────────────────────────────────
--   plugins                     Global public plugin registry. `org_id` is a
--                               nullable attribution, not a tenant boundary:
--                               search / get / download are public and
--                               cross-tenant by design, and download/rating
--                               counters are updated by any user. The access
--                               rule is author-based (`author_id`).
--   showcase_entries            Public gallery curated by platform admins.
--                               Nullable `org_id`, public reads, admin-only
--                               writes.
--   user_public_keys            User-scoped signing keys. `org_id` is an
--                               optional sharing attribute; publish-time
--                               signature verification reads keys across the
--                               publisher's orgs. Needs a user-scoped GUC
--                               (app.current_user_id) to police properly.
--   cloud_plugin_beta_interest  User-scoped waitlist signal, nullable org_id.
--
--   Tables without an `org_id` column (flows, virtual_entities,
--   runtime_captures, ...) are scoped through `workspace_id` / `deployment_id`
--   and still need join-based policies. Tracked as follow-up in #1087.
--
-- ── ROLLBACK ───────────────────────────────────────────────────────────────
-- sqlx has no down migrations. To revert, run as the table owner:
--   DROP POLICY IF EXISTS org_isolation ON api_tokens; ALTER TABLE api_tokens NO FORCE ROW LEVEL SECURITY; ALTER TABLE api_tokens DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON clone_models; ALTER TABLE clone_models NO FORCE ROW LEVEL SECURITY; ALTER TABLE clone_models DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON cloud_test_generation_jobs; ALTER TABLE cloud_test_generation_jobs NO FORCE ROW LEVEL SECURITY; ALTER TABLE cloud_test_generation_jobs DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON feature_usage; ALTER TABLE feature_usage NO FORCE ROW LEVEL SECURITY; ALTER TABLE feature_usage DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON federations; ALTER TABLE federations NO FORCE ROW LEVEL SECURITY; ALTER TABLE federations DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON fixtures; ALTER TABLE fixtures NO FORCE ROW LEVEL SECURITY; ALTER TABLE fixtures DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON incidents; ALTER TABLE incidents NO FORCE ROW LEVEL SECURITY; ALTER TABLE incidents DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON notification_channels; ALTER TABLE notification_channels NO FORCE ROW LEVEL SECURITY; ALTER TABLE notification_channels DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON observability_dashboards; ALTER TABLE observability_dashboards NO FORCE ROW LEVEL SECURITY; ALTER TABLE observability_dashboards DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON observability_saved_queries; ALTER TABLE observability_saved_queries NO FORCE ROW LEVEL SECURITY; ALTER TABLE observability_saved_queries DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON org_ai_budgets; ALTER TABLE org_ai_budgets NO FORCE ROW LEVEL SECURITY; ALTER TABLE org_ai_budgets DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON org_ai_feature_toggles; ALTER TABLE org_ai_feature_toggles NO FORCE ROW LEVEL SECURITY; ALTER TABLE org_ai_feature_toggles DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON org_ai_rate_limits; ALTER TABLE org_ai_rate_limits NO FORCE ROW LEVEL SECURITY; ALTER TABLE org_ai_rate_limits DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON org_ai_usage_logs; ALTER TABLE org_ai_usage_logs NO FORCE ROW LEVEL SECURITY; ALTER TABLE org_ai_usage_logs DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON org_invitations; ALTER TABLE org_invitations NO FORCE ROW LEVEL SECURITY; ALTER TABLE org_invitations DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON org_members; ALTER TABLE org_members NO FORCE ROW LEVEL SECURITY; ALTER TABLE org_members DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON org_settings; ALTER TABLE org_settings NO FORCE ROW LEVEL SECURITY; ALTER TABLE org_settings DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON org_templates; ALTER TABLE org_templates NO FORCE ROW LEVEL SECURITY; ALTER TABLE org_templates DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON organization_trust_roots; ALTER TABLE organization_trust_roots NO FORCE ROW LEVEL SECURITY; ALTER TABLE organization_trust_roots DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON routing_rules; ALTER TABLE routing_rules NO FORCE ROW LEVEL SECURITY; ALTER TABLE routing_rules DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON saml_assertion_ids; ALTER TABLE saml_assertion_ids NO FORCE ROW LEVEL SECURITY; ALTER TABLE saml_assertion_ids DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON services; ALTER TABLE services NO FORCE ROW LEVEL SECURITY; ALTER TABLE services DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON sso_configurations; ALTER TABLE sso_configurations NO FORCE ROW LEVEL SECURITY; ALTER TABLE sso_configurations DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON sso_sessions; ALTER TABLE sso_sessions NO FORCE ROW LEVEL SECURITY; ALTER TABLE sso_sessions DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON subscriptions; ALTER TABLE subscriptions NO FORCE ROW LEVEL SECURITY; ALTER TABLE subscriptions DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON suspicious_activities; ALTER TABLE suspicious_activities NO FORCE ROW LEVEL SECURITY; ALTER TABLE suspicious_activities DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON test_runs; ALTER TABLE test_runs NO FORCE ROW LEVEL SECURITY; ALTER TABLE test_runs DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON tunnel_reservations; ALTER TABLE tunnel_reservations NO FORCE ROW LEVEL SECURITY; ALTER TABLE tunnel_reservations DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON usage_alerts; ALTER TABLE usage_alerts NO FORCE ROW LEVEL SECURITY; ALTER TABLE usage_alerts DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON usage_counters; ALTER TABLE usage_counters NO FORCE ROW LEVEL SECURITY; ALTER TABLE usage_counters DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON workspaces; ALTER TABLE workspaces NO FORCE ROW LEVEL SECURITY; ALTER TABLE workspaces DISABLE ROW LEVEL SECURITY;

-- ===========================================================================
-- Guard: refuse to force RLS on 30+ more tables when the owner role would be
-- fail-closed by it.
--
-- This migration runs as the owner role (`DATABASE_URL`). Every worker,
-- webhook, pre-auth lookup and cross-org admin path also runs as that role
-- and relies on it bypassing RLS. If it does not (the 2026-09 Ashburn cutover
-- shipped exactly that: owner `mockforge` NOBYPASSRLS), applying these
-- policies would silently zero out billing, SSO, token auth and every
-- background worker. Better to stop the deploy with a clear message.
--
-- Fix: `ALTER ROLE <owner> BYPASSRLS;` (as a superuser), then redeploy.
-- ===========================================================================
DO $$
DECLARE
    r record;
BEGIN
    SELECT rolsuper, rolbypassrls INTO r FROM pg_roles WHERE rolname = current_user;
    IF NOT (r.rolsuper OR r.rolbypassrls) THEN
        RAISE EXCEPTION
            'rls coverage (#1087): migrating role "%" has neither SUPERUSER nor BYPASSRLS. '
            'The owner pool (DATABASE_URL) must bypass RLS or workers, webhooks and '
            'pre-auth lookups fail closed. Run: ALTER ROLE % BYPASSRLS; then redeploy.',
            current_user, quote_ident(current_user);
    END IF;
END $$;

-- ===========================================================================
-- Strict org-scoped tables.
-- ===========================================================================

ALTER TABLE api_tokens ENABLE ROW LEVEL SECURITY;
ALTER TABLE api_tokens FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON api_tokens
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE clone_models ENABLE ROW LEVEL SECURITY;
ALTER TABLE clone_models FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON clone_models
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE cloud_test_generation_jobs ENABLE ROW LEVEL SECURITY;
ALTER TABLE cloud_test_generation_jobs FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON cloud_test_generation_jobs
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE feature_usage ENABLE ROW LEVEL SECURITY;
ALTER TABLE feature_usage FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON feature_usage
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE federations ENABLE ROW LEVEL SECURITY;
ALTER TABLE federations FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON federations
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE fixtures ENABLE ROW LEVEL SECURITY;
ALTER TABLE fixtures FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON fixtures
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE incidents ENABLE ROW LEVEL SECURITY;
ALTER TABLE incidents FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON incidents
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE notification_channels ENABLE ROW LEVEL SECURITY;
ALTER TABLE notification_channels FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON notification_channels
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE observability_dashboards ENABLE ROW LEVEL SECURITY;
ALTER TABLE observability_dashboards FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON observability_dashboards
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE observability_saved_queries ENABLE ROW LEVEL SECURITY;
ALTER TABLE observability_saved_queries FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON observability_saved_queries
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE org_ai_budgets ENABLE ROW LEVEL SECURITY;
ALTER TABLE org_ai_budgets FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON org_ai_budgets
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE org_ai_feature_toggles ENABLE ROW LEVEL SECURITY;
ALTER TABLE org_ai_feature_toggles FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON org_ai_feature_toggles
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE org_ai_rate_limits ENABLE ROW LEVEL SECURITY;
ALTER TABLE org_ai_rate_limits FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON org_ai_rate_limits
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE org_ai_usage_logs ENABLE ROW LEVEL SECURITY;
ALTER TABLE org_ai_usage_logs FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON org_ai_usage_logs
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE org_invitations ENABLE ROW LEVEL SECURITY;
ALTER TABLE org_invitations FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON org_invitations
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE org_members ENABLE ROW LEVEL SECURITY;
ALTER TABLE org_members FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON org_members
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE org_settings ENABLE ROW LEVEL SECURITY;
ALTER TABLE org_settings FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON org_settings
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE org_templates ENABLE ROW LEVEL SECURITY;
ALTER TABLE org_templates FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON org_templates
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE organization_trust_roots ENABLE ROW LEVEL SECURITY;
ALTER TABLE organization_trust_roots FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON organization_trust_roots
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE routing_rules ENABLE ROW LEVEL SECURITY;
ALTER TABLE routing_rules FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON routing_rules
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE saml_assertion_ids ENABLE ROW LEVEL SECURITY;
ALTER TABLE saml_assertion_ids FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON saml_assertion_ids
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE services ENABLE ROW LEVEL SECURITY;
ALTER TABLE services FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON services
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE sso_configurations ENABLE ROW LEVEL SECURITY;
ALTER TABLE sso_configurations FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON sso_configurations
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE sso_sessions ENABLE ROW LEVEL SECURITY;
ALTER TABLE sso_sessions FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON sso_sessions
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE subscriptions ENABLE ROW LEVEL SECURITY;
ALTER TABLE subscriptions FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON subscriptions
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE suspicious_activities ENABLE ROW LEVEL SECURITY;
ALTER TABLE suspicious_activities FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON suspicious_activities
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE test_runs ENABLE ROW LEVEL SECURITY;
ALTER TABLE test_runs FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON test_runs
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE tunnel_reservations ENABLE ROW LEVEL SECURITY;
ALTER TABLE tunnel_reservations FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON tunnel_reservations
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE usage_alerts ENABLE ROW LEVEL SECURITY;
ALTER TABLE usage_alerts FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON usage_alerts
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE usage_counters ENABLE ROW LEVEL SECURITY;
ALTER TABLE usage_counters FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON usage_counters
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);

ALTER TABLE workspaces ENABLE ROW LEVEL SECURITY;
ALTER TABLE workspaces FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON workspaces
    USING (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid)
    WITH CHECK (org_id = nullif(current_setting('app.current_org_id', true), '')::uuid);
