//! Proof that the Postgres Row-Level-Security backstop (#832) isolates
//! tenants at the database layer, independent of any application-level
//! `WHERE org_id` filter.
//!
//! These tests are `#[ignore]` and require a real Postgres (RLS is a Postgres
//! feature; SQLite cannot exercise it). They connect as a NON-superuser,
//! NON-BYPASSRLS role — testing as the `postgres` superuser would bypass RLS
//! entirely and give a false pass.
//!
//! ## Running locally
//!
//! ```bash
//! docker run -d --name rls-pg -e POSTGRES_PASSWORD=postgres \
//!   -e POSTGRES_DB=mockforge -p 55432:5432 postgres:16
//!
//! DATABASE_URL=postgres://postgres:postgres@localhost:55432/mockforge \
//!   cargo test -p mockforge-registry-server --test tenant_isolation_rls -- --ignored --nocapture
//!
//! docker rm -f rls-pg
//! ```
//!
//! `DATABASE_URL` must point at a SUPERUSER connection: the test uses it to
//! run migrations and to create/seed as the privileged owner, then opens a
//! SEPARATE pool as the unprivileged `mockforge_rls_app` role to run the
//! actual isolation assertions.

use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// The non-superuser, non-BYPASSRLS role the assertions run as.
const APP_ROLE: &str = "mockforge_rls_app";
const APP_ROLE_PASSWORD: &str = "rls_app_pw";

fn admin_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok()
}

/// Build the app-role connection URL by swapping the userinfo in the admin URL
/// for the unprivileged role's credentials.
fn app_url(admin: &str) -> String {
    // admin looks like: postgres://postgres:postgres@host:port/db[?params]
    // Replace everything between "://" and the "@" with the app creds.
    let scheme_split: Vec<&str> = admin.splitn(2, "://").collect();
    let scheme = scheme_split[0];
    let after = scheme_split[1];
    let host_and_rest = after.split_once('@').map(|x| x.1).unwrap_or(after);
    format!("{scheme}://{APP_ROLE}:{APP_ROLE_PASSWORD}@{host_and_rest}")
}

/// Run migrations as the admin/superuser pool.
async fn migrate(pool: &PgPool) {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("migrations should apply against the docker postgres");
}

/// Create the unprivileged role and grant it CRUD on the seeded tables. Runs
/// as the admin pool. Idempotent across re-runs.
async fn ensure_app_role(admin: &PgPool) {
    // Create the role if it does not already exist. NOSUPERUSER + NOBYPASSRLS
    // is the whole point: without these the role would silently ignore RLS.
    let create = format!(
        "DO $$ BEGIN \
           IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = '{APP_ROLE}') THEN \
             CREATE ROLE {APP_ROLE} LOGIN PASSWORD '{APP_ROLE_PASSWORD}' NOSUPERUSER NOBYPASSRLS; \
           END IF; \
         END $$;"
    );
    sqlx::query(&create).execute(admin).await.expect("create app role");

    // Defensively re-assert the attributes in case a stale role survived from
    // a prior run with different attributes.
    sqlx::query(&format!("ALTER ROLE {APP_ROLE} NOSUPERUSER NOBYPASSRLS"))
        .execute(admin)
        .await
        .expect("assert app role attributes");

    for grant in [
        "GRANT USAGE ON SCHEMA public TO ",
        "GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO ",
        "GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO ",
    ] {
        sqlx::query(&format!("{grant}{APP_ROLE}"))
            .execute(admin)
            .await
            .expect("grant to app role");
    }
}

