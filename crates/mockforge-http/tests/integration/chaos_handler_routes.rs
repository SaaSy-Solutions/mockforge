//! Exercise the legacy Chaos handler states through actual TCP routes.
use axum::{routing::post, Router};
use mockforge_chaos::{
    failure_designer::{FailureDesignRule, FailureDesigner},
    incident_replay::{IncidentFormatAdapter, IncidentReplayGenerator, IncidentTimeline},
};
use mockforge_http::handlers::{failure_designer as failure, incident_replay as replay};
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};

struct Server {
    base: String,
    client: reqwest::Client,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Server {
    async fn start() -> Self {
        // Preserve public downstream struct literal construction with the real engines.
        let failure = failure::FailureDesignerState {
            designer: Arc::new(FailureDesigner::new()),
        };
        let replay = replay::IncidentReplayState {
            generator: Arc::new(IncidentReplayGenerator::new()),
        };
        let router = Router::new()
            .merge(
                Router::new()
                    .route("/validate", post(failure::validate_rule))
                    .route("/generate", post(failure::generate_scenario))
                    .route("/preview", post(failure::preview_config))
                    .with_state(failure),
            )
            .merge(
                Router::new()
                    .route("/replay", post(replay::generate_replay))
                    .route("/import", post(replay::import_incident))
                    .route("/import-and-generate", post(replay::import_and_generate))
                    .with_state(replay),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        Self {
            base: format!("http://{address}"),
            client: reqwest::Client::builder().timeout(Duration::from_secs(5)).build().unwrap(),
            task,
        }
    }
    async fn post(&self, path: &str, body: Value) -> reqwest::Response {
        self.client
            .post(format!("{}{path}", self.base))
            .json(&body)
            .send()
            .await
            .unwrap()
    }
    async fn json(&self, path: &str, body: Value) -> Value {
        let r = self.post(path, body).await;
        assert_eq!(r.status(), reqwest::StatusCode::OK);
        r.json().await.unwrap()
    }
}
fn rule(kind: Value) -> Value {
    json!({"name":"Synthetic QA rule","target":{"endpoints":["/orders/*"],"methods":["POST"],"user_agents":["Chrome"]},"failure_type":kind,"probability":0.1,"conditions":[]})
}
fn timeline() -> Value {
    json!({"incident_id":"synthetic-incident","start_time":"2026-10-13T10:00:00Z","end_time":"2026-10-13T10:02:00Z","events":[{"timestamp":"2026-10-13T10:00:01Z","event_type":{"type":"status_code_change","code":503,"rate":0.5},"endpoint":"/orders","method":"GET"}],"metadata":{"fixture":true}})
}
#[tokio::test]
async fn designer_preserves_all_failure_types_and_preview_wire_fields() {
    let server = Server::start().await;
    let designer = FailureDesigner::new();
    for kind in [
        json!({"type":"status_code","code":503}),
        json!({"type":"latency","delay_ms":50}),
        json!({"type":"timeout","timeout_ms":100}),
        json!({"type":"connection_error"}),
        json!({"type":"partial_response","truncate_percentage":0.25}),
        json!({"type":"webhook_failure","webhook_pattern":"https://example.invalid/*"}),
    ] {
        let rule = rule(kind);
        let parsed: FailureDesignRule = serde_json::from_value(rule.clone()).unwrap();
        let request = json!({"rule":rule});
        assert_eq!(server.json("/validate", request.clone()).await, json!({"success":true}));
        let generated = server.json("/generate", request.clone()).await;
        assert_eq!(
            generated["scenario"],
            serde_json::to_value(designer.rule_to_scenario(&parsed).unwrap()).unwrap()
        );
        assert_eq!(
            generated["route_chaos_config"],
            designer.generate_route_chaos_config(&parsed).unwrap()
        );
        if rule["failure_type"]["type"] == "webhook_failure" {
            assert_eq!(generated["webhook_hook"], designer.generate_webhook_hook(&parsed).unwrap());
        } else {
            assert_eq!(generated["webhook_hook"], Value::Null);
        }
        let mut expected = generated;
        expected["rule"] = serde_json::to_value(parsed).unwrap();
        assert_eq!(server.json("/preview", request).await, expected);
    }
}
#[tokio::test]
async fn designer_retains_validation_and_extractor_error_contract() {
    let server = Server::start().await;
    let mut rule = rule(json!({"type":"status_code","code":503}));
    rule["probability"] = json!(1.5);
    let response = server.json("/validate", json!({"rule":rule})).await;
    assert_eq!(response["success"], false);
    assert!(response["error"].as_str().unwrap().contains("Probability"));
    for (path, prefix) in [
        ("/generate", "Failed to generate scenario:"),
        ("/preview", "Validation failed:"),
    ] {
        let r = server.post(path, json!({"rule":rule})).await;
        assert_eq!(r.status(), reqwest::StatusCode::OK);
        assert!(r.text().await.unwrap().starts_with(prefix));
    }
    assert_eq!(
        server.post("/validate", json!({"rule":{}})).await.status(),
        reqwest::StatusCode::UNPROCESSABLE_ENTITY
    );
}
#[tokio::test]
async fn replay_preserves_custom_and_external_imports_and_exports() {
    let server = Server::start().await;
    let generator = IncidentReplayGenerator::new();
    let custom = timeline();
    let parsed: IncidentTimeline = serde_json::from_value(custom.clone()).unwrap();
    let expected = serde_json::to_value(generator.generate_scenario(&parsed)).unwrap();
    let direct = server
        .json("/replay", json!({"timeline":custom,"format":"ignored-by-existing-handler"}))
        .await;
    assert_eq!(direct["scenario"], expected);
    let pagerduty = json!({"incident":{"id":"pd-fixture","created_at":"2026-10-13T10:00:00Z","resolved_at":"2026-10-13T10:02:00Z"},"log_entries":[{"created_at":"2026-10-13T10:00:01Z","summary":"server error"}]});
    let datadog =
        json!({"id":"dd-fixture","created":1791885600000_i64,"resolved":1791885720000_i64});
    for (format, data) in [
        ("custom", custom),
        ("pagerduty", pagerduty),
        ("datadog", datadog),
    ] {
        let imported = server.json("/import", json!({"format":format,"data":data})).await;
        let expected_timeline = match format {
            "custom" => serde_json::from_value::<IncidentTimeline>(data.clone()).unwrap(),
            "pagerduty" => IncidentFormatAdapter::from_pagerduty(&data).unwrap(),
            _ => IncidentFormatAdapter::from_datadog(&data).unwrap(),
        };
        assert_eq!(imported["timeline"], serde_json::to_value(&expected_timeline).unwrap());
        let combined =
            server.json("/import-and-generate", json!({"format":format,"data":data})).await;
        assert_eq!(combined["timeline"], imported["timeline"]);
        assert_eq!(
            combined["scenario"],
            serde_json::to_value(generator.generate_scenario(&expected_timeline)).unwrap()
        );
        let exported: Value =
            serde_json::from_str(combined["scenario_json"].as_str().unwrap()).unwrap();
        assert_eq!(exported, combined["scenario"]);
        let exported: Value =
            serde_yaml::from_str(combined["scenario_yaml"].as_str().unwrap()).unwrap();
        assert_eq!(exported, combined["scenario"]);
    }
    for (format, data, prefix) in [
        ("unknown", json!({}), "Unsupported format: unknown"),
        ("pagerduty", json!({}), "Failed to parse PagerDuty format:"),
        ("datadog", json!({}), "Failed to parse Datadog format:"),
        ("custom", json!({}), "Failed to parse custom format:"),
    ] {
        for path in ["/import", "/import-and-generate"] {
            let r = server.post(path, json!({"format":format,"data":data})).await;
            assert_eq!(r.status(), reqwest::StatusCode::OK);
            assert!(r.text().await.unwrap().starts_with(prefix));
        }
    }
    assert_eq!(
        server.post("/replay", json!({"timeline":{}})).await.status(),
        reqwest::StatusCode::UNPROCESSABLE_ENTITY
    );
}
