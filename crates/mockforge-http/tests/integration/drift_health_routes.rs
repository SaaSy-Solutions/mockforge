//! Real HTTP compatibility coverage for the extracted drift and timeline handlers.
#![allow(deprecated)]

use axum::Router;
use mockforge_core::contract_drift::{DriftBudget, DriftBudgetConfig, DriftBudgetEngine};
use mockforge_core::incidents::{
    IncidentManager, IncidentSeverity, IncidentStatus, IncidentStore, IncidentType,
    SemanticIncidentManager,
};
use mockforge_http::handlers::contract_health::{contract_health_router, ContractHealthState};
use mockforge_http::handlers::drift_budget::{drift_budget_router, DriftBudgetState};
use serde_json::{json, Value};
use std::sync::Arc;

struct Server(tokio::task::JoinHandle<std::io::Result<()>>);
impl Drop for Server {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn serve(
    config: DriftBudgetConfig,
    manager: Arc<IncidentManager>,
) -> (Server, String, reqwest::Client) {
    serve_with_semantic(config, manager, Arc::new(SemanticIncidentManager::new())).await
}

async fn serve_with_semantic(
    config: DriftBudgetConfig,
    manager: Arc<IncidentManager>,
    semantic: Arc<SemanticIncidentManager>,
) -> (Server, String, reqwest::Client) {
    // Struct literals retain the existing public API under both database feature states.
    let drift = DriftBudgetState {
        engine: Arc::new(DriftBudgetEngine::new(config)),
        incident_manager: manager.clone(),
        gitops_handler: None,
    };
    let health = ContractHealthState {
        incident_manager: manager,
        semantic_manager: semantic,
        #[cfg(feature = "database")]
        database: None,
    };
    let app = Router::new()
        .merge(drift_budget_router(drift))
        .merge(contract_health_router(health));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = Server(tokio::spawn(async move { axum::serve(listener, app).await }));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    (server, url, client)
}

fn manager() -> Arc<IncidentManager> {
    Arc::new(IncidentManager::new(Arc::new(IncidentStore::new())))
}
fn budget(max: u32) -> DriftBudget {
    DriftBudget {
        max_breaking_changes: max,
        ..Default::default()
    }
}

#[tokio::test]
async fn existing_budget_routes_preserve_precedence_and_missing_results() {
    let mut config = DriftBudgetConfig {
        default_budget: Some(budget(5)),
        ..Default::default()
    };
    config.per_endpoint_budgets.insert("GET /payments".into(), budget(4));
    config.per_tag_budgets.insert("billing".into(), budget(3));
    config.per_service_budgets.insert("payments".into(), budget(2));
    config.per_workspace_budgets.insert("workspace-a".into(), budget(1));
    let (_server, base, client) = serve(config, manager()).await;
    for (query, expected) in [
        ("endpoint=/payments&method=GET&workspace_id=workspace-a&service_name=payments&tags=billing", 1),
        ("endpoint=/payments&method=GET&service_name=payments&tags=billing", 2),
        ("endpoint=/payments&method=GET&tags=other,%20billing", 3),
        ("endpoint=/payments&method=GET", 4),
        ("endpoint=/unknown&method=GET", 5),
    ] {
        let response = client.get(format!("{base}/api/v1/drift/budgets/lookup?{query}")).send().await.unwrap();
        assert!(response.status().is_success());
        let result: Value = response.json().await.unwrap();
        assert_eq!(result["budget"]["max_breaking_changes"], expected);
    }
    let list: Value = client
        .get(format!("{base}/api/v1/drift/budgets"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(list["total"], 1);
    assert_eq!(list["budgets"][0]["id"], "GET /payments");
    let response = client
        .get(format!("{base}/api/v1/drift/budgets/GET%20%2Fpayments"))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    assert_eq!(response.json::<Value>().await.unwrap()["budget"]["max_breaking_changes"], 4);
    assert_eq!(
        client
            .get(format!("{base}/api/v1/drift/budgets/missing"))
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::NOT_FOUND
    );
    assert_eq!(
        client
            .post(format!("{base}/api/v1/drift/gitops/generate-pr"))
            .json(&json!({}))
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn incident_lifecycle_uses_the_original_core_manager() {
    let manager = manager();
    let incident = manager
        .create_incident(
            "/payments".into(),
            "GET".into(),
            IncidentType::ThresholdExceeded,
            IncidentSeverity::High,
            json!({"source":"local-fixture"}),
            None,
            Some("workspace-a".into()),
        )
        .await;
    let (_server, base, client) = serve(DriftBudgetConfig::default(), manager.clone()).await;
    let url = format!("{base}/api/v1/drift/incidents/{}", incident.id);
    let invalid = client.patch(&url).json(&json!({"status":"invalid"})).send().await.unwrap();
    assert_eq!(invalid.status(), reqwest::StatusCode::BAD_REQUEST);
    assert_eq!(manager.get_incident(&incident.id).await.unwrap().status, IncidentStatus::Open);
    let updated = client.patch(&url).json(&json!({"status":"acknowledged","external_ticket_id":"T-1","external_ticket_url":"https://tickets.example.test/T-1"})).send().await.unwrap();
    assert!(updated.status().is_success());
    let stored = manager.get_incident(&incident.id).await.unwrap();
    assert_eq!(stored.status, IncidentStatus::Acknowledged);
    assert_eq!(stored.external_ticket_id.as_deref(), Some("T-1"));
    assert_eq!(stored.external_ticket_url.as_deref(), Some("https://tickets.example.test/T-1"));
    assert!(client
        .post(format!("{url}/resolve"))
        .json(&json!({"note":"fixture resolution"}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    assert_eq!(
        manager.get_incident(&incident.id).await.unwrap().status,
        IncidentStatus::Resolved
    );
    assert!(client
        .patch(&url)
        .json(&json!({"status":"closed"}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    assert_eq!(manager.get_incident(&incident.id).await.unwrap().status, IncidentStatus::Closed);
    assert_eq!(
        client
            .get(format!("{base}/api/v1/drift/incidents/missing"))
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::NOT_FOUND
    );
    let stats: Value = client
        .get(format!("{base}/api/v1/drift/incidents/stats"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(stats["stats"]["closed"], 1);
}

#[tokio::test]
async fn timeline_and_incident_filters_share_existing_structural_records() {
    let manager = manager();
    let expected = manager
        .create_incident(
            "/payments".into(),
            "GET".into(),
            IncidentType::ThresholdExceeded,
            IncidentSeverity::High,
            json!({"visible":"payments"}),
            None,
            Some("workspace-a".into()),
        )
        .await;
    manager
        .create_incident(
            "/orders".into(),
            "POST".into(),
            IncidentType::ThresholdExceeded,
            IncidentSeverity::Low,
            json!({"visible":"orders"}),
            None,
            Some("workspace-b".into()),
        )
        .await;
    let (_server, base, client) = serve(DriftBudgetConfig::default(), manager).await;
    let list: Value = client.get(format!("{base}/api/v1/drift/incidents?workspace_id=workspace-a&endpoint=/payments&method=GET&status=open&severity=high")).send().await.unwrap().json().await.unwrap();
    assert_eq!(list["total"], 1);
    assert_eq!(list["incidents"][0]["id"], expected.id);
    let timeline: Value = client.get(format!("{base}/api/v1/contract-health/timeline?workspace_id=workspace-a&endpoint=/payments&method=GET&event_type=structural&limit=1")).send().await.unwrap().json().await.unwrap();
    assert_eq!(timeline["total"], 1);
    assert_eq!(timeline["events"][0]["id"], expected.id);
    assert_eq!(timeline["events"][0]["type"], "structural_drift");
    assert_eq!(timeline["events"][0]["details"]["visible"], "payments");
    let semantic: Value = client
        .get(format!("{base}/api/v1/contract-health/timeline?event_type=semantic"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(semantic["total"], 0);
}

#[tokio::test]
async fn populated_semantic_timeline_preserves_filters_scores_and_combined_order() {
    use mockforge_foundation::contract_diff_types::{SemanticChangeType, SemanticDriftResult};
    use mockforge_intelligence::incidents::SemanticIncident;

    let structural = manager();
    let drift = structural
        .create_incident(
            "/payments".into(),
            "GET".into(),
            IncidentType::ThresholdExceeded,
            IncidentSeverity::High,
            json!({}),
            None,
            Some("workspace-a".into()),
        )
        .await;
    let semantic = Arc::new(SemanticIncidentManager::new());
    let result = SemanticDriftResult {
        semantic_confidence: 0.9,
        soft_breaking_score: 0.85,
        change_type: SemanticChangeType::EnumNarrowing,
        llm_analysis: json!({"source":"synthetic-local-fixture"}),
        before_semantic_state: json!({"enum":["a","b"]}),
        after_semantic_state: json!({"enum":["a"]}),
        semantic_mismatches: Vec::new(),
    };
    let mut expected = SemanticIncident::from_drift_result(
        &result,
        "/payments".into(),
        "GET".into(),
        Some("workspace-a".into()),
        Some(drift.id.clone()),
        None,
    );
    expected.detected_at = drift.detected_at + 50;
    semantic.update_incident(expected.clone()).await;
    for (workspace, endpoint, method) in [
        ("workspace-b", "/payments", "GET"),
        ("workspace-a", "/orders", "GET"),
        ("workspace-a", "/payments", "POST"),
    ] {
        let incident = SemanticIncident::from_drift_result(
            &result,
            endpoint.into(),
            method.into(),
            Some(workspace.into()),
            None,
            None,
        );
        semantic.update_incident(incident).await;
    }
    let (_server, base, client) =
        serve_with_semantic(DriftBudgetConfig::default(), structural, semantic).await;
    let filtered = format!("{base}/api/v1/contract-health/timeline?workspace_id=workspace-a&endpoint=/payments&method=GET");
    let response: Value = client.get(&filtered).send().await.unwrap().json().await.unwrap();
    assert_eq!(response["total"], 2);
    assert_eq!(response["events"][0]["id"], expected.id);
    assert_eq!(response["events"][1]["id"], drift.id);
    assert_eq!(response["events"][0]["change_type"], "EnumNarrowing");
    assert_eq!(response["events"][0]["semantic_confidence"], 0.9);
    assert_eq!(response["events"][0]["soft_breaking_score"], 0.85);
    for (kind, id) in [("semantic", &expected.id), ("structural", &drift.id)] {
        let response: Value = client
            .get(format!("{filtered}&event_type={kind}"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(response["total"], 1);
        assert_eq!(response["events"][0]["id"], *id);
    }
    let limited: Value = client
        .get(format!("{filtered}&limit=1"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(limited["events"].as_array().unwrap().len(), 1);
    assert_eq!(limited["events"][0]["id"], expected.id);
}
