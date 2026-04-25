//! Background worker that prunes `runtime_request_logs` by plan tier.
//!
//! The in-container log shipper (#232) writes one row per request. Without
//! a retention policy the table grows forever and the per-deployment
//! "Requests" tab gets slower over time. This worker runs every six hours
//! and deletes rows older than the per-org plan's retention window.
//!
//! Plan windows (matches the rest of the cloud surface):
//!   * Free  — 24 hours
//!   * Pro   — 7 days
//!   * Team  — 30 days
//!
//! These are conservative for v1 — easy to extend later. Setting the env
//! var `MOCKFORGE_LOG_RETENTION_DAYS_<TIER>` overrides the default for any
//! tier; useful for soak testing without a code change.
//!
//! Deletion happens by joining `runtime_request_logs.deployment_id` to
//! `hosted_mocks.org_id` so we don't have to denormalize the org onto
//! every row. One query per tier keeps the SQL readable.

use sqlx::PgPool;
use std::time::Duration;
use tracing::{error, info};

/// How often the worker runs. 6 hours is a sweet spot — frequent enough
/// to keep the table from ballooning, infrequent enough that the delete
/// query runs against a small backlog and finishes fast.
const RETENTION_TICK: Duration = Duration::from_secs(6 * 60 * 60);

/// Start the retention worker. Returns immediately; the deletion loop
/// runs in a tokio task for the lifetime of the process.
pub fn start_runtime_logs_retention_worker(pool: PgPool) {
    let free_days = retention_for_tier("free", 1);
    let pro_days = retention_for_tier("pro", 7);
    let team_days = retention_for_tier("team", 30);

    info!(
        free_days = free_days,
        pro_days = pro_days,
        team_days = team_days,
        "Runtime request logs retention worker started (runs every 6 hours)"
    );

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(RETENTION_TICK);
        // First tick fires immediately — useful for catching startup-time
        // backlog without waiting six hours.
        interval.tick().await;

        loop {
            if let Err(e) = run_once(&pool, free_days, pro_days, team_days).await {
                error!("Runtime request logs retention pass failed: {:?}", e);
            }
            interval.tick().await;
        }
    });
}

/// Read an env-var override for a plan tier, falling back to the default.
fn retention_for_tier(tier: &str, default_days: i32) -> i32 {
    let key = format!("MOCKFORGE_LOG_RETENTION_DAYS_{}", tier.to_uppercase());
    std::env::var(&key).ok().and_then(|s| s.parse().ok()).unwrap_or(default_days)
}

/// One retention pass. Returns the total rows deleted across all tiers.
/// Public so a future admin endpoint or test can trigger it manually.
pub async fn run_once(
    pool: &PgPool,
    free_days: i32,
    pro_days: i32,
    team_days: i32,
) -> Result<u64, sqlx::Error> {
    let free_deleted = prune_tier(pool, "free", free_days).await?;
    let pro_deleted = prune_tier(pool, "pro", pro_days).await?;
    let team_deleted = prune_tier(pool, "team", team_days).await?;
    let total = free_deleted + pro_deleted + team_deleted;
    if total > 0 {
        info!(
            free = free_deleted,
            pro = pro_deleted,
            team = team_deleted,
            "Pruned runtime_request_logs"
        );
    }
    Ok(total)
}

/// Delete rows older than `retention_days` for orgs on `tier`. The join
/// keeps the per-row org check on the database side; the index on
/// `(deployment_id, occurred_at DESC)` makes this an index-range scan
/// per deployment.
async fn prune_tier(pool: &PgPool, tier: &str, retention_days: i32) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM runtime_request_logs r
        USING hosted_mocks hm, organizations o
        WHERE r.deployment_id = hm.id
          AND hm.org_id = o.id
          AND o.plan = $1
          AND r.occurred_at < NOW() - ($2::int || ' days')::interval
        "#,
    )
    .bind(tier)
    .bind(retention_days)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_override_parses_when_set() {
        std::env::set_var("MOCKFORGE_LOG_RETENTION_DAYS_TESTTIER", "42");
        assert_eq!(retention_for_tier("testtier", 7), 42);
        std::env::remove_var("MOCKFORGE_LOG_RETENTION_DAYS_TESTTIER");
    }

    #[test]
    fn env_override_falls_back_when_unset() {
        std::env::remove_var("MOCKFORGE_LOG_RETENTION_DAYS_OTHERTIER");
        assert_eq!(retention_for_tier("othertier", 99), 99);
    }

    #[test]
    fn env_override_falls_back_when_invalid() {
        std::env::set_var("MOCKFORGE_LOG_RETENTION_DAYS_BADTIER", "not-a-number");
        assert_eq!(retention_for_tier("badtier", 5), 5);
        std::env::remove_var("MOCKFORGE_LOG_RETENTION_DAYS_BADTIER");
    }
}
