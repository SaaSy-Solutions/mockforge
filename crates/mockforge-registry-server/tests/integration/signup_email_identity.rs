//! Isolated actual PostgreSQL and HTTP identity boundaries. Synthetic accounts only.
use mockforge_registry_server::{
    circuit_breaker::CircuitBreakerRegistry,
    config::Config,
    database::Database,
    models::{login_attempt::LoginAttempt, User},
    storage::PluginStorage,
    store::PgRegistryStore,
    AppState,
};
use serde_json::{json, Value};
use std::sync::Arc;
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

#[tokio::test]
#[ignore = "requires isolated PostgreSQL and local HTTP listener"]
async fn signup_case_concurrency_and_login_share_one_identity() {
    let state = state().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let app = mockforge_registry_server::routes::create_router(state.clone())
        .with_state(state.clone())
        .layer(axum::Extension(
            mockforge_registry_server::middleware::rate_limit::RateLimiterState::new(
                state.config.rate_limit_per_minute,
            ),
        ));
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap();
    let tag = Uuid::new_v4().simple().to_string();
    let email = format!("identity-{tag}@example.invalid");
    // Only synthetic, unusable fixture credentials; authentication data stays in memory.
    let payload = json!({"username":format!("identity_{tag}"),"email":email,"password":"fixture-only-unusable-password"});
    let invalid=client.post(format!("{url}/api/v1/auth/register")).json(&json!({"username":format!("invalid_{tag}"),"email":"bad-address","password":"fixture-only-unusable-password"})).send().await.unwrap();
    assert!(invalid.status().is_client_error());
    let mut alternate = payload.clone();
    alternate["username"] = json!(format!("alternate_{tag}"));
    alternate["email"] = json!(format!("  {}  ", email.to_uppercase()));
    let (a, b) = tokio::join!(
        client.post(format!("{url}/api/v1/auth/register")).json(&payload).send(),
        client.post(format!("{url}/api/v1/auth/register")).json(&alternate).send()
    );
    let a = a.unwrap();
    let b = b.unwrap();
    assert_ne!(
        a.status().is_success(),
        b.status().is_success(),
        "registration status pair: {} {}",
        a.status(),
        b.status()
    );
    let denied = if a.status().is_success() { b } else { a };
    assert!(denied.status().is_client_error());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users WHERE lower(btrim(email))=$1")
        .bind(&email)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    assert_eq!(count, 1);
    let user = User::find_by_email(state.db.pool(), &format!(" {} ", email.to_uppercase()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(user.email, email);
    assert!(!user.is_verified);
    let login = client
        .post(format!("{url}/api/v1/auth/login"))
        .json(&json!({"email":email.to_uppercase(),"password":"fixture-only-unusable-password"}))
        .send()
        .await
        .unwrap();
    assert!(login.status().is_success());
    let body: Value = login.json().await.unwrap();
    assert_eq!(body["user_id"], user.id.to_string());
    let wrong = client
        .post(format!("{url}/api/v1/auth/login"))
        .json(&json!({"email":email,"password":"fixture-wrong-password"}))
        .send()
        .await
        .unwrap();
    assert!(wrong.status().is_client_error());
    server.abort();
    let _ = server.await;
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL"]
async fn database_guards_every_email_writer_and_lockout_case_variants() {
    let state = state().await;
    let tag = Uuid::new_v4().simple().to_string();
    let email = format!("owner-{tag}@example.invalid");
    let user = User::create(
        state.db.pool(),
        &format!("owner_{tag}"),
        &email.to_uppercase(),
        "disabled-fixture-account",
    )
    .await
    .unwrap();
    assert_eq!(user.email, email);
    let duplicate = User::create(
        state.db.pool(),
        &format!("other_{tag}"),
        &format!(" {email} "),
        "disabled-fixture-account",
    )
    .await;
    assert!(duplicate.is_err());
    sqlx::query("UPDATE users SET is_verified=TRUE WHERE id=$1")
        .bind(user.id)
        .execute(state.db.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE users SET email=$2 WHERE id=$1")
        .bind(user.id)
        .bind(format!(" {} ", email.to_uppercase()))
        .execute(state.db.pool())
        .await
        .unwrap();
    assert!(User::find_by_id(state.db.pool(), user.id).await.unwrap().unwrap().is_verified);
    sqlx::query("UPDATE users SET email=$2 WHERE id=$1")
        .bind(user.id)
        .bind(format!("new-{email}"))
        .execute(state.db.pool())
        .await
        .unwrap();
    assert!(!User::find_by_id(state.db.pool(), user.id).await.unwrap().unwrap().is_verified);
    LoginAttempt::record(state.db.pool(), &email.to_uppercase(), None, false)
        .await
        .unwrap();
    LoginAttempt::record(state.db.pool(), &format!(" {email} "), None, false)
        .await
        .unwrap();
    assert_eq!(
        LoginAttempt::count_recent_failures(state.db.pool(), &email, 5).await.unwrap(),
        2
    );
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL"]
async fn legacy_collisions_fail_closed_without_merging_accounts() {
    let state = state().await;
    let schema = format!("fixture_{}", Uuid::new_v4().simple());
    let mut tx = state.db.pool().begin().await.unwrap();
    sqlx::raw_sql(&format!("CREATE SCHEMA {schema}; SET LOCAL search_path TO {schema}; CREATE TABLE users(id uuid,email text,is_verified boolean); CREATE TABLE login_attempts(email text,created_at timestamptz); INSERT INTO users VALUES(gen_random_uuid(),'Owner@example.invalid',true),(gen_random_uuid(),' owner@EXAMPLE.invalid ',false);")).execute(&mut *tx).await.unwrap();
    // A savepoint preserves the two distinct legacy accounts after rejection.
    sqlx::raw_sql("SAVEPOINT before_identity_migration")
        .execute(&mut *tx)
        .await
        .unwrap();
    let rejected =
        sqlx::raw_sql(include_str!("../../migrations/20250101000101_canonical_email_identity.sql"))
            .execute(&mut *tx)
            .await;
    assert!(rejected.is_err());
    sqlx::raw_sql("ROLLBACK TO SAVEPOINT before_identity_migration")
        .execute(&mut *tx)
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(count, 2);
    tx.rollback().await.unwrap();
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL"]
async fn oauth_linking_requires_local_ownership_and_preserves_existing_provider_ids() {
    let state = state().await;
    let tag = Uuid::new_v4().simple().to_string();
    let user = User::create(
        state.db.pool(),
        &format!("link_{tag}"),
        &format!("link-{tag}@example.invalid"),
        "disabled-fixture-account",
    )
    .await
    .unwrap();
    let github = format!("fixture-github-{tag}");
    let google = format!("fixture-google-{tag}");
    assert!(state.store.link_user_github_account(user.id, &github, None).await.is_err());
    assert!(state.store.link_user_google_account(user.id, &google, None).await.is_err());
    sqlx::query("UPDATE users SET is_verified=TRUE WHERE id=$1")
        .bind(user.id)
        .execute(state.db.pool())
        .await
        .unwrap();
    state.store.link_user_github_account(user.id, &github, None).await.unwrap();
    state.store.link_user_google_account(user.id, &google, None).await.unwrap();
    state.store.link_user_github_account(user.id, &github, None).await.unwrap();
    state.store.link_user_google_account(user.id, &google, None).await.unwrap();
    assert!(state
        .store
        .link_user_github_account(user.id, "different-fixture-github", None)
        .await
        .is_err());
    assert!(state
        .store
        .link_user_google_account(user.id, "different-fixture-google", None)
        .await
        .is_err());
    let ids: (String, String) = sqlx::query_as("SELECT github_id,google_id FROM users WHERE id=$1")
        .bind(user.id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    assert_eq!(ids, (github, google));
}
