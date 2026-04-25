//! Background worker that prunes `runtime_captures` and `runtime_traces`
//! by plan tier.
//!
//! Mirrors `runtime_logs_retention` for the two newer tables added in
//! #234 part 2 (captures) and #233 (traces). Same plan windows, same
//! tick cadence, same env-var override pattern. Kept as a separate file
//! rather than folded into the logs worker because the SQL differs in
//! the time-column name (`occurred_at` for captures, also `occurred_at`
//! for traces — but the per-row payload size is much larger for both,
//! so a misconfigured retention policy would be more painful here than
//! for logs).
//!
//! Plan windows:
//!   * Free  — 24 hours
//!   * Pro   — 7 days
//!   * Team  — 30 days
//!
//! Overrides:
//!   * `MOCKFORGE_CAPTURE_RETENTION_DAYS_<TIER>`
//!   * `MOCKFORGE_TRACE_RETENTION_DAYS_<TIER>`

use sqlx::PgPool;
use std::time::Duration;
use tracing::{error, info};

const RETENTION_TICK: Duration = Duration::from_secs(6 * 60 * 60);

/// Start the combined retention worker. Returns immediately; the loop
/// runs in a tokio task for the lifetime of the process. One task is
/// fine — the deletes are bounded by the per-tier index range scans
/// and run sequentially, so we don't need separate workers per table.
pub fn start_runtime_observability_retention_worker(pool: PgPool) {
    let capture_free = retention_for("CAPTURE", "free", 1);
    let capture_pro = retention_for("CAPTURE", "pro", 7);
    let capture_team = retention_for("CAPTURE", "team", 30);
    let trace_free = retention_for("TRACE", "free", 1);
    let trace_pro = retention_for("TRACE", "pro", 7);
    let trace_team = retention_for("TRACE", "team", 30);

    info!(
        capture_free = capture_free,
        capture_pro = capture_pro,
        capture_team = capture_team,
        trace_free = trace_free,
        trace_pro = trace_pro,
        trace_team = trace_team,
        "Runtime observability retention worker started (runs every 6 hours)"
    );

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(RETENTION_TICK);
        interval.tick().await; // First tick fires immediately

        loop {
            if let Err(e) = run_captures_pass(&pool, capture_free, capture_pro, capture_team).await
            {
                error!("Runtime captures retention pass failed: {:?}", e);
            }
            if let Err(e) = run_traces_pass(&pool, trace_free, trace_pro, trace_team).await {
                error!("Runtime traces retention pass failed: {:?}", e);
            }
            interval.tick().await;
        }
    });
}

fn retention_for(table: &str, tier: &str, default_days: i32) -> i32 {
    let key = format!("MOCKFORGE_{}_RETENTION_DAYS_{}", table, tier.to_uppercase());
    std::env::var(&key).ok().and_then(|s| s.parse().ok()).unwrap_or(default_days)
}

/// One retention pass over `runtime_captures`. Public for test/admin use.
pub async fn run_captures_pass(
    pool: &PgPool,
    free_days: i32,
    pro_days: i32,
    team_days: i32,
) -> Result<u64, sqlx::Error> {
    let f = prune_captures(pool, "free", free_days).await?;
    let p = prune_captures(pool, "pro", pro_days).await?;
    let t = prune_captures(pool, "team", team_days).await?;
    let total = f + p + t;
    if total > 0 {
        info!(free = f, pro = p, team = t, "Pruned runtime_captures");
    }
    Ok(total)
}

/// One retention pass over `runtime_traces`. Public for test/admin use.
pub async fn run_traces_pass(
    pool: &PgPool,
    free_days: i32,
    pro_days: i32,
    team_days: i32,
) -> Result<u64, sqlx::Error> {
    let f = prune_traces(pool, "free", free_days).await?;
    let p = prune_traces(pool, "pro", pro_days).await?;
    let t = prune_traces(pool, "team", team_days).await?;
    let total = f + p + t;
    if total > 0 {
        info!(free = f, pro = p, team = t, "Pruned runtime_traces");
    }
    Ok(total)
}

async fn prune_captures(
    pool: &PgPool,
    tier: &str,
    retention_days: i32,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM runtime_captures r
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

async fn prune_traces(pool: &PgPool, tier: &str, retention_days: i32) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM runtime_traces r
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
    fn capture_env_override_parses() {
        std::env::set_var("MOCKFORGE_CAPTURE_RETENTION_DAYS_TESTTIER", "42");
        assert_eq!(retention_for("CAPTURE", "testtier", 7), 42);
        std::env::remove_var("MOCKFORGE_CAPTURE_RETENTION_DAYS_TESTTIER");
    }

    #[test]
    fn trace_env_override_parses() {
        std::env::set_var("MOCKFORGE_TRACE_RETENTION_DAYS_TESTTIER", "13");
        assert_eq!(retention_for("TRACE", "testtier", 7), 13);
        std::env::remove_var("MOCKFORGE_TRACE_RETENTION_DAYS_TESTTIER");
    }

    #[test]
    fn falls_back_when_unset() {
        std::env::remove_var("MOCKFORGE_CAPTURE_RETENTION_DAYS_NOTSET");
        assert_eq!(retention_for("CAPTURE", "notset", 99), 99);
    }

    #[test]
    fn falls_back_when_invalid() {
        std::env::set_var("MOCKFORGE_TRACE_RETENTION_DAYS_BAD", "not-a-number");
        assert_eq!(retention_for("TRACE", "bad", 5), 5);
        std::env::remove_var("MOCKFORGE_TRACE_RETENTION_DAYS_BAD");
    }
}
