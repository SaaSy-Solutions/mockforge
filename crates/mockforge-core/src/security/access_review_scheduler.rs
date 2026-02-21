//! Access review scheduler for automated review execution
//!
//! This module provides a scheduler that automatically runs access reviews
//! based on configured frequencies and schedules.

use crate::security::access_review::{AccessReviewConfig, ReviewFrequency, ReviewType};
use crate::security::access_review_notifications::AccessReviewNotificationService;
use crate::security::access_review_service::{is_review_due, AccessReviewService};
use crate::Error;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration as TokioDuration};
use tracing::{debug, error, info, warn};

/// Review schedule tracking
#[derive(Debug, Clone)]
struct ReviewSchedule {
    /// Review type
    review_type: ReviewType,
    /// Review frequency
    frequency: ReviewFrequency,
    /// Last review date
    last_review: Option<DateTime<Utc>>,
    /// Next scheduled review date
    next_review: DateTime<Utc>,
}

/// Access review scheduler
///
/// This scheduler automatically runs access reviews based on configured
/// frequencies. It checks for due reviews periodically and starts them.
pub struct AccessReviewScheduler {
    service: Arc<RwLock<AccessReviewService>>,
    config: AccessReviewConfig,
    schedules: Arc<RwLock<HashMap<ReviewType, ReviewSchedule>>>,
    /// Whether the scheduler is running
    running: Arc<RwLock<bool>>,
    /// Notification service (optional)
    notification_service: Option<Arc<AccessReviewNotificationService>>,
}

impl AccessReviewScheduler {
    /// Create a new access review scheduler
    pub fn new(service: Arc<RwLock<AccessReviewService>>, config: AccessReviewConfig) -> Self {
        Self::with_notifications(service, config, None)
    }

    /// Create a new access review scheduler with notification service
    pub fn with_notifications(
        service: Arc<RwLock<AccessReviewService>>,
        config: AccessReviewConfig,
        notification_service: Option<Arc<AccessReviewNotificationService>>,
    ) -> Self {
        let mut schedules = HashMap::new();
        let now = Utc::now();

        // Initialize schedules based on config
        if config.user_review.enabled {
            let frequency = config.user_review.frequency;
            schedules.insert(
                ReviewType::UserAccess,
                ReviewSchedule {
                    review_type: ReviewType::UserAccess,
                    frequency,
                    last_review: None,
                    next_review: now + frequency.duration(),
                },
            );
        }

        if config.privileged_review.enabled {
            let frequency = config.privileged_review.frequency;
            schedules.insert(
                ReviewType::PrivilegedAccess,
                ReviewSchedule {
                    review_type: ReviewType::PrivilegedAccess,
                    frequency,
                    last_review: None,
                    next_review: now + frequency.duration(),
                },
            );
        }

        if config.token_review.enabled {
            let frequency = config.token_review.frequency;
            schedules.insert(
                ReviewType::ApiToken,
                ReviewSchedule {
                    review_type: ReviewType::ApiToken,
                    frequency,
                    last_review: None,
                    next_review: now + frequency.duration(),
                },
            );
        }

        if config.resource_review.enabled {
            let frequency = config.resource_review.frequency;
            schedules.insert(
                ReviewType::ResourceAccess,
                ReviewSchedule {
                    review_type: ReviewType::ResourceAccess,
                    frequency,
                    last_review: None,
                    next_review: now + frequency.duration(),
                },
            );
        }

        Self {
            service,
            config,
            schedules: Arc::new(RwLock::new(schedules)),
            running: Arc::new(RwLock::new(false)),
            notification_service,
        }
    }

