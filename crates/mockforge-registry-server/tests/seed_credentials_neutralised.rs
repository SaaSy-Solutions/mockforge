//! #1014: the publicly known seed accounts (`admin@mockforge.dev` / `admin123`,
//! `test@example.com` / `test123`) must not be usable on any migrated database.
//!
//! The first two tests are static and always run. The PostgreSQL tests are
//! `#[ignore]`d and need an isolated database:
//!
//! ```text
//! DATABASE_URL=postgres://... JWT_SECRET=test-only \
//!   cargo test -p mockforge-registry-server --test seed_credentials_neutralised \
//!   -- --ignored --test-threads=1
//! ```
use mockforge_registry_server::{
    auth::verify_password, circuit_breaker::CircuitBreakerRegistry, config::Config,
    database::Database, storage::PluginStorage, store::PgRegistryStore, AppState,
};
use regex::Regex;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

const NEUTRALISE_SQL: &str =
    include_str!("../migrations/20250101000102_neutralise_seed_credentials.sql");
const SEED_SQL: &[&str] = &[
    include_str!("../migrations/20250101000002_seed_data.sql"),
    include_str!("../migrations/20250101000027_fix_seed_password_hashes.sql"),
    include_str!("../migrations/20250101000028_fix_seed_password_hashes_v2.sql"),
];
const SEED_ADMIN_ID: &str = "00000000-0000-0000-0000-000000000001";
const SEED_TEST_ID: &str = "00000000-0000-0000-0000-000000000002";
/// The working admin123 / test123 hashes written by ...0028.
const ADMIN123_HASH: &str = "$2b$10$5mcGb.1oYTuLLi9h08MucO1rsfWmJsryFoKtHF6.qv9p5Ge6V9IW6";
const TEST123_HASH: &str = "$2b$10$Uv30udbt8TccWoXbl4r3tufv569B/XxQ2d/SsR.9Fdzjw1/ZroEAi";

fn bcrypt_literals(sql: &str) -> Vec<String> {
    // Not `{53}`: the ...0002 test-user hash is malformed (52 characters).
    Regex::new(r"'(\$2[abxy]\$\d\d\$[./A-Za-z0-9]+)'")
        .unwrap()
        .captures_iter(sql)
        .map(|c| c[1].to_string())
        .collect()
}

/// The hashes the migration matches on (every literal except the replacement)
fn seed_hashes() -> Vec<String> {
    let replacement = replacement_hash();
    bcrypt_literals(NEUTRALISE_SQL)
        .into_iter()
        .filter(|h| *h != replacement)
        .collect()
}

fn replacement_hash() -> String {
    let re = Regex::new(r"SET password_hash = '([^']+)'").unwrap();
    re.captures(NEUTRALISE_SQL).expect("migration sets a replacement hash")[1].to_string()
}

#[test]
fn every_hash_the_seed_migrations_wrote_is_matched() {
    let neutralised = seed_hashes();
    assert_eq!(neutralised.len(), 6);
    let mut seeded = Vec::new();
    for sql in SEED_SQL {
        seeded.extend(bcrypt_literals(sql));
    }
    assert_eq!(seeded.len(), 6, "seed migrations wrote six hashes: {seeded:?}");
    for hash in &seeded {
        assert!(neutralised.contains(hash), "seed hash {hash} is not neutralised");
    }
    // The list really does catch the working published credentials.
    assert!(verify_password("admin123", ADMIN123_HASH).unwrap());
    assert!(verify_password("test123", TEST123_HASH).unwrap());
}

#[test]
fn replacement_hash_is_well_formed_and_matches_no_seed_password() {
    let hash = replacement_hash();
    for password in ["admin123", "test123", "", "admin", "password"] {
        // Ok(false), not Err: login must answer "invalid credentials", not 500.
        assert!(!verify_password(password, &hash).unwrap(), "{password} verified");
    }
    assert!(!seed_hashes().contains(&hash));
}

async fn state() -> AppState {
    let url = std::env::var("DATABASE_URL").expect("isolated DATABASE_URL required");
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

#[derive(sqlx::FromRow, Debug, PartialEq)]
struct Row {
    password_hash: String,
    is_admin: Option<bool>,
    api_token: Option<String>,
    github_id: Option<String>,
}

async fn row(state: &AppState, id: &str) -> Option<Row> {
    sqlx::query_as::<_, Row>(
        "SELECT password_hash, is_admin, api_token, github_id FROM users WHERE id = $1::uuid",
    )
    .bind(id)
    .fetch_optional(state.db.pool())
    .await
    .unwrap()
}

/// Put a seed row back into the state the old migrations left it in.
async fn reseed(state: &AppState, id: &str, hash: &str, tag: &str) {
    sqlx::query(
        "UPDATE users SET password_hash = $2, is_admin = TRUE, api_token = $3, github_id = $3 \
         WHERE id = $1::uuid",
    )
    .bind(id)
    .bind(hash)
    .bind(format!("seed-fixture-{tag}"))
    .execute(state.db.pool())
    .await
    .unwrap();
}

async fn neutralise(state: &AppState) {
    sqlx::raw_sql(NEUTRALISE_SQL).execute(state.db.pool()).await.unwrap();
}

async fn active_refresh_token(state: &AppState, id: &str) -> String {
    let jti = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO token_revocations (jti, user_id, expires_at) \
         VALUES ($1, $2::uuid, NOW() + INTERVAL '1 day')",
    )
    .bind(&jti)
    .bind(id)
    .execute(state.db.pool())
    .await
    .unwrap();
    jti
}

