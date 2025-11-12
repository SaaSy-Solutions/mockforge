//! Background worker for cleaning up expired SAML assertion IDs
//!
//! This worker runs periodically to remove expired SAML assertion IDs from the database,
//! preventing the saml_assertion_ids table from growing indefinitely.

use crate::models::SAMLAssertionId;
use sqlx::PgPool;
use std::time::Duration;
use tracing::{error, info};

/// Start the SAML assertion cleanup worker
/// Runs cleanup every hour to remove expired assertion IDs
pub fn start_saml_cleanup_worker(pool: PgPool) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600)); // 1 hour

        // Run immediately on startup
        interval.tick().await;

        loop {
            interval.tick().await;

            match SAMLAssertionId::cleanup_expired(&pool).await {
                Ok(deleted_count) => {
                    if deleted_count > 0 {
                        info!("SAML cleanup: Removed {} expired assertion IDs", deleted_count);
                    } else {
                        tracing::debug!("SAML cleanup: No expired assertion IDs to remove");
                    }
                }
                Err(e) => {
                    error!("SAML cleanup error: Failed to remove expired assertion IDs: {:?}", e);
                }
            }
        }
    });

    info!("SAML assertion cleanup worker started (runs every hour)");
}
