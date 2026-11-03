//! Integration test for the pre-login SSO discovery store lookup
//! (`find_sso_config_by_email_domain`), which backs `GET /api/v1/sso/discover`.
//!
//! Exercises the three contractual outcomes against a real Postgres store:
//!   1. configured + enabled + domain-matched -> Some((config, org_slug))
//!   2. unknown domain                        -> None (UI falls back to manual)
//!   3. disabled config                       -> None (invisible to discovery)
//!
//! Postgres-gated and `#[ignore]` (mirrors the other `*_e2e` tests): run with
//!   TEST_DATABASE_URL=postgres://postgres:password@localhost:5433/mockforge_registry \
//!   cargo test -p mockforge-registry-server --test sso_discover -- --ignored --nocapture

use mockforge_registry_server::database::Database;
use mockforge_registry_server::models::sso::SSOProvider;
use mockforge_registry_server::models::Plan;
use mockforge_registry_server::store::{PgRegistryStore, RegistryStore};

/// Build a migrated Postgres-backed store, or skip (return None) when no
/// `TEST_DATABASE_URL` is configured so the suite is a no-op in plain CI.
async fn store_or_skip() -> Option<PgRegistryStore> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let db = Database::connect(&url).await.expect("connect to test db");
    db.migrate().await.expect("run migrations");
    Some(PgRegistryStore::new(db.pool().clone()))
}

#[tokio::test]
#[ignore = "requires Postgres (TEST_DATABASE_URL)"]
async fn discover_resolves_enabled_domain_and_hides_disabled_and_unknown() {
    let Some(store) = store_or_skip().await else {
        eprintln!("TEST_DATABASE_URL not set; skipping");
        return;
    };

    // Unique slug/email-domain per run so repeated local runs don't collide on
    // the partial-unique email_domain index.
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let email_domain = format!("acme-{suffix}.example.com");
    let slug = format!("acme-{suffix}");
    let owner_email = format!("owner@{email_domain}");

    let owner = store
        .create_user(&format!("owner-{suffix}"), &owner_email, "x")
        .await
        .expect("create owner");
    let org = store
        .create_organization("Acme", &slug, owner.id, Plan::Team)
        .await
        .expect("create org");

    // Upsert an OIDC config carrying the routing email_domain, then enable it.
    store
        .upsert_sso_config(
            org.id,
            SSOProvider::Oidc,
            None,
            None,
            None,
            None,
            None,
            None,
            true,
            true,
            false,
            Some("https://idp.example.com"),
            Some("client-1"),
            Some("secret-1"),
            Some(&email_domain),
        )
        .await
        .expect("upsert sso config");
    store.enable_sso_config(org.id).await.expect("enable sso");

    // 1. Enabled + matched -> Some, case-insensitively, returning the org slug.
    let hit = store
        .find_sso_config_by_email_domain(&email_domain.to_uppercase())
        .await
        .expect("lookup ok");
    let (config, org_slug) = hit.expect("should resolve an enabled, matched config");
    assert_eq!(org_slug, slug);
    assert_eq!(config.provider(), SSOProvider::Oidc);

    // 2. Unknown domain -> None.
    let miss = store
        .find_sso_config_by_email_domain("not-configured.example.org")
        .await
        .expect("lookup ok");
    assert!(miss.is_none(), "unknown domain must not resolve");

    // 3. Disabled config -> None (invisible to discovery).
    store.disable_sso_config(org.id).await.expect("disable sso");
    let disabled = store.find_sso_config_by_email_domain(&email_domain).await.expect("lookup ok");
    assert!(disabled.is_none(), "disabled config must not resolve");
}
