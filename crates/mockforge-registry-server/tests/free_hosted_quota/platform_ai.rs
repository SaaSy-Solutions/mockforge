//! Platform AI allowance: reservation before the provider call and settlement after cancellation or unknown usage.

use super::*;

#[tokio::test]
#[ignore = "requires isolated Postgres and local provider HTTP listener"]
async fn platform_ai_reserves_before_call_and_settles_after_caller_cancellation() {
    use mockforge_registry_server::ai::{
        reservation::call_with_budget, LlmCall, ProviderSelection,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    sqlx::query("UPDATE organizations SET limits_json=jsonb_set(limits_json,'{ai_tokens_per_month}','1800') WHERE id=$1").bind(org.id).execute(state.db.pool()).await.unwrap();
    let org = Organization::find_by_id(state.db.pool(), org.id).await.unwrap().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let observed = Arc::new(tokio::sync::Notify::new());
    let provider = serve(Router::new().route("/v1/chat/completions", axum::routing::post({
        let calls = calls.clone(); let gate = gate.clone(); let observed = observed.clone();
        move || { let calls = calls.clone(); let gate = gate.clone(); let observed = observed.clone(); async move {
            calls.fetch_add(1, Ordering::SeqCst);
            observed.notify_one();
            gate.acquire().await.unwrap().forget();
            axum::Json(json!({"choices":[{"message":{"content":"done"}}],"usage":{"prompt_tokens":10,"completion_tokens":20}}))
        }}
    }))).await;
    let call = || LlmCall {
        provider: "openai".into(),
        model: "fixture".into(),
        api_key: "synthetic-fixture-key".into(),
        base_url: Some(provider.url.clone()),
        system: "system".into(),
        user: "user".into(),
        temperature: 0.0,
        max_tokens: 1000,
    };
    let first = tokio::spawn({
        let state = state.clone();
        let org = org.clone();
        let call = call();
        async move { call_with_budget(&state, &org, ProviderSelection::Platform, call).await }
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), observed.notified())
        .await
        .unwrap();
    assert!(call_with_budget(&state, &org, ProviderSelection::Platform, call())
        .await
        .is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1, "denied request must not contact provider");
    sqlx::query("UPDATE users SET is_admin=TRUE WHERE id=$1")
        .bind(org.owner_id)
        .execute(state.db.pool())
        .await
        .unwrap();
    let reservation: Uuid = sqlx::query_scalar(
        "SELECT id FROM ai_token_reservations WHERE org_id=$1 AND state='pending'",
    )
    .bind(org.id)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    let recover =
        || mockforge_registry_server::handlers::ai_credit_reconciliation::ReconcileRequest {
            actual_tokens: 0,
            evidence_reference: "fixture-premature-evidence".into(),
        };
    let rejected = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        mockforge_registry_server::handlers::ai_credit_reconciliation::reconcile(
            axum::extract::State(state.clone()),
            mockforge_registry_server::middleware::AuthUser(org.owner_id),
            axum::extract::Path((org.id, reservation)),
            axum::Json(recover()),
        ),
    )
    .await
    .expect("live execution recovery must fail without waiting for provider");
    assert!(rejected.is_err(), "active provider call cannot be reconciled");
    first.abort();
    assert!(first.await.unwrap_err().is_cancelled());
    gate.add_permits(1);
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let used: i64 =
            sqlx::query_scalar("SELECT ai_tokens_used FROM usage_counters WHERE org_id=$1")
                .bind(org.id)
                .fetch_one(state.db.pool())
                .await
                .unwrap();
        if used == 30 {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "canceled caller lost provider settlement"
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    gate.add_permits(1);
    assert_eq!(
        call_with_budget(&state, &org, ProviderSelection::Platform, call())
            .await
            .unwrap()
            .total_tokens(),
        30
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let completed: i64=sqlx::query_scalar("SELECT COUNT(*) FROM ai_token_reservations WHERE org_id=$1 AND state='completed' AND actual_tokens=30").bind(org.id).fetch_one(state.db.pool()).await.unwrap();
    assert_eq!(completed, 2);
}

#[tokio::test]
#[ignore = "requires isolated Postgres and local provider HTTP listener"]
async fn platform_ai_retains_unknown_usage_and_settles_the_admitted_period() {
    use mockforge_registry_server::ai::{
        reservation::call_with_budget, LlmCall, ProviderSelection,
    };
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let observed = Arc::new(tokio::sync::Notify::new());
    let provider=serve(Router::new()
        .route("/missing/v1/chat/completions",axum::routing::post(||async { axum::Json(json!({"choices":[{"message":{"content":"unmetered"}}]})) }))
        .route("/valid/v1/chat/completions",axum::routing::post({ let gate=gate.clone();let observed=observed.clone();move ||{ let gate=gate.clone();let observed=observed.clone();async move {
            observed.notify_one();gate.acquire().await.unwrap().forget();
            axum::Json(json!({"choices":[{"message":{"content":"done"}}],"usage":{"prompt_tokens":10,"completion_tokens":20}}))
        }}}))).await;
    let call = |path| LlmCall {
        provider: "openai".into(),
        model: "fixture".into(),
        api_key: "synthetic-fixture-key".into(),
        base_url: Some(format!("{}/{path}", provider.url)),
        system: "system".into(),
        user: "user".into(),
        temperature: 0.0,
        max_tokens: 1000,
    };
    assert!(call_with_budget(&state, &org, ProviderSelection::Platform, call("missing"))
        .await
        .is_err());
    let uncertain:i64=sqlx::query_scalar("SELECT COUNT(*) FROM ai_token_reservations WHERE org_id=$1 AND state='uncertain' AND actual_tokens IS NULL").bind(org.id).fetch_one(state.db.pool()).await.unwrap();
    assert_eq!(uncertain, 1);
    let other = organization(&state, Plan::Free).await;
    let visible: i64 = with_org_context(state.db.runtime_pool(), other.id, |tx| {
        Box::pin(async move {
            Ok(sqlx::query_scalar("SELECT COUNT(*) FROM ai_token_reservations WHERE org_id=$1")
                .bind(org.id)
                .fetch_one(&mut **tx)
                .await?)
        })
    })
    .await
    .unwrap();
    assert_eq!(visible, 0, "another tenant cannot see held credits");
    let deletion = sqlx::query("DELETE FROM organizations WHERE id=$1")
        .bind(org.id)
        .execute(state.db.pool())
        .await
        .unwrap_err();
    assert_eq!(deletion.as_database_error().unwrap().code().as_deref(), Some("23514"));
    let counter: Uuid = sqlx::query_scalar("SELECT id FROM usage_counters WHERE org_id=$1")
        .bind(org.id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    let first = tokio::spawn({
        let state = state.clone();
        let org = org.clone();
        let call = call("valid");
        async move { call_with_budget(&state, &org, ProviderSelection::Platform, call).await }
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), observed.notified())
        .await
        .unwrap();
    sqlx::query("UPDATE usage_counters SET period_start=(date_trunc('month',NOW())-INTERVAL '1 month')::date WHERE id=$1").bind(counter).execute(state.db.pool()).await.unwrap();
    let shared: Uuid =
        sqlx::query_scalar("SELECT id FROM free_ai_credit_periods WHERE owner_user_id=$1")
            .bind(org.owner_id)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
    sqlx::query("UPDATE free_ai_credit_periods SET period_start=(date_trunc('month',NOW())-INTERVAL '1 month')::date WHERE id=$1")
        .bind(shared).execute(state.db.pool()).await.unwrap();
    let current_shared: Uuid = sqlx::query_scalar("INSERT INTO free_ai_credit_periods(owner_user_id,period_start,granted_tokens) VALUES($1,date_trunc('month',NOW())::date,5000) RETURNING id")
        .bind(org.owner_id).fetch_one(state.db.pool()).await.unwrap();
    with_org_context(state.db.runtime_pool(), org.id, |tx| {
        Box::pin(async move { Ok(UsageCounter::get_or_create_current(&mut **tx, org.id).await?) })
    })
    .await
    .unwrap();
    gate.add_permits(1);
    assert_eq!(first.await.unwrap().unwrap().total_tokens(), 30);
    let old: i64 = sqlx::query_scalar("SELECT ai_tokens_used FROM usage_counters WHERE id=$1")
        .bind(counter)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    assert_eq!(old, 30);
    let current: i64 =
        sqlx::query_scalar("SELECT ai_tokens_used FROM usage_counters WHERE org_id=$1 AND id<>$2")
            .bind(org.id)
            .bind(counter)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
    assert_eq!(current, 0);
    let original_shared_used: i64 =
        sqlx::query_scalar("SELECT used_tokens FROM free_ai_credit_periods WHERE id=$1")
            .bind(shared)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
    assert_eq!(original_shared_used, 30);
    let current_shared_used: (i64, i64) = sqlx::query_as(
        "SELECT used_tokens,reserved_tokens FROM free_ai_credit_periods WHERE id=$1",
    )
    .bind(current_shared)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    assert_eq!(current_shared_used, (0, 0));
}

#[tokio::test]
#[ignore = "requires isolated Postgres"]
async fn free_ai_allowance_and_canceled_paid_fallback_match_the_quota_response() {
    use axum::{extract::State, http::HeaderMap};
    use mockforge_registry_server::{
        handlers::ai_studio::quota,
        middleware::AuthUser,
        models::{Subscription, SubscriptionStatus},
    };
    let state = state().await;
    let free = organization(&state, Plan::Free).await;
    let paid = organization(&state, Plan::Pro).await;
    let tag = Uuid::new_v4().simple().to_string();
    let now = chrono::Utc::now();
    Subscription::upsert_from_stripe(
        state.db.pool(),
        paid.id,
        &format!("sub_fixture_{tag}"),
        &format!("cus_fixture_{tag}"),
        "price_fixture",
        Plan::Pro,
        SubscriptionStatus::Canceled,
        now,
        now,
        true,
        Some(now),
    )
    .await
    .unwrap();
    for org in [free, paid] {
        let mut headers = HeaderMap::new();
        headers.insert("X-Organization-Id", org.id.to_string().parse().unwrap());
        let before = quota(State(state.clone()), AuthUser(org.owner_id), headers.clone())
            .await
            .unwrap()
            .0;
        assert_eq!(before.provider, "platform");
        assert_eq!(before.tokens_limit, 5000);
        assert!(before.call_allowed);
        with_org_context(state.db.runtime_pool(), org.id, |tx| {
            Box::pin(async move {
                UsageCounter::increment_ai_tokens(tx, org.id, 5000).await?;
                Ok(())
            })
        })
        .await
        .unwrap();
        let after = quota(State(state.clone()), AuthUser(org.owner_id), headers).await.unwrap().0;
        assert_eq!(after.tokens_used_this_period, 5000);
        assert!(!after.call_allowed);
    }
}

#[tokio::test]
#[ignore = "requires isolated Postgres, local provider, and MOCKFORGE_PLATFORM_LLM_ENABLED unset"]
async fn disabled_platform_blocks_direct_execution_and_preserves_byok() {
    use mockforge_registry_server::ai::{
        reservation::call_with_budget, resolve_provider, LlmCall, ProviderSelection,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    assert!(
        std::env::var("MOCKFORGE_PLATFORM_LLM_ENABLED").is_err(),
        "run this regression separately with activation unset"
    );
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = serve(Router::new().route("/v1/chat/completions", axum::routing::post({
        let calls = calls.clone();
        move || { let calls = calls.clone(); async move {
            calls.fetch_add(1, Ordering::SeqCst);
            axum::Json(json!({"choices":[{"message":{"content":"done"}}],"usage":{"prompt_tokens":1,"completion_tokens":1}}))
        }}
    }))).await;
    let call = || LlmCall {
        provider: "openai".into(),
        model: "fixture".into(),
        api_key: "synthetic-fixture-key".into(),
        base_url: Some(provider.url.clone()),
        system: "system".into(),
        user: "user".into(),
        temperature: 0.0,
        max_tokens: 10,
    };
    assert!(call_with_budget(&state, &org, ProviderSelection::Platform, call())
        .await
        .is_err());
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "disabled direct/queued platform execution contacted provider"
    );
    let reservations: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM ai_token_reservations WHERE org_id=$1")
            .bind(org.id)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
    assert_eq!(reservations, 0, "disabled platform must not reserve credits");
    assert_eq!(
        resolve_provider(&state, &org, None).await.unwrap().selection(),
        ProviderSelection::Disabled
    );
    let byok = mockforge_registry_core::models::BYOKConfig {
        provider: "openai".into(),
        api_key: "encrypted-fixture".into(),
        base_url: Some(provider.url.clone()),
        model: None,
        enabled: true,
    };
    assert_eq!(
        resolve_provider(&state, &org, Some(byok)).await.unwrap().selection(),
        ProviderSelection::Byok
    );
    assert!(call_with_budget(&state, &org, ProviderSelection::Byok, call()).await.is_ok());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