async fn is_revoked(state: &AppState, jti: &str) -> bool {
    sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(
        "SELECT revoked_at FROM token_revocations WHERE jti = $1",
    )
    .bind(jti)
    .fetch_one(state.db.pool())
    .await
    .unwrap()
    .is_some()
}

fn assert_neutralised(row: &Row, password: &str) {
    assert!(
        !verify_password(password, &row.password_hash).unwrap(),
        "{password} still works"
    );
    assert_eq!(row.is_admin, Some(false));
    assert_eq!(row.api_token, None);
    assert_eq!(row.github_id, None);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL"]
async fn migrated_database_has_no_known_credential_login() {
    let state = state().await;

    // Whatever the migration history, no row may still carry a seed hash...
    let known = seed_hashes();
    let leftover: i64 =
        sqlx::query_scalar("SELECT count(*) FROM users WHERE password_hash = ANY($1)")
            .bind(&known)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
    assert_eq!(leftover, 0);
    // ...and no admin may answer to the published password.
    let admins: Vec<String> =
        sqlx::query_scalar("SELECT password_hash FROM users WHERE is_admin = TRUE")
            .fetch_all(state.db.pool())
            .await
            .unwrap();
    for hash in admins {
        assert!(!verify_password("admin123", &hash).unwrap_or(false));
    }
    // A seed row may legitimately stay admin if its owner set their own
    // password; otherwise it must be the disabled replacement.
    if let Some(admin) = row(&state, SEED_ADMIN_ID).await {
        if admin.password_hash == replacement_hash() {
            assert_neutralised(&admin, "admin123");
        }
    }
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL"]
async fn seeded_rows_are_disabled_and_owner_set_passwords_are_untouched() {
    let state = state().await;
    assert!(row(&state, SEED_ADMIN_ID).await.is_some(), "...0002 inserts the seed admin");

    // A database that applied ...0028 and nothing since.
    reseed(&state, SEED_ADMIN_ID, ADMIN123_HASH, "admin").await;
    reseed(&state, SEED_TEST_ID, TEST123_HASH, "test").await;
    let jti = active_refresh_token(&state, SEED_ADMIN_ID).await;
    neutralise(&state).await;
    assert_neutralised(&row(&state, SEED_ADMIN_ID).await.unwrap(), "admin123");
    assert_neutralised(&row(&state, SEED_TEST_ID).await.unwrap(), "test123");
    assert!(is_revoked(&state, &jti).await);

    // Idempotent: a second run changes nothing.
    let before = row(&state, SEED_ADMIN_ID).await;
    neutralise(&state).await;
    assert_eq!(row(&state, SEED_ADMIN_ID).await, before);

    // The owner kept the seed email but set their own password: hands off.
    let owner_hash = bcrypt::hash("owner-chosen-fixture-password", 4).unwrap();
    reseed(&state, SEED_ADMIN_ID, &owner_hash, "owner").await;
    let owner_jti = active_refresh_token(&state, SEED_ADMIN_ID).await;
    let owned = row(&state, SEED_ADMIN_ID).await;
    neutralise(&state).await;
    assert_eq!(row(&state, SEED_ADMIN_ID).await, owned);
    assert!(!is_revoked(&state, &owner_jti).await);

    // Leave the shared database disabled again.
    reseed(&state, SEED_ADMIN_ID, ADMIN123_HASH, "admin").await;
    neutralise(&state).await;
    assert_neutralised(&row(&state, SEED_ADMIN_ID).await.unwrap(), "admin123");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL and local HTTP listener"]
async fn login_rejects_the_published_seed_credentials() {
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
    let client = reqwest::Client::new();
    for (email, password) in [
        ("admin@mockforge.dev", "admin123"),
        ("test@example.com", "test123"),
    ] {
        let response = client
            .post(format!("{url}/api/v1/auth/login"))
            .json(&json!({ "email": email, "password": password }))
            .send()
            .await
            .unwrap();
        // A 4xx "invalid credentials", not a 5xx from an unparseable hash.
        assert!(response.status().is_client_error(), "{email}: {}", response.status());
        let body = response.text().await.unwrap();
        assert!(!body.contains("access_token") && !body.contains("user_id"), "{email}: {body}");
    }
    server.abort();
    let _ = server.await;
}
