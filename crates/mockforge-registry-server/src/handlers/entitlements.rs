//! Subscription-aware entitlement gating (#870).
//!
//! Feature gates across the registry (SSO, AI, admin-role assignment,
//! protocol selection) historically trusted `org.plan()` alone. But `plan`
//! is only flipped by Stripe webhooks (`handlers::billing`). If a
//! `customer.subscription.deleted` / `.updated` webhook is missed, dropped,
//! or delayed, a canceled or past-due org would keep its paid tier
//! *indefinitely* with no reconciliation.
//!
//! [`effective_plan`] is the single chokepoint that closes that gap: it
//! returns the org's stored plan only when the org has an *active* (or
//! *trialing*) subscription, honoring the same 24h past-due grace window
//! that hosted-mock deployment already uses. Otherwise it downgrades the
//! effective plan to [`Plan::Free`] for gating purposes — without mutating
//! the stored `plan` column (that stays the webhook/reconciliation job's
//! responsibility).
//!
//! Gates keep their existing `plan()`-based check and add this on top, so a
//! plan that *was* always free stays free and a paid plan that lost its
//! subscription loses its paid features.

use chrono::Utc;
use uuid::Uuid;

use crate::error::ApiResult;
use crate::models::{Organization, Plan, Subscription, SubscriptionStatus};
use crate::AppState;

/// Past-due grace window. Mirrors the constant in
/// [`crate::handlers::hosted_mocks::create_deployment`] (#449): a `past_due`
/// subscription keeps access for 24h after the status flip so a transient
/// card-network blip during Stripe's smart-retry sequence doesn't cut a
/// paying customer off mid-flight.
pub const PAST_DUE_GRACE_SECONDS: i64 = 24 * 60 * 60;

/// Resolve the *effective* plan to use for entitlement gating.
///
/// Rules:
/// * No subscription row at all → trust the stored plan. Orgs are upgraded
///   administratively (manual `update_plan`, seeded admin orgs, legacy
///   imports) without a Stripe subscription, and the #870 threat model is a
///   subscription that *exists* but wasn't reconciled. An org that was never
///   billed through Stripe has no dropped-webhook risk, so we don't punish it
///   (and this keeps the e2e suite — which seeds `Plan::Team` orgs directly —
///   green). A warning is logged when a *paid* plan has no subscription so the
///   anomaly is observable.
/// * Subscription `active` / `trialing` → trust the stored plan (trials MUST
///   keep their paid plan — do not break the 14-day trial).
/// * Subscription `past_due` → trust the stored plan only within the 24h
///   grace window (mirrors hosted_mocks); past the window, downgrade to Free.
/// * Any other status (`canceled`, `unpaid`, `incomplete`,
///   `incomplete_expired`) → downgrade to [`Plan::Free`] for gating.
///
/// Never mutates the stored `plan` column.
pub async fn effective_plan(state: &AppState, org: &Organization) -> ApiResult<Plan> {
    let stored = org.plan();
    let sub = Subscription::find_by_org(state.db.pool(), org.id).await?;
    Ok(resolve_effective_plan(stored, sub.as_ref(), Utc::now()))
}

/// Pure decision core for [`effective_plan`], split out so it's unit-testable
/// without a database. `now` is injected for deterministic grace-window tests.
fn resolve_effective_plan(
    stored: Plan,
    sub: Option<&Subscription>,
    now: chrono::DateTime<Utc>,
) -> Plan {
    // Free is never elevated by subscription state, so short-circuit.
    if stored == Plan::Free {
        return Plan::Free;
    }

    let Some(sub) = sub else {
        // Paid plan, no subscription row. See doc comment: trust it but log.
        tracing::warn!(
            org_plan = %stored,
            "effective_plan: paid plan has no subscription row; trusting stored plan (legacy/manual/seed org)"
        );
        return stored;
    };

    match sub.status() {
        SubscriptionStatus::Active | SubscriptionStatus::Trialing => stored,
        SubscriptionStatus::PastDue => {
            let elapsed = (now - sub.updated_at).num_seconds();
            if elapsed > PAST_DUE_GRACE_SECONDS {
                tracing::info!(
                    org_plan = %stored,
                    past_due_seconds = elapsed,
                    "effective_plan: past_due beyond grace; downgrading to Free for gating"
                );
                Plan::Free
            } else {
                stored
            }
        }
        other => {
            tracing::info!(
                org_plan = %stored,
                status = %other,
                "effective_plan: inactive subscription; downgrading to Free for gating"
            );
            Plan::Free
        }
    }
}

