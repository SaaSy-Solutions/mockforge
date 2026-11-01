//! Exercise the compatibility router through real recorder SQLite storage.

#![cfg(feature = "behavioral-cloning")]

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use chrono::{TimeZone, Utc};
use mockforge_http::handlers::behavioral_cloning::{
    behavioral_cloning_router, BehavioralCloningState,
};
use mockforge_intelligence::behavioral_cloning::{BehavioralSequence, SequenceStep};
use mockforge_recorder::{
    database::RecorderDatabase,
    models::{Protocol, RecordedRequest, RecordedResponse},
};
use serde_json::{json, Value};
use std::collections::HashMap;
use tower::ServiceExt;

async fn fixture() -> (tempfile::TempDir, RecorderDatabase, Router) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("recordings.db");
    let db = RecorderDatabase::new(&path).await.unwrap();
    let state = BehavioralCloningState::with_database_path(path.clone());
    assert_eq!(state.database_path.as_ref(), Some(&path));
    let router = behavioral_cloning_router(state);
    (dir, db, router)
}

async fn call(router: &Router, method: &str, suffix: &str, data: Option<Value>) -> Value {
    let request = Request::builder()
        .method(method)
        .uri(format!("/api/v1/behavioral-cloning/{suffix}"))
        .header("content-type", "application/json")
        .body(data.map_or_else(Body::empty, |value| Body::from(value.to_string())))
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap()).unwrap()
}

