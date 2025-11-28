//! Pillar usage tracking utilities
//!
//! Provides helper functions for recording pillar usage events throughout the codebase.
//! These events are used for analytics and understanding which pillars are most used.

use crate::pillars::Pillar;
use chrono::Utc;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Optional analytics database for recording pillar usage
/// This is set globally and can be None if analytics is not enabled
static ANALYTICS_DB: Lazy<Arc<RwLock<Option<Arc<dyn PillarUsageRecorder>>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Trait for recording pillar usage events
/// This allows different implementations (analytics DB, API endpoint, etc.)
#[async_trait::async_trait]
pub trait PillarUsageRecorder: Send + Sync {
    /// Record a pillar usage event
    async fn record(
        &self,
        event: PillarUsageEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Pillar usage event (simplified version for internal use)
#[derive(Debug, Clone)]
pub struct PillarUsageEvent {
    /// Workspace ID where the event occurred
    pub workspace_id: Option<String>,
    /// Organization ID (if applicable)
    pub org_id: Option<String>,
    /// The pillar this event relates to
    pub pillar: Pillar,
    /// Name of the metric being recorded
    pub metric_name: String,
    /// Value of the metric (JSON)
    pub metric_value: Value,
    /// Timestamp when the event occurred
    pub timestamp: chrono::DateTime<Utc>,
}

/// Initialize the pillar usage tracker with a recorder
pub async fn init(recorder: Arc<dyn PillarUsageRecorder>) {
    let mut db = ANALYTICS_DB.write().await;
    *db = Some(recorder);
}

/// Record a reality pillar usage event
///
/// This should be called when:
/// - Reality continuum blend ratio is used
/// - Smart personas are activated
/// - Chaos is enabled/used
/// - Reality level changes
pub async fn record_reality_usage(
    workspace_id: Option<String>,
    org_id: Option<String>,
    metric_name: &str,
    metric_value: Value,
) {
    record_pillar_usage(workspace_id, org_id, Pillar::Reality, metric_name, metric_value).await;
}

/// Record a contracts pillar usage event
///
/// This should be called when:
/// - Contract validation is performed
/// - Drift detection occurs
/// - Contract sync happens
/// - Validation mode changes
pub async fn record_contracts_usage(
    workspace_id: Option<String>,
    org_id: Option<String>,
    metric_name: &str,
    metric_value: Value,
) {
    record_pillar_usage(workspace_id, org_id, Pillar::Contracts, metric_name, metric_value).await;
}

/// Record a DevX pillar usage event
///
/// This should be called when:
/// - SDK is installed/used
/// - Client code is generated
/// - Playground session starts
/// - CLI command is executed
pub async fn record_devx_usage(
    workspace_id: Option<String>,
    org_id: Option<String>,
    metric_name: &str,
    metric_value: Value,
) {
    record_pillar_usage(workspace_id, org_id, Pillar::DevX, metric_name, metric_value).await;
}

/// Record a cloud pillar usage event
///
/// This should be called when:
/// - Scenario is shared
/// - Marketplace download occurs
/// - Workspace is created/shared
/// - Organization template is used
pub async fn record_cloud_usage(
    workspace_id: Option<String>,
    org_id: Option<String>,
    metric_name: &str,
    metric_value: Value,
) {
    record_pillar_usage(workspace_id, org_id, Pillar::Cloud, metric_name, metric_value).await;
}

/// Record an AI pillar usage event
///
/// This should be called when:
/// - AI mock generation occurs
/// - AI contract diff is performed
/// - Voice command is executed
/// - LLM-assisted operation happens
pub async fn record_ai_usage(
    workspace_id: Option<String>,
    org_id: Option<String>,
    metric_name: &str,
    metric_value: Value,
) {
    record_pillar_usage(workspace_id, org_id, Pillar::Ai, metric_name, metric_value).await;
}

/// Record a pillar usage event (internal helper)
async fn record_pillar_usage(
    workspace_id: Option<String>,
    org_id: Option<String>,
    pillar: Pillar,
    metric_name: &str,
    metric_value: Value,
) {
    let db = ANALYTICS_DB.read().await;
    if let Some(recorder) = db.as_ref() {
        let event = PillarUsageEvent {
            workspace_id,
            org_id,
            pillar,
            metric_name: metric_name.to_string(),
            metric_value,
            timestamp: Utc::now(),
        };

        // Record asynchronously without blocking
        let recorder = recorder.clone();
        tokio::spawn(async move {
            if let Err(e) = recorder.record(event).await {
                tracing::warn!("Failed to record pillar usage event: {}", e);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct TestRecorder {
        events: Arc<RwLock<Vec<PillarUsageEvent>>>,
    }

    #[async_trait::async_trait]
    impl PillarUsageRecorder for TestRecorder {
        async fn record(
            &self,
            event: PillarUsageEvent,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let mut events = self.events.write().await;
            events.push(event);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_record_reality_usage() {
        let events = Arc::new(RwLock::new(Vec::new()));
        let recorder = Arc::new(TestRecorder {
            events: events.clone(),
        });
        init(recorder).await;

        record_reality_usage(
            Some("workspace-1".to_string()),
            None,
            "blended_reality_ratio",
            json!({"ratio": 0.5}),
        )
        .await;

        // Give async task time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let recorded = events.read().await;
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].pillar, Pillar::Reality);
        assert_eq!(recorded[0].metric_name, "blended_reality_ratio");
    }
}
