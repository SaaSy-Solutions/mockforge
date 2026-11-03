//! Stripe webhook hardening against a running registry server (#1456).
//!
//! Covers, through the real HTTP route and Postgres:
//!   - a subscription event in the `2025-10-29.clover` shape (billing period
//!     only on the items) upgrades the org and stores the period;
//!   - a redelivered event id is acknowledged and NOT applied again;
//!   - an unknown price id is rejected (422), changes nothing, and is retried
//!     rather than recorded as processed;
//!   - events that can never be acted on (missing `org_id`, unknown type,
//!     malformed objects) get 200; missing prerequisite subscriptions retry;
//!   - an unsigned or tampered request is still refused.
//!
//! Needs the `registry-e2e.yml` scaffolding with `STRIPE_WEBHOOK_SECRET`,
//! `STRIPE_PRICE_ID_PRO=price_test_pro` and `STRIPE_PRICE_ID_TEAM=price_test_team`
//! set on the server. Run manually with:
//! ```
//! REGISTRY_URL=http://localhost:8080 \
//! scripts/dev/cargo test -p mockforge-registry-server --test integration stripe_webhook_e2e:: \
//!   -- --ignored --nocapture
//! ```

use hmac::{Hmac, Mac};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use sha2::Sha256;

const DEFAULT_WEBHOOK_SECRET: &str = "whsec_e2e_test_secret";
const PRO_PRICE: &str = "price_test_pro";
const TEAM_PRICE: &str = "price_test_team";

struct Harness {
    client: Client,
    base_url: String,
    secret: String,
    token: String,
    org_id: String,
}

impl Harness {
    async fn new() -> Self {
        let base_url =
            std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
        let secret = std::env::var("STRIPE_WEBHOOK_SECRET")
            .unwrap_or_else(|_| DEFAULT_WEBHOOK_SECRET.to_string());
        let client = Client::new();
        let tag = uuid::Uuid::new_v4().simple().to_string();
        let username = format!("whk_{}", &tag[..12]);

        let res = client
            .post(format!("{base_url}/api/v1/auth/register"))
            .json(&json!({
                "username": username,
                "email": format!("{username}@e2e-test.local"),
                "password": "SecureP@ssw0rd!2024",
            }))
            .send()
            .await
            .expect("register request");
        let status = res.status();
        let body: Value = res.json().await.expect("register JSON");
        assert!(status.is_success(), "register {status}: {body}");
        let token = body["access_token"].as_str().expect("access_token").to_string();

        let res = client
            .get(format!("{base_url}/api/v1/organizations"))
            .bearer_auth(&token)
            .send()
            .await
            .expect("list orgs");
        let body: Value = res.json().await.expect("orgs JSON");
        let orgs = body
            .as_array()
            .or_else(|| body["organizations"].as_array())
            .expect("organizations payload");
        let org_id = orgs
            .first()
            .and_then(|o| o["id"].as_str())
            .expect("auto-created org")
            .to_string();

        Self {
            client,
            base_url,
            secret,
            token,
            org_id,
        }
    }

    fn sign(&self, payload: &str) -> String {
        let t = chrono::Utc::now().timestamp();
        let mut mac = Hmac::<Sha256>::new_from_slice(self.secret.as_bytes())
            .expect("HMAC accepts any key length");
        mac.update(format!("{t}.{payload}").as_bytes());
        format!("t={t},v1={}", hex::encode(mac.finalize().into_bytes()))
    }

    async fn post_raw(&self, body: &str, signature: &str) -> (StatusCode, Value) {
        let res = self
            .client
            .post(format!("{}/api/v1/billing/webhook", self.base_url))
            .header("stripe-signature", signature)
            .header("content-type", "application/json")
            .body(body.to_string())
            .send()
            .await
            .expect("webhook POST");
        let status = res.status();
        let json = res.json().await.unwrap_or(Value::Null);
        (status, json)
    }

    async fn post(&self, event: &Value) -> (StatusCode, Value) {
        let body = event.to_string();
        let signature = self.sign(&body);
        self.post_raw(&body, &signature).await
    }

