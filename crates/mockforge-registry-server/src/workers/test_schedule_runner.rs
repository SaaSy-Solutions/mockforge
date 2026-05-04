//! Cron-driven test_run trigger (cloud-enablement task #4 / Phase 3).
//!
//! Scans `test_schedules` once a minute. For each enabled row whose next
//! cron-fire timestamp falls between the row's `last_triggered_at` (or
//! its `created_at` floor on first fire) and `now`, the worker triggers
//! a run the same way the public POST /test-suites/{id}/runs path does:
//! insert a test_runs row, push a job onto the Redis queue. The runner
//! pool consumes those jobs identically — schedule-vs-manual is just a
//! `triggered_by` label.
//!
//! Idempotency: TestSchedule::mark_triggered uses `WHERE last_triggered_at
//! IS NULL OR last_triggered_at < $fired_at`, so a worker restart can't
//! double-fire the same schedule for the same minute.
//!
//! Error policy: a single bad cron expression / DB error is logged and
//! the next schedule continues. The worker is non-fatal — a parse
//! failure on one row never stalls the whole tick.

use std::str::FromStr;
use std::time::Duration;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use cron::Schedule;
use mockforge_registry_core::models::test_run::EnqueueTestRun;
use mockforge_registry_core::models::{CloudWorkspace, TestRun, TestSchedule, TestSuite};
use sqlx::PgPool;
use tracing::{error, info, warn};

use crate::redis::RedisPool;
use crate::run_queue::{enqueue, EnqueuedJob};

const TICK_INTERVAL: Duration = Duration::from_secs(60);

pub fn start_test_schedule_worker(pool: PgPool, redis: Option<RedisPool>) {
    info!("test_schedule worker started — ticking every {}s", TICK_INTERVAL.as_secs());
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(TICK_INTERVAL);
        // Skip the immediate first tick at boot — the registry just came
        // up, runners may not be ready, and a stale schedule firing on
        // boot is more likely to be wrong than right.
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(e) = run_tick(&pool, redis.as_ref()).await {
                error!(error = %e, "test_schedule tick failed");
            }
        }
    });
}

/// One tick of the schedule loop. Loads all enabled schedules, computes
/// the most recent fire time relative to now for each, and triggers runs
/// for any that have crossed their next-fire boundary since the last
/// trigger. Returns the number of runs triggered (for observability /
/// tests).
pub async fn run_tick(pool: &PgPool, redis: Option<&RedisPool>) -> sqlx::Result<u32> {
    let schedules = TestSchedule::list_enabled(pool).await?;
    if schedules.is_empty() {
        return Ok(0);
    }
    let now = Utc::now();
    let mut fired = 0u32;
    for sched in schedules {
        let lower_bound = sched.last_triggered_at.unwrap_or(sched.created_at);
        match next_fire_in_window(&sched.cron, &sched.timezone, lower_bound, now) {
            Ok(Some(fire_at)) => {
                if let Err(e) = trigger_scheduled_run(pool, redis, &sched, fire_at).await {
                    error!(
                        schedule_id = %sched.id,
                        error = %e,
                        "scheduled run trigger failed",
                    );
                } else {
                    fired += 1;
                }
            }
            Ok(None) => {
                // Not yet due — fall through.
            }
            Err(e) => {
                warn!(
                    schedule_id = %sched.id,
                    cron = %sched.cron,
                    timezone = %sched.timezone,
                    error = %e,
                    "skipping schedule with unparsable cron / tz",
                );
            }
        }
    }
    if fired > 0 {
        info!(fired, "test_schedule tick triggered runs");
    }
    Ok(fired)
}

