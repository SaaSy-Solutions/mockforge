//! Contract-verification probe worker (#671).
//!
//! Periodically scans every enabled `MonitoredService` with a non-empty
//! `openapi_spec_url` and enqueues a `contract_diff` job for each. The
//! runner-side `ContractExecutor` (in `mockforge-test-runner`) already
//! knows how to fetch + parse the spec — this worker just owns the
//! "fire it on schedule, not only when the user clicks the button"
//! piece that the audit called out as missing.
//!
//! ## Per-service cadence (#720)
//!
//! `probe_cron` schedules use the service's IANA `probe_timezone` (UTC by
//! default). A cron schedule takes precedence over `probe_interval_secs`.
//! Services without a cron retain their existing interval or global default.
//! The worker polls every 30 seconds, independently of the interval default.
//! Missed cron occurrences coalesce into one probe rather than a burst.
//!
//! ## Concurrent ticks
//!
//! Each candidate is locked with `FOR UPDATE SKIP LOCKED` and its due state
//! is rechecked. The queued run and probe timestamp commit together, so
//! racing ticks and database failures cannot create duplicate claims.
//!
//! ## Error policy
//!
//! Every failure is logged at WARN/ERROR and the loop continues —
//! probe scheduling must never stall the registry-server boot or
//! crash the worker process. Same posture as
//! `test_schedule_runner.rs`.

use std::time::Duration;

use chrono::{DateTime, Utc};

use mockforge_registry_core::models::test_run::EnqueueTestRun;
use mockforge_registry_core::models::{CloudWorkspace, MonitoredService, TestRun};
use sqlx::PgPool;
use tracing::{debug, error, info, warn};

use crate::redis::RedisPool;
use crate::run_queue::{enqueue, EnqueuedJob};
use crate::storage::{PluginStorage, RUNNER_FETCH_TTL};

fn cadence_interval_from_env() -> Duration {
    const DEFAULT_SECS: u64 = 1800;
    let secs = std::env::var("MOCKFORGE_CONTRACT_PROBE_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|&s| s >= 60 && s <= i32::MAX as u64)
        .unwrap_or(DEFAULT_SECS);
    Duration::from_secs(secs)
}

/// Start the contract-probe worker. Spawns a tokio task that ticks at
/// the configured interval and enqueues `contract_diff` jobs for every
/// enabled, probeable monitored service.
/// Where due probes go: the run queue plus storage to presign uploaded
/// specs the runner fetches.
#[derive(Clone, Copy)]
pub struct ProbeQueue<'a> {
    pub redis: &'a RedisPool,
    pub storage: &'a PluginStorage,
}

pub fn start_contract_probe_worker(pool: PgPool, redis: Option<RedisPool>, storage: PluginStorage) {
    let interval = Duration::from_secs(30);
    info!(interval_secs = interval.as_secs(), "contract_probe worker started");
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        // Skip the immediate first tick — boot is a noisy moment.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            let queue = redis.as_ref().map(|redis| ProbeQueue {
                redis,
                storage: &storage,
            });
            if let Err(e) = run_tick(&pool, queue).await {
                error!(error = %e, "contract_probe tick failed");
            }
        }
    });
}

