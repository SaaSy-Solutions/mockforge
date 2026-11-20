//! Site-admin authorization for showcase moderation endpoints.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use mockforge_registry_core::models::showcase::CreateShowcaseEntry;
use mockforge_registry_server::{
    circuit_breaker::CircuitBreakerRegistry,
    config::Config,
    database::Database,
    handlers::showcase::{
        admin_create_showcase_entry, admin_delete_showcase_entry, admin_list_showcase_entries,
        admin_update_showcase_entry, AdminCreateShowcaseRequest, AdminUpdateShowcaseRequest,
    },
    middleware::AuthUser,
    models::ShowcaseEntry,
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

async fn user(state: &AppState, admin: bool) -> Uuid {
    let tag = Uuid::new_v4().simple().to_string();
    let user = PgRegistryStore::new(state.db.pool().clone())
        .create_user(&format!("showcase-{tag}"), &format!("{tag}@example.invalid"), "disabled")
        .await
        .unwrap();
    if admin {
        sqlx::query("UPDATE users SET is_admin = TRUE WHERE id = $1")
            .bind(user.id)
            .execute(state.db.pool())
            .await
            .unwrap();
    }
    user.id
}

async fn entry(state: &AppState) -> Uuid {
    ShowcaseEntry::create(
        state.db.pool(),
        CreateShowcaseEntry {
            slug: &format!("entry-{}", Uuid::new_v4().simple()),
            org_id: None,
            submitted_by: None,
            title: "Fixture",
            description: "Fixture",
            body: None,
            screenshots: &[],
            demo_url: None,
            source_url: None,
            tags: &[],
        },
    )
    .await
    .unwrap()
    .id
}

#[tokio::test]
#[ignore = "requires isolated Postgres"]
async fn showcase_admin_endpoints_require_site_admin() {
    let state = state().await;
    let ordinary = user(&state, false).await;
    let admin = user(&state, true).await;
    let existing = entry(&state).await;

    let denied = admin_list_showcase_entries(State(state.clone()), AuthUser(ordinary)).await;
    assert_eq!(
        axum::response::IntoResponse::into_response(denied.unwrap_err()).status(),
        StatusCode::FORBIDDEN
    );

    let create = AdminCreateShowcaseRequest {
        slug: format!("submission-{}", Uuid::new_v4().simple()),
        title: "Submission".into(),
        description: "Description".into(),
        body: None,
        screenshots: vec![],
        demo_url: None,
        source_url: None,
        tags: vec![],
    };
    let denied =
        admin_create_showcase_entry(State(state.clone()), AuthUser(ordinary), Json(create)).await;
    assert_eq!(
        axum::response::IntoResponse::into_response(denied.unwrap_err()).status(),
        StatusCode::FORBIDDEN
    );

    let denied = admin_update_showcase_entry(
        State(state.clone()),
        AuthUser(ordinary),
        Path(existing),
        Json(AdminUpdateShowcaseRequest {
            is_published: Some(true),
            is_featured: None,
        }),
    )
    .await;
    assert_eq!(
        axum::response::IntoResponse::into_response(denied.unwrap_err()).status(),
        StatusCode::FORBIDDEN
    );

    let denied =
        admin_delete_showcase_entry(State(state.clone()), AuthUser(ordinary), Path(existing)).await;
    assert_eq!(
        axum::response::IntoResponse::into_response(denied.unwrap_err()).status(),
        StatusCode::FORBIDDEN
    );

    assert!(admin_list_showcase_entries(State(state.clone()), AuthUser(admin)).await.is_ok());
    let create = AdminCreateShowcaseRequest {
        slug: format!("admin-{}", Uuid::new_v4().simple()),
        title: "Admin entry".into(),
        description: "Description".into(),
        body: None,
        screenshots: vec![],
        demo_url: None,
        source_url: None,
        tags: vec![],
    };
    assert!(admin_create_showcase_entry(State(state.clone()), AuthUser(admin), Json(create))
        .await
        .is_ok());
    assert!(admin_update_showcase_entry(
        State(state.clone()),
        AuthUser(admin),
        Path(existing),
        Json(AdminUpdateShowcaseRequest {
            is_published: Some(true),
            is_featured: None
        }),
    )
    .await
    .is_ok());
    assert!(admin_delete_showcase_entry(State(state), AuthUser(admin), Path(existing))
        .await
        .is_ok());
}
