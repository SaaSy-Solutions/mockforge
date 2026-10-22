use super::{organization, serve, state};
use axum::{
    extract::{Path, State},
    routing::post,
    Json, Router,
};
use mockforge_registry_server::{
    handlers::ai_credit_reconciliation::{reconcile, ReconcileRequest},
    middleware::AuthUser,
    models::{Plan, UsageCounter},
    store::with_org_context,
};
use serde_json::{json, Value};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires isolated Postgres and owned HTTP listener"]
async fn reconciliation_rolls_back_on_audit_failure_and_charges_original_period() {
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    let actor = org.owner_id;
    sqlx::query("UPDATE users SET is_admin=TRUE WHERE id=$1")
        .bind(actor)
        .execute(state.db.pool())
        .await
        .unwrap();
    let old:Uuid=sqlx::query_scalar("INSERT INTO usage_counters(org_id,period_start,ai_tokens_used) VALUES($1,(date_trunc('month',NOW())-interval '1 month')::date,17) RETURNING id")
        .bind(org.id).fetch_one(state.db.pool()).await.unwrap();
    let current = with_org_context(state.db.runtime_pool(), org.id, |tx| {
        Box::pin(async move { Ok(UsageCounter::get_or_create_current(&mut **tx, org.id).await?) })
    })
    .await
    .unwrap();
    let reservation = Uuid::new_v4();
    let zero = Uuid::new_v4();
    for id in [reservation, zero] {
        sqlx::query("INSERT INTO ai_token_reservations(id,org_id,usage_counter_id,budget,state) VALUES($1,$2,$3,777,'uncertain')")
            .bind(id).bind(org.id).bind(old).execute(state.db.pool()).await.unwrap();
    }
    let handler_state = state.clone();
    let server = serve(Router::new().route(
        "/organizations/{org}/reservations/{reservation}",
        post(move |Path(ids): Path<(Uuid, Uuid)>, Json(request): Json<ReconcileRequest>| {
            let state = handler_state.clone();
            async move { reconcile(State(state), AuthUser(actor), Path(ids), Json(request)).await }
        }),
    ))
    .await;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    let url = format!("{}/organizations/{}/reservations/{}", server.url, org.id, reservation);
    // This database belongs exclusively to this serial test fixture. Fail only
    // its synthetic evidence reference, then remove the injection before asserts.
    sqlx::raw_sql("CREATE FUNCTION fixture_reject_recovery_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.metadata->>'evidence_reference'='fixture-audit-failure' THEN RAISE EXCEPTION 'synthetic audit failure'; END IF; RETURN NEW; END; $$; CREATE TRIGGER fixture_recovery_audit_failure BEFORE INSERT ON audit_logs FOR EACH ROW EXECUTE FUNCTION fixture_reject_recovery_audit();")
        .execute(state.db.pool()).await.unwrap();
    let failed = client
        .post(&url)
        .json(&json!({"actual_tokens":11,"evidence_reference":"fixture-audit-failure"}))
        .send()
        .await
        .unwrap();
    sqlx::raw_sql("DROP TRIGGER fixture_recovery_audit_failure ON audit_logs; DROP FUNCTION fixture_reject_recovery_audit();").execute(state.db.pool()).await.unwrap();
    assert!(failed.status().is_server_error());
    let status: String = sqlx::query_scalar("SELECT state FROM ai_token_reservations WHERE id=$1")
        .bind(reservation)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    assert_eq!(status, "uncertain");
    let used: i64 = sqlx::query_scalar("SELECT ai_tokens_used FROM usage_counters WHERE id=$1")
        .bind(old)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    assert_eq!(used, 17);
    assert!(client
        .post(&url)
        .json(&json!({"actual_tokens":11,"evidence_reference":"fixture-provider-usage"}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let zero_url = format!("{}/organizations/{}/reservations/{}", server.url, org.id, zero);
    assert!(client
        .post(zero_url)
        .json(&json!({"actual_tokens":0,"evidence_reference":"fixture-confirmed-no-charge"}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let used: i64 = sqlx::query_scalar("SELECT ai_tokens_used FROM usage_counters WHERE id=$1")
        .bind(old)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    assert_eq!(used, 28);
    let used: i64 = sqlx::query_scalar("SELECT ai_tokens_used FROM usage_counters WHERE id=$1")
        .bind(current.id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    assert_eq!(used, 0);
    let held: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_token_reservations WHERE org_id=$1 AND state!='completed'",
    )
    .bind(org.id)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    assert_eq!(held, 0);
}

#[tokio::test]
#[ignore = "requires isolated Postgres and owned HTTP listener"]
async fn authoritative_reconciliation_is_admin_only_atomic_and_idempotent() {
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    let foreign = organization(&state, Plan::Free).await;
    let counter = with_org_context(state.db.runtime_pool(), org.id, |tx| {
        Box::pin(async move { Ok(UsageCounter::get_or_create_current(&mut **tx, org.id).await?) })
    })
    .await
    .unwrap();
    let uncertain = Uuid::new_v4();
    let pending = Uuid::new_v4();
    for (id, status) in [(uncertain, "uncertain"), (pending, "pending")] {
        sqlx::query("INSERT INTO ai_token_reservations(id,org_id,usage_counter_id,budget,state) VALUES($1,$2,$3,777,$4)")
            .bind(id).bind(org.id).bind(counter.id).bind(status).execute(state.db.pool()).await.unwrap();
    }
    let handler_state = state.clone();
    let actor = org.owner_id;
    // The real handler/accounting transaction is exercised over HTTP. JWT
    // middleware remains a separate route/authentication acceptance requirement.
    let server = serve(Router::new().route(
        "/organizations/{org}/reservations/{reservation}",
        post(move |Path(ids): Path<(Uuid, Uuid)>, Json(request): Json<ReconcileRequest>| {
            let state = handler_state.clone();
            async move { reconcile(State(state), AuthUser(actor), Path(ids), Json(request)).await }
        }),
    ))
    .await;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    let url = format!("{}/organizations/{}/reservations/{}", server.url, org.id, uncertain);
    let payload = json!({"actual_tokens":901,"evidence_reference":"provider-ticket-123"});
    assert_eq!(
        client.post(&url).json(&payload).send().await.unwrap().status(),
        reqwest::StatusCode::FORBIDDEN
    );
    sqlx::query("UPDATE users SET is_admin=TRUE WHERE id=$1")
        .bind(actor)
        .execute(state.db.pool())
        .await
        .unwrap();
    let wrong = format!("{}/organizations/{}/reservations/{}", server.url, foreign.id, uncertain);
    assert!(!client.post(wrong).json(&payload).send().await.unwrap().status().is_success());
    let running = format!("{}/organizations/{}/reservations/{}", server.url, org.id, pending);
    assert!(!client.post(running).json(&payload).send().await.unwrap().status().is_success());
    assert!(!client
        .post(&url)
        .json(&json!({"actual_tokens":0,"evidence_reference":"https://invalid.example/secret"}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let (a, b) = tokio::join!(
        client.post(&url).json(&payload).send(),
        client.post(&url).json(&payload).send()
    );
    let a = a.unwrap();
    let b = b.unwrap();
    assert!(a.status().is_success());
    assert!(b.status().is_success());
    let a: Value = a.json().await.unwrap();
    let b: Value = b.json().await.unwrap();
    assert_ne!(a["already_reconciled"], b["already_reconciled"]);
    let used: i64 = sqlx::query_scalar("SELECT ai_tokens_used FROM usage_counters WHERE id=$1")
        .bind(counter.id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    assert_eq!(used, 901, "charge actual usage once, including totals above the reservation");
    let row:(String,Option<i64>,Option<Uuid>,Option<String>)=sqlx::query_as("SELECT state,actual_tokens,reconciled_by,reconciliation_reference FROM ai_token_reservations WHERE id=$1").bind(uncertain).fetch_one(state.db.pool()).await.unwrap();
    assert_eq!(
        row,
        ("completed".into(), Some(901), Some(actor), Some("provider-ticket-123".into()))
    );
    let audits:i64=sqlx::query_scalar("SELECT count(*) FROM audit_logs WHERE org_id=$1 AND metadata->>'action'='ai_credit_reconciliation'").bind(org.id).fetch_one(state.db.pool()).await.unwrap();
    assert_eq!(audits, 1);
    assert!(!client
        .post(&url)
        .json(&json!({"actual_tokens":0,"evidence_reference":"provider-ticket-123"}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let held:i64=sqlx::query_scalar("SELECT SUM(budget)::bigint FROM ai_token_reservations WHERE org_id=$1 AND state!='completed'").bind(org.id).fetch_one(state.db.pool()).await.unwrap();
    assert_eq!(held, 777, "pending provider calls retain their full budget");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL"]
async fn pending_recovery_rejects_live_execution_and_accepts_stopped_fenced_task() {
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    sqlx::query("UPDATE users SET is_admin=TRUE WHERE id=$1")
        .bind(org.owner_id)
        .execute(state.db.pool())
        .await
        .unwrap();
    let counter = with_org_context(state.db.runtime_pool(), org.id, |tx| {
        Box::pin(async move { Ok(UsageCounter::get_or_create_current(&mut **tx, org.id).await?) })
    })
    .await
    .unwrap();
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO ai_token_reservations(id,org_id,usage_counter_id,budget,state,execution_fenced) VALUES($1,$2,$3,777,'pending',TRUE)")
        .bind(id).bind(org.id).bind(counter.id).execute(state.db.pool()).await.unwrap();
    let mut execution = state.db.pool().begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,107))")
        .bind(id.to_string())
        .execute(&mut *execution)
        .await
        .unwrap();
    let request = || ReconcileRequest {
        actual_tokens: 35,
        evidence_reference: "fixture-final-provider-usage".into(),
    };
    assert!(reconcile(
        State(state.clone()),
        AuthUser(org.owner_id),
        Path((org.id, id)),
        Json(request())
    )
    .await
    .is_err());
    let status: String = sqlx::query_scalar("SELECT state FROM ai_token_reservations WHERE id=$1")
        .bind(id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    assert_eq!(status, "pending");
    execution.rollback().await.unwrap();
    let result = reconcile(
        State(state.clone()),
        AuthUser(org.owner_id),
        Path((org.id, id)),
        Json(request()),
    )
    .await
    .unwrap();
    assert!(!result.0.already_reconciled);
    let replay = reconcile(
        State(state.clone()),
        AuthUser(org.owner_id),
        Path((org.id, id)),
        Json(request()),
    )
    .await
    .unwrap();
    assert!(replay.0.already_reconciled);
    let used: i64 = sqlx::query_scalar("SELECT ai_tokens_used FROM usage_counters WHERE id=$1")
        .bind(counter.id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    assert_eq!(used, 35);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL"]
async fn saturated_execution_pool_keeps_restricted_request_pool_available() {
    let state = state().await;
    let mut fences = Vec::new();
    for _ in 0..4 {
        fences.push(state.db.ai_execution_pool().acquire().await.unwrap());
    }
    let role: (bool, bool) =
        sqlx::query_as("SELECT rolsuper,rolbypassrls FROM pg_roles WHERE rolname=current_user")
            .fetch_one(&mut *fences[0])
            .await
            .unwrap();
    assert_eq!(role, (false, false), "execution uses the restricted runtime role");
    let available: i32 = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        sqlx::query_scalar("SELECT 1").fetch_one(state.db.runtime_pool()),
    )
    .await
    .expect("provider execution must not exhaust request connections")
    .unwrap();
    assert_eq!(available, 1);
    assert!(
        state.db.ai_execution_pool().try_acquire().is_none(),
        "execution capacity is bounded rather than opening unlimited connections"
    );
    drop(fences);
}
