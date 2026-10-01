-- Join-based tenant isolation for tenant tables WITHOUT an `org_id` column
-- (#1087).
--
-- 20250101000082 and 20250101000086 forced RLS on every table that carries
-- `org_id`. Tenant data also lives in tables that are scoped through a parent:
-- `flows.workspace_id -> workspaces.org_id`,
-- `runtime_captures.deployment_id -> hosted_mocks.org_id`, and so on. This
-- migration forces RLS on all 38 of them, with a policy that walks the
-- foreign-key chain to the org-keyed ancestor.
--
-- The table list was produced mechanically: every `public` table with no
-- `org_id` column, its FK chain followed until it reaches a table that has one
-- (see "DELIBERATELY NOT COVERED" for the rest).
--
-- ── POLICY SHAPE ───────────────────────────────────────────────────────────
-- One hop (child -> org-keyed parent):
--
--   USING (EXISTS (SELECT 1 FROM workspaces p1
--                  WHERE p1.id = flows.workspace_id
--                    AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
--
-- Several hops join the whole chain and test `org_id` on the last table,
-- e.g. contract_diff_findings -> contract_diff_runs -> monitored_services ->
-- workspaces. WITH CHECK is the same expression, so a row can only be written
-- under a parent that belongs to the bound org.
--
-- `resilience_patterns` is the one variation: `workspace_id IS NULL` marks a
-- platform-provided pattern every tenant may READ, so USING also admits NULL;
-- WITH CHECK does not, so platform rows are written only by the owner role.
--
-- The `nullif(.., '')` is load-bearing, as in 082/086: a transaction-local GUC
-- reverts to the EMPTY STRING at COMMIT on a pooled connection, and a bare
-- `::uuid` cast of '' raises 22P02. Unbound -> NULL -> no parent matches ->
-- zero rows / WITH CHECK failure. Fail closed.
--
-- ── NESTED RLS ─────────────────────────────────────────────────────────────
-- A policy's subquery runs as the invoking role, so the PARENT's own policy
-- applies inside it too (`workspaces` is itself org-isolated). With the GUC
-- bound that is redundant, not harmful: both predicates name the same org.
-- With the GUC unbound both are false, which is the fail-closed answer. The
-- explicit `org_id = ..` on the last hop keeps each policy correct on its own
-- even if a parent's policy is ever loosened. There are no cycles: every
-- policy points strictly up the FK graph towards an org-keyed table
-- (`flows.current_version_id -> flow_versions` is not used by any policy), so
-- Postgres cannot hit "infinite recursion detected in policy".
--
-- Foreign-key checks and ON DELETE CASCADE actions run as the table owner and
-- bypass RLS, so deleting a workspace still cascades to its children.
--
-- ── PERFORMANCE ────────────────────────────────────────────────────────────
-- Every EXISTS probes the parent's primary key (`p.id = child.fk`), and every
-- join column in a multi-hop chain is an indexed FK. The child FK columns the
-- application filters by are already indexed (verified when this migration
-- was written: capture_session_members.session_id,
-- chaos_campaign_reports.campaign_id, contract_diff_runs.monitored_service_id,
-- contract_diff_findings.run_id, deployment_logs/metrics.hosted_mock_id,
-- federation_scenario_activations.federation_id, fitness_evaluations.function_id,
-- flow_versions.flow_id, hosted_mock_plugins.deployment_id,
-- incident_events.incident_id, runtime_*.deployment_id,
-- scenario_promotions/scenario_environment_versions.workspace_id,
-- test_run_artifacts/test_run_events.run_id, test_schedules.suite_id,
-- tunnel_sessions.reservation_id, workspace_env_variables.environment_id,
-- workspace_request_history.workspace_id, and workspace_id on every direct
-- workspace child). No index is added here.
--
-- ── WHO IS AFFECTED ────────────────────────────────────────────────────────
-- Only the NOBYPASSRLS runtime role (`APP_DATABASE_URL`). Workers, the
-- deployment orchestrator, data-plane ingest (runtime logs / captures / OTLP
-- spans authenticated by a deployment token) and the internal runner API run
-- on the owner role, which must have BYPASSRLS (guard below).
-- `scripts/check_rls_coverage.py` audits every statement on these tables.
--
-- ── TABLES COVERED HERE ────────────────────────────────────────────────────
--   via workspaces:       capture_sessions, chaos_campaigns,
--                         cloud_mockai_rule_explanations, fitness_functions,
--                         flows, mock_environments (no FK constraint, column
--                         only), monitored_services, resilience_patterns,
--                         scenario_environment_versions, scenario_promotions,
--                         snapshot_schedules, snapshots, test_suites,
--                         verification_suites, virtual_entities,
--                         workspace_environments, workspace_folders,
--                         workspace_request_history, workspace_requests
--   via hosted_mocks:     deployment_logs, deployment_metrics,
--                         hosted_mock_plugins, runtime_captures,
--                         runtime_request_logs, runtime_traces
--   via other org parents: federation_scenario_activations (federations),
--                         incident_events (incidents), test_run_artifacts and
--                         test_run_events (test_runs), tunnel_sessions
--                         (tunnel_reservations)
--   two or more hops:     capture_session_members, chaos_campaign_reports,
--                         contract_diff_runs, contract_diff_findings,
--                         fitness_evaluations, flow_versions, test_schedules,
--                         workspace_env_variables
--
--   scenario_promotions / scenario_environment_versions are keyed by
--   `workspace_id`, NOT `scenario_id`: the promoted scenario can be a
--   marketplace item owned by a different org, while the promotion record
--   belongs to the promoting workspace.
--
-- ── DELIBERATELY NOT COVERED ───────────────────────────────────────────────
--   Public marketplace children (readers and reviewers are, by design, in
--   other orgs; the parent policies already admit public rows):
--     template_versions, template_tags, template_reviews, template_stars,
--     scenario_versions, scenario_tags, scenario_reviews, scenario_stars
--   Global plugin registry (parent `plugins` is not tenant-scoped, see 086):
--     plugin_versions, plugin_tags, plugin_dependencies,
--     plugin_security_scans, reviews
--   Public gallery: showcase_likes (parent showcase_entries is public).
--   User-scoped, not tenant data (pre-auth/auth flows read them by user or
--   email): users, user_settings, verification_tokens, token_revocations,
--   login_attempts, learning_progress. user_public_keys and
--   cloud_plugin_beta_interest get a per-user GUC in 20250101000088.
--   Global reference / platform content: organizations (the tenant table
--   itself; org resolution reads it to PRODUCE the binding), tags,
--   osv_vulnerabilities, learning_tracks, learning_lessons,
--   learning_recipes, waitlist_subscribers.
--
-- ── ROLLBACK ───────────────────────────────────────────────────────────────
-- sqlx has no down migrations. To revert, run as the table owner:
--   DROP POLICY IF EXISTS org_isolation ON capture_session_members; ALTER TABLE capture_session_members NO FORCE ROW LEVEL SECURITY; ALTER TABLE capture_session_members DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON capture_sessions; ALTER TABLE capture_sessions NO FORCE ROW LEVEL SECURITY; ALTER TABLE capture_sessions DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON chaos_campaign_reports; ALTER TABLE chaos_campaign_reports NO FORCE ROW LEVEL SECURITY; ALTER TABLE chaos_campaign_reports DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON chaos_campaigns; ALTER TABLE chaos_campaigns NO FORCE ROW LEVEL SECURITY; ALTER TABLE chaos_campaigns DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON cloud_mockai_rule_explanations; ALTER TABLE cloud_mockai_rule_explanations NO FORCE ROW LEVEL SECURITY; ALTER TABLE cloud_mockai_rule_explanations DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON contract_diff_findings; ALTER TABLE contract_diff_findings NO FORCE ROW LEVEL SECURITY; ALTER TABLE contract_diff_findings DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON contract_diff_runs; ALTER TABLE contract_diff_runs NO FORCE ROW LEVEL SECURITY; ALTER TABLE contract_diff_runs DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON deployment_logs; ALTER TABLE deployment_logs NO FORCE ROW LEVEL SECURITY; ALTER TABLE deployment_logs DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON deployment_metrics; ALTER TABLE deployment_metrics NO FORCE ROW LEVEL SECURITY; ALTER TABLE deployment_metrics DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON federation_scenario_activations; ALTER TABLE federation_scenario_activations NO FORCE ROW LEVEL SECURITY; ALTER TABLE federation_scenario_activations DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON fitness_evaluations; ALTER TABLE fitness_evaluations NO FORCE ROW LEVEL SECURITY; ALTER TABLE fitness_evaluations DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON fitness_functions; ALTER TABLE fitness_functions NO FORCE ROW LEVEL SECURITY; ALTER TABLE fitness_functions DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON flow_versions; ALTER TABLE flow_versions NO FORCE ROW LEVEL SECURITY; ALTER TABLE flow_versions DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON flows; ALTER TABLE flows NO FORCE ROW LEVEL SECURITY; ALTER TABLE flows DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON hosted_mock_plugins; ALTER TABLE hosted_mock_plugins NO FORCE ROW LEVEL SECURITY; ALTER TABLE hosted_mock_plugins DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON incident_events; ALTER TABLE incident_events NO FORCE ROW LEVEL SECURITY; ALTER TABLE incident_events DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON mock_environments; ALTER TABLE mock_environments NO FORCE ROW LEVEL SECURITY; ALTER TABLE mock_environments DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON monitored_services; ALTER TABLE monitored_services NO FORCE ROW LEVEL SECURITY; ALTER TABLE monitored_services DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON resilience_patterns; ALTER TABLE resilience_patterns NO FORCE ROW LEVEL SECURITY; ALTER TABLE resilience_patterns DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON runtime_captures; ALTER TABLE runtime_captures NO FORCE ROW LEVEL SECURITY; ALTER TABLE runtime_captures DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON runtime_request_logs; ALTER TABLE runtime_request_logs NO FORCE ROW LEVEL SECURITY; ALTER TABLE runtime_request_logs DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON runtime_traces; ALTER TABLE runtime_traces NO FORCE ROW LEVEL SECURITY; ALTER TABLE runtime_traces DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON scenario_environment_versions; ALTER TABLE scenario_environment_versions NO FORCE ROW LEVEL SECURITY; ALTER TABLE scenario_environment_versions DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON scenario_promotions; ALTER TABLE scenario_promotions NO FORCE ROW LEVEL SECURITY; ALTER TABLE scenario_promotions DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON snapshot_schedules; ALTER TABLE snapshot_schedules NO FORCE ROW LEVEL SECURITY; ALTER TABLE snapshot_schedules DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON snapshots; ALTER TABLE snapshots NO FORCE ROW LEVEL SECURITY; ALTER TABLE snapshots DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON test_run_artifacts; ALTER TABLE test_run_artifacts NO FORCE ROW LEVEL SECURITY; ALTER TABLE test_run_artifacts DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON test_run_events; ALTER TABLE test_run_events NO FORCE ROW LEVEL SECURITY; ALTER TABLE test_run_events DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON test_schedules; ALTER TABLE test_schedules NO FORCE ROW LEVEL SECURITY; ALTER TABLE test_schedules DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON test_suites; ALTER TABLE test_suites NO FORCE ROW LEVEL SECURITY; ALTER TABLE test_suites DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON tunnel_sessions; ALTER TABLE tunnel_sessions NO FORCE ROW LEVEL SECURITY; ALTER TABLE tunnel_sessions DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON verification_suites; ALTER TABLE verification_suites NO FORCE ROW LEVEL SECURITY; ALTER TABLE verification_suites DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON virtual_entities; ALTER TABLE virtual_entities NO FORCE ROW LEVEL SECURITY; ALTER TABLE virtual_entities DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON workspace_env_variables; ALTER TABLE workspace_env_variables NO FORCE ROW LEVEL SECURITY; ALTER TABLE workspace_env_variables DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON workspace_environments; ALTER TABLE workspace_environments NO FORCE ROW LEVEL SECURITY; ALTER TABLE workspace_environments DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON workspace_folders; ALTER TABLE workspace_folders NO FORCE ROW LEVEL SECURITY; ALTER TABLE workspace_folders DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON workspace_request_history; ALTER TABLE workspace_request_history NO FORCE ROW LEVEL SECURITY; ALTER TABLE workspace_request_history DISABLE ROW LEVEL SECURITY;
--   DROP POLICY IF EXISTS org_isolation ON workspace_requests; ALTER TABLE workspace_requests NO FORCE ROW LEVEL SECURITY; ALTER TABLE workspace_requests DISABLE ROW LEVEL SECURITY;

-- ===========================================================================
-- Guard: the owner pool must bypass RLS (same check as 086). Workers, the
-- deployment orchestrator, data-plane ingest and the internal runner API all
-- touch these tables on the owner role; without BYPASSRLS they would silently
-- read nothing.
-- ===========================================================================
DO $$
DECLARE
    r record;
BEGIN
    SELECT rolsuper, rolbypassrls INTO r FROM pg_roles WHERE rolname = current_user;
    IF NOT (r.rolsuper OR r.rolbypassrls) THEN
        RAISE EXCEPTION
            'rls join policies (#1087): migrating role "%" has neither SUPERUSER nor BYPASSRLS. '
            'The owner pool (DATABASE_URL) must bypass RLS or workers, the deployment '
            'orchestrator and data-plane ingest fail closed. Run: ALTER ROLE % BYPASSRLS; then redeploy.',
            current_user, quote_ident(current_user);
    END IF;
END $$;

-- Every policy is created only if missing, and ENABLE / FORCE are no-ops when
-- already set, so re-running this file is harmless.

ALTER TABLE capture_session_members ENABLE ROW LEVEL SECURITY;
ALTER TABLE capture_session_members FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'capture_session_members' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON capture_session_members
            USING (EXISTS (SELECT 1 FROM capture_sessions p1
                    JOIN workspaces p2 ON p2.id = p1.workspace_id
                WHERE p1.id = capture_session_members.session_id
                  AND p2.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM capture_sessions p1
                    JOIN workspaces p2 ON p2.id = p1.workspace_id
                WHERE p1.id = capture_session_members.session_id
                  AND p2.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE capture_sessions ENABLE ROW LEVEL SECURITY;
ALTER TABLE capture_sessions FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'capture_sessions' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON capture_sessions
            USING (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = capture_sessions.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = capture_sessions.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE chaos_campaign_reports ENABLE ROW LEVEL SECURITY;
ALTER TABLE chaos_campaign_reports FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'chaos_campaign_reports' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON chaos_campaign_reports
            USING (EXISTS (SELECT 1 FROM chaos_campaigns p1
                    JOIN workspaces p2 ON p2.id = p1.workspace_id
                WHERE p1.id = chaos_campaign_reports.campaign_id
                  AND p2.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM chaos_campaigns p1
                    JOIN workspaces p2 ON p2.id = p1.workspace_id
                WHERE p1.id = chaos_campaign_reports.campaign_id
                  AND p2.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE chaos_campaigns ENABLE ROW LEVEL SECURITY;
ALTER TABLE chaos_campaigns FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'chaos_campaigns' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON chaos_campaigns
            USING (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = chaos_campaigns.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = chaos_campaigns.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE cloud_mockai_rule_explanations ENABLE ROW LEVEL SECURITY;
ALTER TABLE cloud_mockai_rule_explanations FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'cloud_mockai_rule_explanations' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON cloud_mockai_rule_explanations
            USING (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = cloud_mockai_rule_explanations.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = cloud_mockai_rule_explanations.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE contract_diff_findings ENABLE ROW LEVEL SECURITY;
ALTER TABLE contract_diff_findings FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'contract_diff_findings' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON contract_diff_findings
            USING (EXISTS (SELECT 1 FROM contract_diff_runs p1
                    JOIN monitored_services p2 ON p2.id = p1.monitored_service_id
                    JOIN workspaces p3 ON p3.id = p2.workspace_id
                WHERE p1.id = contract_diff_findings.run_id
                  AND p3.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM contract_diff_runs p1
                    JOIN monitored_services p2 ON p2.id = p1.monitored_service_id
                    JOIN workspaces p3 ON p3.id = p2.workspace_id
                WHERE p1.id = contract_diff_findings.run_id
                  AND p3.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE contract_diff_runs ENABLE ROW LEVEL SECURITY;
ALTER TABLE contract_diff_runs FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'contract_diff_runs' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON contract_diff_runs
            USING (EXISTS (SELECT 1 FROM monitored_services p1
                    JOIN workspaces p2 ON p2.id = p1.workspace_id
                WHERE p1.id = contract_diff_runs.monitored_service_id
                  AND p2.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM monitored_services p1
                    JOIN workspaces p2 ON p2.id = p1.workspace_id
                WHERE p1.id = contract_diff_runs.monitored_service_id
                  AND p2.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE deployment_logs ENABLE ROW LEVEL SECURITY;
ALTER TABLE deployment_logs FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'deployment_logs' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON deployment_logs
            USING (EXISTS (SELECT 1 FROM hosted_mocks p1
                WHERE p1.id = deployment_logs.hosted_mock_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM hosted_mocks p1
                WHERE p1.id = deployment_logs.hosted_mock_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE deployment_metrics ENABLE ROW LEVEL SECURITY;
ALTER TABLE deployment_metrics FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'deployment_metrics' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON deployment_metrics
            USING (EXISTS (SELECT 1 FROM hosted_mocks p1
                WHERE p1.id = deployment_metrics.hosted_mock_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM hosted_mocks p1
                WHERE p1.id = deployment_metrics.hosted_mock_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE federation_scenario_activations ENABLE ROW LEVEL SECURITY;
ALTER TABLE federation_scenario_activations FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'federation_scenario_activations' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON federation_scenario_activations
            USING (EXISTS (SELECT 1 FROM federations p1
                WHERE p1.id = federation_scenario_activations.federation_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM federations p1
                WHERE p1.id = federation_scenario_activations.federation_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE fitness_evaluations ENABLE ROW LEVEL SECURITY;
ALTER TABLE fitness_evaluations FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'fitness_evaluations' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON fitness_evaluations
            USING (EXISTS (SELECT 1 FROM fitness_functions p1
                    JOIN workspaces p2 ON p2.id = p1.workspace_id
                WHERE p1.id = fitness_evaluations.function_id
                  AND p2.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM fitness_functions p1
                    JOIN workspaces p2 ON p2.id = p1.workspace_id
                WHERE p1.id = fitness_evaluations.function_id
                  AND p2.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE fitness_functions ENABLE ROW LEVEL SECURITY;
ALTER TABLE fitness_functions FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'fitness_functions' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON fitness_functions
            USING (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = fitness_functions.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = fitness_functions.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE flow_versions ENABLE ROW LEVEL SECURITY;
ALTER TABLE flow_versions FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'flow_versions' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON flow_versions
            USING (EXISTS (SELECT 1 FROM flows p1
                    JOIN workspaces p2 ON p2.id = p1.workspace_id
                WHERE p1.id = flow_versions.flow_id
                  AND p2.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM flows p1
                    JOIN workspaces p2 ON p2.id = p1.workspace_id
                WHERE p1.id = flow_versions.flow_id
                  AND p2.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE flows ENABLE ROW LEVEL SECURITY;
ALTER TABLE flows FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'flows' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON flows
            USING (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = flows.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = flows.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE hosted_mock_plugins ENABLE ROW LEVEL SECURITY;
ALTER TABLE hosted_mock_plugins FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'hosted_mock_plugins' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON hosted_mock_plugins
            USING (EXISTS (SELECT 1 FROM hosted_mocks p1
                WHERE p1.id = hosted_mock_plugins.deployment_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM hosted_mocks p1
                WHERE p1.id = hosted_mock_plugins.deployment_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE incident_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE incident_events FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'incident_events' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON incident_events
            USING (EXISTS (SELECT 1 FROM incidents p1
                WHERE p1.id = incident_events.incident_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM incidents p1
                WHERE p1.id = incident_events.incident_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE mock_environments ENABLE ROW LEVEL SECURITY;
ALTER TABLE mock_environments FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'mock_environments' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON mock_environments
            USING (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = mock_environments.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = mock_environments.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE monitored_services ENABLE ROW LEVEL SECURITY;
ALTER TABLE monitored_services FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'monitored_services' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON monitored_services
            USING (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = monitored_services.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = monitored_services.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE runtime_captures ENABLE ROW LEVEL SECURITY;
ALTER TABLE runtime_captures FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'runtime_captures' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON runtime_captures
            USING (EXISTS (SELECT 1 FROM hosted_mocks p1
                WHERE p1.id = runtime_captures.deployment_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM hosted_mocks p1
                WHERE p1.id = runtime_captures.deployment_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE runtime_request_logs ENABLE ROW LEVEL SECURITY;
ALTER TABLE runtime_request_logs FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'runtime_request_logs' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON runtime_request_logs
            USING (EXISTS (SELECT 1 FROM hosted_mocks p1
                WHERE p1.id = runtime_request_logs.deployment_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM hosted_mocks p1
                WHERE p1.id = runtime_request_logs.deployment_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE runtime_traces ENABLE ROW LEVEL SECURITY;
ALTER TABLE runtime_traces FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'runtime_traces' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON runtime_traces
            USING (EXISTS (SELECT 1 FROM hosted_mocks p1
                WHERE p1.id = runtime_traces.deployment_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM hosted_mocks p1
                WHERE p1.id = runtime_traces.deployment_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE scenario_environment_versions ENABLE ROW LEVEL SECURITY;
ALTER TABLE scenario_environment_versions FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'scenario_environment_versions' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON scenario_environment_versions
            USING (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = scenario_environment_versions.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = scenario_environment_versions.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE scenario_promotions ENABLE ROW LEVEL SECURITY;
ALTER TABLE scenario_promotions FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'scenario_promotions' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON scenario_promotions
            USING (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = scenario_promotions.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = scenario_promotions.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE snapshot_schedules ENABLE ROW LEVEL SECURITY;
ALTER TABLE snapshot_schedules FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'snapshot_schedules' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON snapshot_schedules
            USING (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = snapshot_schedules.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = snapshot_schedules.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE snapshots ENABLE ROW LEVEL SECURITY;
ALTER TABLE snapshots FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'snapshots' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON snapshots
            USING (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = snapshots.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = snapshots.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE test_run_artifacts ENABLE ROW LEVEL SECURITY;
ALTER TABLE test_run_artifacts FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'test_run_artifacts' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON test_run_artifacts
            USING (EXISTS (SELECT 1 FROM test_runs p1
                WHERE p1.id = test_run_artifacts.run_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM test_runs p1
                WHERE p1.id = test_run_artifacts.run_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE test_run_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE test_run_events FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'test_run_events' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON test_run_events
            USING (EXISTS (SELECT 1 FROM test_runs p1
                WHERE p1.id = test_run_events.run_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM test_runs p1
                WHERE p1.id = test_run_events.run_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE test_schedules ENABLE ROW LEVEL SECURITY;
ALTER TABLE test_schedules FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'test_schedules' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON test_schedules
            USING (EXISTS (SELECT 1 FROM test_suites p1
                    JOIN workspaces p2 ON p2.id = p1.workspace_id
                WHERE p1.id = test_schedules.suite_id
                  AND p2.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM test_suites p1
                    JOIN workspaces p2 ON p2.id = p1.workspace_id
                WHERE p1.id = test_schedules.suite_id
                  AND p2.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE test_suites ENABLE ROW LEVEL SECURITY;
ALTER TABLE test_suites FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'test_suites' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON test_suites
            USING (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = test_suites.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = test_suites.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE tunnel_sessions ENABLE ROW LEVEL SECURITY;
ALTER TABLE tunnel_sessions FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'tunnel_sessions' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON tunnel_sessions
            USING (EXISTS (SELECT 1 FROM tunnel_reservations p1
                WHERE p1.id = tunnel_sessions.reservation_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM tunnel_reservations p1
                WHERE p1.id = tunnel_sessions.reservation_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE verification_suites ENABLE ROW LEVEL SECURITY;
ALTER TABLE verification_suites FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'verification_suites' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON verification_suites
            USING (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = verification_suites.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = verification_suites.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE virtual_entities ENABLE ROW LEVEL SECURITY;
ALTER TABLE virtual_entities FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'virtual_entities' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON virtual_entities
            USING (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = virtual_entities.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = virtual_entities.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE workspace_env_variables ENABLE ROW LEVEL SECURITY;
ALTER TABLE workspace_env_variables FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'workspace_env_variables' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON workspace_env_variables
            USING (EXISTS (SELECT 1 FROM workspace_environments p1
                    JOIN workspaces p2 ON p2.id = p1.workspace_id
                WHERE p1.id = workspace_env_variables.environment_id
                  AND p2.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM workspace_environments p1
                    JOIN workspaces p2 ON p2.id = p1.workspace_id
                WHERE p1.id = workspace_env_variables.environment_id
                  AND p2.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE workspace_environments ENABLE ROW LEVEL SECURITY;
ALTER TABLE workspace_environments FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'workspace_environments' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON workspace_environments
            USING (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = workspace_environments.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = workspace_environments.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE workspace_folders ENABLE ROW LEVEL SECURITY;
ALTER TABLE workspace_folders FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'workspace_folders' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON workspace_folders
            USING (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = workspace_folders.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = workspace_folders.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE workspace_request_history ENABLE ROW LEVEL SECURITY;
ALTER TABLE workspace_request_history FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'workspace_request_history' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON workspace_request_history
            USING (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = workspace_request_history.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = workspace_request_history.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE workspace_requests ENABLE ROW LEVEL SECURITY;
ALTER TABLE workspace_requests FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'workspace_requests' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON workspace_requests
            USING (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = workspace_requests.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = workspace_requests.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;

ALTER TABLE resilience_patterns ENABLE ROW LEVEL SECURITY;
ALTER TABLE resilience_patterns FORCE ROW LEVEL SECURITY;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies
                   WHERE schemaname = 'public' AND tablename = 'resilience_patterns' AND policyname = 'org_isolation') THEN
        CREATE POLICY org_isolation ON resilience_patterns
            USING (resilience_patterns.workspace_id IS NULL
                OR EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = resilience_patterns.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid))
            WITH CHECK (EXISTS (SELECT 1 FROM workspaces p1
                WHERE p1.id = resilience_patterns.workspace_id
                  AND p1.org_id = nullif(current_setting('app.current_org_id', true), '')::uuid));
    END IF;
END $$;
