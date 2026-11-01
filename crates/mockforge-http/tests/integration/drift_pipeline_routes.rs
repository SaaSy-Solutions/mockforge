//! Extracted GitOps route behavior through the real HTTP pipeline adapter.
#![cfg(feature = "pipelines")]
#![allow(deprecated)]

use async_trait::async_trait;
use mockforge_core::contract_drift::{DriftBudgetConfig, DriftBudgetEngine};
use mockforge_core::incidents::{IncidentManager, IncidentSeverity, IncidentStore, IncidentType};
use mockforge_foundation::incidents_types::DriftIncident;
use mockforge_http::handlers::drift_budget::{
    drift_budget_router, DriftBudgetState, DriftBudgetStateProvider, DriftGitOps,
};
use mockforge_intelligence::pr_generation::PRResult;
use mockforge_pipelines::{events::get_global_event_bus, PipelineEventType};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy)]
enum Outcome {
    Created,
    Empty,
    Failed,
}

#[derive(Clone)]
struct GitOpsFixture {
    outcome: Outcome,
    selected: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl DriftGitOps for GitOpsFixture {
    async fn generate_pr_from_incidents(
        &self,
        incidents: &[DriftIncident],
    ) -> mockforge_foundation::Result<Option<PRResult>> {
        *self.selected.lock().unwrap() = incidents.iter().map(|i| i.id.clone()).collect();
        match self.outcome {
            Outcome::Created => Ok(Some(PRResult {
                number: 7,
                url: "https://example.invalid/pull/7".into(),
                branch: "local-fixture".into(),
                title: "Local drift fixture".into(),
            })),
            Outcome::Empty => Ok(None),
            Outcome::Failed => Err(mockforge_foundation::Error::generic("local fixture failure")),
        }
    }
}

#[derive(Clone)]
struct StateFixture {
    original: DriftBudgetState,
    gitops: GitOpsFixture,
}

impl DriftBudgetStateProvider for StateFixture {
    type Engine = DriftBudgetEngine;
    type Incidents = IncidentManager;
    type GitOps = GitOpsFixture;

    fn engine(&self) -> &Self::Engine {
        self.original.engine.as_ref()
    }
    fn incident_manager(&self) -> &Self::Incidents {
        self.original.incident_manager.as_ref()
    }
    fn gitops_handler(&self) -> Option<&Self::GitOps> {
        Some(&self.gitops)
    }
    fn publish_drift_threshold(
        &self,
        workspace: uuid::Uuid,
        endpoint: String,
        exceeded: i32,
        total: i32,
    ) {
        self.original.publish_drift_threshold(workspace, endpoint, exceeded, total);
    }
}

struct Server(tokio::task::JoinHandle<std::io::Result<()>>);
impl Drop for Server {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[tokio::test]
async fn gitops_results_publish_only_successful_threshold_events() {
    let workspace = uuid::Uuid::new_v4();
    let manager = Arc::new(IncidentManager::new(Arc::new(IncidentStore::new())));
    let threshold = manager
        .create_incident(
            "/payments".into(),
            "GET".into(),
            IncidentType::ThresholdExceeded,
            IncidentSeverity::High,
            json!({}),
            None,
            Some(workspace.to_string()),
        )
        .await;
    let breaking = manager
        .create_incident(
            "/orders".into(),
            "POST".into(),
            IncidentType::BreakingChange,
            IncidentSeverity::Low,
            json!({}),
            None,
            Some(uuid::Uuid::new_v4().to_string()),
        )
        .await;
    let mut receiver = get_global_event_bus().subscribe();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    for (outcome, request, expected_status, expected_id, emits) in [
        (
            Outcome::Created,
            json!({"incident_ids":[threshold.id]}),
            200,
            &threshold.id,
            true,
        ),
        (
            Outcome::Created,
            json!({"workspace_id":workspace,"status":"open"}),
            200,
            &threshold.id,
            true,
        ),
        (
            Outcome::Empty,
            json!({"incident_ids":[threshold.id]}),
            200,
            &threshold.id,
            false,
        ),
        (
            Outcome::Failed,
            json!({"incident_ids":[threshold.id]}),
            500,
            &threshold.id,
            false,
        ),
        (
            Outcome::Created,
            json!({"incident_ids":[breaking.id]}),
            200,
            &breaking.id,
            false,
        ),
    ] {
        let selected = Arc::new(Mutex::new(Vec::new()));
        let state = StateFixture {
            original: DriftBudgetState {
                engine: Arc::new(DriftBudgetEngine::new(DriftBudgetConfig::default())),
                incident_manager: manager.clone(),
                gitops_handler: None,
            },
            gitops: GitOpsFixture {
                outcome,
                selected: selected.clone(),
            },
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = drift_budget_router(state);
        let _server = Server(tokio::spawn(async move { axum::serve(listener, app).await }));
        let response = client
            .post(format!("http://{address}/api/v1/drift/gitops/generate-pr"))
            .json(&request)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), expected_status);
        assert_eq!(*selected.lock().unwrap(), vec![expected_id.clone()]);
        if expected_status == 200 {
            let body: Value = response.json().await.unwrap();
            assert_eq!(body["success"], matches!(outcome, Outcome::Created));
        }
        if emits {
            let event = receiver.try_recv().expect("published before the HTTP success response");
            assert_eq!(event.event_type, PipelineEventType::DriftThresholdExceeded);
            assert_eq!(event.workspace_id, Some(workspace));
            assert_eq!(event.payload["endpoint"], "GET /payments");
            assert_eq!(event.payload["drift_count"], 1);
            assert_eq!(event.payload["threshold"], 1);
        }
        assert!(matches!(
            receiver.try_recv(),
            Err(tokio::sync::broadcast::error::TryRecvError::Empty)
        ));
    }
}
