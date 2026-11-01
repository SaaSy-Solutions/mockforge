//! PostgreSQL timeline acceptance against the actual threat/forecast schemas.
#![cfg(feature = "database")]

use mockforge_core::incidents::{IncidentManager, IncidentStore, SemanticIncidentManager};
use mockforge_http::database::Database;
use mockforge_http::handlers::contract_health::{contract_health_router, ContractHealthState};
use serde_json::Value;
use std::sync::Arc;

struct Server(tokio::task::JoinHandle<std::io::Result<()>>);
impl Drop for Server {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[tokio::test]
#[ignore = "requires an explicitly owned disposable PostgreSQL fixture"]
async fn database_timeline_reads_existing_threat_and_forecast_rows() {
    let url = std::env::var("MOCKFORGE_TIMELINE_FIXTURE_DB")
        .expect("owned PostgreSQL fixture required; never use an account database");
    assert!(url.starts_with("postgresql://postgres@127.0.0.1:"));
    let database = Database::connect_optional_with_pool_size(Some(&url), Some(2)).await.unwrap();
    let pool = database.pool().unwrap();
    // The launcher supplies a fresh disposable database; these are the actual
    // append-only schema migrations, rather than a hand-written test schema.
    sqlx::raw_sql(include_str!("../../migrations/20250128000001_api_change_forecasts.sql"))
        .execute(pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../../migrations/20250128000004_contract_threats.sql"))
        .execute(pool)
        .await
        .unwrap();
    let workspace = uuid::Uuid::new_v4();
    let other_workspace = uuid::Uuid::new_v4();
    for scope in [Some(workspace), Some(other_workspace), None] {
        sqlx::query("INSERT INTO contract_threat_assessments (workspace_id, endpoint, method, aggregation_level, threat_level, threat_score, findings) VALUES ($1, '/payments', 'GET', 'endpoint', 'high', 0.8, '[{},{}]'::jsonb)")
            .bind(scope).execute(pool).await.unwrap();
        sqlx::query("INSERT INTO api_change_forecasts (workspace_id, endpoint, method, forecast_window_days, predicted_change_probability, predicted_break_probability, volatility_score, confidence, expires_at) VALUES ($1, '/payments', 'GET', 30, 0.7, 0.2, 0.4, 0.9, NOW() + INTERVAL '1 day')")
            .bind(scope).execute(pool).await.unwrap();
    }
    let app = contract_health_router(ContractHealthState {
        incident_manager: Arc::new(IncidentManager::new(Arc::new(IncidentStore::new()))),
        semantic_manager: Arc::new(SemanticIncidentManager::new()),
        database: Some(database),
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let _server = Server(tokio::spawn(async move { axum::serve(listener, app).await }));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    let response = client
        .get(format!(
            "http://{address}/api/v1/contract-health/timeline?workspace_id={workspace}"
        ))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    let body: Value = response.json().await.unwrap();
    // Existing scope includes requested-workspace rows and global NULL rows,
    // excluding the other workspace. Empty success must not hide SQL errors.
    assert_eq!(body["total"], 4, "database rows must reach the actual HTTP timeline");
    let events = body["events"].as_array().unwrap();
    assert_eq!(events.iter().filter(|e| e["type"] == "threat_assessment").count(), 2);
    assert_eq!(events.iter().filter(|e| e["type"] == "forecast").count(), 2);
    for event in events {
        if event["type"] == "threat_assessment" {
            assert_eq!(event["findings_count"], 2);
            assert_eq!(event["threat_score"], 0.8);
        } else {
            assert_eq!(event["window_days"], 30);
            assert_eq!(event["change_probability"], 0.7);
            assert_eq!(event["break_probability"], 0.2);
        }
    }
    let global: Value = client
        .get(format!("http://{address}/api/v1/contract-health/timeline"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(global["total"], 2, "omitted scope retains only existing global rows");
    let invalid = client
        .get(format!("http://{address}/api/v1/contract-health/timeline?workspace_id=invalid"))
        .send()
        .await
        .unwrap();
    assert_eq!(invalid.status(), reqwest::StatusCode::BAD_REQUEST);
}
