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
/// is `None` because a user is unnecessary to exercise the hash chain / trigger /
/// tamper detection (its FK was likewise dropped in migration 085, #1087).
async fn insert_event(pool: &PgPool, org_id: Uuid, n: usize) -> AuditLog {
    AuditLog::create(
        &mut pool.acquire().await.expect("acquire"),
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

/// PR 2 (#866 / #873): the new `ai_usage` + `payment_failed` enum values must
/// be present in the Postgres `audit_event_type` enum (added by migration
/// 20250101000081). If migration ...081 were missing or the values mismatched
/// the Rust `as_str()` literals, `AuditLog::create` would fail the enum bind
/// here. This also confirms the rows extend the tamper-evident chain normally.
#[tokio::test]
#[ignore = "requires DATABASE_URL Postgres"]
async fn new_event_types_insert_and_extend_chain() {
    let pool = pool().await;
    let org_id = Uuid::new_v4();

    for event_type in [AuditEventType::AiUsage, AuditEventType::PaymentFailed] {
        AuditLog::create(
            &mut pool.acquire().await.expect("acquire"),
            org_id,
            None,
            event_type,
            format!("pr2 event {}", event_type.as_str()),
            Some(serde_json::json!({ "event": event_type.as_str() })),
            None,
            None,
        )
        .await
        .unwrap_or_else(|e| panic!("create {} failed: {e}", event_type.as_str()));
    }

    assert!(
        AuditLog::verify_chain(&pool, org_id).await.expect("verify_chain"),
        "chain with ai_usage + payment_failed rows must verify"
    );
}

/// #1087: GDPR erasure (`delete_user_data_cascade`) must succeed for a user who
/// has audit rows. The old `audit_logs_user_id_fkey ... ON DELETE SET NULL`
/// turned the `DELETE FROM users` into an UPDATE of every audit row, which the
/// append-only trigger rejects ("audit_logs is append-only (#872): UPDATE is not
/// permitted"), so erasure failed for any user who had ever done anything.
///
/// Mirrors the #872 treatment of `org_id`: the rows outlive the user, keep
/// their original `user_id` (a bare UUID once the `users` row is gone), and the
/// per-org hash chain still verifies because nothing in the row changed.
#[tokio::test]
#[ignore = "requires DATABASE_URL Postgres"]
async fn gdpr_erase_succeeds_for_user_with_audit_rows_and_chain_survives() {
    use mockforge_registry_core::store::{PgRegistryStore, RegistryStore};

    let pool = pool().await;
    let org_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let tag = user_id.simple().to_string();
    let username = format!("gdpr-{tag}");
    let email = format!("gdpr-{tag}@example.test");

    sqlx::query("INSERT INTO users (id, username, email, password_hash) VALUES ($1, $2, $3, 'x')")
        .bind(user_id)
        .bind(&username)
        .bind(&email)
        .execute(&pool)
        .await
        .expect("seed user");

    for n in 0..3 {
        AuditLog::create(
            &mut pool.acquire().await.expect("acquire"),
            org_id,
            Some(user_id),
            AuditEventType::LoginSucceeded,
            format!("user event {n}"),
            Some(serde_json::json!({ "seq": n })),
            Some("203.0.113.9"),
            Some("gdpr-test/1.0"),
        )
        .await
        .expect("create audit event for user");
    }
    assert!(AuditLog::verify_chain(&pool, org_id).await.expect("verify_chain"));

    let store = PgRegistryStore::new(pool.clone());
    store
        .delete_user_data_cascade(user_id)
        .await
        .expect("GDPR erase must succeed for a user with audit rows");

    let user_left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(user_left, 0, "users row must be deleted");

    let audit_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_logs WHERE org_id = $1 AND user_id = $2")
            .bind(org_id)
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(audit_rows, 3, "audit rows must survive erasure with their original user_id");

    assert!(
        AuditLog::verify_chain(&pool, org_id).await.expect("verify_chain"),
        "hash chain must still verify after the user is erased"
    );

    // The handler's own post-erasure audit row must not reintroduce the
    // erased user's email (or username): it identifies them by user_id only.
    mockforge_registry_server::handlers::gdpr::record_erasure_audit(
        &store,
        user_id,
        Some("smoke".to_string()),
        0,
    )
    .await;

    let (description, metadata): (String, Option<serde_json::Value>) = sqlx::query_as(
        "SELECT description, metadata FROM audit_logs \
         WHERE user_id = $1 AND metadata->>'action' = 'gdpr_data_erasure'",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("erasure audit row must be recorded");

    let metadata = metadata.expect("erasure row has metadata").to_string();
    for (field, text) in [("description", &description), ("metadata", &metadata)] {
        assert!(!text.contains(&email), "erasure {field} must not contain the email: {text}");
        assert!(!text.contains('@'), "erasure {field} must not contain an email: {text}");
        assert!(
            !text.contains(&username),
            "erasure {field} must not contain the username: {text}"
        );
    }
    assert!(description.contains(&user_id.to_string()));
    assert!(metadata.contains(&user_id.to_string()));
}
