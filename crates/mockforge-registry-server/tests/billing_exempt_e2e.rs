//! Real Postgres tests for billing-exempt (internal) organizations.
//!
//! Run explicitly against an isolated DB with TEST_DATABASE_URL, DATABASE_URL
//! and JWT_SECRET configured; the test accounts have unusable login hashes.
//!
//!   TEST_DATABASE_URL=postgres://... DATABASE_URL=postgres://... JWT_SECRET=... \
//!   cargo test -p mockforge-registry-server --test billing_exempt_e2e -- --ignored

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Extension, Json,
};
use mockforge_registry_server::{
    circuit_breaker::CircuitBreakerRegistry,
    config::Config,
    database::Database,
    error::ApiError,
    handlers::{
        admin::{set_org_billing_exempt, SetBillingExemptRequest},
        entitlements::effective_plan,
    },
    middleware::org_rate_limit::check_org_limits,
    models::{Organization, Plan, User},
    storage::PluginStorage,
    store::{PgRegistryStore, RegistryStore},
    AppState,
};
use reqwest::StatusCode;
use uuid::Uuid;

async fn state() -> AppState {
    let url = std::env::var("TEST_DATABASE_URL").expect("isolated TEST_DATABASE_URL required");
    let db = Database::connect(&url).await.unwrap();
    db.migrate().await.unwrap();
    let config = Config::load().unwrap();
    let storage = PluginStorage::new(&config).await.unwrap();
    let store = Arc::new(
        PgRegistryStore::new(db.runtime_pool().clone()).with_owner_pool(db.pool().clone()),
    );
    AppState {
        db,
        config,
        storage,
        store,
        metrics: Arc::new(mockforge_observability::prometheus::MetricsRegistry::new()),
        analytics_db: None,
        redis: None,
        circuit_breakers: CircuitBreakerRegistry::new(),
        platform_signing: None,
    }
}

async fn user(state: &AppState) -> User {
    let tag = Uuid::new_v4().simple().to_string();
    PgRegistryStore::new(state.db.pool().clone())
        .create_user(&format!("exempt-{tag}"), &format!("{tag}@example.invalid"), "disabled")
        .await
        .unwrap()
}

async fn free_org(state: &AppState, owner: &User) -> Organization {
    let slug = format!("exempt-{}", Uuid::new_v4().simple());
    PgRegistryStore::new(state.db.pool().clone())
        .create_organization("Exempt fixture", &slug, owner.id, Plan::Free)
        .await
        .unwrap()
}

async fn set_exempt(
    state: &AppState,
    actor: &User,
    org: &Organization,
    billing_exempt: bool,
) -> Result<bool, ApiError> {
    set_org_billing_exempt(
        State(state.clone()),
        Path(org.id),
        Extension(actor.id.to_string()),
        Json(SetBillingExemptRequest { billing_exempt }),
    )
    .await
    .map(|Json(response)| response.billing_exempt)
}

async fn reload(state: &AppState, org: &Organization) -> Organization {
    Organization::find_by_id(state.db.pool(), org.id).await.unwrap().unwrap()
}

#[tokio::test]
#[ignore = "requires isolated Postgres"]
async fn non_admin_cannot_set_billing_exempt() {
    let state = state().await;
    let owner = user(&state).await;
    let org = free_org(&state, &owner).await;

    // The org's own owner is not a site admin.
    let denied = set_exempt(&state, &owner, &org, true).await.unwrap_err();
    assert_eq!(denied.into_response().status(), StatusCode::FORBIDDEN);
    assert!(!reload(&state, &org).await.billing_exempt);

    let admin = user(&state).await;
    sqlx::query("UPDATE users SET is_admin = TRUE WHERE id = $1")
        .bind(admin.id)
        .execute(state.db.pool())
        .await
        .unwrap();
    assert!(set_exempt(&state, &admin, &org, true).await.unwrap());
    assert!(reload(&state, &org).await.billing_exempt);
    assert!(!set_exempt(&state, &admin, &org, false).await.unwrap());
    assert!(!reload(&state, &org).await.billing_exempt);
}

#[tokio::test]
#[ignore = "requires isolated Postgres"]
async fn exempt_org_passes_request_limit_free_org_fails() {
    let state = state().await;
    let mut orgs = Vec::new();
    for exempt in [false, true] {
        let owner = user(&state).await;
        let org = free_org(&state, &owner).await;
        sqlx::query("UPDATE organizations SET billing_exempt = $2 WHERE id = $1")
            .bind(org.id)
            .bind(exempt)
            .execute(state.db.pool())
            .await
            .unwrap();
        // Twice the Free plan's 10,000 requests this month.
        mockforge_registry_server::models::UsageCounter::get_or_create_current(
            state.db.pool(),
            org.id,
        )
        .await
        .unwrap();
        sqlx::query("UPDATE usage_counters SET requests = 20000 WHERE org_id = $1")
            .bind(org.id)
            .execute(state.db.pool())
            .await
            .unwrap();
        orgs.push((reload(&state, &org).await, owner));
    }
    let (free, free_owner) = &orgs[0];
    let (exempt, exempt_owner) = &orgs[1];

    let denied = check_org_limits(state.db.pool(), None, free, free_owner.id).await;
    assert!(matches!(denied, Err(ApiError::UsageLimitExceeded { .. })));
    check_org_limits(state.db.pool(), None, exempt, exempt_owner.id)
        .await
        .expect("exempt org is never over its request limit");

    // No subscription row, stored plan Free: still resolves to Team.
    assert_eq!(effective_plan(&state, free).await.unwrap(), Plan::Free);
    assert_eq!(effective_plan(&state, exempt).await.unwrap(), Plan::Team);
}

