use super::{organization, serve, state};
use axum::{
    extract::{Path, State},
    routing::post,
    Json, Router,
};
use mockforge_registry_server::{
    ai::{quota::check_ai_quota, reservation::call_with_budget, LlmCall, ProviderSelection},
    handlers::ai_credit_reconciliation::{reconcile, ReconcileRequest},
    middleware::AuthUser,
    models::{Organization, Plan},
    store::with_org_context,
};
use serde_json::json;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use uuid::Uuid;

fn call(url: &str) -> LlmCall {
    LlmCall {
        provider: "openai".into(),
        model: "fixture".into(),
        api_key: "synthetic-fixture-key".into(),
        base_url: Some(url.into()),
        system: "system".into(),
        user: "user".into(),
        temperature: 0.0,
        max_tokens: 100,
    }
}
async fn cap(state: &mockforge_registry_server::AppState, org: Organization) -> Organization {
    sqlx::query("UPDATE organizations SET limits_json=jsonb_set(limits_json,'{ai_tokens_per_month}','1000') WHERE id=$1").bind(org.id).execute(state.db.pool()).await.unwrap();
    Organization::find_by_id(state.db.pool(), org.id).await.unwrap().unwrap()
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL and owned provider listener"]
async fn organizations_share_owner_credit_admission_and_deletion_cannot_reset_spend() {
    let state = state().await;
    let first = cap(&state, organization(&state, Plan::Free).await).await;
    let tag = Uuid::new_v4().simple().to_string();
    let second = state
        .store
        .create_organization("Shared owner", &tag, first.owner_id, Plan::Free)
        .await
        .unwrap();
    let second = cap(&state, second).await;
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    let released = gate.clone();
    let provider=serve(Router::new().route("/v1/chat/completions",post(move ||{let counted=counted.clone();let released=released.clone();async move {counted.fetch_add(1,Ordering::SeqCst);let permit=released.acquire().await.unwrap();permit.forget();Json(json!({"choices":[{"message":{"content":"complete"}}],"usage":{"prompt_tokens":500,"completion_tokens":500}}))}}))).await;
    let cloned = state.clone();
    let admitted = first.clone();
    let request = call(&provider.url);
    let running = tokio::spawn(async move {
        call_with_budget(&cloned, &admitted, ProviderSelection::Platform, request).await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while calls.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        call_with_budget(&state, &second, ProviderSelection::Platform, call(&provider.url))
            .await
            .is_err()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    gate.add_permits(1);
    running.await.unwrap().unwrap();
    let credits: (i64, i64) = sqlx::query_as(
        "SELECT used_tokens,reserved_tokens FROM free_ai_credit_periods WHERE owner_user_id=$1",
    )
    .bind(first.owner_id)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    assert_eq!(credits, (1000, 0));
    assert!(
        !check_ai_quota(&state, &second, ProviderSelection::Platform)
            .await
            .unwrap()
            .allowed
    );
    sqlx::query("DELETE FROM organizations WHERE id=$1")
        .bind(first.id)
        .execute(state.db.pool())
        .await
        .unwrap();
    let tag = Uuid::new_v4().simple().to_string();
    let recreated = state
        .store
        .create_organization("Fresh org", &tag, first.owner_id, Plan::Free)
        .await
        .unwrap();
    let recreated = cap(&state, recreated).await;
    assert!(
        call_with_budget(&state, &recreated, ProviderSelection::Platform, call(&provider.url))
            .await
            .is_err()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let foreign = organization(&state, Plan::Free).await;
    let owner = first.owner_id;
    let visible: i64 = with_org_context(state.db.runtime_pool(), foreign.id, |tx| {
        Box::pin(async move {
            Ok(sqlx::query_scalar(
                "SELECT COUNT(*) FROM free_ai_credit_periods WHERE owner_user_id=$1",
            )
            .bind(owner)
            .fetch_one(&mut **tx)
            .await?)
        })
    })
    .await
    .unwrap();
    assert_eq!(visible, 0);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL and owned provider listener"]
async fn uncertain_owner_credits_release_only_after_audited_operator_reconciliation() {
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    let provider = serve(Router::new().route(
        "/v1/chat/completions",
        post(|| async { Json(json!({"choices":[{"message":{"content":"unknown usage"}}]})) }),
    ))
    .await;
    assert!(call_with_budget(&state, &org, ProviderSelection::Platform, call(&provider.url))
        .await
        .is_err());
    let id:Uuid=sqlx::query_scalar("SELECT id FROM ai_token_reservations WHERE org_id=$1 AND state='uncertain' AND free_credit_period_id IS NOT NULL").bind(org.id).fetch_one(state.db.pool()).await.unwrap();
    let before: (i64, i64) = sqlx::query_as(
        "SELECT used_tokens,reserved_tokens FROM free_ai_credit_periods WHERE owner_user_id=$1",
    )
    .bind(org.owner_id)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    assert_eq!(before.0, 0);
    assert!(before.1 > 0);
    sqlx::query("UPDATE users SET is_admin=TRUE WHERE id=$1")
        .bind(org.owner_id)
        .execute(state.db.pool())
        .await
        .unwrap();
    for expected in [false, true] {
        let result = reconcile(
            State(state.clone()),
            AuthUser(org.owner_id),
            Path((org.id, id)),
            Json(ReconcileRequest {
                actual_tokens: 7,
                evidence_reference: "fixture-shared-provider-proof".into(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(result.0.already_reconciled, expected);
    }
    let after: (i64, i64) = sqlx::query_as(
        "SELECT used_tokens,reserved_tokens FROM free_ai_credit_periods WHERE owner_user_id=$1",
    )
    .bind(org.owner_id)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    assert_eq!(after, (7, 0));
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL"]
async fn shared_credit_migration_preserves_prior_free_usage_and_uncertain_budget() {
    let state = state().await;
    let schema = format!("fixture_shared_{}", Uuid::new_v4().simple());
    let mut tx = state.db.pool().begin().await.unwrap();
    sqlx::raw_sql(&format!("CREATE SCHEMA {schema}; SET LOCAL search_path TO {schema}; CREATE TABLE users(id uuid PRIMARY KEY); CREATE TABLE organizations(id uuid PRIMARY KEY,owner_id uuid,plan text,limits_json jsonb); CREATE TABLE org_settings(org_id uuid,setting_key text,setting_value jsonb); CREATE TABLE usage_counters(id uuid PRIMARY KEY,org_id uuid,period_start date,ai_tokens_used bigint); CREATE TABLE ai_token_reservations(id uuid PRIMARY KEY,org_id uuid,usage_counter_id uuid,budget bigint,state text); ")).execute(&mut *tx).await.unwrap();
    let owner = Uuid::new_v4();
    sqlx::query("INSERT INTO users VALUES($1)")
        .bind(owner)
        .execute(&mut *tx)
        .await
        .unwrap();
    for (used, budget, plan) in [
        (17_i64, 777_i64, "free"),
        (23, 888, "free"),
        (99, 999, "pro"),
    ] {
        let org = Uuid::new_v4();
        let counter = Uuid::new_v4();
        sqlx::query("INSERT INTO organizations VALUES($1,$2,$3,jsonb_build_object('ai_tokens_per_month',5000))")
            .bind(org)
            .bind(owner)
            .bind(plan)
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("INSERT INTO usage_counters VALUES($1,$2,CURRENT_DATE, $3)")
            .bind(counter)
            .bind(org)
            .bind(used)
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("INSERT INTO ai_token_reservations VALUES($1,$2,$3,$4,'uncertain')")
            .bind(Uuid::new_v4())
            .bind(org)
            .bind(counter)
            .bind(budget)
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    sqlx::raw_sql(include_str!("../../migrations/20250101000103_free_account_ai_allowance.sql"))
        .execute(&mut *tx)
        .await
        .unwrap();
    let row: (i64, i64) = sqlx::query_as(
        "SELECT used_tokens,reserved_tokens FROM free_ai_credit_periods WHERE owner_user_id=$1",
    )
    .bind(owner)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(row, (40, 1665));
    let attributed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_token_reservations WHERE free_credit_period_id IS NOT NULL",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(attributed, 2);
    tx.rollback().await.unwrap();
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL and owned provider listener"]
async fn account_deletion_blocks_held_calls_and_preserves_settled_financial_history() {
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    let successor = organization(&state, Plan::Free).await;
    sqlx::query("INSERT INTO org_members(org_id,user_id,role) VALUES($1,$2,'member')")
        .bind(org.id)
        .bind(successor.owner_id)
        .execute(state.db.pool())
        .await
        .unwrap();
    let provider = serve(Router::new().route(
        "/v1/chat/completions",
        post(|| async { Json(json!({"choices":[{"message":{"content":"unknown usage"}}]})) }),
    ))
    .await;
    assert!(call_with_budget(&state, &org, ProviderSelection::Platform, call(&provider.url))
        .await
        .is_err());
    assert!(state.store.delete_user_data_cascade(org.owner_id).await.is_err());
    let owner: Uuid = sqlx::query_scalar("SELECT owner_id FROM organizations WHERE id=$1")
        .bind(org.id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    assert_eq!(owner, org.owner_id, "failed deletion must roll ownership transfer back");
    let id: Uuid = sqlx::query_scalar("SELECT id FROM ai_token_reservations WHERE org_id=$1")
        .bind(org.id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE users SET is_admin=TRUE WHERE id=$1")
        .bind(org.owner_id)
        .execute(state.db.pool())
        .await
        .unwrap();
    let reconciled = reconcile(
        State(state.clone()),
        AuthUser(org.owner_id),
        Path((org.id, id)),
        Json(ReconcileRequest {
            actual_tokens: 7,
            evidence_reference: "fixture-settled-before-account-deletion".into(),
        }),
    )
    .await
    .unwrap();
    assert!(!reconciled.0.already_reconciled);
    state.store.delete_user_data_cascade(org.owner_id).await.unwrap();
    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id=$1")
        .bind(org.owner_id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    assert_eq!(users, 0);
    let history: (i64, i64) = sqlx::query_as(
        "SELECT used_tokens,reserved_tokens FROM free_ai_credit_periods WHERE owner_user_id=$1",
    )
    .bind(org.owner_id)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    assert_eq!(history, (7, 0));
    let owner: Uuid = sqlx::query_scalar("SELECT owner_id FROM organizations WHERE id=$1")
        .bind(org.id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    assert_eq!(owner, successor.owner_id);
    sqlx::query("UPDATE users SET is_admin=TRUE WHERE id=$1")
        .bind(successor.owner_id)
        .execute(state.db.pool())
        .await
        .unwrap();
    let replay = reconcile(
        State(state.clone()),
        AuthUser(successor.owner_id),
        Path((org.id, id)),
        Json(ReconcileRequest {
            actual_tokens: 7,
            evidence_reference: "fixture-settled-before-account-deletion".into(),
        }),
    )
    .await
    .unwrap();
    assert!(replay.0.already_reconciled);
    let history: (i64, i64) = sqlx::query_as(
        "SELECT used_tokens,reserved_tokens FROM free_ai_credit_periods WHERE owner_user_id=$1",
    )
    .bind(org.owner_id)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    assert_eq!(history, (7, 0));
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL and owned provider listener"]
async fn recreating_verified_account_cannot_reset_spent_free_credits() {
    let state = state().await;
    let first = cap(&state, organization(&state, Plan::Free).await).await;
    let email: String = sqlx::query_scalar("SELECT email FROM users WHERE id=$1")
        .bind(first.owner_id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    let provider = serve(Router::new().route(
        "/v1/chat/completions",
        post(move || {
            let counted = counted.clone();
            async move {
                counted.fetch_add(1, Ordering::SeqCst);
                Json(json!({"choices":[{"message":{"content":"complete"}}],
                "usage":{"prompt_tokens":500,"completion_tokens":500}}))
            }
        }),
    ))
    .await;
    call_with_budget(&state, &first, ProviderSelection::Platform, call(&provider.url))
        .await
        .unwrap();
    state.store.delete_user_data_cascade(first.owner_id).await.unwrap();
    let tag = Uuid::new_v4().simple().to_string();
    let recreated = state
        .store
        .create_user(&format!("recreated-{tag}"), &email, "disabled-test-account")
        .await
        .unwrap();
    assert_ne!(recreated.id, first.owner_id);
    // Preserve any pre-existing legacy accounting on the new, unverified user.
    sqlx::query("INSERT INTO free_ai_credit_periods(owner_user_id,period_start,granted_tokens,used_tokens) VALUES($1,date_trunc('month',CURRENT_DATE)::date,1000,3)")
        .bind(recreated.id).execute(state.db.pool()).await.unwrap();
    // Re-verification must reuse history without charging it again.
    sqlx::query("UPDATE users SET is_verified=TRUE WHERE id=$1")
        .bind(recreated.id)
        .execute(state.db.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE users SET is_verified=TRUE WHERE id=$1")
        .bind(recreated.id)
        .execute(state.db.pool())
        .await
        .unwrap();
    let fresh = state
        .store
        .create_organization("Recreated account", &tag, recreated.id, Plan::Free)
        .await
        .unwrap();
    let fresh = cap(&state, fresh).await;
    assert!(
        call_with_budget(&state, &fresh, ProviderSelection::Platform, call(&provider.url))
            .await
            .is_err(),
        "re-verifying the same email must not reset this month's spent Free credits"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let spent: i64 =
        sqlx::query_scalar("SELECT used_tokens FROM free_ai_credit_periods WHERE owner_user_id=$1")
            .bind(recreated.id)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
    assert_eq!(spent, 1003, "merge retains both histories exactly once");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL"]
async fn verified_email_changes_keep_aliases_bound_to_existing_allowance() {
    let state = state().await;
    let first = organization(&state, Plan::Free).await;
    let original: String = sqlx::query_scalar("SELECT email FROM users WHERE id=$1")
        .bind(first.owner_id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    sqlx::query("INSERT INTO free_ai_credit_periods(owner_user_id,period_start,granted_tokens,used_tokens) VALUES($1,date_trunc('month',CURRENT_DATE)::date,5000,5000)")
        .bind(first.owner_id).execute(state.db.pool()).await.unwrap();
    let tag = Uuid::new_v4().simple().to_string();
    sqlx::query("UPDATE users SET email=$2 WHERE id=$1")
        .bind(first.owner_id)
        .bind(format!("changed-{tag}@example.invalid"))
        .execute(state.db.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE users SET is_verified=TRUE WHERE id=$1")
        .bind(first.owner_id)
        .execute(state.db.pool())
        .await
        .unwrap();
    let second = state
        .store
        .create_user(&format!("alias-{tag}"), &original, "disabled-test-account")
        .await
        .unwrap();
    assert!(
        sqlx::query("UPDATE users SET is_verified=TRUE WHERE id=$1")
            .bind(second.id)
            .execute(state.db.pool())
            .await
            .is_err(),
        "an alias attached to a live account cannot create a second funded identity"
    );
    state.store.delete_user_data_cascade(first.owner_id).await.unwrap();
    sqlx::query("UPDATE users SET is_verified=TRUE WHERE id=$1")
        .bind(second.id)
        .execute(state.db.pool())
        .await
        .unwrap();
    let used: i64 =
        sqlx::query_scalar("SELECT used_tokens FROM free_ai_credit_periods WHERE owner_user_id=$1")
            .bind(second.id)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
    assert_eq!(used, 5000);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL"]
async fn concurrent_historical_alias_recreation_cannot_split_spent_allowance() {
    let state = state().await;
    let first = organization(&state, Plan::Free).await;
    let original: String = sqlx::query_scalar("SELECT email FROM users WHERE id=$1")
        .bind(first.owner_id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    sqlx::query("INSERT INTO free_ai_credit_periods(owner_user_id,period_start,granted_tokens,used_tokens) VALUES($1,date_trunc('month',CURRENT_DATE)::date,5000,5000)")
        .bind(first.owner_id).execute(state.db.pool()).await.unwrap();
    let tag = Uuid::new_v4().simple().to_string();
    let changed = format!("changed-{tag}@example.invalid");
    sqlx::query("UPDATE users SET email=$2 WHERE id=$1")
        .bind(first.owner_id)
        .bind(&changed)
        .execute(state.db.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE users SET is_verified=TRUE WHERE id=$1")
        .bind(first.owner_id)
        .execute(state.db.pool())
        .await
        .unwrap();
    state.store.delete_user_data_cascade(first.owner_id).await.unwrap();
    let a = state
        .store
        .create_user(&format!("alias-a-{tag}"), &original, "disabled-test-account")
        .await
        .unwrap();
    let b = state
        .store
        .create_user(&format!("alias-b-{tag}"), &changed, "disabled-test-account")
        .await
        .unwrap();
    let (left, right) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(
            sqlx::query("UPDATE users SET is_verified=TRUE WHERE id=$1")
                .bind(a.id)
                .execute(state.db.pool()),
            sqlx::query("UPDATE users SET is_verified=TRUE WHERE id=$1")
                .bind(b.id)
                .execute(state.db.pool())
        )
    })
    .await
    .expect("alias recreation must not deadlock");
    assert_ne!(left.is_ok(), right.is_ok(), "exactly one identity may inherit the funded pool");
    let winner = if left.is_ok() { a.id } else { b.id };
    let used: i64 =
        sqlx::query_scalar("SELECT used_tokens FROM free_ai_credit_periods WHERE owner_user_id=$1")
            .bind(winner)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
    assert_eq!(used, 5000);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL"]
async fn identity_retention_purges_expired_deleted_accounts_only() {
    let state = state().await;
    let expired = organization(&state, Plan::Free).await;
    let recent = organization(&state, Plan::Free).await;
    let active = organization(&state, Plan::Free).await;
    state.store.delete_user_data_cascade(expired.owner_id).await.unwrap();
    state.store.delete_user_data_cascade(recent.owner_id).await.unwrap();
    sqlx::query("UPDATE free_verified_identities SET account_deleted_at=NOW()-INTERVAL '36 days' WHERE owner_user_id IN ($1,$2)")
        .bind(expired.owner_id).bind(active.owner_id).execute(state.db.pool()).await.unwrap();
    mockforge_registry_server::workers::audit_log_retention::purge_deleted_free_identities(
        state.db.pool(),
    )
    .await
    .unwrap();
    let present: Vec<Uuid> = sqlx::query_scalar(
        "SELECT owner_user_id FROM free_verified_identities WHERE owner_user_id IN ($1,$2,$3)",
    )
    .bind(expired.owner_id)
    .bind(recent.owner_id)
    .bind(active.owner_id)
    .fetch_all(state.db.pool())
    .await
    .unwrap();
    assert!(!present.contains(&expired.owner_id));
    assert!(present.contains(&recent.owner_id));
    assert!(present.contains(&active.owner_id));
}
