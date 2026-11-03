//! Upgrade qualification uses a separate empty disposable database.
use std::borrow::Cow;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires empty isolated TEST_UPGRADE_DATABASE_URL"]
async fn migration_108_preserves_and_identifies_legacy_pending_reservations() {
    let url = std::env::var("TEST_UPGRADE_DATABASE_URL")
        .expect("empty disposable upgrade database required");
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let mut before = sqlx::migrate!("./migrations");
    before.migrations =
        Cow::Owned(before.iter().filter(|m| m.version <= 20250101000107).cloned().collect());
    before.run(&pool).await.unwrap();
    let user = Uuid::new_v4();
    let org = Uuid::new_v4();
    let reservation = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users(id,username,email,password_hash) VALUES($1,$2,$3,'unusable-fixture')",
    )
    .bind(user)
    .bind(format!("upgrade-{user}"))
    .bind(format!("{user}@example.invalid"))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO organizations(id,name,slug,owner_id) VALUES($1,'upgrade fixture',$2,$3)",
    )
    .bind(org)
    .bind(format!("upgrade-{org}"))
    .bind(user)
    .execute(&pool)
    .await
    .unwrap();
    let counter =
        mockforge_registry_server::models::UsageCounter::get_or_create_current(&pool, org)
            .await
            .unwrap();
    sqlx::query("INSERT INTO ai_token_reservations(id,org_id,usage_counter_id,budget,state) VALUES($1,$2,$3,777,'pending')").bind(reservation).bind(org).bind(counter.id).execute(&pool).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let legacy: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ai_token_reservations WHERE state='pending' AND NOT execution_fenced",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(legacy, 1, "rollout preflight must detect the unreleased legacy pending call");
    let record: (String, bool, i64) = sqlx::query_as(
        "SELECT state,execution_fenced,budget FROM ai_token_reservations WHERE id=$1",
    )
    .bind(reservation)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        record,
        ("pending".into(), false, 777),
        "migration must not settle or legitimize legacy provider usage"
    );
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let latest: i64 = sqlx::query_scalar("SELECT max(version) FROM _sqlx_migrations WHERE success")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(
        latest >= 20261031000001,
        "upgrade includes current hosted usage receipt migration"
    );
}
