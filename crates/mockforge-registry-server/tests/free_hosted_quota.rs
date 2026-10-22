//! Real Postgres and HTTP boundary tests for the Free hosted request cap.
//! Run explicitly against an isolated DB with TEST_DATABASE_URL, DATABASE_URL,
//! and JWT_SECRET configured; the test accounts have unusable login hashes.

use std::sync::Arc;

use axum::{routing::get, Router};
use mockforge_registry_server::{
    circuit_breaker::CircuitBreakerRegistry,
    config::Config,
    database::Database,
    deployment::router::{custom_domain_fallback, MultitenantRouter},
    models::{Organization, Plan, UsageCounter},
    storage::PluginStorage,
    store::{with_org_context, PgRegistryStore, RegistryStore, CURRENT_ORG},
    AppState,
};
use reqwest::StatusCode;
use serde_json::{json, Value};
use uuid::Uuid;

#[path = "free_hosted_quota/ai_attribution.rs"]
mod attribution;

#[path = "free_hosted_quota/verified_identity.rs"]
mod verified_identity;

#[path = "free_hosted_quota/credit_reconciliation.rs"]
mod credit_reconciliation;

#[path = "free_hosted_quota/account_allowance.rs"]
mod account_allowance;

#[path = "free_hosted_quota/request_allowance.rs"]
mod request_allowance;

#[path = "free_hosted_quota/owner_capacity.rs"]
mod owner_capacity;

struct Server {
    url: String,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn serve(router: Router) -> Server {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    Server { url, task }
}

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

async fn organization(state: &AppState, plan: Plan) -> Organization {
    let store = PgRegistryStore::new(state.db.pool().clone());
    let tag = Uuid::new_v4().simple().to_string();
    let owner = store
        .create_user(
            &format!("quota-{tag}"),
            &format!("{tag}@example.invalid"),
            "disabled-test-account",
        )
        .await
        .unwrap();
    sqlx::query("UPDATE users SET is_verified=TRUE WHERE id=$1")
        .bind(owner.id)
        .execute(state.db.pool())
        .await
        .unwrap();
    store.create_organization("Quota fixture", &tag, owner.id, plan).await.unwrap()
}

async fn deployment(state: &AppState, org: &Organization, target: &str) -> String {
    let slug = format!("quota-{}", Uuid::new_v4().simple());
    sqlx::query(
        "INSERT INTO hosted_mocks (org_id, name, slug, config_json, status, internal_url) VALUES ($1, 'Quota fixture', $2, '{}'::jsonb, 'active', $3)",
    )
    .bind(org.id)
    .bind(&slug)
    .bind(target)
    .execute(state.db.pool())
    .await
    .unwrap();
    slug
}

async fn set_limit(state: &AppState, org: &Organization, limit: i64) {
    sqlx::query("UPDATE organizations SET limits_json = jsonb_set(limits_json, '{requests_per_30d}', to_jsonb($2::bigint)) WHERE id = $1")
        .bind(org.id)
        .bind(limit)
        .execute(state.db.pool())
        .await
        .unwrap();
}

async fn requests(state: &AppState, org_id: Uuid) -> i64 {
    with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(UsageCounter::get_or_create_current(&mut **tx, org_id).await?.requests)
        })
    })
    .await
    .unwrap()
}

#[path = "free_hosted_quota/platform_ai.rs"]
mod platform_ai;

#[path = "free_hosted_quota/hosted_openapi.rs"]
mod hosted_openapi;

#[path = "free_hosted_quota/request_cap.rs"]
mod request_cap;

#[path = "free_hosted_quota/slot_lifecycle.rs"]
mod slot_lifecycle;

#[path = "free_hosted_quota/tenant_boundaries.rs"]
mod tenant_boundaries;
