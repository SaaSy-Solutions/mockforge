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

/// Bounded retention (400 days) stays compatible with the append-only trigger
/// and the hash chain:
///   * an expired row cannot be deleted without the purge opt-in;
///   * with the opt-in, a row inside the window still cannot be deleted;
///   * `purge_expired` removes the expired prefix of an org's chain, and
///     `verify_chain` accepts the oldest survivor as the chain start;
///   * tampering with a surviving row is still detected afterwards.
#[tokio::test]
#[ignore = "requires DATABASE_URL Postgres"]
async fn retention_purges_expired_prefix_and_chain_still_verifies() {
    use chrono::{Duration, Utc};
    use mockforge_registry_core::models::audit_log::AUDIT_LOG_RETENTION_DAYS;

    let pool = pool().await;
    let org_id = Uuid::new_v4();
    let now = Utc::now();
    let ages = [
        AUDIT_LOG_RETENTION_DAYS + 100,
        AUDIT_LOG_RETENTION_DAYS + 1,
        30,
        1,
    ];
    let mut ids = Vec::new();
    for (n, age) in ages.iter().enumerate() {
        let row = AuditLog::create_at(
            &mut pool.acquire().await.expect("acquire"),
            org_id,
            None,
            AuditEventType::LoginSucceeded,
            format!("retention event {n}"),
            Some(serde_json::json!({ "seq": n })),
            Some("203.0.113.7"),
            None,
            now - Duration::days(*age),
        )
        .await
        .expect("create backdated audit event");
        ids.push(row.id);
    }
    assert!(AuditLog::verify_chain(&pool, org_id).await.unwrap());

    // No opt-in: even an expired row is protected.
    let plain = sqlx::query("DELETE FROM audit_logs WHERE id = $1")
        .bind(ids[0])
        .execute(&pool)
        .await;
    assert!(plain.is_err(), "DELETE without the purge opt-in must be rejected");

    // Opt-in, but the row is inside the window: still rejected.
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('mockforge.audit_retention_purge', 'on', true)")
        .execute(&mut *tx)
        .await
        .unwrap();
    let young = sqlx::query("DELETE FROM audit_logs WHERE id = $1")
        .bind(ids[2])
        .execute(&mut *tx)
        .await;
    assert!(
        young.is_err(),
        "a row younger than the retention window must never be deletable"
    );
    tx.rollback().await.unwrap();

    let purged = AuditLog::purge_expired(&pool).await.expect("purge_expired");
    assert!(purged >= 2, "both expired rows must be purged (got {purged})");

    let left: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM audit_logs WHERE org_id = $1 ORDER BY created_at")
            .bind(org_id)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(left, ids[2..].to_vec(), "only rows inside the window survive");
    assert!(
        AuditLog::verify_chain(&pool, org_id).await.unwrap(),
        "oldest surviving row is accepted as the chain start"
    );

    // Appending after a purge extends the chain normally.
    insert_event(&pool, org_id, 99).await;
    assert!(AuditLog::verify_chain(&pool, org_id).await.unwrap());

    // Tamper detection still covers the surviving rows.
    sqlx::query("ALTER TABLE audit_logs DISABLE TRIGGER audit_logs_append_only")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE audit_logs SET description = 'tampered' WHERE id = $1")
        .bind(ids[3])
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE audit_logs ENABLE TRIGGER audit_logs_append_only")
        .execute(&pool)
        .await
        .unwrap();
    assert!(!AuditLog::verify_chain(&pool, org_id).await.unwrap());
}

/// Rows written from the person-related builders hold no email address or
/// username once stored, for every converted event type.
#[tokio::test]
#[ignore = "requires DATABASE_URL Postgres"]
async fn person_related_audit_rows_store_no_email_or_username() {
    use mockforge_registry_server::audit_entries as ae;

    let pool = pool().await;
    let org_id = Uuid::new_v4();
    let tag = Uuid::new_v4().simple().to_string();
    let email = format!("pii-{tag}@example.test");
    let username = format!("pii-{tag}");
    let user: mockforge_registry_core::models::User = serde_json::from_value(serde_json::json!({
        "id": Uuid::new_v4(),
        "username": username,
        "email": email,
        "password_hash": "x",
        "api_token": null,
        "is_verified": true,
        "is_admin": false,
        "two_factor_enabled": false,
        "two_factor_secret": null,
        "two_factor_backup_codes": null,
        "two_factor_verified_at": null,
        "created_at": chrono::Utc::now(),
        "updated_at": chrono::Utc::now(),
    }))
    .expect("build user");

    let events = [
        (AuditEventType::MemberAdded, ae::member_added(&user, "member")),
        (AuditEventType::MemberRemoved, ae::member_removed(&user)),
        (
            AuditEventType::MemberRoleChanged,
            ae::member_role_changed(&user, "member", "admin"),
        ),
        (AuditEventType::InvitationCreated, ae::invitation_created("n-1", "member")),
        (AuditEventType::InvitationRevoked, ae::invitation_revoked("n-1", "member")),
        (
            AuditEventType::InvitationAccepted,
            ae::invitation_accepted("n-1", &user, "member"),
        ),
        (AuditEventType::LoginFailed, ae::login_failed_unknown_user("secret", &email)),
        (AuditEventType::LoginFailed, ae::login_failed_bad_password("secret", &email)),
        (AuditEventType::LoginSucceeded, ae::saml_login_succeeded(&user, true)),
        (AuditEventType::LoginSucceeded, ae::oidc_login_succeeded(&user, false)),
        (AuditEventType::PasswordChanged, ae::password_reset_completed(&user)),
    ];
    for (event_type, entry) in events {
        AuditLog::create(
            &mut pool.acquire().await.expect("acquire"),
            org_id,
            Some(user.id),
            event_type,
            entry.description,
            entry.metadata,
            Some("203.0.113.7"),
            Some("pii-test/1.0"),
        )
        .await
        .expect("create");
    }

    let rows: Vec<(String, String, Option<serde_json::Value>)> = sqlx::query_as(
        "SELECT event_type::text, description, metadata FROM audit_logs WHERE org_id = $1",
    )
    .bind(org_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 11);
    for (event, description, metadata) in rows {
        let text = format!("{description}|{}", metadata.map(|m| m.to_string()).unwrap_or_default());
        assert!(!text.contains('@'), "{event} row holds an email: {text}");
        assert!(!text.contains(&username), "{event} row holds the username: {text}");
    }
    assert!(AuditLog::verify_chain(&pool, org_id).await.unwrap());
}
