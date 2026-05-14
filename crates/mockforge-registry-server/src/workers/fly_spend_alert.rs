//! Background worker that flags orgs whose **estimated** Fly.io compute spend
//! on hosted-mock deployments exceeds 2× their plan price for the current
//! billing period (#450 acceptance criterion 3).
//!
//! ## Why estimated, not invoiced
//!
//! Fly.io does not currently expose a live billing endpoint we can poll, and
//! they don't emit webhooks per-customer. The closest signal we have is the
//! `hosted_mocks` table — `created_at`, `instance_type`, and `status` are
//! enough to compute machine-hours-per-period and multiply by published
//! shared-cpu hourly rates. We deliberately ignore egress, volume storage,
//! and IPv4 allocations: the spend ceiling this PR is gating against is the
//! "left a Pro customer's 3 mocks running 24/7 → \$333 Fly bill on a \$29
//! plan" scenario from the issue body, which is overwhelmingly compute.
//!
//! What this worker is NOT:
//!   - It is not the source of truth for billing reconciliation.
//!   - It is not a hard cap (those are in `router.rs` via [[#450 cost ceiling]]).
//!   - It does not stop deployments — Fly suspension is out of scope here.
//!
//! What it IS:
//!   - A daily idempotent email to the org owner the *first* time we see the
//!     compute-cost projection cross 2× plan price for a given calendar month.
//!   - A signal for ops to investigate before the actual Fly invoice arrives.
//!
//! Idempotency is via `usage_alerts(org_id, metric, period_start, threshold_pct)`
//! UNIQUE — `try_insert` returns `None` for the second-and-later run of the
//! same calendar month, so re-running the worker on any cadence is safe.

use std::time::Duration;

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    email::EmailService,
    models::{DeploymentStatus, HostedMock, Organization, Plan, UsageAlert, User},
    AppState,
};

const DEFAULT_INTERVAL_SECS: u64 = 24 * 60 * 60; // daily
const THRESHOLD_PCT: i16 = 200; // 2× plan price → alert at $58 (Pro) / $198 (Team)
const METRIC: &str = "fly_compute_spend";

/// Start the Fly spend alert worker. Interval can be overridden by the
/// `FLY_SPEND_ALERT_INTERVAL_SECS` env var; falls back to 24h.
pub fn start_fly_spend_alert_worker(state: AppState) {
    let interval_secs = std::env::var("FLY_SPEND_ALERT_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| *n >= 60)
        .unwrap_or(DEFAULT_INTERVAL_SECS);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        // Skip the immediate first tick so we don't bombard a fresh boot.
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(e) = run_once(&state).await {
                error!("fly_spend_alert: scan failed: {e:?}");
            }
        }
    });

    info!("Fly spend alert worker started (every {interval_secs}s)");
}

/// One pass over every org that has at least one hosted-mock row. Inserts
/// `usage_alerts` rows for newly-crossed thresholds and emails the org owner.
/// Returns the count of newly-emitted alerts for log/test introspection.
pub async fn run_once(state: &AppState) -> Result<usize, sqlx::Error> {
    let pool = state.db.pool();
    let period_start_date = current_period_start();
    let period_start = match period_start_date.and_hms_opt(0, 0, 0) {
        Some(naive) => DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc),
        None => {
            warn!("fly_spend_alert: could not derive period start datetime");
            return Ok(0);
        }
    };
    let now = Utc::now();

    // Only orgs that actually have a deployment — keeps the scan O(active
    // tenants), not O(every org we ever created).
    let org_ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT DISTINCT org_id FROM hosted_mocks WHERE deleted_at IS NULL")
            .fetch_all(pool)
            .await?;
    debug!("fly_spend_alert: scanning {} orgs", org_ids.len());

    let mut emitted = 0usize;
    for org_id in org_ids {
        match check_org(state, org_id, period_start_date, period_start, now).await {
            Ok(true) => emitted += 1,
            Ok(false) => {}
            Err(e) => {
                warn!(org_id = %org_id, "fly_spend_alert: per-org check failed: {e:?}");
            }
        }
    }
    if emitted > 0 {
        info!(
            "fly_spend_alert: emitted {} new alerts for period {}",
            emitted, period_start_date
        );
    }
    Ok(emitted)
}

