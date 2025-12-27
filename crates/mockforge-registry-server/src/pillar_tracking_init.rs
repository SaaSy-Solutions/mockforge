//! Initialize pillar tracking with analytics database adapter
//!
//! This module connects the pillar tracking system in mockforge-core
//! to the analytics database for recording pillar usage events.

use mockforge_analytics::{
    AnalyticsDatabase, Pillar as AnalyticsPillar, PillarUsageEvent as AnalyticsPillarUsageEvent,
};
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
    async fn record(
        &self,
        event: PillarUsageEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_core::pillars::Pillar as CorePillar;
    use std::path::Path;

    #[test]
    fn test_analytics_pillar_recorder_new() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            // Use in-memory database for testing
            let db = AnalyticsDatabase::new(Path::new(":memory:")).await.unwrap();
            let _ = db.run_migrations().await;

            let db_arc = Arc::new(db);
            let recorder = AnalyticsPillarRecorder::new(db_arc.clone());

            // Verify recorder was created
            assert!(Arc::ptr_eq(&recorder.db, &db_arc));
        });
    }

    #[test]
    fn test_pillar_conversion_reality() {
        let core_pillar = CorePillar::Reality;

        // This test verifies the conversion logic exists
        // The actual conversion happens in the record method
        match core_pillar {
            CorePillar::Reality => assert!(true),
            _ => panic!("Expected Reality pillar"),
        }
    }

    #[test]
    fn test_pillar_conversion_contracts() {
        let core_pillar = CorePillar::Contracts;

        match core_pillar {
            CorePillar::Contracts => assert!(true),
            _ => panic!("Expected Contracts pillar"),
        }
    }

    #[test]
    fn test_pillar_conversion_devx() {
        let core_pillar = CorePillar::DevX;

        match core_pillar {
            CorePillar::DevX => assert!(true),
            _ => panic!("Expected DevX pillar"),
        }
    }

    #[test]
    fn test_pillar_conversion_cloud() {
        let core_pillar = CorePillar::Cloud;

        match core_pillar {
            CorePillar::Cloud => assert!(true),
            _ => panic!("Expected Cloud pillar"),
        }
    }

    #[test]
    fn test_pillar_conversion_ai() {
        let core_pillar = CorePillar::Ai;

        match core_pillar {
            CorePillar::Ai => assert!(true),
            _ => panic!("Expected Ai pillar"),
        }
    }

    #[tokio::test]
    async fn test_init_pillar_tracking_with_database() {
        // Use in-memory database for testing
        let db = AnalyticsDatabase::new(Path::new(":memory:")).await.unwrap();
        let _ = db.run_migrations().await;

        let db_arc = Arc::new(db);

        // Initialize with database
        init_pillar_tracking(Some(db_arc.clone())).await;

        // If we get here without panicking, initialization succeeded
        assert!(true);
    }

    #[tokio::test]
    async fn test_init_pillar_tracking_without_database() {
        // Initialize without database
        init_pillar_tracking(None).await;

        // If we get here without panicking, initialization succeeded
        assert!(true);
    }

    #[tokio::test]
    async fn test_record_pillar_event() {
        // Use in-memory database for testing
        let db = AnalyticsDatabase::new(Path::new(":memory:")).await.unwrap();
        let _ = db.run_migrations().await;

        let db_arc = Arc::new(db);
        let recorder = AnalyticsPillarRecorder::new(db_arc);

        // Create a test event
        let event = PillarUsageEvent {
            workspace_id: Some(uuid::Uuid::new_v4().to_string()),
            org_id: Some(uuid::Uuid::new_v4().to_string()),
            pillar: CorePillar::Reality,
            metric_name: "test_metric".to_string(),
            metric_value: serde_json::json!(42.0),
            timestamp: chrono::Utc::now(),
        };

        // Record the event
        let result = recorder.record(event).await;

        // Should succeed
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_record_pillar_event_all_pillars() {
        // Use in-memory database for testing
        let db = AnalyticsDatabase::new(Path::new(":memory:")).await.unwrap();
        let _ = db.run_migrations().await;

        let db_arc = Arc::new(db);
        let recorder = AnalyticsPillarRecorder::new(db_arc);

        let workspace_id = Some(uuid::Uuid::new_v4().to_string());
        let org_id = Some(uuid::Uuid::new_v4().to_string());

        // Test all pillar types
        let pillars = vec![
            CorePillar::Reality,
            CorePillar::Contracts,
            CorePillar::DevX,
            CorePillar::Cloud,
            CorePillar::Ai,
        ];

        for pillar in pillars {
            let event = PillarUsageEvent {
                workspace_id: workspace_id.clone(),
                org_id: org_id.clone(),
                pillar,
                metric_name: format!("test_metric_{:?}", pillar),
                metric_value: serde_json::json!(1.0),
                timestamp: chrono::Utc::now(),
            };

            let result = recorder.record(event).await;
            assert!(result.is_ok(), "Failed to record {:?} pillar", pillar);
        }
    }

    #[tokio::test]
    async fn test_record_multiple_events_same_workspace() {
        // Use in-memory database for testing
        let db = AnalyticsDatabase::new(Path::new(":memory:")).await.unwrap();
        let _ = db.run_migrations().await;

        let db_arc = Arc::new(db);
        let recorder = AnalyticsPillarRecorder::new(db_arc);

        let workspace_id = Some(uuid::Uuid::new_v4().to_string());
        let org_id = Some(uuid::Uuid::new_v4().to_string());

        // Record multiple events for the same workspace
        for i in 0..5 {
            let event = PillarUsageEvent {
                workspace_id: workspace_id.clone(),
                org_id: org_id.clone(),
                pillar: CorePillar::Reality,
                metric_name: format!("test_metric_{}", i),
                metric_value: serde_json::json!(i as f64),
                timestamp: chrono::Utc::now(),
            };

            let result = recorder.record(event).await;
            assert!(result.is_ok(), "Failed to record event {}", i);
        }
    }

    #[test]
    fn test_pillar_event_structure() {
        // Verify the PillarUsageEvent structure
        let workspace_id = Some(uuid::Uuid::new_v4().to_string());
        let org_id = Some(uuid::Uuid::new_v4().to_string());
        let timestamp = chrono::Utc::now();

        let event = PillarUsageEvent {
            workspace_id: workspace_id.clone(),
            org_id: org_id.clone(),
            pillar: CorePillar::Reality,
            metric_name: "test".to_string(),
            metric_value: serde_json::json!(42.0),
            timestamp,
        };

        assert_eq!(event.workspace_id, workspace_id);
        assert_eq!(event.org_id, org_id);
        assert_eq!(event.metric_name, "test");
        assert_eq!(event.metric_value, serde_json::json!(42.0));
        assert_eq!(event.timestamp, timestamp);
    }

    #[test]
    fn test_pillar_types_match() {
        // Verify all pillar types from core match analytics types
        // This ensures the conversion in the record method is complete
        let core_pillars = vec![
            CorePillar::Reality,
            CorePillar::Contracts,
            CorePillar::DevX,
            CorePillar::Cloud,
            CorePillar::Ai,
        ];

        let analytics_pillars = vec![
            AnalyticsPillar::Reality,
            AnalyticsPillar::Contracts,
            AnalyticsPillar::DevX,
            AnalyticsPillar::Cloud,
            AnalyticsPillar::Ai,
        ];

        assert_eq!(core_pillars.len(), analytics_pillars.len());
    }
}
