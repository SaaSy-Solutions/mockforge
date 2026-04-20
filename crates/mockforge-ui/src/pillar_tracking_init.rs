//! Initialize pillar tracking with the analytics-database adapter.
//!
//! Bridges `mockforge_core::pillar_tracking` to `AnalyticsDatabase` so that
//! pillar-usage events emitted anywhere in the workspace are persisted to the
//! admin UI's analytics store. Mirrors the adapter in `mockforge-registry-server`
//! but is invoked whenever the admin UI's lazy `AnalyticsDatabase` OnceCell is
//! initialized.

use mockforge_analytics::{
    AnalyticsDatabase, Pillar as AnalyticsPillar, PillarUsageEvent as AnalyticsPillarUsageEvent,
};
use mockforge_core::pillar_tracking::{PillarUsageEvent, PillarUsageRecorder};
use std::sync::Arc;

pub struct AnalyticsPillarRecorder {
    db: Arc<AnalyticsDatabase>,
}

impl AnalyticsPillarRecorder {
    pub fn new(db: Arc<AnalyticsDatabase>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl PillarUsageRecorder for AnalyticsPillarRecorder {
    async fn record(
        &self,
        event: PillarUsageEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let pillar = match event.pillar {
            mockforge_core::pillars::Pillar::Reality => AnalyticsPillar::Reality,
            mockforge_core::pillars::Pillar::Contracts => AnalyticsPillar::Contracts,
            mockforge_core::pillars::Pillar::DevX => AnalyticsPillar::DevX,
            mockforge_core::pillars::Pillar::Cloud => AnalyticsPillar::Cloud,
            mockforge_core::pillars::Pillar::Ai => AnalyticsPillar::Ai,
        };

        let analytics_event = AnalyticsPillarUsageEvent {
            workspace_id: event.workspace_id,
            org_id: event.org_id,
            pillar,
            metric_name: event.metric_name,
            metric_value: event.metric_value,
            timestamp: event.timestamp,
        };

        self.db
            .record_pillar_usage(&analytics_event)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}

/// Install the analytics-backed pillar-usage recorder as the global sink.
pub async fn init_pillar_tracking(db: Arc<AnalyticsDatabase>) {
    let recorder = Arc::new(AnalyticsPillarRecorder::new(db));
    mockforge_core::pillar_tracking::init(recorder).await;
    tracing::info!("Pillar tracking initialized with analytics database");
}