async fn check_org(
    state: &AppState,
    org_id: Uuid,
    period_start_date: NaiveDate,
    period_start: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    let pool = state.db.pool();
    let Some(org) = Organization::find_by_id(pool, org_id).await? else {
        return Ok(false);
    };

    // Free has no compute headroom — the body cap + RPS gate in router.rs
    // is the only protection that applies. No alerting target here.
    let Some(plan_cents) = plan_price_cents(org.plan()) else {
        return Ok(false);
    };
    let threshold_cents = plan_cents.saturating_mul(2);

    let deployments = state
        .store
        .list_hosted_mocks_by_org(org_id)
        .await
        .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
    let spend_cents = estimate_org_spend_cents(&deployments, period_start, now);
    if spend_cents < threshold_cents {
        return Ok(false);
    }

    // try_insert returns None if a row already exists for this period — the
    // worker can re-run daily without re-emailing.
    let inserted =
        UsageAlert::try_insert(pool, org.id, METRIC, period_start_date, THRESHOLD_PCT).await?;
    if inserted.is_none() {
        return Ok(false);
    }

    send_email(state, &org, plan_cents, spend_cents).await;
    info!(
        org_id = %org.id,
        plan = %org.plan().to_string(),
        spend_cents = spend_cents,
        threshold_cents = threshold_cents,
        "fly_spend_alert: new alert"
    );
    Ok(true)
}

async fn send_email(state: &AppState, org: &Organization, plan_cents: i64, spend_cents: i64) {
    let _ = state;
    let pool_owner = match User::find_by_id(state.db.pool(), org.owner_id).await {
        Ok(Some(u)) if u.email_notifications => u,
        Ok(_) => return,
        Err(e) => {
            warn!(org_id = %org.id, "fly_spend_alert: owner lookup failed: {e:?}");
            return;
        }
    };

    let plan = org.plan().to_string();
    let owner_email = pool_owner.email;
    let username = pool_owner.username;
    let used_pretty = format_dollars(spend_cents);
    let limit_pretty = format_dollars(plan_cents.saturating_mul(2));

    tokio::spawn(async move {
        let email_service = match EmailService::from_env() {
            Ok(s) => s,
            Err(e) => {
                debug!("fly_spend_alert email skipped — service init failed: {e:?}");
                return;
            }
        };
        let msg = EmailService::generate_usage_threshold_warning(
            &username,
            &owner_email,
            "Fly compute spend",
            &plan,
            &used_pretty,
            &limit_pretty,
            THRESHOLD_PCT as u16,
        );
        if let Err(e) = email_service.send(msg).await {
            warn!("fly_spend_alert email send failed: {e:?}");
        }
    });
}

/// Sum estimated compute spend in cents across the org's deployments since
/// the start of the current billing period. Pure for unit-testability.
///
/// Hours-since-start uses `created_at.max(period_start)` so a deployment that
/// was created on Jan 5 only contributes Jan-5-onwards spend toward the
/// January threshold, not its full lifetime.
fn estimate_org_spend_cents(
    deployments: &[HostedMock],
    period_start: DateTime<Utc>,
    now: DateTime<Utc>,
) -> i64 {
    let mut total: i64 = 0;
    for d in deployments {
        if d.deleted_at.is_some() {
            continue;
        }
        // Only states that meaningfully burn compute. `Stopped` machines do
        // not accrue compute on Fly's billing model. `Failed` machines also
        // don't run, but we keep counting `Deploying` since a stuck deploy
        // can hold a hot rootfs for hours.
        match d.status() {
            DeploymentStatus::Active | DeploymentStatus::Deploying => {}
            DeploymentStatus::Pending
            | DeploymentStatus::Stopped
            | DeploymentStatus::Failed
            | DeploymentStatus::Deleting => continue,
        }
        let start = d.created_at.max(period_start);
        if start >= now {
            continue;
        }
        let hours = (now - start).num_seconds() / 3600;
        if hours <= 0 {
            continue;
        }
        total = total.saturating_add(hours.saturating_mul(hourly_rate_cents(&d.instance_type)));
    }
    total
}

/// Published Fly.io hourly rates in **cents**, rounded up to the next cent
/// to err on the side of paging. Unknown instance types default to the
/// `shared-cpu-1x` baseline rate so a renamed/new sku doesn't silently
/// suppress alerts. Numbers as of 2026; document expected refresh in
/// `[[reference_fly_pricing]]` if Fly publishes a rate change.
fn hourly_rate_cents(instance_type: &str) -> i64 {
    match instance_type {
        "shared-cpu-1x" => 16,
        "shared-cpu-2x" => 31,
        "shared-cpu-4x" => 61,
        "shared-cpu-8x" => 122,
        "performance-1x" => 366,
        "performance-2x" => 731,
        "performance-4x" => 1462,
        "performance-8x" => 2925,
        _ => 16,
    }
}

/// Per-month plan-price ceiling in cents. `None` for Free — that plan has
/// `max_hosted_mocks: 0`, so there's no compute headroom to alert on.
fn plan_price_cents(plan: Plan) -> Option<i64> {
    match plan {
        Plan::Free => None,
        Plan::Pro => Some(2900),  // $29/mo
        Plan::Team => Some(9900), // $99/mo
    }
}

fn current_period_start() -> NaiveDate {
    let today = Utc::now().date_naive();
    NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today)
}

