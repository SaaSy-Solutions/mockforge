//! Existing admin databases must upgrade without changing migration checksums.
use sqlx::{migrate::Migrator, sqlite::SqlitePoolOptions, SqlitePool};
use std::path::Path;

async fn database() -> SqlitePool {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap()
}

#[tokio::test]
async fn fresh_database_has_no_builtin_accounts() {
    let pool = database().await;
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admin_users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn historical_database_upgrades_and_preserves_operator_credentials() {
    let pool = database().await;
    let historical = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/admin_auth_v1");
    Migrator::new(historical).await.unwrap().run(&pool).await.unwrap();
    sqlx::query(
        "UPDATE admin_users SET password_hash = 'operator-changed-hash' WHERE username = 'admin'",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO admin_users (id, username, password_hash, role) VALUES ('operator-id', 'operator', 'operator-password-hash', 'admin')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES ('old-viewer-session', '00000000-0000-0000-0000-000000000002', 'old-session-hash', '2099-01-01 00:00:00')")
        .execute(&pool).await.unwrap();

    let current = sqlx::migrate!("./migrations");
    current
        .run(&pool)
        .await
        .expect("historical migration checksum must remain valid");
    current.run(&pool).await.expect("replaying migrations must be safe");

    let users: Vec<(String, String)> =
        sqlx::query_as("SELECT username, password_hash FROM admin_users ORDER BY username")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        users,
        vec![
            ("admin".to_owned(), "operator-changed-hash".to_owned()),
            ("operator".to_owned(), "operator-password-hash".to_owned()),
        ]
    );
    let sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM refresh_tokens")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(sessions, 0, "sessions belonging to unchanged built-in users must be revoked");
}