/// Insert an organization + a single project owned by it. Returns the org id.
/// Runs as the admin pool (RLS does not block the superuser seeding rows).
async fn seed_org_with_project(admin: &PgPool, name: &str, slug: &str) -> (Uuid, Uuid) {
    let owner_id: Uuid = sqlx::query(
        "INSERT INTO users (username, email, password_hash, is_verified) \
         VALUES ($1, $2, 'x', true) RETURNING id",
    )
    .bind(slug)
    .bind(format!("{slug}@example.test"))
    .fetch_one(admin)
    .await
    .expect("seed user")
    .get("id");

    let org_id: Uuid = sqlx::query(
        "INSERT INTO organizations (name, slug, owner_id) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(name)
    .bind(slug)
    .bind(owner_id)
    .fetch_one(admin)
    .await
    .expect("seed org")
    .get("id");

    let project_id: Uuid =
        sqlx::query("INSERT INTO projects (org_id, slug, name) VALUES ($1, $2, $3) RETURNING id")
            .bind(org_id)
            .bind(format!("{slug}-proj"))
            .bind(format!("{name} Project"))
            .fetch_one(admin)
            .await
            .expect("seed project")
            .get("id");

    (org_id, project_id)
}

/// Set the per-transaction org GUC. Mirrors what `with_org_context` does in
/// the production code path.
async fn set_org_guc(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, org_id: Uuid) {
    sqlx::query("SELECT set_config('app.current_org_id', $1, true)")
        .bind(org_id.to_string())
        .execute(&mut **tx)
        .await
        .expect("set app.current_org_id");
}

#[tokio::test]
#[ignore] // Requires a real Postgres via DATABASE_URL.
async fn rls_isolates_tenants_at_the_database() {
    let Some(admin) = admin_url() else {
        eprintln!("DATABASE_URL not set; skipping RLS proof test");
        return;
    };

    // 1. Admin/superuser pool: migrate, create the unprivileged role, seed.
    let admin_pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&admin)
        .await
        .expect("connect as admin");

    migrate(&admin_pool).await;
    ensure_app_role(&admin_pool).await;

    // Unique slugs so re-runs against a persistent DB don't collide.
    let suffix = Uuid::new_v4().simple().to_string();
    let (org_a, project_a) =
        seed_org_with_project(&admin_pool, "Org A", &format!("org-a-{suffix}")).await;
    let (org_b, project_b) =
        seed_org_with_project(&admin_pool, "Org B", &format!("org-b-{suffix}")).await;

    println!("seeded org_a={org_a} project_a={project_a}");
    println!("seeded org_b={org_b} project_b={project_b}");

    // 2. App-role pool: this is where RLS actually bites.
    let app_pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&app_url(&admin))
        .await
        .expect("connect as unprivileged app role");

    // Confirm we really are NOT bypassing RLS — guards against a false pass.
    let bypasses: bool =
        sqlx::query("SELECT rolbypassrls FROM pg_roles WHERE rolname = current_user")
            .fetch_one(&app_pool)
            .await
            .expect("read current_user rolbypassrls")
            .get("rolbypassrls");
    assert!(!bypasses, "app role must NOT have BYPASSRLS or the test is meaningless");
    let is_super: bool = sqlx::query("SELECT current_setting('is_superuser') = 'on' AS s")
        .fetch_one(&app_pool)
        .await
        .expect("read is_superuser")
        .get("s");
    assert!(!is_super, "app role must NOT be superuser or RLS is bypassed");

    // --- Assertion 1: with org A's GUC, a SELECT with NO `WHERE org_id`
    //     returns ONLY org A's project (simulates a handler that forgot the
    //     filter). ----------------------------------------------------------
    {
        let mut tx = app_pool.begin().await.expect("begin tx A");
        set_org_guc(&mut tx, org_a).await;
        let rows = sqlx::query("SELECT id, org_id FROM projects")
            .fetch_all(&mut *tx)
            .await
            .expect("select projects under org A context");
        tx.commit().await.expect("commit tx A");

        let ids: Vec<Uuid> = rows.iter().map(|r| r.get::<Uuid, _>("id")).collect();
        println!("[org A ctx, no WHERE] visible project ids: {ids:?}");
        assert_eq!(ids.len(), 1, "org A context must see exactly one project");
        assert_eq!(ids[0], project_a, "org A context must see only org A's project");
        assert!(!ids.contains(&project_b), "org A must NOT see org B's project");
    }

    // --- Assertion 2: with NO GUC set, the same query returns 0 rows
    //     (fail closed). ----------------------------------------------------
    {
        let mut tx = app_pool.begin().await.expect("begin tx none");
        // Deliberately do NOT set app.current_org_id.
        let rows = sqlx::query("SELECT id FROM projects")
            .fetch_all(&mut *tx)
            .await
            .expect("select projects with no org context");
        tx.commit().await.expect("commit tx none");
        println!("[no GUC] visible project count: {}", rows.len());
        assert_eq!(rows.len(), 0, "with no org GUC the backstop must fail closed (0 rows)");
    }

    // --- Assertion 3: a cross-tenant UPDATE (org A context trying to modify
    //     org B's project by id) affects 0 rows. ----------------------------
    {
        let mut tx = app_pool.begin().await.expect("begin tx update");
        set_org_guc(&mut tx, org_a).await;
        let res = sqlx::query("UPDATE projects SET name = 'pwned' WHERE id = $1")
            .bind(project_b)
            .execute(&mut *tx)
            .await
            .expect("cross-tenant update under org A context");
        tx.commit().await.expect("commit tx update");
        println!("[org A ctx] cross-tenant UPDATE rows_affected: {}", res.rows_affected());
        assert_eq!(res.rows_affected(), 0, "org A must not be able to UPDATE org B's project");
    }

    // --- Assertion 4: a cross-tenant DELETE affects 0 rows. ---------------
    {
        let mut tx = app_pool.begin().await.expect("begin tx delete");
        set_org_guc(&mut tx, org_a).await;
        let res = sqlx::query("DELETE FROM projects WHERE id = $1")
            .bind(project_b)
            .execute(&mut *tx)
            .await
            .expect("cross-tenant delete under org A context");
        tx.commit().await.expect("commit tx delete");
        println!("[org A ctx] cross-tenant DELETE rows_affected: {}", res.rows_affected());
        assert_eq!(res.rows_affected(), 0, "org A must not be able to DELETE org B's project");
    }

    // Confirm org B's project is still intact (untouched by A's attempts),
    // read back as the admin pool which is unconstrained.
    let still_there: i64 =
        sqlx::query("SELECT COUNT(*) AS c FROM projects WHERE id = $1 AND name <> 'pwned'")
            .bind(project_b)
            .fetch_one(&admin_pool)
            .await
            .expect("read back org B project")
            .get("c");
    assert_eq!(still_there, 1, "org B's project must remain intact and unmodified");

    println!("RLS tenant-isolation proof: ALL ASSERTIONS PASSED");
}