fn format_dollars(cents: i64) -> String {
    let dollars = cents / 100;
    let frac = (cents % 100).abs();
    format!("${}.{:02}", dollars, frac)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn mk_deployment(
        instance_type: &str,
        created_at: DateTime<Utc>,
        status: &str,
        deleted_at: Option<DateTime<Utc>>,
    ) -> HostedMock {
        HostedMock {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            project_id: None,
            name: "t".to_string(),
            slug: "t".to_string(),
            description: None,
            config_json: serde_json::Value::Null,
            openapi_spec_url: None,
            status: status.to_string(),
            deployment_url: None,
            internal_url: None,
            region: "iad".to_string(),
            instance_type: instance_type.to_string(),
            health_check_url: None,
            last_health_check: None,
            health_status: "unknown".to_string(),
            error_message: None,
            metadata_json: serde_json::Value::Null,
            created_at,
            updated_at: created_at,
            deleted_at,
        }
    }

    #[test]
    fn plan_price_pro_team_only() {
        assert_eq!(plan_price_cents(Plan::Free), None);
        assert_eq!(plan_price_cents(Plan::Pro), Some(2900));
        assert_eq!(plan_price_cents(Plan::Team), Some(9900));
    }

    #[test]
    fn hourly_rate_defaults_to_shared_cpu_1x_for_unknown_types() {
        assert_eq!(hourly_rate_cents("shared-cpu-1x"), 16);
        // A future Fly sku we haven't mapped yet still gets the baseline rate,
        // so spend isn't silently zeroed-out.
        assert_eq!(hourly_rate_cents("future-fly-sku-99x"), 16);
    }

    #[test]
    fn hourly_rate_climbs_with_class() {
        assert!(hourly_rate_cents("shared-cpu-2x") > hourly_rate_cents("shared-cpu-1x"));
        assert!(hourly_rate_cents("performance-1x") > hourly_rate_cents("shared-cpu-8x"));
    }

    #[test]
    fn estimate_skips_stopped_and_deleted_deployments() {
        let period = Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap();
        let now = period + chrono::Duration::hours(100);
        let active = mk_deployment("shared-cpu-1x", period, "active", None);
        let stopped = mk_deployment("shared-cpu-1x", period, "stopped", None);
        let deleted = mk_deployment("shared-cpu-1x", period, "active", Some(now));
        let failed = mk_deployment("shared-cpu-1x", period, "failed", None);

        let spend = estimate_org_spend_cents(&[active, stopped, deleted, failed], period, now);
        // Only the one active deployment counted: 100 hours × $0.16/hr = $16.
        assert_eq!(spend, 100 * 16);
    }

    #[test]
    fn estimate_clamps_created_at_to_period_start() {
        // Deployment was created last month but is still running — we should
        // only attribute spend *this period*, not its full lifetime.
        let period = Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap();
        let last_month = Utc.with_ymd_and_hms(2026, 4, 15, 0, 0, 0).unwrap();
        let now = period + chrono::Duration::hours(48);

        let d = mk_deployment("shared-cpu-1x", last_month, "active", None);
        let spend = estimate_org_spend_cents(&[d], period, now);
        assert_eq!(spend, 48 * 16); // 48h × $0.16/hr, NOT (48h + April uptime)
    }

    #[test]
    fn estimate_returns_zero_when_deployment_starts_after_now() {
        // Should never happen in practice — defensive.
        let period = Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap();
        let future = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
        let now = period + chrono::Duration::hours(48);
        let d = mk_deployment("shared-cpu-1x", future, "active", None);
        assert_eq!(estimate_org_spend_cents(&[d], period, now), 0);
    }

    #[test]
    fn estimate_pro_running_3_mocks_24_7_crosses_2x_after_a_week() {
        // The exact scenario in #450's cost-math table: 3 shared-cpu-1x mocks
        // running 24/7. Pro 2× threshold is $58. At $0.48/hr (3 mocks × $0.16),
        // that's 121 hours ≈ 5 days. Verify the worker would actually fire.
        let period = Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap();
        let now = period + chrono::Duration::hours(168); // 1 week
        let mocks: Vec<HostedMock> =
            (0..3).map(|_| mk_deployment("shared-cpu-1x", period, "active", None)).collect();
        let spend = estimate_org_spend_cents(&mocks, period, now);
        // 168 × 3 × 16 = 8064 cents = $80.64; well over the $58 Pro 2× threshold.
        assert!(spend >= plan_price_cents(Plan::Pro).unwrap() * 2);
        assert_eq!(spend, 168 * 3 * 16);
    }

    #[test]
    fn format_dollars_pads_cents() {
        assert_eq!(format_dollars(0), "$0.00");
        assert_eq!(format_dollars(5), "$0.05");
        assert_eq!(format_dollars(2900), "$29.00");
        assert_eq!(format_dollars(8064), "$80.64");
    }

    #[test]
    fn current_period_start_is_day_one_of_month() {
        let p = current_period_start();
        assert_eq!(p.day(), 1);
    }
}
