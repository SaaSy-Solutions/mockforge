//! Background worker that enforces the audit-log retention period.
//!
//! Audit rows are kept for `AUDIT_LOG_RETENTION_DAYS` (400) days and then
//! deleted, as the privacy policy (§5) and DPA (§10) state. The append-only
//! trigger only lets this purge delete rows past that age, so a bug here can
//! remove too little but never a row inside the window. See
//! `AuditLog::purge_expired` and migration
//! `20250101000089_audit_logs_bounded_retention.sql`.

use mockforge_registry_core::models::audit_log::{AuditLog, AUDIT_LOG_RETENTION_DAYS};
use sqlx::PgPool;
use std::time::Duration;
use tracing::{error, info};

/// Daily is plenty: the window is measured in days and each pass deletes at
/// most one day's worth of rows once the backlog is cleared.
const RETENTION_TICK: Duration = Duration::from_secs(24 * 60 * 60);

/// Start the worker on the owner pool (the purge is cross-org). Returns
/// immediately; the loop runs for the life of the process.
pub fn start_audit_log_retention_worker(pool: PgPool) {
    info!(
        retention_days = AUDIT_LOG_RETENTION_DAYS,
        "Audit log retention worker started (runs daily)"
    );
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(RETENTION_TICK);
        loop {
            interval.tick().await;
            match AuditLog::purge_expired(&pool).await {
                Ok(0) => {}
                Ok(n) => info!(deleted = n, "audit log retention: purged expired rows"),
                Err(e) => error!(error = %e, "audit log retention pass failed"),
            }
        }
    });
}
