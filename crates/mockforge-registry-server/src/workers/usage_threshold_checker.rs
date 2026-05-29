//! Background worker that scans every org's current-month usage against its
//! effective limits (plan + custom quota) and inserts a `usage_alerts` row the
//! first time a 75%, 90%, or 100% band is crossed. Inserts are idempotent on
//! the unique index, so the worker is safe to re-run on any cadence.
//!
//! The 100% band is the "notification sent" half of the hosted-mock overage
//! promise (#748): request mocks keep responding past the plan allotment (no
//! cutoff by default), but the owner is emailed the moment they enter overage.
//!
//! On a newly inserted alert, the worker emails the org owner (gated on
//! `users.email_notifications`). Multiple bands can be inserted in a single run
//! if usage hops past them between checks; only the highest newly-inserted band
//! per (org, metric) triggers an email so users are not double-pinged.

use std::time::Duration;

use chrono::{Datelike, NaiveDate};
use sqlx::PgPool;
use tracing::{debug, error, info, warn};

use crate::{
    email::EmailService,
    models::{Organization, UsageAlert, UsageCounter, User},
    AppState,
};

const DEFAULT_INTERVAL_SECS: u64 = 30 * 60; // 30 minutes
const BANDS: [i16; 3] = [75, 90, 100];

/// Start the usage-threshold checker. Interval can be overridden by the
/// `USAGE_THRESHOLD_CHECK_INTERVAL_SECS` env var; falls back to 30 minutes.
pub fn start_usage_threshold_checker(state: AppState) {
    let interval_secs = std::env::var("USAGE_THRESHOLD_CHECK_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| *n >= 60)
        .unwrap_or(DEFAULT_INTERVAL_SECS);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        // Skip the immediate first tick — give the server a moment to settle.
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(e) = run_once(&state).await {
                error!("usage_threshold_checker: scan failed: {e:?}");
            }
        }
    });

    info!("Usage threshold checker started (every {interval_secs}s)");
}

async fn run_once(state: &AppState) -> Result<(), sqlx::Error> {
    let pool = state.db.pool();
    let period_start = current_period_start();

    // Only scan orgs that have a usage counter for this period — i.e. orgs
    // with any activity. Avoids enumerating every org when most are idle.
    let counters: Vec<UsageCounter> =
        sqlx::query_as::<_, UsageCounter>("SELECT * FROM usage_counters WHERE period_start = $1")
            .bind(period_start)
            .fetch_all(pool)
            .await?;

    debug!(
        "usage_threshold_checker: scanning {} active counters for {}",
        counters.len(),
        period_start
    );

    let mut alerts_emitted = 0usize;
    for counter in counters {
        match check_org(state, pool, period_start, &counter).await {
            Ok(n) => alerts_emitted += n,
            Err(e) => {
                warn!(org_id = %counter.org_id, "threshold check failed: {e:?}");
            }
        }
    }

    if alerts_emitted > 0 {
        info!("usage_threshold_checker: emitted {alerts_emitted} new alert(s)");
    }
    Ok(())
}

async fn check_org(
    state: &AppState,
    pool: &PgPool,
    period_start: NaiveDate,
    counter: &UsageCounter,
) -> Result<usize, sqlx::Error> {
    let Some(org) = Organization::find_by_id(pool, counter.org_id).await? else {
        return Ok(0);
    };

    // Effective limits: plan defaults + custom quota override (matches
    // handlers::usage::effective_limits semantics, but inlined to avoid taking
    // an ApiError dependency in worker code).
    let mut limits = org.limits_json.clone();
    if let Ok(Some(setting)) = state.store.get_org_setting(org.id, "quota").await {
        if let (Some(base), Some(over)) =
            (limits.as_object_mut(), setting.setting_value.as_object())
        {
            for (k, v) in over {
                base.insert(k.clone(), v.clone());
            }
        }
    }

    let metrics: [(&str, i64, i64); 3] = [
        (
            "requests",
            counter.requests,
            limits.get("requests_per_30d").and_then(|v| v.as_i64()).unwrap_or(0),
        ),
        (
            "storage",
            counter.storage_bytes,
            limits
                .get("storage_gb")
                .and_then(|v| v.as_i64())
                .map(|gb| gb.saturating_mul(1_000_000_000))
                .unwrap_or(0),
        ),
        (
            "ai_tokens",
            counter.ai_tokens_used,
            limits.get("ai_tokens_per_month").and_then(|v| v.as_i64()).unwrap_or(0),
        ),
    ];

    let mut emitted = 0usize;
    for (metric, used, limit) in metrics {
        if limit <= 0 || used <= 0 {
            continue;
        }
        let pct_crossed = compute_pct(used, limit);
        let mut highest_new: Option<i16> = None;
        for &band in &BANDS {
            if pct_crossed >= band
                && UsageAlert::try_insert(pool, org.id, metric, period_start, band)
                    .await?
                    .is_some()
            {
                // BANDS is ascending; this overwrites with the highest.
                highest_new = Some(band);
                emitted += 1;
            }
        }
        if let Some(band) = highest_new {
            send_email(state, pool, &org, metric, used, limit, band).await;
        }
    }
    Ok(emitted)
}

