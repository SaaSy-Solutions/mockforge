//! Contract-verification probe worker (#671).
//!
//! Periodically scans every enabled `MonitoredService` with a non-empty
//! `openapi_spec_url` and enqueues a `contract_diff` job for each. The
//! runner-side `ContractExecutor` (in `mockforge-test-runner`) already
//! knows how to fetch + parse the spec — this worker just owns the
//! "fire it on schedule, not only when the user clicks the button"
//! piece that the audit called out as missing.
//!
//! ## Why a single global interval instead of per-service cron
//!
//! `MonitoredService` doesn't yet carry a per-row probe interval, and
//! adding that would force a migration. For the first cut, the worker
//! ticks at `MOCKFORGE_CONTRACT_PROBE_INTERVAL_SECS` (default 1800 =
//! 30 min) and enqueues a job for every probeable service. A
//! per-service cron is a clean follow-up: swap `list_probeable` for a
//! filtered query and reuse the same `enqueue_for_service` helper.
//!
//! ## Idempotency
//!
//! Two ticks racing wouldn't double-fire because each enqueue creates a
//! fresh `test_runs` row with a new UUID — the runner pool drains both
//! without crashing. The only concern is wasted work on a stuck
//! runner, which a per-service cron would fix.
//!
//! ## Error policy
//!
//! Every failure is logged at WARN/ERROR and the loop continues —
//! probe scheduling must never stall the registry-server boot or
//! crash the worker process. Same posture as
//! `test_schedule_runner.rs`.

use std::time::Duration;

use mockforge_registry_core::models::test_run::EnqueueTestRun;
use mockforge_registry_core::models::{CloudWorkspace, MonitoredService, TestRun};
use sqlx::PgPool;
use tracing::{debug, error, info, warn};

use crate::redis::RedisPool;
use crate::run_queue::{enqueue, EnqueuedJob};

fn tick_interval_from_env() -> Duration {
    const DEFAULT_SECS: u64 = 1800;
    let secs = std::env::var("MOCKFORGE_CONTRACT_PROBE_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|&s| s >= 60) // sanity floor — sub-minute is wasteful
        .unwrap_or(DEFAULT_SECS);
    Duration::from_secs(secs)
}

/// Start the contract-probe worker. Spawns a tokio task that ticks at
/// the configured interval and enqueues `contract_diff` jobs for every
/// enabled, probeable monitored service.
pub fn start_contract_probe_worker(pool: PgPool, redis: Option<RedisPool>) {
    let interval = tick_interval_from_env();
    info!(interval_secs = interval.as_secs(), "contract_probe worker started");
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        // Skip the immediate first tick — boot is a noisy moment.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            if let Err(e) = run_tick(&pool, redis.as_ref()).await {
                error!(error = %e, "contract_probe tick failed");
            }
        }
    });
}

/// One tick: enqueue a contract_diff job for every probeable service.
/// Returns the number of jobs enqueued (for observability + tests).
pub async fn run_tick(pool: &PgPool, redis: Option<&RedisPool>) -> sqlx::Result<u32> {
    let services = MonitoredService::list_probeable(pool).await?;
    if services.is_empty() {
        debug!("contract_probe: no probeable services");
        return Ok(0);
    }
    let mut enqueued = 0u32;
    for svc in services {
        match enqueue_for_service(pool, redis, &svc).await {
            Ok(()) => enqueued += 1,
            Err(e) => warn!(
                service_id = %svc.id,
                service_name = %svc.name,
                error = %e,
                "contract_probe enqueue failed for service",
            ),
        }
    }
    if enqueued > 0 {
        info!(enqueued, "contract_probe tick enqueued runs");
    }
    Ok(enqueued)
}

async fn enqueue_for_service(
    pool: &PgPool,
    redis: Option<&RedisPool>,
    svc: &MonitoredService,
) -> sqlx::Result<()> {
    // Look up the workspace's org so the test_run is org-attributable
    // (the runner uses `org_id` for tenant scoping).
    let workspace = CloudWorkspace::find_by_id(pool, svc.workspace_id)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)?;

    let run = TestRun::enqueue(
        pool,
        EnqueueTestRun {
            suite_id: svc.id,
            org_id: workspace.org_id,
            kind: "contract_diff",
            triggered_by: "scheduled",
            triggered_by_user: None,
            git_ref: None,
            git_sha: None,
        },
    )
    .await?;

    if let Some(redis) = redis {
        if let Err(e) = enqueue(
            Some(redis),
            EnqueuedJob {
                run_id: run.id,
                org_id: run.org_id,
                source_id: svc.id,
                kind: "contract_diff",
                payload: serde_json::json!({
                    "service_name": svc.name,
                    "base_url": svc.base_url,
                    "openapi_spec_url": svc.openapi_spec_url,
                    "traffic_source": svc.traffic_source,
                    "workspace_id": svc.workspace_id,
                }),
            },
        )
        .await
        {
            // The run row already exists; the runner pool will surface
            // it as "stuck queued" eventually. We log loudly here so
            // the operator notices Redis dropped a job.
            error!(
                run_id = %run.id,
                service_id = %svc.id,
                error = %e,
                "contract_probe Redis enqueue failed",
            );
        }
    } else {
        debug!(
            run_id = %run.id,
            service_id = %svc.id,
            "contract_probe enqueued without Redis (in-process runner only)",
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // The interval env var is process-global; serialise the three tests
    // that read/mutate it so a parallel runner doesn't race them.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn interval_from_env_falls_back_to_default() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::remove_var("MOCKFORGE_CONTRACT_PROBE_INTERVAL_SECS");
        assert_eq!(tick_interval_from_env(), Duration::from_secs(1800));
    }

    #[test]
    fn interval_from_env_rejects_sub_minute_values() {
        let _g = ENV_LOCK.lock().unwrap();
        // Sanity floor: 30s should be replaced by the 1800s default.
        std::env::set_var("MOCKFORGE_CONTRACT_PROBE_INTERVAL_SECS", "30");
        assert_eq!(tick_interval_from_env(), Duration::from_secs(1800));
        std::env::remove_var("MOCKFORGE_CONTRACT_PROBE_INTERVAL_SECS");
    }

    #[test]
    fn interval_from_env_honours_valid_override() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var("MOCKFORGE_CONTRACT_PROBE_INTERVAL_SECS", "300");
        assert_eq!(tick_interval_from_env(), Duration::from_secs(300));
        std::env::remove_var("MOCKFORGE_CONTRACT_PROBE_INTERVAL_SECS");
    }
}