    async fn subscription(&self) -> Value {
        let res = self
            .client
            .get(format!("{}/api/v1/billing/subscription", self.base_url))
            .bearer_auth(&self.token)
            .header("X-Org-Id", &self.org_id)
            .send()
            .await
            .expect("subscription GET");
        let status = res.status();
        let body: Value = res.json().await.expect("subscription JSON");
        assert!(status.is_success(), "subscription GET {status}: {body}");
        body
    }

    async fn plan(&self) -> String {
        self.subscription().await["plan"].as_str().expect("plan").to_string()
    }
}

/// A `customer.subscription.*` event in the `2025-10-29.clover` shape: no
/// `current_period_*` on the subscription, only on its items, and none of the
/// fields `async-stripe 0.41` required.
fn clover_subscription_event(
    event_id: &str,
    event_type: &str,
    sub_id: &str,
    price_id: &str,
    metadata: Value,
    period: (i64, i64),
) -> Value {
    static EVENT_SEQUENCE: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
    let event_created =
        period.0 + 1000 + EVENT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    json!({
        "id": event_id,
        "object": "event",
        "api_version": "2025-10-29.clover",
        "created": event_created,
        "type": event_type,
        "livemode": false,
        "data": {
            "object": {
                "id": sub_id,
                "object": "subscription",
                "created": period.0,
                "livemode": false,
                "customer": format!("cus_{sub_id}"),
                "status": if event_type == "customer.subscription.deleted" { "canceled" } else { "active" },
                "cancel_at_period_end": false,
                "billing_mode": { "type": "flexible" },
                "items": {
                    "object": "list",
                    "data": [{
                        "id": format!("si_{sub_id}"),
                        "object": "subscription_item",
                        "current_period_start": period.0,
                        "current_period_end": period.1,
                        "price": { "id": price_id, "object": "price", "unit_amount": 2900 },
                    }],
                },
                "metadata": metadata,
            }
        }
    })
}

fn evt() -> String {
    format!("evt_whk_{}", uuid::Uuid::new_v4().simple())
}