async fn trigger_scheduled_run(
    pool: &PgPool,
    redis: Option<&RedisPool>,
    sched: &TestSchedule,
    fire_at: DateTime<Utc>,
) -> sqlx::Result<()> {
    // Mark the schedule first so a crash mid-trigger doesn't double-fire
    // — better to skip a run than run it twice. The mark is idempotent on
    // its own (compares fire_at to last_triggered_at).
    let marked = TestSchedule::mark_triggered(pool, sched.id, fire_at).await?;
    if marked.is_none() {
        // Another worker already advanced this row. Nothing to do.
        return Ok(());
    }

    let suite = match TestSuite::find_by_id(pool, sched.suite_id).await? {
        Some(s) => s,
        None => {
            warn!(
                schedule_id = %sched.id,
                suite_id = %sched.suite_id,
                "schedule references missing suite — skipping",
            );
            return Ok(());
        }
    };
    let workspace = match CloudWorkspace::find_by_id(pool, suite.workspace_id).await {
        Ok(Some(w)) => w,
        Ok(None) => {
            warn!(
                schedule_id = %sched.id,
                workspace_id = %suite.workspace_id,
                "suite references missing workspace — skipping",
            );
            return Ok(());
        }
        Err(e) => {
            error!(
                schedule_id = %sched.id,
                error = %e,
                "DB error loading workspace for scheduled run",
            );
            return Err(e);
        }
    };

    let run = TestRun::enqueue(
        pool,
        EnqueueTestRun {
            suite_id: suite.id,
            org_id: workspace.org_id,
            kind: &suite.kind,
            triggered_by: "schedule",
            triggered_by_user: None,
            git_ref: None,
            git_sha: None,
        },
    )
    .await?;

    if let Err(e) = enqueue(
        redis,
        EnqueuedJob {
            run_id: run.id,
            org_id: run.org_id,
            source_id: suite.id,
            kind: &suite.kind,
            payload: serde_json::json!({ "schedule_id": sched.id }),
        },
    )
    .await
    {
        error!(
            schedule_id = %sched.id,
            run_id = %run.id,
            error = %e,
            "scheduled run inserted but Redis enqueue failed",
        );
    }
    Ok(())
}

/// Returns the most recent cron fire time strictly after `lower_bound`
/// and at or before `now`, if any. None means the cron hasn't ticked
/// since `lower_bound` yet. Errors only on cron / tz parse failures.
fn next_fire_in_window(
    cron_expr: &str,
    timezone: &str,
    lower_bound: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<Option<DateTime<Utc>>, String> {
    let tz: Tz = timezone.parse().map_err(|e| format!("invalid timezone: {e}"))?;
    let schedule = Schedule::from_str(cron_expr).map_err(|e| format!("invalid cron: {e}"))?;
    // Convert lower_bound to the schedule's local tz and ask cron for the
    // next fire after it. If that fire is <= now, the schedule has crossed
    // a tick since we last fired and we should run it.
    let local_lb = lower_bound.with_timezone(&tz);
    let next_local = schedule.after(&local_lb).next();
    let Some(next_local) = next_local else {
        return Ok(None);
    };
    let next_utc = next_local.with_timezone(&Utc);
    if next_utc <= now {
        Ok(Some(next_utc))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn fire_in_window_returns_some_when_cron_ticked_since_lb() {
        // "every minute" cron; lower_bound at 12:00:00, now at 12:01:30.
        // Next fire after 12:00 is 12:01 which is <= now → Some.
        let lb = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 12, 1, 30).unwrap();
        let r = next_fire_in_window("0 * * * * *", "UTC", lb, now).unwrap();
        assert!(r.is_some(), "expected a fire in window");
    }

    #[test]
    fn fire_in_window_returns_none_when_lb_just_after_last_fire() {
        // lb at 12:01:01, now at 12:01:30, every-minute cron.
        // Next after 12:01:01 is 12:02:00 which is > now → None.
        let lb = Utc.with_ymd_and_hms(2026, 1, 1, 12, 1, 1).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 12, 1, 30).unwrap();
        let r = next_fire_in_window("0 * * * * *", "UTC", lb, now).unwrap();
        assert!(r.is_none(), "expected no fire in window, got {r:?}");
    }

    #[test]
    fn fire_in_window_rejects_invalid_cron() {
        let lb = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 1, 0, 0).unwrap();
        let r = next_fire_in_window("not a cron", "UTC", lb, now);
        assert!(r.is_err());
    }

    #[test]
    fn fire_in_window_rejects_invalid_timezone() {
        let lb = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 1, 0, 0).unwrap();
        let r = next_fire_in_window("0 * * * * *", "Mars/Olympus", lb, now);
        assert!(r.is_err());
    }
}