    /// Start the scheduler
    ///
    /// This spawns a background task that periodically checks for due reviews
    /// and starts them automatically.
    pub fn start(&self) -> tokio::task::JoinHandle<()> {
        let schedules = self.schedules.clone();
        let service = self.service.clone();
        let running = self.running.clone();
        let config = self.config.clone();
        let notification_service = self.notification_service.clone();

        // Mark as running (spawn a task to do this)
        let running_init = running.clone();
        tokio::spawn(async move {
            *running_init.write().await = true;
        });

        tokio::spawn(async move {
            // Check every hour for due reviews
            let mut interval = interval(TokioDuration::from_secs(3600));

            loop {
                interval.tick().await;

                // Check if still running
                if !*running.read().await {
                    debug!("Access review scheduler stopped");
                    break;
                }

                // Check each scheduled review type
                let schedules_guard = schedules.read().await;
                let mut reviews_to_run = Vec::new();

                for (review_type, schedule) in schedules_guard.iter() {
                    if is_review_due(schedule.frequency, schedule.last_review) {
                        reviews_to_run.push(*review_type);
                    }
                }
                drop(schedules_guard);

                // Run due reviews
                for review_type in reviews_to_run {
                    let mut service_guard = service.write().await;
                    let result = match review_type {
                        ReviewType::UserAccess => {
                            if config.user_review.enabled {
                                service_guard.start_user_access_review().await
                            } else {
                                continue;
                            }
                        }
                        ReviewType::PrivilegedAccess => {
                            if config.privileged_review.enabled {
                                service_guard.start_privileged_access_review().await
                            } else {
                                continue;
                            }
                        }
                        ReviewType::ApiToken => {
                            if config.token_review.enabled {
                                service_guard.start_token_review().await
                            } else {
                                continue;
                            }
                        }
                        ReviewType::ResourceAccess => {
                            if config.resource_review.enabled {
                                service_guard.start_resource_access_review(Vec::new()).await
                            } else {
                                continue;
                            }
                        }
                    };

                    match result {
                        Ok(review_id) => {
                            info!(
                                "Started automatic {} review: {}",
                                format!("{:?}", review_type),
                                review_id
                            );

                            // Send notification if configured
                            if let Some(ref notif_service) = notification_service {
                                let service_guard = service.read().await;
                                if let Some(review) = service_guard.get_review(&review_id) {
                                    if let Err(e) =
                                        notif_service.notify_review_started(review).await
                                    {
                                        warn!("Failed to send review notification: {}", e);
                                    }
                                }
                            }

                            // Update schedule
                            let mut schedules_guard = schedules.write().await;
                            if let Some(schedule) = schedules_guard.get_mut(&review_type) {
                                let now = Utc::now();
                                schedule.last_review = Some(now);
                                schedule.next_review = schedule.frequency.next_review_date(now);
                            }
                        }
                        Err(e) => {
                            error!(
                                "Failed to start automatic {} review: {}",
                                format!("{:?}", review_type),
                                e
                            );
                        }
                    }
                }

                // Check for auto-revocations
                let mut service_guard = service.write().await;
                match service_guard.check_auto_revocations().await {
                    Ok(revoked) => {
                        if !revoked.is_empty() {
                            info!(
                                "Auto-revoked {} user(s) due to expired approvals",
                                revoked.len()
                            );

                            // Send notifications for auto-revocations
                            if let Some(ref notif_service) = notification_service {
                                for (review_id, user_id) in &revoked {
                                    if let Some(review_item) = service_guard
                                        .engine()
                                        .get_review_items(review_id)
                                        .and_then(|items| items.get(user_id))
                                    {
                                        let reason = review_item
                                            .rejection_reason
                                            .clone()
                                            .unwrap_or_else(|| {
                                                "Auto-revoked due to missing approval".to_string()
                                            });

                                        if let Err(e) = notif_service
                                            .notify_auto_revocation(review_id, *user_id, &reason)
                                            .await
                                        {
                                            warn!(
                                                "Failed to send auto-revocation notification: {}",
                                                e
                                            );
                                        }
                                    }
                                }
                            }

                            for (review_id, user_id) in revoked {
                                debug!("Auto-revoked user {} in review {}", user_id, review_id);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to check auto-revocations: {}", e);
                    }
                }
            }
        })
    }

    /// Stop the scheduler
    pub async fn stop(&self) {
        *self.running.write().await = false;
        info!("Access review scheduler stopping...");
    }

    /// Get the next review date for a review type
    pub async fn get_next_review_date(&self, review_type: ReviewType) -> Option<DateTime<Utc>> {
        let schedules = self.schedules.read().await;
        schedules.get(&review_type).map(|s| s.next_review)
    }

    /// Get all schedules
    pub async fn get_schedules(&self) -> Vec<(ReviewType, DateTime<Utc>, Option<DateTime<Utc>>)> {
        let schedules = self.schedules.read().await;
        schedules.iter().map(|(t, s)| (*t, s.next_review, s.last_review)).collect()
    }

    /// Manually trigger a review (for testing or manual execution)
    pub async fn trigger_review(&self, review_type: ReviewType) -> Result<String, Error> {
        let mut service = self.service.write().await;

        let review_id = match review_type {
            ReviewType::UserAccess => {
                if !self.config.user_review.enabled {
                    return Err(Error::Generic("User access review is not enabled".to_string()));
                }
                service.start_user_access_review().await?
            }
            ReviewType::PrivilegedAccess => {
                if !self.config.privileged_review.enabled {
                    return Err(Error::Generic(
                        "Privileged access review is not enabled".to_string(),
                    ));
                }
                service.start_privileged_access_review().await?
            }
            ReviewType::ApiToken => {
                if !self.config.token_review.enabled {
                    return Err(Error::Generic("API token review is not enabled".to_string()));
                }
                service.start_token_review().await?
            }
            ReviewType::ResourceAccess => {
                if !self.config.resource_review.enabled {
                    return Err(Error::Generic(
                        "Resource access review is not enabled".to_string(),
                    ));
                }
                service.start_resource_access_review(Vec::new()).await?
            }
        };

        // Update schedule
        let mut schedules = self.schedules.write().await;
        if let Some(schedule) = schedules.get_mut(&review_type) {
            let now = Utc::now();
            schedule.last_review = Some(now);
            schedule.next_review = schedule.frequency.next_review_date(now);
        }

        Ok(review_id)
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_scheduler_creation() {
        // This test would require a mock UserDataProvider
        // For now, just verify the structure compiles
    }
}