async fn record(
    db: &RecorderDatabase,
    id: &str,
    path: &str,
    trace: &str,
    ts: i64,
    status: i32,
    response_status: Option<i32>,
) {
    let timestamp = Utc.timestamp_millis_opt(ts).unwrap();
    db.insert_request(&RecordedRequest {
        id: id.into(),
        protocol: Protocol::Http,
        timestamp,
        method: "GET".into(),
        path: path.into(),
        query_params: None,
        headers: "{}".into(),
        body: Some("{\"input\":1}".into()),
        body_encoding: "utf8".into(),
        client_ip: None,
        trace_id: Some(trace.into()),
        span_id: None,
        duration_ms: Some(80),
        status_code: Some(status),
        tags: None,
    })
    .await
    .unwrap();
    if let Some(status_code) = response_status {
        db.insert_response(&RecordedResponse {
            request_id: id.into(),
            status_code,
            headers: "{}".into(),
            body: Some(
                if status_code >= 400 {
                    "{\"error\":\"unavailable\"}"
                } else {
                    "{\"ok\":true}"
                }
                .into(),
            ),
            body_encoding: "utf8".into(),
            size_bytes: 20,
            timestamp,
        })
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn probability_models_keep_recorder_filtering_persistence_and_response_precedence() {
    let (_dir, db, router) = fixture().await;
    record(&db, "success", "/widgets", "t", 1000, 418, Some(200)).await;
    record(&db, "error", "/widgets", "t", 1100, 418, Some(503)).await;
    record(&db, "fallback", "/widgets", "t", 1200, 404, None).await;
    record(&db, "other", "/other", "t", 1300, 201, None).await;
    let built = call(
        &router,
        "POST",
        "probability-models",
        Some(json!({"endpoint":"/widgets","method":"GET"})),
    )
    .await;
    let distribution = &built["model"]["status_code_distribution"];
    assert_eq!(distribution.as_object().unwrap().len(), 3);
    assert!(distribution.get("200").is_some());
    assert!(distribution.get("503").is_some());
    assert!(distribution.get("404").is_some());
    assert!(distribution.get("418").is_none());
    assert!(distribution.get("201").is_none());
    assert_eq!(db.get_all_endpoint_probability_models().await.unwrap().len(), 1);
    let disabled = call(
        &router,
        "POST",
        "amplification/apply",
        Some(json!({"config": {
            "enabled": false, "amplification_factor": 0.5, "scope": "Global",
            "target_patterns": null, "rare_threshold": 0.4
        }})),
    )
    .await;
    assert_eq!(disabled["message"], "Amplification disabled");
    let applied = call(
        &router,
        "POST",
        "amplification/apply",
        Some(json!({"config": {
            "enabled": true, "amplification_factor": 0.5,
            "scope": {"Endpoint": {"endpoint":"/widgets", "method":"GET"}},
            "target_patterns": null, "rare_threshold": 0.4
        }})),
    )
    .await;
    assert_eq!(applied["updated_models"], 1);
    let listed = call(&router, "GET", "probability-models", None).await;
    assert_eq!(listed["count"], 1);
    let fetched = call(&router, "GET", "probability-models/%2Fwidgets/GET", None).await;
    assert_eq!(fetched["model"], listed["models"][0]);
    let sampled =
        call(&router, "POST", "probability-models/%2Fwidgets/GET/sample/status-code", None).await;
    assert!([200, 404, 503].contains(&sampled["status_code"].as_u64().unwrap()));
    assert!(
        call(&router, "POST", "probability-models/%2Fwidgets/GET/sample/latency", None).await
            ["latency_ms"]
            .is_number()
    );
    assert!(
        call(&router, "GET", "amplification/rare-edges/%2Fwidgets/GET?threshold=0.4", None).await
            ["rare_patterns"]
            .is_array()
    );
    let limited = call(
        &router,
        "POST",
        "probability-models",
        Some(json!({"endpoint":"/widgets","method":"GET","sample_limit":1})),
    )
    .await;
    assert_eq!(limited["model"]["status_code_distribution"].as_object().unwrap().len(), 1);
}

#[tokio::test]
async fn trace_discovery_preserves_minimum_and_timestamp_order() {
    let (_dir, db, router) = fixture().await;
    for trace in ["one", "two"] {
        // Insert out of chronological order to prove the adapter retains SQL ordering.
        record(&db, &format!("{trace}-later"), "/second", trace, 2500, 200, None).await;
        record(&db, &format!("{trace}-first"), "/first", trace, 1000, 200, None).await;
    }
    let none =
        call(&router, "POST", "sequences/discover", Some(json!({"min_requests_per_trace":3})))
            .await;
    assert_eq!(none["count"], 0);
    let learned = call(
        &router,
        "POST",
        "sequences/discover",
        Some(json!({"min_requests_per_trace":2,"min_frequency":0.1})),
    )
    .await;
    assert!(learned["count"].as_u64().unwrap() > 0);
    assert_eq!(learned["sequences"][0]["steps"][0]["endpoint"], "/first");
    assert_eq!(learned["sequences"][0]["steps"][1]["endpoint"], "/second");
    assert_eq!(learned["sequences"][0]["steps"][1]["expected_delay_ms"], 1500);
    assert_eq!(call(&router, "GET", "sequences", None).await["count"], learned["count"]);
}

#[tokio::test]
async fn scenario_conversion_retains_shared_foundation_fields() {
    let (_dir, db, router) = fixture().await;
    db.insert_behavioral_sequence(&BehavioralSequence {
        id: "checkout".into(),
        name: "Checkout".into(),
        description: Some("Recorded checkout".into()),
        tags: vec!["stable-ci".into()],
        frequency: 1.0,
        confidence: 0.9,
        learned_from: vec![],
        steps: vec![SequenceStep {
            endpoint: "/cart".into(),
            method: "POST".into(),
            expected_delay_ms: Some(125),
            conditions: HashMap::from([("currency".into(), "USD".into())]),
            probability: 1.0,
            name: Some("Create cart".into()),
        }],
    })
    .await
    .unwrap();
    assert_eq!(
        call(&router, "GET", "sequences/checkout", None).await["sequence"]["id"],
        "checkout"
    );
    let generated = call(&router, "POST", "sequences/checkout/scenario", None).await;
    let scenario = &generated["scenario"];
    assert_eq!(scenario["id"], "checkout");
    assert_eq!(scenario["description"], "Recorded checkout");
    assert_eq!(scenario["tags"], json!(["stable-ci"]));
    assert_eq!(scenario["steps"][0]["name"], "Create cart");
    assert_eq!(scenario["steps"][0]["delay_ms"], 125);
    assert_eq!(scenario["steps"][0]["expected_status"], 200);
    assert_eq!(scenario["steps"][0]["query_params"]["currency"], "USD");
}
