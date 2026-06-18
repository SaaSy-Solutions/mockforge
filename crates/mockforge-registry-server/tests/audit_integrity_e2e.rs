//! Integrity tests for the SOC2 audit-log hardening (#872).
//!
//! These exercise the real Postgres schema + Rust model directly (no HTTP
//! server needed): the per-org hash chain in `AuditLog::create`, the
//! append-only UPDATE/DELETE trigger, and `verify_chain`'s tamper detection.
//!
//! `#[ignore]`-gated like the other `*_e2e.rs` suites — they need a live
//! Postgres on `DATABASE_URL`. Run with:
//!
//!   DATABASE_URL=postgres://postgres:postgres@localhost:55433/mockforge \
//!   cargo test -p mockforge-registry-server --test audit_integrity_e2e -- --ignored --nocapture

use mockforge_registry_core::models::audit_log::AuditLog;
use mockforge_registry_core::models::AuditEventType;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

async fn pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .expect("DB connect failed");
    // Run the registry migrations so the audit_logs table + integrity migration
    // (20250101000080) are present. Idempotent across repeated test runs.
    sqlx::migrate!("./migrations").run(&pool).await.expect("migrations failed");
    pool
}

/// Insert a row via the real `AuditLog::create` chain logic. `org_id` is now an
/// FK-less plain column (the CASCADE FK was dropped in migration 080), so a fresh
/// random org per test gives isolation without seeding `organizations`. `user_id`
/// is `None`: `audit_logs.user_id` still references `users(id)`, and seeding a user
/// is unnecessary to exercise the hash chain / trigger / tamper detection.
async fn insert_event(pool: &PgPool, org_id: Uuid, n: usize) -> AuditLog {
    AuditLog::create(
        pool,
        org_id,
        None,
        AuditEventType::LoginSucceeded,
        format!("event {n}"),
        Some(serde_json::json!({ "seq": n })),
        Some("203.0.113.7"),
        Some("integrity-test/1.0"),
    )
    .await
    .expect("create audit event")
}

#[tokio::test]
#[ignore = "requires DATABASE_URL Postgres"]
async fn chain_verifies_and_is_tamper_evident_and_append_only() {
    let pool = pool().await;
    let org_id = Uuid::new_v4();

    // 1. Insert 3 events and assert the chain verifies.
    let r1 = insert_event(&pool, org_id, 1).await;
    let _r2 = insert_event(&pool, org_id, 2).await;
    let _r3 = insert_event(&pool, org_id, 3).await;

    assert!(
        AuditLog::verify_chain(&pool, org_id).await.expect("verify_chain"),
        "freshly written 3-event chain must verify"
    );

    // First row of the org chain has a NULL prev_hash; all rows have an entry_hash.
    let first_prev: Option<String> =
        sqlx::query_scalar("SELECT prev_hash FROM audit_logs WHERE id = $1")
            .bind(r1.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(first_prev.is_none(), "first chain row must have NULL prev_hash");

    // 2. A direct UPDATE must be rejected by the append-only trigger.
    let update_err = sqlx::query("UPDATE audit_logs SET description = 'hacked' WHERE id = $1")
        .bind(r1.id)
        .execute(&pool)
        .await;
    assert!(update_err.is_err(), "UPDATE on audit_logs must be blocked by the trigger");

    // 3. A direct DELETE must be rejected by the append-only trigger.
    let delete_err = sqlx::query("DELETE FROM audit_logs WHERE id = $1")
        .bind(r1.id)
        .execute(&pool)
        .await;
    assert!(delete_err.is_err(), "DELETE on audit_logs must be blocked by the trigger");

    // The blocked mutations left the chain intact.
    assert!(
        AuditLog::verify_chain(&pool, org_id).await.expect("verify_chain"),
        "chain must still verify after blocked UPDATE/DELETE"
    );

    // 4. Simulate an attacker with DB-owner rights bypassing the trigger to
    //    tamper a row, then assert verify_chain DETECTS the break. We disable
    //    the trigger, mutate, re-enable — the stored entry_hash no longer
    //    matches the recomputed hash of the new description.
    sqlx::query("ALTER TABLE audit_logs DISABLE TRIGGER audit_logs_append_only")
        .execute(&pool)
        .await
        .expect("disable trigger");
    sqlx::query("UPDATE audit_logs SET description = 'tampered' WHERE id = $1")
        .bind(r1.id)
        .execute(&pool)
        .await
        .expect("tamper update");
    sqlx::query("ALTER TABLE audit_logs ENABLE TRIGGER audit_logs_append_only")
        .execute(&pool)
        .await
        .expect("re-enable trigger");

    assert!(
        !AuditLog::verify_chain(&pool, org_id).await.expect("verify_chain"),
        "verify_chain must detect the tampered description"
    );
}
