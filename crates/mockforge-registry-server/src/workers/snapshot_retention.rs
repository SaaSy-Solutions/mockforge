//! Snapshot retention worker (cloud-enablement task #10 / Phase 2).
//!
//! Periodically scans `snapshots` for `ready` rows whose `expires_at`
//! has passed and flips them to `expired`. Blob reclamation (S3 delete)
//! lives in a follow-up slice — once we have the storage backend wired,
//! this worker will delete the underlying object before flipping the
//! row, so the storage cost actually drops.
//!
//! Tick cadence is intentionally slow (15 min): expirations are
//! eventually-consistent by design, and a tight loop hammering the DB
//! would burn budget for no user-visible benefit. The
//! `mark_expired_batch` helper is idempotent so a missed tick at most
//! delays reclamation, never double-reclaims.

use std::time::Duration;

use mockforge_registry_core::models::Snapshot;
use sqlx::PgPool;
use tracing::{debug, error, info};

const TICK_INTERVAL: Duration = Duration::from_secs(15 * 60);
const BATCH_LIMIT: i64 = 100;

pub fn start_snapshot_retention_worker(pool: PgPool) {
    info!(
        "snapshot retention worker started — ticking every {}s, batch={}",
        TICK_INTERVAL.as_secs(),
        BATCH_LIMIT
    );
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(TICK_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        // Don't fire on boot — wait one full interval so the registry
        // settles before doing destructive work.
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(e) = run_tick(&pool).await {
                error!(error = %e, "snapshot retention tick failed");
            }
        }
    });
}

/// One tick. Returns the number of snapshots transitioned to expired
/// (used by tests + for log lines).
pub async fn run_tick(pool: &PgPool) -> sqlx::Result<u32> {
    let expired = Snapshot::mark_expired_batch(pool, BATCH_LIMIT).await?;
    if expired.is_empty() {
        debug!("snapshot retention tick: nothing to expire");
        return Ok(0);
    }
    let count = expired.len();
    info!(count, "snapshot retention: marked snapshots expired");
    // TODO(retention-blob-reclaim): once a storage backend lands here,
    // walk `expired` and call backend.delete(&snapshot.storage_url) for
    // each. Failure to reclaim should not roll back the row flip — a
    // separate "orphaned blob" sweep can clean those up later.
    Ok(count as u32)
}

#[cfg(test)]
mod tests {
    // Like the dispatcher, the substantive logic (which rows get
    // flipped) lives in the SQL UPDATE in mark_expired_batch — that
    // belongs in an integration test against real Postgres, not a
    // unit test. This anchors the module so main.rs wiring compiles.
    #[test]
    fn smoke_module_links() {}
}
