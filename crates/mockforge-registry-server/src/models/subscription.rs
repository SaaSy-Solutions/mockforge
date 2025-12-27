//! Subscription and billing models

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::Plan;

/// Subscription status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active,
    Trialing,
    PastDue,
    Canceled,
    Unpaid,
    Incomplete,
    IncompleteExpired,
}

impl SubscriptionStatus {
    pub fn to_string(&self) -> String {
        match self {
            SubscriptionStatus::Active => "active".to_string(),
            SubscriptionStatus::Trialing => "trialing".to_string(),
            SubscriptionStatus::PastDue => "past_due".to_string(),
            SubscriptionStatus::Canceled => "canceled".to_string(),
            SubscriptionStatus::Unpaid => "unpaid".to_string(),
            SubscriptionStatus::Incomplete => "incomplete".to_string(),
            SubscriptionStatus::IncompleteExpired => "incomplete_expired".to_string(),
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "active" => SubscriptionStatus::Active,
            "trialing" => SubscriptionStatus::Trialing,
            "past_due" => SubscriptionStatus::PastDue,
            "canceled" => SubscriptionStatus::Canceled,
            "unpaid" => SubscriptionStatus::Unpaid,
            "incomplete" => SubscriptionStatus::Incomplete,
            "incomplete_expired" => SubscriptionStatus::IncompleteExpired,
            _ => SubscriptionStatus::Canceled,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, SubscriptionStatus::Active | SubscriptionStatus::Trialing)
    }
}

