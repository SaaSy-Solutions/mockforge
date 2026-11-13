//! Serialize provider snapshots against immutable subscription lineage.

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use super::stripe_webhook::{PriceIds, StripeSubscription, WebhookError};
use crate::{
    config::Config,
    models::{Organization, Plan, Subscription, SubscriptionStatus},
};

pub(super) struct EventContext {
    pub id: String,
    pub created: Option<i64>,
    pub livemode: Option<bool>,
    pub account: Option<String>,
}

pub(super) struct Applied {
    pub subscription: Subscription,
    pub old_plan: Plan,
    pub new_plan: Plan,
    pub amount: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
struct Lineage {
    stripe_subscription_id: String,
    stripe_customer_id: String,
    stripe_created_at: Option<i64>,
    last_stripe_event_created: Option<i64>,
    last_stripe_event_id: Option<String>,
    stripe_snapshot_authoritative: bool,
    status: String,
    updated_at: DateTime<Utc>,
    revision: String,
}

#[derive(FromRow)]
struct OrgSnapshot {
    #[sqlx(flatten)]
    org: Organization,
    revision: String,
}

async fn lineage(
    executor: impl sqlx::PgExecutor<'_>,
    org_id: Uuid,
) -> Result<Vec<Lineage>, sqlx::Error> {
    sqlx::query_as("SELECT stripe_subscription_id, stripe_customer_id, stripe_created_at, last_stripe_event_created, last_stripe_event_id, stripe_snapshot_authoritative, status, updated_at, xmin::text AS revision FROM subscriptions WHERE org_id = $1 ORDER BY stripe_subscription_id")
        .bind(org_id).fetch_all(executor).await
}

fn pending(message: impl Into<String>) -> WebhookError {
    WebhookError::Pending(message.into())
}

fn timestamp(value: Option<i64>, name: &str) -> Result<i64, WebhookError> {
    value
        .filter(|value| *value > 0 && DateTime::<Utc>::from_timestamp(*value, 0).is_some())
        .ok_or_else(|| pending(format!("missing or invalid Stripe {name}; recovery required")))
}

fn identity(
    subscription: &StripeSubscription,
    id: &str,
    customer: &str,
    org_id: Uuid,
    mode: bool,
) -> Result<(), WebhookError> {
    let owner = subscription.metadata.get("org_id").and_then(|id| Uuid::parse_str(id).ok());
    if subscription.id != id
        || subscription.customer.id() != customer
        || owner != Some(org_id)
        || subscription.livemode != Some(mode)
    {
        return Err(pending(
            "Stripe readback identity or mode does not match the signed subscription",
        ));
    }
    Ok(())
}

async fn readback(client: &stripe::Client, id: &str) -> Result<StripeSubscription, WebhookError> {
    // Only a resource ID may become a path segment; never a caller-supplied URL.
    if !id.starts_with("sub_")
        || !id.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(pending("invalid Stripe subscription identity"));
    }
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        client.get::<StripeSubscription>(&format!("/subscriptions/{id}")),
    )
    .await
    .map_err(|_| pending("Stripe subscription recovery timed out"))?
    .map_err(|_| pending("Stripe subscription recovery unavailable"))
}

fn requires_readback(
    previous: Option<&Lineage>,
    created: i64,
    event_id: &str,
    subscription: &StripeSubscription,
) -> bool {
    subscription.created.is_none()
        || subscription.livemode.is_none()
        || previous.is_some_and(|row| {
            row.stripe_created_at.is_none()
                || row.last_stripe_event_created.is_none()
                || row.stripe_snapshot_authoritative
                || (row.last_stripe_event_created == Some(created)
                    && row.last_stripe_event_id.as_deref() != Some(event_id))
        })
}

