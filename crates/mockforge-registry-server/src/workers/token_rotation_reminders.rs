//! Background worker that emails reminders for API tokens older than the
//! rotation threshold.
//!
//! Wraps `handlers::token_rotation::send_rotation_reminders`, which on its own
//! is just an async function — without this worker it would never run.
//!
//! The worker tolerates a missing email provider: `EmailService::from_env()`
//! defaults to a `Disabled` provider that logs instead of sending, so on dev
//! environments without SMTP the worker just iterates and produces info logs
//! rather than failing.

use sqlx::PgPool;
use std::time::Duration;
use tracing::{error, info};

use crate::handlers::token_rotation::send_rotation_reminders;

/// One day. Long-lived tokens don't need a tighter cadence than this — even a
/// week would be defensible — but daily keeps the per-tick work small and
/// makes the reminder land within 24h of crossing the threshold.
const REMINDER_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Default rotation age threshold. Matches the value baked into
/// `tokens.rs::list_tokens` and the `needs_rotation` helper so that the
/// "needs rotation" badge in the UI and the reminder email agree.
const DEFAULT_THRESHOLD_DAYS: i64 = 90;

/// Spawn the rotation-reminder worker. Runs the first sweep one tick in (so
/// startup isn't blocked by an SMTP round-trip storm) and every 24h after.
pub fn start_token_rotation_reminders_worker(pool: PgPool) {
    let threshold_days = std::env::var("TOKEN_ROTATION_THRESHOLD_DAYS")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(DEFAULT_THRESHOLD_DAYS);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(REMINDER_INTERVAL);
        // First tick fires immediately; skip it so the worker doesn't blast
        // every old token the moment the server boots (especially after a
        // crash-loop restart).
        interval.tick().await;

        loop {
            interval.tick().await;

            match send_rotation_reminders(&pool, threshold_days).await {
                Ok(count) if count > 0 => {
                    info!(
                        "Token rotation reminders: sent {} reminder(s) for tokens older than {} days",
                        count, threshold_days
                    );
                }
                Ok(_) => {
                    tracing::debug!(
                        "Token rotation reminders: no tokens needed reminding (threshold {} days)",
                        threshold_days
                    );
                }
                Err(e) => {
                    error!("Token rotation reminders failed: {:?}", e);
                }
            }
        }
    });

    info!(
        "Token rotation reminder worker started (runs every 24h, threshold {} days)",
        threshold_days
    );
}
