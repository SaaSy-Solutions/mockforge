-- Defense-in-depth tenant-isolation backstop via Postgres Row-Level Security (#832).
--
-- Tenant isolation today is enforced ONLY in application code: the
-- `resolve_org_context` middleware plus a per-handler `WHERE org_id = $1`
-- filter. A single handler that forgets that filter becomes a cross-tenant
-- data exposure (IDOR) — this is the structural pattern behind #746/#778.
--
-- This migration adds a database-level backstop. With RLS enabled and FORCED,
-- every row visible to the application role is constrained to the org bound to
-- the connection via the `app.current_org_id` GUC, EVEN IF a query omits its
-- `WHERE org_id` clause. The app-layer filters stay in place; this is a second
-- line of defense, not a replacement.
--
-- ── HOW THE POLICY FAILS CLOSED ────────────────────────────────────────────
--   current_setting('app.current_org_id', true)::uuid
-- The second `true` arg ("missing_ok") makes a never-set GUC return NULL
-- instead of raising. A NULL on either side of `org_id = <null>` yields
-- UNKNOWN, which RLS treats as "row not visible". So a connection that never
-- set the GUC sees ZERO org-scoped rows — fail closed, not fail open.
--
-- ── ROLE REQUIREMENT (CRITICAL) ────────────────────────────────────────────
-- RLS is BYPASSED by:
--   * the table owner, UNLESS `FORCE ROW LEVEL SECURITY` is set (it is, below)
--   * any superuser role
--   * any role with the BYPASSRLS attribute
-- Therefore the application's runtime DB role MUST be a NON-superuser role
-- WITHOUT BYPASSRLS for this backstop to bite. If the app connects as the
-- Postgres superuser (common in dev/`docker run postgres`), RLS is silently
-- ignored and you get a FALSE sense of safety. Provision a dedicated
-- `mockforge_app` role:
--
--   CREATE ROLE mockforge_app LOGIN PASSWORD '...' NOSUPERUSER NOBYPASSRLS;
--   GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public
--     TO mockforge_app;
--   GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO mockforge_app;
--   ALTER DEFAULT PRIVILEGES IN SCHEMA public
--     GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO mockforge_app;
--
-- ── ELEVATED / CROSS-ORG PATHS ─────────────────────────────────────────────
-- Paths that legitimately need cross-org access (the migration runner itself,
-- platform admin tooling, background workers that sweep all orgs, webhook
-- ingestion that resolves the org from the payload) must either:
--   * run as an elevated role (e.g. the migration/owner role), or
--   * set `app.current_org_id` appropriately per unit of work.
-- The migration runner runs as the owner/superuser and is unaffected by FORCE
-- on these app tables (owner is still subject to FORCE, but the runner uses
-- the owner role with the GUC unset — it touches schema, not org-scoped rows,
-- during normal migrations). Keep that in mind when adding data-backfill
-- migrations that touch these tables: set the GUC or run as a BYPASSRLS role.

-- ── COVERED TABLES ─────────────────────────────────────────────────────────
-- Strictly org-scoped tables with a NOT NULL `org_id` column:
--   projects, audit_logs, hosted_mocks
-- Public/shared tables with a NULLABLE `org_id` (public marketplace rows):
--   templates, scenarios  → policy also admits `org_id IS NULL` so public
--   rows stay readable to everyone while writes remain org-scoped.
--
-- DELIBERATELY NOT COVERED in this slice (see PR rollout plan):
--   flows, virtual_entities  → scoped by `workspace_id`, no `org_id` column;
--                              need a workspace→org join policy (follow-up).
--   runtime_captures         → scoped by `deployment_id` → hosted_mocks.org_id;
--                              needs a subquery/join policy (follow-up).

-- ===========================================================================
-- Strictly org-scoped tables: org_id = current GUC, fail closed when unset.
-- ===========================================================================

ALTER TABLE projects ENABLE ROW LEVEL SECURITY;
ALTER TABLE projects FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON projects
    USING (org_id = current_setting('app.current_org_id', true)::uuid)
    WITH CHECK (org_id = current_setting('app.current_org_id', true)::uuid);

ALTER TABLE audit_logs ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_logs FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON audit_logs
    USING (org_id = current_setting('app.current_org_id', true)::uuid)
    WITH CHECK (org_id = current_setting('app.current_org_id', true)::uuid);

ALTER TABLE hosted_mocks ENABLE ROW LEVEL SECURITY;
ALTER TABLE hosted_mocks FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON hosted_mocks
    USING (org_id = current_setting('app.current_org_id', true)::uuid)
    WITH CHECK (org_id = current_setting('app.current_org_id', true)::uuid);

-- ===========================================================================
-- Public/shared tables: org-owned rows are isolated, public (NULL org_id)
-- rows are readable by everyone. Writes are still constrained to the current
-- org — a tenant cannot mint a row owned by another org, and a connection
-- with no GUC set can only touch public rows it is explicitly allowed to.
--
-- READ  (USING):      own-org rows OR public rows.
-- WRITE (WITH CHECK): only own-org rows. Inserting/updating a row to
--                     org_id = NULL (publishing to the public marketplace)
--                     is intentionally NOT allowed through the org-scoped
--                     connection; that path runs through an elevated role /
--                     a dedicated publish flow. This keeps a tenant from
--                     "escaping" isolation by nulling out org_id.
-- ===========================================================================

ALTER TABLE templates ENABLE ROW LEVEL SECURITY;
ALTER TABLE templates FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON templates
    USING (
        org_id = current_setting('app.current_org_id', true)::uuid
        OR org_id IS NULL
    )
    WITH CHECK (org_id = current_setting('app.current_org_id', true)::uuid);

ALTER TABLE scenarios ENABLE ROW LEVEL SECURITY;
ALTER TABLE scenarios FORCE ROW LEVEL SECURITY;
CREATE POLICY org_isolation ON scenarios
    USING (
        org_id = current_setting('app.current_org_id', true)::uuid
        OR org_id IS NULL
    )
    WITH CHECK (org_id = current_setting('app.current_org_id', true)::uuid);

-- ===========================================================================
-- ROLLBACK (reversible) — uncomment and run as the table owner to undo.
-- sqlx has no down-migration mechanism, so this is documented here for
-- operators applying a manual revert:
--
--   DROP POLICY IF EXISTS org_isolation ON projects;
--   ALTER TABLE projects NO FORCE ROW LEVEL SECURITY;
--   ALTER TABLE projects DISABLE ROW LEVEL SECURITY;
--
--   DROP POLICY IF EXISTS org_isolation ON audit_logs;
--   ALTER TABLE audit_logs NO FORCE ROW LEVEL SECURITY;
--   ALTER TABLE audit_logs DISABLE ROW LEVEL SECURITY;
--
--   DROP POLICY IF EXISTS org_isolation ON hosted_mocks;
--   ALTER TABLE hosted_mocks NO FORCE ROW LEVEL SECURITY;
--   ALTER TABLE hosted_mocks DISABLE ROW LEVEL SECURITY;
--
--   DROP POLICY IF EXISTS org_isolation ON templates;
--   ALTER TABLE templates NO FORCE ROW LEVEL SECURITY;
--   ALTER TABLE templates DISABLE ROW LEVEL SECURITY;
--
--   DROP POLICY IF EXISTS org_isolation ON scenarios;
--   ALTER TABLE scenarios NO FORCE ROW LEVEL SECURITY;
--   ALTER TABLE scenarios DISABLE ROW LEVEL SECURITY;
-- ===========================================================================