mod bootstrap {
    use super::*;
    use mockforge_registry_server::billing_exempt_bootstrap::{
        apply, parse_list, BootstrapConfig, BootstrapReport,
    };

    async fn verified_user(state: &AppState) -> User {
        let owner = super::user(state).await;
        sqlx::query("UPDATE users SET is_verified = TRUE WHERE id = $1")
            .bind(owner.id)
            .execute(state.db.pool())
            .await
            .unwrap();
        User::find_by_id(state.db.pool(), owner.id).await.unwrap().unwrap()
    }

    #[tokio::test]
    #[ignore = "requires isolated Postgres"]
    async fn bootstrap_refuses_unverified_email_and_claimed_slug() {
        let state = state().await;
        let owner = super::user(&state).await;
        let org = free_org(&state, &owner).await;
        let config = BootstrapConfig {
            site_admin_emails: vec![owner.email.clone()],
            billing_exempt_slugs: vec![org.slug.clone()],
        };
        let report = apply(state.db.pool(), &config).await.unwrap();
        assert!(report.changed_nothing());
        assert_eq!(flags(&state, &owner, &org).await, (false, false));
        assert_eq!(report.site_admins_not_found, vec![owner.email.clone()]);
        assert_eq!(report.orgs_not_found, vec![org.slug.clone()]);
    }

    #[tokio::test]
    #[ignore = "requires isolated Postgres"]
    async fn bootstrap_refuses_slug_owned_by_verified_non_admin() {
        let state = state().await;
        let owner = verified_user(&state).await;
        let org = free_org(&state, &owner).await;
        let config = BootstrapConfig {
            site_admin_emails: Vec::new(),
            billing_exempt_slugs: vec![org.slug.clone()],
        };
        let report = apply(state.db.pool(), &config).await.unwrap();
        assert!(report.changed_nothing());
        assert_eq!(flags(&state, &owner, &org).await, (false, false));
        assert_eq!(report.orgs_not_found, vec![org.slug.clone()]);
    }

    async fn flags(state: &AppState, owner: &User, org: &Organization) -> (bool, bool) {
        let admin: bool = sqlx::query_scalar("SELECT is_admin FROM users WHERE id = $1")
            .bind(owner.id)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
        (admin, reload(state, org).await.billing_exempt)
    }

    #[tokio::test]
    #[ignore = "requires isolated Postgres"]
    async fn bootstrap_sets_listed_user_and_org() {
        let state = state().await;
        let owner = verified_user(&state).await;
        let org = free_org(&state, &owner).await;
        let config = BootstrapConfig {
            // Emails match case-insensitively.
            site_admin_emails: vec![owner.email.to_uppercase()],
            billing_exempt_slugs: vec![org.slug.clone()],
        };
        let report = apply(state.db.pool(), &config).await.unwrap();
        assert_eq!(report.site_admins_granted, vec![owner.email.to_uppercase()]);
        assert_eq!(report.orgs_exempted, vec![org.slug.clone()]);
        assert_eq!(flags(&state, &owner, &org).await, (true, true));
    }

    #[tokio::test]
    #[ignore = "requires isolated Postgres"]
    async fn bootstrap_second_run_changes_nothing() {
        let state = state().await;
        let owner = verified_user(&state).await;
        let org = free_org(&state, &owner).await;
        let config = BootstrapConfig {
            site_admin_emails: vec![owner.email.clone()],
            billing_exempt_slugs: vec![org.slug.clone()],
        };
        assert!(!apply(state.db.pool(), &config).await.unwrap().changed_nothing());
        assert_eq!(apply(state.db.pool(), &config).await.unwrap(), BootstrapReport::default());
        assert_eq!(flags(&state, &owner, &org).await, (true, true));
    }

    #[tokio::test]
    #[ignore = "requires isolated Postgres"]
    async fn bootstrap_empty_and_missing_values_change_nothing() {
        let state = state().await;
        let owner = verified_user(&state).await;
        let org = free_org(&state, &owner).await;
        for value in [None, Some(""), Some(" , ")] {
            let config = BootstrapConfig {
                site_admin_emails: parse_list(value),
                billing_exempt_slugs: parse_list(value),
            };
            assert_eq!(apply(state.db.pool(), &config).await.unwrap(), BootstrapReport::default());
        }
        assert_eq!(flags(&state, &owner, &org).await, (false, false));
    }

    #[tokio::test]
    #[ignore = "requires isolated Postgres"]
    async fn bootstrap_unknown_email_and_slug_are_skipped() {
        let state = state().await;
        let owner = verified_user(&state).await;
        let org = free_org(&state, &owner).await;
        let config = BootstrapConfig {
            site_admin_emails: vec!["nobody@example.invalid".into(), owner.email.clone()],
            billing_exempt_slugs: vec!["no-such-org-bootstrap".into(), org.slug.clone()],
        };
        let report = apply(state.db.pool(), &config).await.unwrap();
        assert_eq!(report.site_admins_not_found, vec!["nobody@example.invalid"]);
        assert_eq!(report.orgs_not_found, vec!["no-such-org-bootstrap"]);
        // Known entries in the same run are still applied.
        assert_eq!(report.site_admins_granted, vec![owner.email.clone()]);
        assert_eq!(report.orgs_exempted, vec![org.slug.clone()]);
    }
}
