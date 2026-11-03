use super::*;
use axum::{
    body::{to_bytes, Body},
    http::Request,
    middleware::from_fn,
    routing::post,
    Router,
};
use mockforge_recorder::models::Protocol;
use tower::ServiceExt;

fn spec(kind: &str) -> Value {
    json!({"openapi":"3.0.3","info":{"title":"fidelity-test","version":"1"},"paths":{
        "/items":{"get":{"responses":{"200":{"description":"ok","content":{"application/json":{
            "schema":{"type":"object","required":["id"],"properties":{"id":{"type":kind}}}
        }}}}}}
    }})
}

fn record(path: &str, body: &str) -> RecordedExchange {
    serde_json::from_value(json!({"request":{"id":"sample","protocol":Protocol::Http,
        "timestamp":"2026-10-20T00:00:00Z","method":"GET","path":path,"headers":"{}",
        "body_encoding":"utf8"},"response":{"request_id":"sample","status_code":200,
        "headers":"{\"content-type\":\"application/json\"}","body":body,"body_encoding":"utf8",
        "size_bytes":body.len(),"timestamp":"2026-10-20T00:00:00Z"}}))
    .unwrap()
}

fn payload(source: Value, sample: Option<Vec<RecordedExchange>>) -> CompareRequest {
    CompareRequest {
        mock_spec: source.to_string(),
        source_spec: source.to_string(),
        sample,
    }
}

#[test]
fn comparison_reports_definition_fields_and_rejects_invalid_inputs() {
    let mut input = payload(spec("string"), None);
    input.mock_spec = spec("integer").to_string();
    let report = assess(input).unwrap();
    assert_eq!(report.definition.routes_with_drift, 1);
    assert!(report.definition.routes[0]
        .deltas
        .iter()
        .any(|d| d.location.ends_with("/id/type")));
    let mut invalid = payload(spec("integer"), None);
    invalid.source_spec = "invalid".into();
    assert!(assess(invalid).is_err());
    assert!(assess(payload(spec("integer"), Some(vec![]))).is_err());
    let mut oversized = payload(spec("integer"), None);
    oversized.source_spec = " ".repeat(2 * 1024 * 1024 + 1);
    assert!(assess(oversized).is_err());
}

#[test]
fn samples_distinguish_verified_unsampled_and_invalid_responses() {
    let mut source = spec("integer");
    source["paths"]["/other"] = source["paths"]["/items"].clone();
    let report = serde_json::to_value(
        assess(payload(source.clone(), Some(vec![record("/items", r#"{"id":1}"#)]))).unwrap(),
    )
    .unwrap();
    let routes = report["sample"]["routes"].as_array().unwrap();
    assert!(routes
        .iter()
        .any(|r| r["endpoint"] == "GET /items" && r["status"] == "verified"));
    assert!(routes
        .iter()
        .any(|r| r["endpoint"] == "GET /other" && r["status"] == "not-sampled"));
    let report = serde_json::to_value(
        assess(payload(
            source,
            Some(vec![
                record("/items", r#"{"id":"wrong"}"#),
                record("/missing", "{}"),
            ]),
        ))
        .unwrap(),
    )
    .unwrap();
    assert!(
        report["sample"]["routes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["status"] == "drift-detected")
            .count()
            >= 2
    );
}

#[test]
fn unsupported_media_is_not_verified_and_bad_encoding_is_an_error() {
    let mut source = spec("integer");
    source["paths"]["/items"]["get"]["responses"]["200"]["content"] =
        json!({"text/plain":{"schema":{"type":"string"}}});
    let mut sample = record("/items", "hello");
    sample.response.as_mut().unwrap().headers = "{\"content-type\":\"text/plain\"}".into();
    let report =
        serde_json::to_value(assess(payload(source, Some(vec![sample]))).unwrap()).unwrap();
    assert_eq!(report["sample"]["routes"][0]["status"], "not-verified");
    let mut sample = record("/items", "not-base64");
    sample.response.as_mut().unwrap().body_encoding = "base64".into();
    assert!(assess(payload(spec("integer"), Some(vec![sample]))).is_err());
}

#[test]
fn sampled_requests_validate_bodies_queries_and_cookies_without_false_verification() {
    let mut source = spec("integer");
    let mut operation = source["paths"]["/items"]["get"].take();
    source["paths"]["/items"].as_object_mut().unwrap().remove("get");
    operation["requestBody"] = json!({"required":true,"content":{"application/json":{"schema":{
        "type":"object","required":["name"],"properties":{"name":{"type":"string"}}
    }}}});
    operation["parameters"] = json!([
        {"name":"kind","in":"query","required":true,"schema":{"type":"string","enum":["one"]}},
        {"name":"session","in":"cookie","required":true,"schema":{"type":"string"}}
    ]);
    source["paths"]["/items"]["post"] = operation;
    let mut valid = record("/items", r#"{"id":1}"#);
    valid.request.method = "POST".into();
    valid.request.headers =
        r#"{"Content-Type":"application/json","Cookie":"session=present"}"#.into();
    valid.request.query_params = Some(r#"{"kind":"one"}"#.into());
    valid.request.body = Some(r#"{"name":"valid"}"#.into());
    let report =
        serde_json::to_value(assess(payload(source.clone(), Some(vec![valid.clone()]))).unwrap())
            .unwrap();
    assert_eq!(report["sample"]["routes"][0]["status"], "verified");
    valid.request.body = Some("malformed JSON".into());
    let report =
        serde_json::to_value(assess(payload(source.clone(), Some(vec![valid.clone()]))).unwrap())
            .unwrap();
    assert_eq!(report["sample"]["routes"][0]["status"], "drift-detected");
    valid.request.body = None;
    let report = serde_json::to_value(assess(payload(source, Some(vec![valid]))).unwrap()).unwrap();
    assert_eq!(report["sample"]["routes"][0]["status"], "drift-detected");
    let mut other = record("/items", "{}");
    other.request.protocol = Protocol::Mqtt;
    assert!(assess(payload(spec("integer"), Some(vec![other]))).is_err());
}

#[tokio::test]
async fn endpoint_requires_auth_and_allows_a_viewer_to_compare_supplied_definitions() {
    let app = Router::new()
        .route("/__mockforge/fidelity/compare", post(compare))
        .layer(from_fn(crate::rbac::rbac_middleware));
    let input =
        json!({"mock_spec":spec("integer").to_string(),"source_spec":spec("integer").to_string()})
            .to_string();
    let request = || {
        Request::builder()
            .method("POST")
            .uri("/__mockforge/fidelity/compare")
            .header("content-type", "application/json")
    };
    let response = app
        .clone()
        .oneshot(request().body(Body::from(input.clone())).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let store = crate::auth::UserStore::new();
    let user = store
        .create_user(
            "fidelity-test-viewer".into(),
            format!("{}Aa1!", uuid::Uuid::new_v4()),
            mockforge_collab::models::UserRole::Viewer,
            None,
        )
        .await
        .unwrap();
    let token = crate::auth::generate_token(&user, 60).unwrap();
    let response = app
        .oneshot(
            request()
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(input))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let report: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    assert_eq!(report["definition"]["routes_with_drift"], 0);
}
