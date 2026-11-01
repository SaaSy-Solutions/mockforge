//! Stripe webhook plumbing that does not depend on the endpoint's API version
//! (#1456).
//!
//! `async-stripe 0.41` deserializes a whole event into types built for API
//! version `2023-10-16`. Newer versions (for example `2025-10-29.clover`) moved
//! `current_period_start/_end` from the subscription onto its items, so the
//! typed parse failed and every subscription event was rejected. This module
//! verifies the signature itself and reads only the fields the handlers use,
//! from wherever the sending API version puts them.

use std::collections::HashMap;

use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

use crate::{error::ApiError, models::Plan};

/// Stripe's default signature tolerance: reject signatures older or newer than
/// five minutes, which bounds replay of a captured request.
pub const SIGNATURE_TOLERANCE_SECS: i64 = 300;

#[derive(Debug, PartialEq, Eq)]
pub enum SignatureError {
    Malformed,
    Mismatch,
    Expired(i64),
}

impl std::fmt::Display for SignatureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed => f.write_str("malformed stripe-signature header"),
            Self::Mismatch => f.write_str("no v1 signature matches the payload"),
            Self::Expired(t) => write!(f, "signature timestamp {t} is outside the tolerance"),
        }
    }
}

/// Verify a `stripe-signature` header (`t=<unix>,v1=<hex>[,v1=<hex>...]`).
///
/// Any `v1` entry may match: Stripe sends one per active secret while a secret
/// is being rolled. The HMAC comparison is constant-time.
pub fn verify_signature(
    payload: &str,
    header: &str,
    secret: &str,
    now: i64,
) -> Result<(), SignatureError> {
    let mut timestamp = None;
    let mut candidates = Vec::new();
    for part in header.split(',') {
        match part.trim().split_once('=') {
            Some(("t", v)) => timestamp = v.parse::<i64>().ok(),
            Some(("v1", v)) => candidates.push(v),
            _ => {}
        }
    }
    let t = timestamp.ok_or(SignatureError::Malformed)?;
    if candidates.is_empty() {
        return Err(SignatureError::Malformed);
    }

    let signed_payload = format!("{t}.{payload}");
    let matched = candidates.iter().any(|candidate| {
        let Ok(expected) = hex::decode(candidate) else {
            return false;
        };
        let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
            return false;
        };
        mac.update(signed_payload.as_bytes());
        mac.verify_slice(&expected).is_ok()
    });
    if !matched {
        return Err(SignatureError::Mismatch);
    }
    if (now - t).abs() > SIGNATURE_TOLERANCE_SECS {
        return Err(SignatureError::Expired(t));
    }
    Ok(())
}

/// The event envelope. `data.object` stays raw so an event type the handlers
/// do not act on can never fail to parse.
#[derive(Debug, Deserialize)]
pub struct WebhookEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub api_version: Option<String>,
    pub data: WebhookEventData,
}

#[derive(Debug, Deserialize)]
pub struct WebhookEventData {
    pub object: serde_json::Value,
}

/// A Stripe reference that is either a bare id or an expanded object.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum StripeRef {
    Id(String),
    Object { id: String },
}