/// Subscription model
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Subscription {
    pub id: Uuid,
    pub org_id: Uuid,
    pub stripe_subscription_id: String,
    pub stripe_customer_id: String,
    pub price_id: String,
    pub plan: String,   // Stored as VARCHAR, converted via methods
    pub status: String, // Stored as VARCHAR, converted via methods
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub cancel_at_period_end: bool,
    pub canceled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Subscription {
    /// Get plan as enum
    pub fn plan(&self) -> Plan {
        match self.plan.as_str() {
            "free" => Plan::Free,
            "pro" => Plan::Pro,
            "team" => Plan::Team,
            _ => Plan::Free,
        }
    }

    /// Get status as enum
    pub fn status(&self) -> SubscriptionStatus {
        SubscriptionStatus::from_string(&self.status)
    }

    /// Create or update subscription from Stripe webhook
    pub async fn upsert_from_stripe(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        stripe_subscription_id: &str,
        stripe_customer_id: &str,
        price_id: &str,
        plan: Plan,
        status: SubscriptionStatus,
        current_period_start: DateTime<Utc>,
        current_period_end: DateTime<Utc>,
        cancel_at_period_end: bool,
        canceled_at: Option<DateTime<Utc>>,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO subscriptions (
                org_id, stripe_subscription_id, stripe_customer_id, price_id,
                plan, status, current_period_start, current_period_end,
                cancel_at_period_end, canceled_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (stripe_subscription_id) DO UPDATE SET
                org_id = EXCLUDED.org_id,
                stripe_customer_id = EXCLUDED.stripe_customer_id,
                price_id = EXCLUDED.price_id,
                plan = EXCLUDED.plan,
                status = EXCLUDED.status,
                current_period_start = EXCLUDED.current_period_start,
                current_period_end = EXCLUDED.current_period_end,
                cancel_at_period_end = EXCLUDED.cancel_at_period_end,
                canceled_at = EXCLUDED.canceled_at,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(stripe_subscription_id)
        .bind(stripe_customer_id)
        .bind(price_id)
        .bind(plan.to_string())
        .bind(status.to_string())
        .bind(current_period_start)
        .bind(current_period_end)
        .bind(cancel_at_period_end)
        .bind(canceled_at)
        .fetch_one(pool)
        .await
    }

    /// Find subscription by org_id
    pub async fn find_by_org(pool: &sqlx::PgPool, org_id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM subscriptions WHERE org_id = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(org_id)
        .fetch_optional(pool)
        .await
    }

    /// Find subscription by Stripe subscription ID
    pub async fn find_by_stripe_subscription_id(
        pool: &sqlx::PgPool,
        stripe_subscription_id: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM subscriptions WHERE stripe_subscription_id = $1")
            .bind(stripe_subscription_id)
            .fetch_optional(pool)
            .await
    }

    /// Update subscription status
    pub async fn update_status(
        pool: &sqlx::PgPool,
        subscription_id: Uuid,
        status: SubscriptionStatus,
    ) -> sqlx::Result<()> {
        sqlx::query("UPDATE subscriptions SET status = $1, updated_at = NOW() WHERE id = $2")
            .bind(status.to_string())
            .bind(subscription_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Cancel subscription (mark for cancellation at period end)
    pub async fn cancel_at_period_end(
        pool: &sqlx::PgPool,
        subscription_id: Uuid,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            UPDATE subscriptions
            SET cancel_at_period_end = TRUE, canceled_at = NOW(), updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(subscription_id)
        .execute(pool)
        .await?;

        Ok(())
    }
}

/// Usage counter for monthly tracking
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UsageCounter {
    pub id: Uuid,
    pub org_id: Uuid,
    pub period_start: NaiveDate,
    pub requests: i64,
    pub egress_bytes: i64,
    pub storage_bytes: i64,
    pub ai_tokens_used: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UsageCounter {
    /// Get or create usage counter for current month
    pub async fn get_or_create_current(pool: &sqlx::PgPool, org_id: Uuid) -> sqlx::Result<Self> {
        let period_start = chrono::Utc::now().date_naive();
        let period_start = NaiveDate::from_ymd_opt(period_start.year(), period_start.month(), 1)
            .unwrap_or(period_start);

        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO usage_counters (org_id, period_start)
            VALUES ($1, $2)
            ON CONFLICT (org_id, period_start) DO UPDATE SET
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(period_start)
        .fetch_one(pool)
        .await
    }

    /// Increment request count
    pub async fn increment_requests(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        count: i64,
    ) -> sqlx::Result<()> {
        let counter = Self::get_or_create_current(pool, org_id).await?;

        sqlx::query(
            "UPDATE usage_counters SET requests = requests + $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(count)
        .bind(counter.id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Increment egress bytes
    pub async fn increment_egress(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        bytes: i64,
    ) -> sqlx::Result<()> {
        let counter = Self::get_or_create_current(pool, org_id).await?;

        sqlx::query(
            "UPDATE usage_counters SET egress_bytes = egress_bytes + $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(bytes)
        .bind(counter.id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Update storage bytes (absolute value, not increment)
    pub async fn update_storage(pool: &sqlx::PgPool, org_id: Uuid, bytes: i64) -> sqlx::Result<()> {
        let counter = Self::get_or_create_current(pool, org_id).await?;

        sqlx::query(
            "UPDATE usage_counters SET storage_bytes = $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(bytes)
        .bind(counter.id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Increment AI tokens used
    pub async fn increment_ai_tokens(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        tokens: i64,
    ) -> sqlx::Result<()> {
        let counter = Self::get_or_create_current(pool, org_id).await?;

        sqlx::query(
            "UPDATE usage_counters SET ai_tokens_used = ai_tokens_used + $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(tokens)
        .bind(counter.id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get usage for a specific period
    pub async fn get_for_period(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        period_start: NaiveDate,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM usage_counters WHERE org_id = $1 AND period_start = $2",
        )
        .bind(org_id)
        .bind(period_start)
        .fetch_optional(pool)
        .await
    }

    /// Get all usage counters for an org
    pub async fn get_all_for_org(pool: &sqlx::PgPool, org_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM usage_counters WHERE org_id = $1 ORDER BY period_start DESC",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_status_to_string() {
        assert_eq!(SubscriptionStatus::Active.to_string(), "active");
        assert_eq!(SubscriptionStatus::Trialing.to_string(), "trialing");
        assert_eq!(SubscriptionStatus::PastDue.to_string(), "past_due");
        assert_eq!(SubscriptionStatus::Canceled.to_string(), "canceled");
        assert_eq!(SubscriptionStatus::Unpaid.to_string(), "unpaid");
        assert_eq!(SubscriptionStatus::Incomplete.to_string(), "incomplete");
        assert_eq!(SubscriptionStatus::IncompleteExpired.to_string(), "incomplete_expired");
    }

    #[test]
    fn test_subscription_status_from_string() {
        assert_eq!(SubscriptionStatus::from_string("active"), SubscriptionStatus::Active);
        assert_eq!(SubscriptionStatus::from_string("trialing"), SubscriptionStatus::Trialing);
        assert_eq!(SubscriptionStatus::from_string("past_due"), SubscriptionStatus::PastDue);
        assert_eq!(SubscriptionStatus::from_string("canceled"), SubscriptionStatus::Canceled);
        assert_eq!(SubscriptionStatus::from_string("unpaid"), SubscriptionStatus::Unpaid);
        assert_eq!(SubscriptionStatus::from_string("incomplete"), SubscriptionStatus::Incomplete);
        assert_eq!(
            SubscriptionStatus::from_string("incomplete_expired"),
            SubscriptionStatus::IncompleteExpired
        );

        // Unknown status should default to Canceled
        assert_eq!(SubscriptionStatus::from_string("unknown"), SubscriptionStatus::Canceled);
        assert_eq!(SubscriptionStatus::from_string(""), SubscriptionStatus::Canceled);
    }

    #[test]
    fn test_subscription_status_is_active() {
        assert!(SubscriptionStatus::Active.is_active());
        assert!(SubscriptionStatus::Trialing.is_active());
        assert!(!SubscriptionStatus::PastDue.is_active());
        assert!(!SubscriptionStatus::Canceled.is_active());
        assert!(!SubscriptionStatus::Unpaid.is_active());
        assert!(!SubscriptionStatus::Incomplete.is_active());
        assert!(!SubscriptionStatus::IncompleteExpired.is_active());
    }

    #[test]
    fn test_subscription_status_serialization() {
        let status = SubscriptionStatus::Active;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"active\"");

        let status = SubscriptionStatus::PastDue;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"past_due\"");
    }

    #[test]
    fn test_subscription_status_deserialization() {
        let status: SubscriptionStatus = serde_json::from_str("\"active\"").unwrap();
        assert_eq!(status, SubscriptionStatus::Active);

        let status: SubscriptionStatus = serde_json::from_str("\"trialing\"").unwrap();
        assert_eq!(status, SubscriptionStatus::Trialing);

        let status: SubscriptionStatus = serde_json::from_str("\"past_due\"").unwrap();
        assert_eq!(status, SubscriptionStatus::PastDue);
    }

    #[test]
    fn test_subscription_status_equality() {
        assert_eq!(SubscriptionStatus::Active, SubscriptionStatus::Active);
        assert_ne!(SubscriptionStatus::Active, SubscriptionStatus::Canceled);
    }

    #[test]
    fn test_subscription_status_copy_and_clone() {
        let status1 = SubscriptionStatus::Active;
        let status2 = status1;
        let status3 = status1.clone();

        assert_eq!(status1, status2);
        assert_eq!(status1, status3);
    }

    #[test]
    fn test_subscription_plan_method() {
        let subscription = Subscription {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            stripe_subscription_id: "sub_123".to_string(),
            stripe_customer_id: "cus_123".to_string(),
            price_id: "price_123".to_string(),
            plan: "free".to_string(),
            status: "active".to_string(),
            current_period_start: Utc::now(),
            current_period_end: Utc::now(),
            cancel_at_period_end: false,
            canceled_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(subscription.plan(), Plan::Free);

        let subscription = Subscription {
            plan: "pro".to_string(),
            ..subscription
        };
        assert_eq!(subscription.plan(), Plan::Pro);

        let subscription = Subscription {
            plan: "team".to_string(),
            ..subscription
        };
        assert_eq!(subscription.plan(), Plan::Team);

        // Invalid plan should default to Free
        let subscription = Subscription {
            plan: "invalid".to_string(),
            ..subscription
        };
        assert_eq!(subscription.plan(), Plan::Free);
    }

    #[test]
    fn test_subscription_status_method() {
        let subscription = Subscription {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            stripe_subscription_id: "sub_123".to_string(),
            stripe_customer_id: "cus_123".to_string(),
            price_id: "price_123".to_string(),
            plan: "pro".to_string(),
            status: "active".to_string(),
            current_period_start: Utc::now(),
            current_period_end: Utc::now(),
            cancel_at_period_end: false,
            canceled_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(subscription.status(), SubscriptionStatus::Active);

        let subscription = Subscription {
            status: "canceled".to_string(),
            ..subscription
        };
        assert_eq!(subscription.status(), SubscriptionStatus::Canceled);
    }

    #[test]
    fn test_subscription_serialization() {
        let subscription = Subscription {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            stripe_subscription_id: "sub_123".to_string(),
            stripe_customer_id: "cus_123".to_string(),
            price_id: "price_123".to_string(),
            plan: "pro".to_string(),
            status: "active".to_string(),
            current_period_start: Utc::now(),
            current_period_end: Utc::now(),
            cancel_at_period_end: false,
            canceled_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&subscription).unwrap();
        assert!(json.contains("sub_123"));
        assert!(json.contains("cus_123"));
        assert!(json.contains("price_123"));
        assert!(json.contains("pro"));
        assert!(json.contains("active"));
    }

    #[test]
    fn test_usage_counter_serialization() {
        let usage = UsageCounter {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            period_start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            requests: 1000,
            egress_bytes: 50000,
            storage_bytes: 10000,
            ai_tokens_used: 5000,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&usage).unwrap();
        assert!(json.contains("1000"));
        assert!(json.contains("50000"));
        assert!(json.contains("10000"));
        assert!(json.contains("5000"));
    }

    #[test]
    fn test_usage_counter_clone() {
        let usage = UsageCounter {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            period_start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            requests: 1000,
            egress_bytes: 50000,
            storage_bytes: 10000,
            ai_tokens_used: 5000,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let cloned = usage.clone();
        assert_eq!(usage.id, cloned.id);
        assert_eq!(usage.requests, cloned.requests);
        assert_eq!(usage.egress_bytes, cloned.egress_bytes);
        assert_eq!(usage.storage_bytes, cloned.storage_bytes);
        assert_eq!(usage.ai_tokens_used, cloned.ai_tokens_used);
    }

    #[test]
    fn test_subscription_clone() {
        let subscription = Subscription {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            stripe_subscription_id: "sub_123".to_string(),
            stripe_customer_id: "cus_123".to_string(),
            price_id: "price_123".to_string(),
            plan: "pro".to_string(),
            status: "active".to_string(),
            current_period_start: Utc::now(),
            current_period_end: Utc::now(),
            cancel_at_period_end: false,
            canceled_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let cloned = subscription.clone();
        assert_eq!(subscription.id, cloned.id);
        assert_eq!(subscription.stripe_subscription_id, cloned.stripe_subscription_id);
        assert_eq!(subscription.cancel_at_period_end, cloned.cancel_at_period_end);
    }

    #[test]
    fn test_subscription_with_cancellation() {
        let now = Utc::now();
        let subscription = Subscription {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            stripe_subscription_id: "sub_123".to_string(),
            stripe_customer_id: "cus_123".to_string(),
            price_id: "price_123".to_string(),
            plan: "pro".to_string(),
            status: "active".to_string(),
            current_period_start: now,
            current_period_end: now,
            cancel_at_period_end: true,
            canceled_at: Some(now),
            created_at: now,
            updated_at: now,
        };

        assert!(subscription.cancel_at_period_end);
        assert!(subscription.canceled_at.is_some());
        assert_eq!(subscription.status(), SubscriptionStatus::Active);
    }
}