async fn apply_inner(
    pool: &PgPool,
    config: &Config,
    event: &EventContext,
    subscription: StripeSubscription,
    deleted: bool,
    authoritative: bool,
) -> Result<Option<Applied>, WebhookError> {
    // The deployed billing client addresses the platform account, not Connect.
    if event.account.is_some() {
        return Err(WebhookError::Rejected("Connect subscription events are not supported".into()));
    }
    let created = timestamp(event.created, "event creation time")?;
    let mode = event.livemode.ok_or_else(|| pending("missing Stripe event mode"))?;
    if let Some(key) = config.stripe_secret_key.as_deref() {
        let configured_mode = if key.starts_with("sk_live_") || key.starts_with("rk_live_") {
            Some(true)
        } else if key.starts_with("sk_test_") || key.starts_with("rk_test_") {
            Some(false)
        } else {
            None
        };
        if configured_mode.is_some_and(|configured| configured != mode) {
            return Err(WebhookError::Rejected(
                "event mode differs from configured Stripe account mode".into(),
            ));
        }
    }

    let existing = Subscription::find_by_stripe_subscription_id(pool, &subscription.id).await?;
    let org_id = if deleted {
        if let Some(record) = existing.as_ref() {
            record.org_id
        } else {
            let org_id =
                subscription.metadata.get("org_id").and_then(|id| Uuid::parse_str(id).ok());
            let Some(org_id) = org_id else {
                return Err(WebhookError::Ignored("missing subscription organization".into()));
            };
            if Organization::find_by_id(pool, org_id).await?.is_none() {
                return Err(WebhookError::Ignored(
                    "subscription organization no longer exists".into(),
                ));
            }
            return Err(pending("subscription deletion arrived before its prerequisite"));
        }
    } else {
        subscription
            .metadata
            .get("org_id")
            .and_then(|id| Uuid::parse_str(id).ok())
            .ok_or_else(|| WebhookError::Ignored("missing subscription organization".into()))?
    };
    if existing.as_ref().is_some_and(|record| {
        record.org_id != org_id || record.stripe_customer_id != subscription.customer.id()
    }) {
        return Err(WebhookError::Rejected("subscription ownership changed".into()));
    }
    let observed: OrgSnapshot = sqlx::query_as(
        "SELECT organizations.*, xmin::text AS revision FROM organizations WHERE id = $1",
    )
    .bind(org_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| WebhookError::Ignored("subscription organization no longer exists".into()))?;
    let org = &observed.org;
    let snapshot = lineage(pool, org_id).await?;
    let previous = snapshot.iter().find(|row| row.stripe_subscription_id == subscription.id);
    // A canceled Stripe identity cannot be reactivated by a later delivery.
    if previous.is_some_and(|row| row.status == "canceled") {
        return Ok(None);
    }
    if previous
        .and_then(|row| row.last_stripe_event_created)
        .is_some_and(|last| created < last)
        && !authoritative
        && !previous.is_some_and(|row| row.stripe_snapshot_authoritative)
    {
        return Ok(None);
    }
    let needs_readback =
        authoritative || requires_readback(previous, created, &event.id, &subscription);
    let needs_legacy = snapshot.iter().any(|row| row.stripe_created_at.is_none());
    let client = if needs_readback || needs_legacy {
        Some(stripe::Client::new(
            config
                .stripe_secret_key
                .as_ref()
                .ok_or_else(|| pending("Stripe recovery requires the configured secret key"))?
                .clone(),
        ))
    } else {
        None
    };
    let subscription = if needs_readback {
        let recovered =
            readback(client.as_ref().expect("recovery client established"), &subscription.id)
                .await?;
        identity(&recovered, &subscription.id, subscription.customer.id(), org_id, mode)?;
        if deleted && recovered.status != "canceled" {
            return Err(pending("Stripe deletion conflicts with authoritative status"));
        }
        recovered
    } else {
        subscription
    };
    if subscription.livemode.is_some_and(|value| value != mode) {
        return Err(WebhookError::Rejected("subscription mode differs from event mode".into()));
    }
    let provider_created = timestamp(subscription.created, "subscription creation time")?;
    if provider_created > created {
        return Err(pending("subscription creation time follows its event"));
    }
    if previous
        .and_then(|row| row.stripe_created_at)
        .is_some_and(|value| value != provider_created)
    {
        return Err(pending("immutable Stripe subscription creation time changed"));
    }
    let mut ordered = Vec::new();
    // Legacy recovery reads happen before acquiring a database lock. The full
    // snapshot is compared again under that lock before anything is written.
    for row in &snapshot {
        let value = if row.stripe_subscription_id == subscription.id {
            provider_created
        } else if let Some(value) = row.stripe_created_at {
            value
        } else {
            let recovered = readback(
                client.as_ref().expect("legacy client established"),
                &row.stripe_subscription_id,
            )
            .await?;
            identity(
                &recovered,
                &row.stripe_subscription_id,
                &row.stripe_customer_id,
                org_id,
                mode,
            )?;
            timestamp(recovered.created, "legacy subscription creation time")?
        };
        ordered.push((row.stripe_subscription_id.as_str(), value));
    }
    if previous.is_none() {
        ordered.push((&subscription.id, provider_created));
    }
    let newest = ordered
        .iter()
        .map(|(_, created)| *created)
        .max()
        .expect("incoming subscription present");
    let current: Vec<_> = ordered.iter().filter(|(_, created)| *created == newest).collect();
    if current.len() != 1 {
        return Err(pending("subscription creation lineage is ambiguous; recovery required"));
    }
    let is_current = current[0].0 == subscription.id;
    if !matches!(
        subscription.status.as_str(),
        "active"
            | "trialing"
            | "past_due"
            | "canceled"
            | "unpaid"
            | "incomplete"
            | "incomplete_expired"
    ) {
        return Err(pending("unsupported authoritative subscription status"));
    }
    let status = if deleted {
        SubscriptionStatus::Canceled
    } else {
        SubscriptionStatus::from_string(&subscription.status)
    };
    let plan = if deleted {
        existing.as_ref().expect("deletion prerequisite checked").plan()
    } else {
        let price = subscription.price_id().ok_or_else(|| pending("subscription has no price"))?;
        PriceIds::from_config(config)
            .plan_for(price)
            .ok_or_else(|| WebhookError::Rejected(format!("unknown Stripe price {price}")))?
    };
    let (start, end) = subscription
        .period()
        .and_then(|(start, end)| {
            Some((
                DateTime::<Utc>::from_timestamp(start, 0)?,
                DateTime::<Utc>::from_timestamp(end, 0)?,
            ))
        })
        .or_else(|| {
            existing
                .as_ref()
                .map(|record| (record.current_period_start, record.current_period_end))
        })
        .ok_or_else(|| pending("subscription period unavailable"))?;
    let amount = subscription.unit_amount().map(|amount| amount as f64 / 100.0);
    let mut tx = pool.begin().await?;
    let locked: OrgSnapshot = sqlx::query_as("SELECT organizations.*, xmin::text AS revision FROM organizations WHERE id = $1 FOR UPDATE")
        .bind(org_id).fetch_one(&mut *tx).await?;
    if locked.revision != observed.revision || lineage(&mut *tx, org_id).await? != snapshot {
        return Err(pending("subscription changed during recovery; retry with a fresh snapshot"));
    }
    for (id, created) in &ordered {
        sqlx::query("UPDATE subscriptions SET stripe_created_at = $2 WHERE stripe_subscription_id = $1 AND org_id = $3 AND stripe_created_at IS NULL")
            .bind(id).bind(created).bind(org_id).execute(&mut *tx).await?;
    }
    let record = if deleted {
        let record = existing.as_ref().expect("deletion prerequisite checked");
        Subscription::update_status(&mut *tx, record.id, SubscriptionStatus::Canceled).await?;
        Subscription::find_by_stripe_subscription_id(&mut *tx, &subscription.id)
            .await?
            .expect("locked existing subscription")
    } else {
        Subscription::upsert_from_stripe(
            &mut *tx,
            org_id,
            &subscription.id,
            subscription.customer.id(),
            subscription.price_id().expect("price checked"),
            plan,
            status,
            start,
            end,
            subscription.cancel_at_period_end,
            subscription
                .canceled_at
                .and_then(|value| DateTime::<Utc>::from_timestamp(value, 0)),
        )
        .await?
    };
    sqlx::query("UPDATE subscriptions SET stripe_created_at = $2, last_stripe_event_created = $3, last_stripe_event_id = $4, stripe_snapshot_authoritative = $5 WHERE stripe_subscription_id = $1 AND org_id = $6")
        .bind(&subscription.id).bind(provider_created).bind(created).bind(&event.id).bind(needs_readback).bind(org_id).execute(&mut *tx).await?;
    let new_plan =
        if matches!(status, SubscriptionStatus::Canceled | SubscriptionStatus::IncompleteExpired) {
            Plan::Free
        } else {
            plan
        };
    if is_current {
        Organization::update_plan(&mut *tx, org_id, new_plan).await?;
        sqlx::query("UPDATE organizations SET stripe_customer_id = $2 WHERE id = $1")
            .bind(org_id)
            .bind(subscription.customer.id())
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(is_current.then_some(Applied {
        subscription: record,
        old_plan: org.plan(),
        new_plan,
        amount,
    }))
}

pub(super) async fn apply(
    pool: &PgPool,
    config: &Config,
    event: &EventContext,
    subscription: StripeSubscription,
    deleted: bool,
) -> Result<Option<Applied>, WebhookError> {
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        apply_inner(pool, config, event, subscription, deleted, false),
    )
    .await
    .map_err(|_| pending("subscription reconciliation timed out; retry required"))?
}