impl StripeRef {
    pub fn id(&self) -> &str {
        match self {
            Self::Id(id) | Self::Object { id } => id,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StripeSubscription {
    pub id: String,
    pub customer: StripeRef,
    pub status: String,
    #[serde(default)]
    pub items: StripeList<StripeSubscriptionItem>,
    /// Present up to API version 2025-03-31; moved onto items afterwards.
    #[serde(default)]
    pub current_period_start: Option<i64>,
    #[serde(default)]
    pub current_period_end: Option<i64>,
    #[serde(default)]
    pub cancel_at_period_end: bool,
    #[serde(default)]
    pub canceled_at: Option<i64>,
    #[serde(default)]
    pub trial_end: Option<i64>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct StripeList<T> {
    #[serde(default = "Vec::new")]
    pub data: Vec<T>,
}

impl<T> Default for StripeList<T> {
    fn default() -> Self {
        Self { data: Vec::new() }
    }
}

#[derive(Debug, Deserialize)]
pub struct StripeSubscriptionItem {
    #[serde(default)]
    pub price: Option<StripePrice>,
    #[serde(default)]
    pub current_period_start: Option<i64>,
    #[serde(default)]
    pub current_period_end: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct StripePrice {
    pub id: String,
    #[serde(default)]
    pub unit_amount: Option<i64>,
}

impl StripeSubscription {
    fn first_price(&self) -> Option<&StripePrice> {
        self.items.data.first().and_then(|item| item.price.as_ref())
    }

    pub fn price_id(&self) -> Option<&str> {
        self.first_price().map(|price| price.id.as_str())
    }

    pub fn unit_amount(&self) -> Option<i64> {
        self.first_price().and_then(|price| price.unit_amount)
    }

    /// The current billing period as `(start, end)` unix seconds, read from the
    /// subscription (API versions up to 2025-03-31) or from its items (newer
    /// versions). Items can differ per item; the subscription's period is the
    /// earliest start and the latest end across them.
    pub fn period(&self) -> Option<(i64, i64)> {
        if let (Some(start), Some(end)) = (self.current_period_start, self.current_period_end) {
            return Some((start, end));
        }
        let start = self.items.data.iter().filter_map(|i| i.current_period_start).min()?;
        let end = self.items.data.iter().filter_map(|i| i.current_period_end).max()?;
        Some((start, end))
    }
}

#[derive(Debug, Deserialize)]
pub struct StripeInvoice {
    pub id: String,
    #[serde(default)]
    pub customer: Option<StripeRef>,
    #[serde(default)]
    pub amount_due: Option<i64>,
    #[serde(default)]
    pub next_payment_attempt: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct StripeCheckoutSession {
    #[serde(default)]
    pub client_reference_id: Option<String>,
    #[serde(default)]
    pub subscription: Option<StripeRef>,
    #[serde(default)]
    pub customer: Option<StripeRef>,
}

/// Parse `data.object` for an event the handlers act on. A shape that does not
/// parse can never be acted on, so it is `Ignored`, not retried.
pub fn parse_object<T: serde::de::DeserializeOwned>(
    object: serde_json::Value,
) -> Result<T, WebhookError> {
    serde_json::from_value(object)
        .map_err(|e| WebhookError::Ignored(format!("unparsable event object: {e}")))
}

/// Why an event was not applied.
#[derive(Debug)]
pub enum WebhookError {
    /// Nothing can ever be done with it (missing `org_id` metadata, an unknown
    /// subscription, an unparsable object). Acknowledged with 200 and logged,
    /// so Stripe does not retry it for days.
    Ignored(String),
    /// Refused on purpose and alerted, e.g. a price id this deployment does not
    /// know. Answered non-2xx so Stripe retries: once the operator configures
    /// the price, a retry applies the event.
    Rejected(String),
    /// A transient failure (database). Answered 500 so Stripe retries.
    Failed(ApiError),
}

impl From<ApiError> for WebhookError {
    fn from(e: ApiError) -> Self {
        Self::Failed(e)
    }
}

impl From<sqlx::Error> for WebhookError {
    fn from(e: sqlx::Error) -> Self {
        Self::Failed(ApiError::Database(e))
    }
}

/// The configured price ids, one per plan and cadence.
#[derive(Debug, Default, Clone, Copy)]
pub struct PriceIds<'a> {
    pub pro: Option<&'a str>,
    pub team: Option<&'a str>,
    pub pro_annual: Option<&'a str>,
    pub team_annual: Option<&'a str>,
}

impl<'a> PriceIds<'a> {
    pub fn from_config(config: &'a crate::config::Config) -> Self {
        Self {
            pro: config.stripe_price_id_pro.as_deref(),
            team: config.stripe_price_id_team.as_deref(),
            pro_annual: config.stripe_price_id_pro_annual.as_deref(),
            team_annual: config.stripe_price_id_team_annual.as_deref(),
        }
    }

    /// Exact match only. An unknown price id is `None`: guessing from the id's
    /// text, or falling back to Free, would grant or strip a paid plan without
    /// anyone noticing.
    pub fn plan_for(&self, price_id: &str) -> Option<Plan> {
        let is = |configured: Option<&str>| configured == Some(price_id);
        if is(self.pro) || is(self.pro_annual) {
            Some(Plan::Pro)
        } else if is(self.team) || is(self.team_annual) {
            Some(Plan::Team)
        } else {
            None
        }
    }
}

/// Result of trying to claim an event id for processing.
#[derive(Debug, PartialEq, Eq)]
pub enum Claim {
    /// This request owns the event and must finish or release it.
    Acquired,
    /// Already processed, or another delivery is processing it right now.
    Duplicate,
}

/// Claim `event_id`. A claim older than five minutes that never finished
/// (crashed process) can be retaken.
pub async fn claim_event(
    pool: &sqlx::PgPool,
    event_id: &str,
    event_type: &str,
) -> Result<Claim, sqlx::Error> {
    let claimed: Option<String> = sqlx::query_scalar(
        "INSERT INTO stripe_webhook_events (event_id, event_type) VALUES ($1, $2) \
         ON CONFLICT (event_id) DO UPDATE SET claimed_at = NOW() \
         WHERE stripe_webhook_events.processed_at IS NULL \
           AND stripe_webhook_events.claimed_at < NOW() - INTERVAL '5 minutes' \
         RETURNING event_id",
    )
    .bind(event_id)
    .bind(event_type)
    .fetch_optional(pool)
    .await?;
    Ok(if claimed.is_some() {
        Claim::Acquired
    } else {
        Claim::Duplicate
    })
}

/// Mark a claimed event done, with `outcome` ("applied" or "ignored: ...").
pub async fn finish_event(
    pool: &sqlx::PgPool,
    event_id: &str,
    outcome: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE stripe_webhook_events SET processed_at = NOW(), outcome = $2 \
         WHERE event_id = $1",
    )
    .bind(event_id)
    .bind(outcome)
    .execute(pool)
    .await?;
    Ok(())
}

/// Drop an unfinished claim so Stripe's retry processes the event again.
pub async fn release_event(pool: &sqlx::PgPool, event_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM stripe_webhook_events WHERE event_id = $1 AND processed_at IS NULL")
        .bind(event_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SECRET: &str = "whsec_unit_test";

    fn sign(payload: &str, t: i64, secret: &str) -> String {
        let mut mac =
            Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
        mac.update(format!("{t}.{payload}").as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    #[test]
    fn signature_accepts_valid_and_any_rolled_v1() {
        let body = r#"{"id":"evt_1"}"#;
        let good = sign(body, 1000, SECRET);
        let header = format!("t=1000,v1={good}");
        assert_eq!(verify_signature(body, &header, SECRET, 1000), Ok(()));
        let old = sign(body, 1000, "whsec_old");
        let rolled = format!("t=1000,v1={old},v1={good}");
        assert_eq!(verify_signature(body, &rolled, SECRET, 1100), Ok(()));
    }

    #[test]
    fn signature_rejects_tampering_staleness_and_garbage() {
        let body = r#"{"id":"evt_1"}"#;
        let header = format!("t=1000,v1={}", sign(body, 1000, SECRET));
        assert_eq!(
            verify_signature(r#"{"id":"evt_2"}"#, &header, SECRET, 1000),
            Err(SignatureError::Mismatch)
        );
        assert_eq!(
            verify_signature(body, &header, "whsec_other", 1000),
            Err(SignatureError::Mismatch)
        );
        assert_eq!(
            verify_signature(body, &header, SECRET, 1000 + SIGNATURE_TOLERANCE_SECS + 1),
            Err(SignatureError::Expired(1000))
        );
        // A re-timestamped header without a fresh HMAC does not verify.
        let forged = header.replace("t=1000", "t=5000");
        assert_eq!(verify_signature(body, &forged, SECRET, 5000), Err(SignatureError::Mismatch));
        for bad in ["", "v1=abc", "t=1000", "t=x,v1=ab", "t=1000,v0=ab"] {
            assert_eq!(
                verify_signature(body, bad, SECRET, 1000),
                Err(SignatureError::Malformed),
                "{bad}"
            );
        }
        let not_hex = "t=1000,v1=zz";
        assert_eq!(verify_signature(body, not_hex, SECRET, 1000), Err(SignatureError::Mismatch));
    }

    fn subscription_json(period_on_items: bool) -> serde_json::Value {
        let mut item = json!({
            "id": "si_1",
            "object": "subscription_item",
            "price": { "id": "price_pro", "unit_amount": 2900 },
        });
        let mut sub = json!({
            "id": "sub_1",
            "object": "subscription",
            "customer": "cus_1",
            "status": "active",
            "cancel_at_period_end": false,
            "metadata": { "org_id": "7d0c5b8e-1d53-4f6c-9a4e-2a8b51c3f001" },
        });
        if period_on_items {
            item["current_period_start"] = json!(1_700_000_000);
            item["current_period_end"] = json!(1_702_592_000);
        } else {
            sub["current_period_start"] = json!(1_700_000_000);
            sub["current_period_end"] = json!(1_702_592_000);
        }
        sub["items"] = json!({ "object": "list", "data": [item] });
        sub
    }

    #[test]
    fn period_is_read_on_2023_and_clover_api_versions() {
        for on_items in [false, true] {
            let sub: StripeSubscription =
                parse_object(subscription_json(on_items)).expect("parses");
            assert_eq!(sub.period(), Some((1_700_000_000, 1_702_592_000)), "items={on_items}");
            assert_eq!(sub.price_id(), Some("price_pro"));
            assert_eq!(sub.unit_amount(), Some(2900));
            assert_eq!(sub.customer.id(), "cus_1");
        }
    }

    #[test]
    fn period_spans_all_items_and_is_none_when_absent() {
        let mut raw = subscription_json(true);
        raw["items"]["data"].as_array_mut().expect("items").push(json!({
            "price": { "id": "price_addon" },
            "current_period_start": 1_699_000_000,
            "current_period_end": 1_703_000_000,
        }));
        let sub: StripeSubscription = parse_object(raw).expect("parses");
        assert_eq!(sub.period(), Some((1_699_000_000, 1_703_000_000)));

        let mut bare = subscription_json(true);
        bare["items"]["data"][0]
            .as_object_mut()
            .expect("item")
            .retain(|k, _| !k.starts_with("current_period"));
        let sub: StripeSubscription = parse_object(bare).expect("parses");
        assert_eq!(sub.period(), None);
    }

    #[test]
    fn expanded_customer_and_unknown_fields_parse() {
        let mut raw = subscription_json(false);
        raw["customer"] = json!({ "id": "cus_9", "object": "customer", "email": "x" });
        raw["billing_mode"] = json!({ "type": "flexible" });
        let sub: StripeSubscription = parse_object(raw).expect("parses");
        assert_eq!(sub.customer.id(), "cus_9");
    }

    #[test]
    fn a_shape_that_cannot_parse_is_ignored_not_retried() {
        let err = parse_object::<StripeSubscription>(json!({ "id": 5 })).expect_err("fails");
        assert!(matches!(err, WebhookError::Ignored(_)), "{err:?}");
    }

    #[test]
    fn envelope_parses_any_event_type_and_version() {
        let raw = json!({
            "id": "evt_1",
            "object": "event",
            "api_version": "2025-10-29.clover",
            "type": "v2.core.something_new",
            "data": { "object": { "whatever": [1, 2, 3] } },
        });
        let event: WebhookEvent = serde_json::from_value(raw).expect("parses");
        assert_eq!(event.event_type, "v2.core.something_new");
        assert_eq!(event.api_version.as_deref(), Some("2025-10-29.clover"));
    }

    #[test]
    fn unknown_price_ids_map_to_no_plan() {
        let ids = PriceIds {
            pro: Some("price_1Pro"),
            team: Some("price_1Team"),
            pro_annual: Some("price_1ProY"),
            team_annual: None,
        };
        assert_eq!(ids.plan_for("price_1Pro"), Some(Plan::Pro));
        assert_eq!(ids.plan_for("price_1ProY"), Some(Plan::Pro));
        assert_eq!(ids.plan_for("price_1Team"), Some(Plan::Team));
        // The old heuristic mapped these from their text; now they are unknown.
        assert_eq!(ids.plan_for("price_pro_monthly"), None);
        assert_eq!(ids.plan_for("price_team_2024"), None);
        assert_eq!(ids.plan_for(""), None);
        assert_eq!(PriceIds::default().plan_for("price_1Pro"), None);
    }
}