/// Convenience wrapper for gates that only have an `org_id` + stored plan.
/// Loads the org, then delegates to [`effective_plan`]. Most call sites
/// already hold the `Organization`, so [`effective_plan`] is preferred.
pub async fn effective_plan_for_org_id(state: &AppState, org_id: Uuid) -> ApiResult<Plan> {
    let org = Organization::find_by_id(state.db.pool(), org_id)
        .await?
        .ok_or_else(|| crate::error::ApiError::InvalidRequest("Organization not found".into()))?;
    effective_plan(state, &org).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn sub_with(status: SubscriptionStatus, updated_at: chrono::DateTime<Utc>) -> Subscription {
        Subscription {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            stripe_subscription_id: "sub_test".into(),
            stripe_customer_id: "cus_test".into(),
            price_id: "price_test".into(),
            plan: "team".into(),
            status: status.to_string(),
            current_period_start: Utc::now(),
            current_period_end: Utc::now(),
            cancel_at_period_end: false,
            canceled_at: None,
            created_at: Utc::now(),
            updated_at,
        }
    }

    #[test]
    fn free_stays_free_regardless_of_subscription() {
        let now = Utc::now();
        assert_eq!(resolve_effective_plan(Plan::Free, None, now), Plan::Free);
        let sub = sub_with(SubscriptionStatus::Active, now);
        assert_eq!(resolve_effective_plan(Plan::Free, Some(&sub), now), Plan::Free);
    }

    #[test]
    fn paid_with_no_subscription_trusts_stored_plan() {
        let now = Utc::now();
        // Legacy/manual/seed org: keep the paid plan (don't break e2e seeds).
        assert_eq!(resolve_effective_plan(Plan::Team, None, now), Plan::Team);
        assert_eq!(resolve_effective_plan(Plan::Pro, None, now), Plan::Pro);
    }

    #[test]
    fn active_subscription_keeps_paid_plan() {
        let now = Utc::now();
        let sub = sub_with(SubscriptionStatus::Active, now);
        assert_eq!(resolve_effective_plan(Plan::Team, Some(&sub), now), Plan::Team);
    }

    #[test]
    fn trialing_subscription_keeps_paid_plan() {
        // CRITICAL: trials MUST not be downgraded (14-day trial #870).
        let now = Utc::now();
        let sub = sub_with(SubscriptionStatus::Trialing, now);
        assert_eq!(resolve_effective_plan(Plan::Pro, Some(&sub), now), Plan::Pro);
    }

    #[test]
    fn canceled_subscription_downgrades_to_free() {
        let now = Utc::now();
        let sub = sub_with(SubscriptionStatus::Canceled, now);
        assert_eq!(resolve_effective_plan(Plan::Team, Some(&sub), now), Plan::Free);
    }

    #[test]
    fn unpaid_subscription_downgrades_to_free() {
        let now = Utc::now();
        let sub = sub_with(SubscriptionStatus::Unpaid, now);
        assert_eq!(resolve_effective_plan(Plan::Team, Some(&sub), now), Plan::Free);
    }

    #[test]
    fn incomplete_expired_subscription_downgrades_to_free() {
        let now = Utc::now();
        let sub = sub_with(SubscriptionStatus::IncompleteExpired, now);
        assert_eq!(resolve_effective_plan(Plan::Pro, Some(&sub), now), Plan::Free);
    }

    #[test]
    fn past_due_within_grace_keeps_paid_plan() {
        let now = Utc::now();
        // Flipped to past_due 1h ago — inside the 24h grace window.
        let sub = sub_with(SubscriptionStatus::PastDue, now - Duration::hours(1));
        assert_eq!(resolve_effective_plan(Plan::Team, Some(&sub), now), Plan::Team);
    }

    #[test]
    fn past_due_beyond_grace_downgrades_to_free() {
        let now = Utc::now();
        // Flipped to past_due 25h ago — outside the 24h grace window.
        let sub = sub_with(SubscriptionStatus::PastDue, now - Duration::hours(25));
        assert_eq!(resolve_effective_plan(Plan::Team, Some(&sub), now), Plan::Free);
    }
}