#[tokio::test]
#[ignore] // needs a running registry server + Postgres (registry-e2e.yml)
async fn stripe_webhook_is_version_independent_idempotent_and_safe() {
    let h = Harness::new().await;
    assert_eq!(h.plan().await, "free");

    let sub_id = format!("sub_whk_{}", uuid::Uuid::new_v4().simple());
    let start = chrono::Utc::now().timestamp() - 60;
    let end = start + 30 * 86_400;
    let owned = json!({ "org_id": h.org_id });

    // 1. Clover-shaped event: the period is read from the items.
    let created_id = evt();
    let created = clover_subscription_event(
        &created_id,
        "customer.subscription.created",
        &sub_id,
        PRO_PRICE,
        owned.clone(),
        (start, end),
    );
    let (status, body) = h.post(&created).await;
    assert_eq!(status, StatusCode::OK, "clover subscription event: {body}");
    let sub = h.subscription().await;
    assert_eq!(sub["plan"], "pro", "{sub}");
    let stored_end = sub["current_period_end"].as_str().expect("period end stored");
    let stored_end = chrono::DateTime::parse_from_rfc3339(stored_end).expect("rfc3339");
    assert_eq!(stored_end.timestamp(), end, "period end comes from the item");

    // 2. Redelivery of the same event id is acknowledged and skipped, even
    //    if its body now says something else.
    let mut redelivered = created.clone();
    redelivered["data"]["object"]["items"]["data"][0]["price"]["id"] = json!(TEAM_PRICE);
    let (status, body) = h.post(&redelivered).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["duplicate"], true, "{body}");
    assert_eq!(h.plan().await, "pro", "a repeated event id must not be applied again");

    // 3. Unknown price: rejected, nothing changes, and a retry is processed
    //    again (not short-circuited as a duplicate).
    let unknown_id = evt();
    let unknown = clover_subscription_event(
        &unknown_id,
        "customer.subscription.updated",
        &sub_id,
        "price_not_configured_team",
        owned.clone(),
        (start, end),
    );
    for attempt in 0..2 {
        let (status, body) = h.post(&unknown).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "attempt {attempt}: {body}");
        assert!(
            body["error"].as_str().is_some_and(|e| e.contains("price_not_configured_team")),
            "{body}"
        );
    }
    assert_eq!(h.plan().await, "pro", "an unknown price must not change the plan");

    // 4. Events that can never be acted on are acknowledged with 200.
    let no_org = clover_subscription_event(
        &evt(),
        "customer.subscription.updated",
        &sub_id,
        TEAM_PRICE,
        json!({}),
        (start, end),
    );
    let unknown_type = json!({
        "id": evt(),
        "object": "event",
        "type": "customer.tax_id.created",
        "data": { "object": { "id": "txi_1", "object": "tax_id" } },
    });
    let unknown_sub_deleted = clover_subscription_event(
        &evt(),
        "customer.subscription.deleted",
        "sub_never_seen",
        PRO_PRICE,
        owned.clone(),
        (start, end),
    );
    let malformed = json!({
        "id": evt(),
        "object": "event",
        "type": "customer.subscription.updated",
        "data": { "object": { "id": 42 } },
    });
    for event in [&no_org, &unknown_type, &malformed] {
        let (status, body) = h.post(event).await;
        assert_eq!(status, StatusCode::OK, "{} → {body}", event["type"]);
        assert!(body["ignored"].is_string(), "{body}");
    }
    assert_eq!(h.plan().await, "pro", "ignored events change nothing");
    for _ in 0..2 {
        let (status, body) = h.post(&unknown_sub_deleted).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
        assert!(body["retry"].is_string(), "{body}");
    }

    // 5. A known event with a bad or missing signature is still refused.
    let fresh = clover_subscription_event(
        &evt(),
        "customer.subscription.updated",
        &sub_id,
        TEAM_PRICE,
        owned.clone(),
        (start, end),
    )
    .to_string();
    let signed_other = h.sign(r#"{"id":"evt_other"}"#);
    let (status, _) = h.post_raw(&fresh, &signed_other).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "tampered body must be refused");
    let (status, _) = h.post_raw(&fresh, "t=1,v1=00").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "stale/forged signature must be refused");
    assert_eq!(h.plan().await, "pro");

    // 6. The same event, properly signed, applies (Team is configured).
    let (status, body) = h.post_raw(&fresh, &h.sign(&fresh)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(h.plan().await, "team");

    // A deletion delivered before creation must remain retryable, not become
    // a permanently ignored event when the prerequisite arrives later.
    let late = Harness::new().await;
    let late_sub = format!("sub_late_{}", uuid::Uuid::new_v4().simple());
    let metadata = json!({ "org_id": late.org_id });
    let deleted = clover_subscription_event(
        &evt(),
        "customer.subscription.deleted",
        &late_sub,
        PRO_PRICE,
        metadata.clone(),
        (start, end),
    );
    assert_eq!(late.post(&deleted).await.0, StatusCode::SERVICE_UNAVAILABLE);
    let created = clover_subscription_event(
        &evt(),
        "customer.subscription.created",
        &late_sub,
        PRO_PRICE,
        metadata,
        (start, end),
    );
    assert_eq!(late.post(&created).await.0, StatusCode::OK);
    assert_eq!(late.plan().await, "pro");
    assert_eq!(late.post(&deleted).await.0, StatusCode::OK);
    assert_eq!(late.plan().await, "free");
    let resurrect = clover_subscription_event(
        &evt(),
        "customer.subscription.updated",
        &late_sub,
        TEAM_PRICE,
        json!({"org_id":late.org_id}),
        (start, end),
    );
    assert_eq!(late.post(&resurrect).await.0, StatusCode::OK);
    assert_eq!(late.plan().await, "free", "canceled identity cannot reactivate");

    let ordered = Harness::new().await;
    let newer_id = format!("sub_newer_{}", uuid::Uuid::new_v4().simple());
    let older_id = format!("sub_older_{}", uuid::Uuid::new_v4().simple());
    let owner = json!({"org_id":ordered.org_id});
    let mut newer = clover_subscription_event(
        &evt(),
        "customer.subscription.created",
        &newer_id,
        TEAM_PRICE,
        owner.clone(),
        (start, end),
    );
    newer["data"]["object"]["created"] = json!(start + 100);
    assert_eq!(ordered.post(&newer).await.0, StatusCode::OK);
    for kind in [
        "customer.subscription.created",
        "customer.subscription.updated",
        "customer.subscription.deleted",
    ] {
        let older = clover_subscription_event(
            &evt(),
            kind,
            &older_id,
            PRO_PRICE,
            owner.clone(),
            (start, end),
        );
        assert_eq!(ordered.post(&older).await.0, StatusCode::OK);
        assert_eq!(ordered.plan().await, "team", "old {kind} cannot replace newer subscription");
        assert_eq!(
            ordered.subscription().await["status"],
            "active",
            "read path must retain newer subscription after old {kind}"
        );
    }
    for kind in ["invoice.payment_failed", "invoice.payment_succeeded"] {
        for (harness, sub_id) in [(&ordered, &older_id), (&late, &late_sub)] {
            let invoice = json!({"id":evt(), "type":kind, "created":start+2000,
                "livemode":false,"data":{"object":{"id":"in_old",
                    "customer":format!("cus_{sub_id}"),
                    "parent":{"subscription_details":{"subscription":sub_id}}}}});
            let before = harness.subscription().await;
            assert_eq!(harness.post(&invoice).await.0, StatusCode::OK);
            assert_eq!(
                harness.subscription().await,
                before,
                "old invoice must not mutate subscription"
            );
            let mut foreign = invoice.clone();
            foreign["id"] = json!(evt());
            foreign["data"]["object"]["customer"] = json!("cus_foreign");
            assert_eq!(harness.post(&foreign).await.0, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(harness.subscription().await, before);
        }
    }
    let url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("ordering regression requires the registry database");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    let customer_before: Option<String> =
        sqlx::query_scalar("SELECT stripe_customer_id FROM organizations WHERE id=$1")
            .bind(uuid::Uuid::parse_str(&ordered.org_id).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(customer_before.as_deref(), Some(format!("cus_{newer_id}").as_str()));
    let old_checkout = json!({"id":evt(),"type":"checkout.session.completed",
        "created":start+3000,"livemode":false,"data":{"object":{
            "client_reference_id":ordered.org_id,"subscription":older_id,
            "customer":format!("cus_{older_id}")}}});
    assert_eq!(ordered.post(&old_checkout).await.0, StatusCode::OK);
    let customer_after: Option<String> =
        sqlx::query_scalar("SELECT stripe_customer_id FROM organizations WHERE id=$1")
            .bind(uuid::Uuid::parse_str(&ordered.org_id).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(customer_after, customer_before, "old checkout cannot replace current customer");
    pool.close().await;
    let mut stale = clover_subscription_event(
        &evt(),
        "customer.subscription.updated",
        &newer_id,
        PRO_PRICE,
        owner,
        (start, end),
    );
    stale["created"] = json!(newer["created"].as_i64().unwrap() - 1);
    stale["data"]["object"]["created"] = json!(start + 100);
    assert_eq!(ordered.post(&stale).await.0, StatusCode::OK);
    assert_eq!(ordered.plan().await, "team", "older same-ID update cannot change current plan");
}

async fn isolated_claim_pool() -> sqlx::PgPool {
    let url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("claim regressions require an isolated Postgres URL");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&url)
        .await
        .unwrap();
    // One connection owns a temporary copy of the actual migration table.
    // No fixture mutates another test's or the running server's claim records.
    let ddl = include_str!("../../migrations/20261101000001_stripe_webhook_events.sql")
        .replace("CREATE TABLE stripe_webhook_events", "CREATE TEMP TABLE stripe_webhook_events");
    assert!(ddl.contains("CREATE TEMP TABLE stripe_webhook_events"));
    sqlx::raw_sql(&ddl).execute(&pool).await.unwrap();
    pool
}

#[tokio::test]
#[ignore = "requires isolated Postgres"]
async fn unfinished_claim_retries_and_only_completed_claim_is_a_duplicate() {
    use mockforge_registry_server::handlers::stripe_webhook::{
        claim_event, finish_event, release_event, Claim,
    };
    let pool = isolated_claim_pool().await;
    let Claim::Acquired { claimed_at } = claim_event(&pool, "evt_busy", "test").await.unwrap()
    else {
        panic!("first delivery must acquire claim");
    };
    assert_eq!(claim_event(&pool, "evt_busy", "test").await.unwrap(), Claim::InFlight);
    finish_event(&pool, "evt_busy", claimed_at, "applied").await.unwrap();
    release_event(&pool, "evt_busy", claimed_at).await.unwrap();
    assert_eq!(claim_event(&pool, "evt_busy", "test").await.unwrap(), Claim::Processed);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires isolated Postgres"]
async fn expired_claim_cannot_finish_or_release_its_successor() {
    use mockforge_registry_server::handlers::stripe_webhook::{
        claim_event, finish_event, release_event, Claim,
    };
    let pool = isolated_claim_pool().await;
    let stale: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "INSERT INTO stripe_webhook_events(event_id,event_type,claimed_at) VALUES('evt_reclaim','test',NOW()-INTERVAL '6 minutes') RETURNING claimed_at",
    ).fetch_one(&pool).await.unwrap();
    let Claim::Acquired { claimed_at } = claim_event(&pool, "evt_reclaim", "test").await.unwrap()
    else {
        panic!("expired delivery must be reclaimed");
    };
    assert_ne!(stale, claimed_at);
    release_event(&pool, "evt_reclaim", stale).await.unwrap();
    assert_eq!(claim_event(&pool, "evt_reclaim", "test").await.unwrap(), Claim::InFlight);
    assert!(matches!(
        finish_event(&pool, "evt_reclaim", stale, "applied").await,
        Err(sqlx::Error::RowNotFound)
    ));
    assert_eq!(claim_event(&pool, "evt_reclaim", "test").await.unwrap(), Claim::InFlight);
    finish_event(&pool, "evt_reclaim", claimed_at, "applied").await.unwrap();
    assert_eq!(claim_event(&pool, "evt_reclaim", "test").await.unwrap(), Claim::Processed);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires isolated Postgres"]
async fn provider_lineage_selects_current_subscription_and_preserves_legacy_order() {
    use mockforge_registry_server::models::{Plan, Subscription, SubscriptionStatus};
    let pool = isolated_claim_pool().await;
    sqlx::raw_sql("CREATE TEMP TABLE organizations(id uuid PRIMARY KEY)")
        .execute(&pool)
        .await
        .unwrap();
    let ddl = include_str!("../../migrations/20250101000004_billing.sql")
        .split("-- Usage counters table")
        .next()
        .unwrap()
        .replace("CREATE TABLE subscriptions", "CREATE TEMP TABLE subscriptions");
    assert!(ddl.contains("CREATE TEMP TABLE subscriptions"));
    sqlx::raw_sql(&ddl).execute(&pool).await.unwrap();
    let init = include_str!("../../migrations/20250101000001_init.sql");
    let function = init
        .split("CREATE OR REPLACE FUNCTION update_updated_at_column()")
        .nth(1)
        .unwrap()
        .split("$$ language 'plpgsql';")
        .next()
        .unwrap();
    let function = format!(
        "CREATE FUNCTION pg_temp.update_updated_at_column(){function}$$ language 'plpgsql';"
    );
    sqlx::raw_sql(&function).execute(&pool).await.unwrap();
    let billing = include_str!("../../migrations/20250101000004_billing.sql");
    let trigger = billing
        .split("CREATE TRIGGER update_subscriptions_updated_at")
        .nth(1)
        .unwrap()
        .split(';')
        .next()
        .unwrap();
    let trigger = format!("CREATE TRIGGER update_subscriptions_updated_at{trigger};").replace(
        "FUNCTION update_updated_at_column()",
        "FUNCTION pg_temp.update_updated_at_column()",
    );
    sqlx::raw_sql(&trigger).execute(&pool).await.unwrap();
    let migration = include_str!("../../migrations/20261103000001_stripe_subscription_order.sql")
        .replace(
            "FUNCTION update_subscription_timestamp()",
            "FUNCTION pg_temp.update_subscription_timestamp()",
        );
    sqlx::raw_sql(&migration).execute(&pool).await.unwrap();
    let org = uuid::Uuid::new_v4();
    let foreign = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO organizations(id) VALUES ($1),($2)")
        .bind(org)
        .bind(foreign)
        .execute(&pool)
        .await
        .unwrap();
    let start = chrono::Utc::now();
    let end = start + chrono::Duration::days(30);
    for id in ["sub_newer", "sub_older"] {
        Subscription::upsert_from_stripe(
            &pool,
            org,
            id,
            "cus_owner",
            TEAM_PRICE,
            Plan::Team,
            SubscriptionStatus::Active,
            start,
            end,
            false,
            None,
        )
        .await
        .unwrap();
    }
    sqlx::raw_sql("UPDATE subscriptions SET created_at = NOW()-INTERVAL '1 day', stripe_created_at=200 WHERE stripe_subscription_id='sub_newer'; UPDATE subscriptions SET created_at=NOW(), stripe_created_at=100 WHERE stripe_subscription_id='sub_older';")
        .execute(&pool).await.unwrap();
    assert_eq!(
        Subscription::find_by_org(&pool, org)
            .await
            .unwrap()
            .unwrap()
            .stripe_subscription_id,
        "sub_newer"
    );
    let refused = Subscription::upsert_from_stripe(
        &pool,
        foreign,
        "sub_newer",
        "cus_owner",
        TEAM_PRICE,
        Plan::Team,
        SubscriptionStatus::Active,
        start,
        end,
        false,
        None,
    )
    .await;
    assert!(matches!(refused, Err(sqlx::Error::RowNotFound)));
    assert_eq!(
        Subscription::find_by_stripe_subscription_id(&pool, "sub_newer")
            .await
            .unwrap()
            .unwrap()
            .org_id,
        org
    );
    sqlx::query(
        "UPDATE subscriptions SET stripe_created_at=NULL WHERE stripe_subscription_id='sub_older'",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        Subscription::find_by_org(&pool, org)
            .await
            .unwrap()
            .unwrap()
            .stripe_subscription_id,
        "sub_older",
        "unqualified legacy rows preserve the old read policy"
    );
    sqlx::raw_sql("ALTER TABLE subscriptions DISABLE TRIGGER update_subscriptions_updated_at; UPDATE subscriptions SET status='past_due', updated_at=NOW()-INTERVAL '2 days' WHERE stripe_subscription_id='sub_newer'; ALTER TABLE subscriptions ENABLE TRIGGER update_subscriptions_updated_at;")
        .execute(&pool).await.unwrap();
    let past_due = Subscription::upsert_from_stripe(
        &pool,
        org,
        "sub_newer",
        "cus_owner",
        TEAM_PRICE,
        Plan::Team,
        SubscriptionStatus::PastDue,
        start,
        end,
        false,
        None,
    )
    .await
    .unwrap();
    Subscription::update_status(&pool, past_due.id, SubscriptionStatus::PastDue)
        .await
        .unwrap();
    Subscription::cancel_at_period_end(&pool, past_due.id).await.unwrap();
    let unchanged = Subscription::find_by_stripe_subscription_id(&pool, "sub_newer")
        .await
        .unwrap()
        .unwrap();
    assert!(
        unchanged.updated_at < chrono::Utc::now() - chrono::Duration::hours(24),
        "repeat snapshots must not renew expired grace"
    );
    Subscription::update_status(&pool, past_due.id, SubscriptionStatus::Active)
        .await
        .unwrap();
    Subscription::update_status(&pool, past_due.id, SubscriptionStatus::PastDue)
        .await
        .unwrap();
    let renewed = Subscription::find_by_stripe_subscription_id(&pool, "sub_newer")
        .await
        .unwrap()
        .unwrap();
    assert!(
        renewed.updated_at > chrono::Utc::now() - chrono::Duration::minutes(1),
        "new past-due episode starts grace"
    );
    pool.close().await;
}