/// One tick: enqueue a contract_diff job for every probeable service
/// whose cadence is due (#720). Returns the number of jobs enqueued
/// (for observability + tests).
pub async fn run_tick(pool: &PgPool, queue: Option<ProbeQueue<'_>>) -> sqlx::Result<u32> {
    let now = Utc::now();
    let default_secs = cadence_interval_from_env().as_secs() as i64;
    let services = MonitoredService::list_probeable_due(pool, default_secs).await?;
    if services.is_empty() {
        debug!("contract_probe: no services due for probing");
        return Ok(0);
    }
    let mut enqueued = 0u32;
    for svc in services {
        match enqueue_for_service(pool, queue, &svc, now, default_secs).await {
            Ok(true) => enqueued += 1,
            Ok(false) => {}
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

fn probe_is_due(
    svc: &MonitoredService,
    now: DateTime<Utc>,
    default_secs: i64,
) -> Result<bool, String> {
    if let Some(expression) = svc.probe_cron.as_deref() {
        return super::test_schedule_runner::next_fire_in_window(
            expression,
            &svc.probe_timezone,
            svc.last_probed_at.unwrap_or(svc.created_at),
            now,
        )
        .map(|fire| fire.is_some());
    }
    Ok(svc.last_probed_at.is_none_or(|last| {
        now.signed_duration_since(last).num_seconds()
            >= i64::from(svc.probe_interval_secs.unwrap_or(default_secs as i32))
    }))
}

async fn enqueue_for_service(
    pool: &PgPool,
    queue: Option<ProbeQueue<'_>>,
    candidate: &MonitoredService,
    now: DateTime<Utc>,
    default_secs: i64,
) -> sqlx::Result<bool> {
    let mut tx = pool.begin().await?;
    let Some(svc) = sqlx::query_as::<_, MonitoredService>(
        "SELECT * FROM monitored_services WHERE id = $1 AND enabled = TRUE \
         AND openapi_spec_url IS NOT NULL AND openapi_spec_url <> '' \
         FOR UPDATE SKIP LOCKED",
    )
    .bind(candidate.id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        tx.rollback().await?;
        return Ok(false);
    };
    // Roll back explicitly when the service is not due. Dropping the
    // transaction only queues the ROLLBACK, so the `FOR UPDATE` row lock
    // can outlive this call and make the next claim on another pooled
    // connection skip the row as locked.
    match probe_is_due(&svc, now, default_secs) {
        Ok(true) => {}
        Ok(false) => {
            tx.rollback().await?;
            return Ok(false);
        }
        Err(error) => {
            warn!(service_id = %svc.id, %error, "skipping invalid probe schedule");
            tx.rollback().await?;
            return Ok(false);
        }
    }
    // Look up the workspace's org so the test_run is org-attributable
    // (the runner uses `org_id` for tenant scoping).
    let workspace = CloudWorkspace::find_by_id(&mut *tx, svc.workspace_id)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)?;

    let run = TestRun::enqueue(
        &mut *tx,
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
    sqlx::query("UPDATE monitored_services SET last_probed_at = $2 WHERE id = $1")
        .bind(svc.id)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    if let Some(ProbeQueue { redis, storage }) = queue {
        let spec_url = match svc.openapi_spec_url.as_deref() {
            Some(url) => Some(storage.fetch_spec_url(url, run.org_id, RUNNER_FETCH_TTL).await),
            None => None,
        };
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
                    "openapi_spec_url": spec_url,
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
    Ok(true)
}

#[cfg(test)]
mod postgres_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::sync::Mutex;

    // The interval env var is process-global; serialise the three tests
    // that read/mutate it so a parallel runner doesn't race them.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn service() -> MonitoredService {
        // An older serialized service has no cron/timezone fields.
        serde_json::from_value(serde_json::json!({
            "id": uuid::Uuid::new_v4(), "workspace_id": uuid::Uuid::new_v4(),
            "name": "probe fixture", "base_url": "http://example.test",
            "openapi_spec_url": "http://example.test/api.json", "traffic_source": "probe",
            "enabled": true, "created_at": "2026-01-01T12:00:00Z",
            "updated_at": "2026-01-01T12:00:00Z"
        }))
        .unwrap()
    }

    #[test]
    fn cron_waits_for_fire_and_does_not_replay_a_claimed_occurrence() {
        let mut svc = service();
        assert_eq!(svc.probe_timezone, "UTC");
        svc.probe_cron = Some("0 */5 * * * *".into());
        svc.probe_interval_secs = Some(1); // Cron takes precedence.
        let before = Utc.with_ymd_and_hms(2026, 1, 1, 12, 4, 59).unwrap();
        let fire = Utc.with_ymd_and_hms(2026, 1, 1, 12, 5, 0).unwrap();
        assert!(!probe_is_due(&svc, before, 1800).unwrap());
        assert!(probe_is_due(&svc, fire, 1800).unwrap());
        svc.last_probed_at = Some(fire);
        assert!(!probe_is_due(&svc, fire, 1800).unwrap());
        assert!(!probe_is_due(&svc, fire + chrono::Duration::seconds(30), 1800).unwrap());
        assert!(probe_is_due(&svc, fire + chrono::Duration::minutes(5), 1800).unwrap());
    }

    #[test]
    fn cron_respects_timezone_across_daylight_saving_offsets() {
        let mut svc = service();
        svc.probe_cron = Some("0 0 9 * * *".into());
        svc.probe_timezone = "America/New_York".into();
        for (month, utc_hour) in [(1, 14), (7, 13)] {
            let fire = Utc.with_ymd_and_hms(2026, month, 2, utc_hour, 0, 0).unwrap();
            svc.last_probed_at = Some(fire - chrono::Duration::hours(1));
            assert!(!probe_is_due(&svc, fire - chrono::Duration::seconds(1), 1800).unwrap());
            assert!(probe_is_due(&svc, fire, 1800).unwrap());
        }
    }

    #[test]
    fn legacy_interval_and_default_cadence_remain_independent() {
        let mut svc = service();
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 12, 10, 0).unwrap();
        assert!(probe_is_due(&svc, now, 1800).unwrap());
        svc.last_probed_at = Some(now - chrono::Duration::seconds(60));
        assert!(!probe_is_due(&svc, now, 1800).unwrap());
        svc.probe_interval_secs = Some(60);
        assert!(probe_is_due(&svc, now, 1800).unwrap());
        svc.probe_interval_secs = Some(3600);
        assert!(!probe_is_due(&svc, now, 1800).unwrap());
    }

    #[test]
    fn malformed_persisted_cron_or_timezone_is_rejected() {
        let mut svc = service();
        let now = Utc::now();
        svc.probe_cron = Some("invalid".into());
        assert!(probe_is_due(&svc, now, 1800).is_err());
        svc.probe_cron = Some("0 * * * * *".into());
        svc.probe_timezone = "invalid".into();
        assert!(probe_is_due(&svc, now, 1800).is_err());
    }

    #[test]
    fn interval_from_env_falls_back_to_default() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::remove_var("MOCKFORGE_CONTRACT_PROBE_INTERVAL_SECS");
        assert_eq!(cadence_interval_from_env(), Duration::from_secs(1800));
    }

    #[test]
    fn interval_from_env_rejects_sub_minute_values() {
        let _g = ENV_LOCK.lock().unwrap();
        // Sanity floor: 30s should be replaced by the 1800s default.
        std::env::set_var("MOCKFORGE_CONTRACT_PROBE_INTERVAL_SECS", "30");
        assert_eq!(cadence_interval_from_env(), Duration::from_secs(1800));
        std::env::remove_var("MOCKFORGE_CONTRACT_PROBE_INTERVAL_SECS");
    }

    #[test]
    fn interval_from_env_honours_valid_override() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var("MOCKFORGE_CONTRACT_PROBE_INTERVAL_SECS", "300");
        assert_eq!(cadence_interval_from_env(), Duration::from_secs(300));
        std::env::remove_var("MOCKFORGE_CONTRACT_PROBE_INTERVAL_SECS");
    }
}
