//! Initialize pillar tracking with analytics database adapter
//!
//! This module connects the pillar tracking system in mockforge-core
//! to the analytics database for recording pillar usage events.

use mockforge_analytics::{AnalyticsDatabase, PillarUsageEvent as AnalyticsPillarUsageEvent, Pillar as AnalyticsPillar};
use mockforge_core::pillar_tracking::{PillarUsageEvent, PillarUsageRecorder};
use std::sync::Arc;

/// Adapter that implements PillarUsageRecorder for AnalyticsDatabase
pub struct AnalyticsPillarRecorder {
    db: Arc<AnalyticsDatabase>,
}

impl AnalyticsPillarRecorder {
    /// Create a new adapter
    pub fn new(db: Arc<AnalyticsDatabase>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl PillarUsageRecorder for AnalyticsPillarRecorder {
    async fn record(&self, event: PillarUsageEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Convert from core PillarUsageEvent to analytics PillarUsageEvent
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

/// Initialize pillar tracking with analytics database
pub async fn init_pillar_tracking(analytics_db: Option<Arc<AnalyticsDatabase>>) {
    if let Some(db) = analytics_db {
        let recorder = Arc::new(AnalyticsPillarRecorder::new(db));
        mockforge_core::pillar_tracking::init(recorder).await;
        tracing::info!("Pillar tracking initialized with analytics database");
    } else {
        tracing::debug!("Pillar tracking not initialized (analytics database not available)");
    }
}