fn compute_pct(used: i64, limit: i64) -> i16 {
    if limit <= 0 {
        return 0;
    }
    let pct = (used as f64 / limit as f64 * 100.0).floor();
    pct.clamp(0.0, 1000.0) as i16
}

async fn send_email(
    state: &AppState,
    pool: &PgPool,
    org: &Organization,
    metric: &str,
    used: i64,
    limit: i64,
    band: i16,
) {
    let owner = match User::find_by_id(pool, org.owner_id).await {
        Ok(Some(u)) if u.email_notifications => u,
        Ok(_) => return,
        Err(e) => {
            warn!(org_id = %org.id, "threshold email skipped — owner lookup failed: {e:?}");
            return;
        }
    };

    let metric_label: String = match metric {
        "requests" => "API Requests",
        "storage" => "Storage",
        "ai_tokens" => "AI Tokens",
        other => other,
    }
    .to_string();
    let used_pretty = format_metric_value(metric, used);
    let limit_pretty = format_metric_value(metric, limit);
    let plan = org.plan().to_string();
    let owner_email = owner.email.clone();
    let username = owner.username.clone();
    let band_u = band as u16;

    // Same pattern as billing.rs — read config from env per send.
    let _ = state; // suppress unused-var warning; future config use lives here
    tokio::spawn(async move {
        let email_service = match EmailService::from_env() {
            Ok(s) => s,
            Err(e) => {
                debug!("usage threshold email skipped — service init failed: {e:?}");
                return;
            }
        };
        let msg = EmailService::generate_usage_threshold_warning(
            &username,
            &owner_email,
            &metric_label,
            &plan,
            &used_pretty,
            &limit_pretty,
            band_u,
        );
        if let Err(e) = email_service.send(msg).await {
            warn!("usage threshold email send failed: {e:?}");
        }
    });
}

fn format_metric_value(metric: &str, value: i64) -> String {
    match metric {
        "storage" => format_bytes_si(value),
        _ => format_count(value),
    }
}

fn format_count(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn format_bytes_si(b: i64) -> String {
    if b <= 0 {
        return "0 B".into();
    }
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = b as f64;
    let mut idx = 0;
    while v >= 1000.0 && idx < UNITS.len() - 1 {
        v /= 1000.0;
        idx += 1;
    }
    format!("{:.2} {}", v, UNITS[idx])
}

fn current_period_start() -> NaiveDate {
    let today = chrono::Utc::now().date_naive();
    NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pct_basic() {
        assert_eq!(compute_pct(0, 100), 0);
        assert_eq!(compute_pct(74, 100), 74);
        assert_eq!(compute_pct(75, 100), 75);
        assert_eq!(compute_pct(90, 100), 90);
        assert_eq!(compute_pct(150, 100), 150);
    }

    #[test]
    fn pct_zero_or_negative_limit_is_zero() {
        assert_eq!(compute_pct(50, 0), 0);
        assert_eq!(compute_pct(50, -1), 0);
    }

    #[test]
    fn count_formatting() {
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(1_500), "1.5K");
        assert_eq!(format_count(2_500_000), "2.5M");
    }

    #[test]
    fn bytes_formatting_si() {
        assert_eq!(format_bytes_si(0), "0 B");
        assert_eq!(format_bytes_si(500), "500.00 B");
        assert_eq!(format_bytes_si(1_000), "1.00 KB");
        assert_eq!(format_bytes_si(1_500_000), "1.50 MB");
        assert_eq!(format_bytes_si(20_000_000_000), "20.00 GB");
    }
}
