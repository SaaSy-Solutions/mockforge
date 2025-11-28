//! Risk review scheduler for automated risk review execution
//!
//! This module provides a scheduler that automatically reviews risks
//! based on their review frequency and schedules.

use crate::security::risk_assessment::{Risk, RiskAssessmentEngine, RiskReviewFrequency};
use crate::Error;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration as TokioDuration};
use tracing::{debug, error, info, warn};

/// Risk review scheduler
///
/// This scheduler automatically reviews risks based on their configured
/// review frequencies. It checks for due reviews periodically and reviews them.
pub struct RiskReviewScheduler {
    engine: Arc<RwLock<RiskAssessmentEngine>>,
    /// Whether the scheduler is running
    running: Arc<RwLock<bool>>,
    /// System user ID for automated reviews
    system_user_id: uuid::Uuid,
}

impl RiskReviewScheduler {
    /// Create a new risk review scheduler
    pub fn new(engine: Arc<RwLock<RiskAssessmentEngine>>) -> Self {
        Self {
            engine,
            running: Arc::new(RwLock::new(false)),
            // Use a fixed system UUID for automated reviews
            system_user_id: uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001")
                .expect("Invalid system UUID"),
        }
    }

    /// Start the risk review scheduler
    ///
    /// This spawns a background task that periodically checks for risks
    /// due for review and automatically reviews them.
    pub fn start(&self) -> tokio::task::JoinHandle<()> {
        let engine = self.engine.clone();
        let running = self.running.clone();
        let system_user_id = self.system_user_id;

        // Mark as running
        let running_init = running.clone();
        tokio::spawn(async move {
            *running_init.write().await = true;
        });

        tokio::spawn(async move {
            // Check every 6 hours for due reviews
            let mut interval = interval(TokioDuration::from_secs(6 * 3600));

            loop {
                interval.tick().await;

                // Check if still running
                if !*running.read().await {
                    debug!("Risk review scheduler stopped");
                    break;
                }

                // Get risks due for review
                let risks_due = {
                    let engine_guard = engine.read().await;
                    match engine_guard.get_risks_due_for_review().await {
                        Ok(risks) => risks,
                        Err(e) => {
                            error!("Failed to get risks due for review: {}", e);
                            continue;
                        }
                    }
                };

                if risks_due.is_empty() {
                    debug!("No risks due for review");
                    continue;
                }

                info!(
                    "Found {} risk(s) due for review, starting automated review",
                    risks_due.len()
                );

                // Review each risk
                for risk in risks_due {
                    let risk_id = risk.risk_id.clone();
                    let mut engine_guard = engine.write().await;

                    match engine_guard.review_risk(&risk_id, system_user_id).await {
                        Ok(()) => {
                            info!("Automated review completed for risk {}", risk_id);
                        }
                        Err(e) => {
                            error!("Failed to review risk {}: {}", risk_id, e);
                        }
                    }
                }
            }
        })
    }

    /// Stop the risk review scheduler
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        info!("Risk review scheduler stopping...");
    }

    /// Check if the scheduler is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}