/// An invoice outcome never establishes subscription status. Recover the exact
/// provider resource under the same snapshot/lock protocol as subscription events.
pub(super) async fn reconcile_invoice(
    pool: &PgPool,
    config: &Config,
    event: &EventContext,
    invoice: &super::stripe_webhook::StripeInvoice,
) -> Result<Option<Applied>, WebhookError> {
    let id = invoice.subscription_id()?;
    let customer = invoice.customer.as_ref().ok_or_else(|| pending("invoice customer missing"))?;
    let record = Subscription::find_by_stripe_subscription_id(pool, id)
        .await?
        .ok_or_else(|| pending("invoice subscription prerequisite missing"))?;
    if record.stripe_customer_id != customer.id() {
        return Err(WebhookError::Rejected(
            "invoice customer differs from subscription owner".into(),
        ));
    }
    let subscription = StripeSubscription {
        id: id.to_owned(),
        customer: super::stripe_webhook::StripeRef::Id(customer.id().to_owned()),
        created: None,
        livemode: None,
        status: record.status.clone(),
        items: Default::default(),
        current_period_start: None,
        current_period_end: None,
        cancel_at_period_end: false,
        canceled_at: None,
        trial_end: None,
        metadata: std::collections::HashMap::from([("org_id".into(), record.org_id.to_string())]),
    };
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        apply_inner(pool, config, event, subscription, false, true),
    )
    .await
    .map_err(|_| pending("invoice reconciliation timed out; retry required"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn tied_legacy_and_previously_reconciled_snapshots_require_readback() {
        let subscription: StripeSubscription = serde_json::from_value(serde_json::json!({
            "id":"sub_example", "customer":"cus_example", "status":"active", "created":50,"livemode":false
        })).unwrap();
        let mut row = Lineage {
            stripe_subscription_id: "sub_example".into(),
            stripe_customer_id: "cus_example".into(),
            stripe_created_at: Some(50),
            last_stripe_event_created: Some(100),
            last_stripe_event_id: Some("evt_a".into()),
            stripe_snapshot_authoritative: false,
            status: "active".into(),
            updated_at: Utc::now(),
            revision: "1".into(),
        };
        assert!(!requires_readback(None, 100, "evt_a", &subscription));
        assert!(!requires_readback(Some(&row), 101, "evt_b", &subscription));
        assert!(requires_readback(Some(&row), 100, "evt_b", &subscription));
        row.last_stripe_event_created = None;
        assert!(requires_readback(Some(&row), 101, "evt_b", &subscription));
        row.last_stripe_event_created = Some(100);
        row.stripe_snapshot_authoritative = true;
        for created in [99, 100, 101] {
            assert!(requires_readback(Some(&row), created, "evt_b", &subscription));
        }
    }

    #[tokio::test]
    async fn authenticated_readback_uses_narrow_subscription_shape() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let org = Uuid::new_v4();
        let body = serde_json::json!({
            "id":"sub_example", "customer":"cus_example", "status":"active",
            "created":1700000000, "livemode":false, "metadata":{"org_id":org},
            "items":{"data":[]}, "future_clover_field":{"ignored":true}
        })
        .to_string();
        let server = async {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                let mut buffer = [0; 1024];
                let length = stream.read(&mut buffer).await.unwrap();
                assert!(length > 0 && request.len() < 8192);
                request.extend_from_slice(&buffer[..length]);
            }
            let request = String::from_utf8(request).unwrap().to_ascii_lowercase();
            assert!(request.starts_with("get /v1/subscriptions/sub_example "));
            assert!(request.contains("authorization: bearer sk_test_fixture"));
            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body);
            stream.write_all(response.as_bytes()).await.unwrap();
        };
        let client =
            stripe::Client::from_url(format!("http://{address}/").as_str(), "sk_test_fixture");
        let request = async {
            let recovered = readback(&client, "sub_example").await.unwrap();
            identity(&recovered, "sub_example", "cus_example", org, false).unwrap();
            assert!(identity(&recovered, "sub_other", "cus_example", org, false).is_err());
            assert!(identity(&recovered, "sub_example", "cus_other", org, false).is_err());
            assert!(
                identity(&recovered, "sub_example", "cus_example", Uuid::new_v4(), false).is_err()
            );
            assert!(identity(&recovered, "sub_example", "cus_example", org, true).is_err());
        };
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            tokio::join!(server, request);
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn readback_rejects_paths_before_contacting_transport() {
        let client = stripe::Client::from_url("http://127.0.0.1:1/", "sk_test_fixture");
        for id in [
            "https://elsewhere.test",
            "sub_a/../customers",
            "sub_a?expand=secret",
        ] {
            assert!(matches!(readback(&client, id).await, Err(WebhookError::Pending(_))));
        }
    }
}
