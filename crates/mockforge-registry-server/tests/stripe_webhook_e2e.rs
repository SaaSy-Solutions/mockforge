//! Stripe webhook hardening against a running registry server (#1456).
//!
//! Covers, through the real HTTP route and Postgres:
//!   - a subscription event in the `2025-10-29.clover` shape (billing period
//!     only on the items) upgrades the org and stores the period;
//!   - a redelivered event id is acknowledged and NOT applied again;
//!   - an unknown price id is rejected (422), changes nothing, and is retried
//!     rather than recorded as processed;
//!   - events that can never be acted on (missing `org_id`, unknown type,
//!     unknown subscription) get 200, not a 400 that Stripe retries for days;
//!   - an unsigned or tampered request is still refused.
//!
//! Needs the `registry-e2e.yml` scaffolding with `STRIPE_WEBHOOK_SECRET`,
//! `STRIPE_PRICE_ID_PRO=price_test_pro` and `STRIPE_PRICE_ID_TEAM=price_test_team`
//! set on the server. Run manually with:
//! ```
//! REGISTRY_URL=http://localhost:8080 \
//! cargo test -p mockforge-registry-server --test stripe_webhook_e2e \
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
    json!({
        "id": event_id,
        "object": "event",
        "api_version": "2025-10-29.clover",
        "created": period.0,
        "type": event_type,
        "livemode": false,
        "data": {
            "object": {
                "id": sub_id,
                "object": "subscription",
                "customer": format!("cus_{sub_id}"),
                "status": "active",
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
    for event in [&no_org, &unknown_type, &unknown_sub_deleted, &malformed] {
        let (status, body) = h.post(event).await;
        assert_eq!(status, StatusCode::OK, "{} → {body}", event["type"]);
        assert!(body["ignored"].is_string(), "{body}");
    }
    assert_eq!(h.plan().await, "pro", "ignored events change nothing");

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
}
