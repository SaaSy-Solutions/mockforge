use super::*;
use serde_json::json;

#[test]
fn recorded_cookie_parsing_matches_serving_without_folding_cookie_names() {
    let headers = header_value_map(r#"{"cOoKiE":"theme=dark; mode=a=b; Theme=light; ignored"}"#);
    assert_eq!(
        cookie_value_map(&headers),
        json!({"theme":"dark","mode":"a=b","Theme":"light"})
            .as_object()
            .unwrap()
            .clone()
    );
    assert!(cookie_value_map(&header_value_map("{}")).is_empty());
}

#[test]
fn recorded_query_parsing_matches_form_semantics_used_by_serving() {
    assert_eq!(
        query_map(Some("full+name=one+two&literal=%2B&encoded=one%20two&flag&empty=&escaped=a%26%3Db")),
        json!({"full name":"one two","literal":"+","encoded":"one two","flag":"","empty":"","escaped":"a&=b"})
            .as_object()
            .unwrap()
            .clone()
    );
    assert!(query_map(None).is_empty());
    assert!(query_map(Some("")).is_empty());
}

fn schema() -> Value {
    json!({"description":"OK","content":{"application/json":{"schema":{
        "type":"object","required":["id"],"properties":{"id":{"type":"integer"}}
    }}}})
}
fn check(
    responses: Value,
    components: Value,
    status: u16,
    body: &[u8],
) -> Option<responses::ResponseDrift> {
    let spec = json!({"openapi":"3.0.3","info":{"title":"fixture","version":"1"},
        "paths":{"/items":{"get":{"responses":responses}}},"components":components});
    let registry = OpenApiRouteRegistry::new(OpenApiSpec::from_json(spec.clone()).unwrap());
    match check_response(&registry, &spec, "GET", "/items", status, None, Some(body)) {
        ResponseAssessment::Verified => None,
        ResponseAssessment::Drift(drift) => Some(drift),
        ResponseAssessment::Unverified(reason) => {
            panic!("unexpected unsupported test response: {reason}")
        }
    }
}

#[test]
fn undeclared_response_status_is_drift() {
    assert!(check(json!({"200":schema()}), json!({}), 418, br#"{"id":1}"#).is_some());
    assert!(check(json!({"200":schema()}), json!({}), 200, br#"{"id":1}"#).is_none());
}

#[test]
fn range_and_default_response_schemas_are_checked() {
    for responses in [json!({"2XX":schema()}), json!({"default":schema()})] {
        assert!(check(responses.clone(), json!({}), 200, br#"{"id":"wrong"}"#).is_some());
        assert!(check(responses, json!({}), 200, br#"{"id":1}"#).is_none());
    }
}

#[test]
fn exact_response_takes_precedence_over_range_and_default() {
    let exact = json!({"description":"empty object","content":{"application/json":{"schema":{"type":"object","maxProperties":0}}}});
    let responses = json!({"200":exact,"2XX":schema(),"default":schema()});
    assert!(check(responses.clone(), json!({}), 200, br#"{}"#).is_none());
    assert!(check(responses.clone(), json!({}), 200, br#"{"id":1}"#).is_some());
    assert!(check(responses.clone(), json!({}), 201, br#"{"id":1}"#).is_none());
    assert!(check(responses, json!({}), 503, br#"{"id":"wrong"}"#).is_some());
}

#[test]
fn referenced_response_is_resolved_and_validated() {
    let responses = json!({"200":{"$ref":"#/components/responses/Success"}});
    let components = json!({"responses":{"Success":schema()}});
    assert!(check(responses.clone(), components.clone(), 200, br#"{"id":"wrong"}"#).is_some());
    assert!(check(responses, components, 200, br#"{"id":1}"#).is_none());
}

#[test]
fn unresolved_and_cyclic_response_references_are_not_a_pass() {
    for components in [
        json!({}),
        json!({"responses":{"Success":{"$ref":"#/components/responses/Success"}}}),
    ] {
        assert!(check(
            json!({"200":{"$ref":"#/components/responses/Success"}}),
            components,
            200,
            b"{}"
        )
        .is_some());
    }
}

#[test]
fn limit_rejects_zero_and_negative_values() {
    use clap::Args;
    for limit in ["0", "-1"] {
        let command = VerifyMocksArgs::augment_args(clap::Command::new("verify"));
        assert!(command
            .try_get_matches_from([
                "verify",
                "--spec",
                "api.json",
                "--capture-db",
                "record.sqlite",
                "--limit",
                limit
            ])
            .is_err());
    }
}

#[test]
fn requires_evidence_and_accepts_definition_capture_or_both() {
    use clap::Args;
    let command = || VerifyMocksArgs::augment_args(clap::Command::new("verify"));
    assert!(command().try_get_matches_from(["verify", "--spec", "source.json"]).is_err());
    for flags in [
        vec!["--mock-spec", "mock.json"],
        vec!["--fixtures", "fixtures"],
        vec![
            "--fixtures",
            "fixtures",
            "--capture-db",
            "capture.sqlite",
            "--mock-spec",
            "mock.json",
        ],
        vec!["--capture-db", "capture.sqlite"],
        vec!["--mock-spec", "mock.json", "--capture-db", "capture.sqlite"],
    ] {
        let mut args = vec!["verify", "--spec", "source.json"];
        args.extend(flags);
        assert!(command().try_get_matches_from(args).is_ok());
    }
}

#[test]
fn literal_paths_take_precedence_and_parameter_paths_keep_their_bindings() {
    let spec = json!({"openapi":"3.0.3","info":{"title":"precedence","version":"1"},"paths":{
        "/items/{id}":{"get":{"responses":{"200":{"description":"parameter"}}}},
        "/items/~special":{"get":{"responses":{"200":{"description":"literal"}}}}
    }});
    let registry = OpenApiRouteRegistry::new(OpenApiSpec::from_json(spec).unwrap());
    let (template, params) = match_template(&registry, "GET", "/items/~special").unwrap();
    assert_eq!(template, "/items/~special");
    assert!(params.is_empty());
    let (template, params) = match_template(&registry, "GET", "/items/other").unwrap();
    assert_eq!(template, "/items/{id}");
    assert_eq!(params.get("id").unwrap(), "other");
}

#[test]
fn empty_capture_records_no_traffic_verification() {
    let report = empty_capture_report(std::path::Path::new("api.json"));
    let record = verification_record(&report, Some(std::path::Path::new("record.sqlite")));
    assert_eq!(record.outcome, crate::status_commands::VerificationOutcome::NoTraffic);
    assert_eq!(record.exchanges_checked, 0);
    assert!(record.findings.is_empty());
    assert_eq!(record.spec, "api.json");
}

#[tokio::test]
async fn empty_capture_succeeds_even_with_fail_on_drift() {
    let dir = tempfile::tempdir().unwrap();
    let spec = dir.path().join("api.json");
    std::fs::write(&spec, r#"{"openapi":"3.0.0","info":{"title":"t","version":"1"},"paths":{}}"#)
        .unwrap();
    let capture_db = dir.path().join("record.sqlite");
    drop(mockforge_recorder::RecorderDatabase::new(&capture_db).await.unwrap());
    let args = VerifyMocksArgs {
        spec,
        capture_db: Some(capture_db),
        mock_spec: None,
        fixtures: None,
        limit: 500,
        fail_on_drift: true,
        output: None,
        record: false,
    };
    handle_verify_mocks_command(args).await.unwrap();
}

#[tokio::test]
async fn fixture_report_exposes_unverified_media_without_counting_proven_drift() {
    let dir = tempfile::tempdir().unwrap();
    let spec = dir.path().join("api.json");
    let fixture = dir.path().join("fixture.json");
    let output = dir.path().join("report.json");
    std::fs::write(&spec, json!({"openapi":"3.0.3","info":{"title":"media","version":"1"},
        "paths":{"/items":{"get":{"responses":{"200":{"description":"ok","content":{"text/plain":{"schema":{"type":"string"}}}}}}}}}).to_string()).unwrap();
    std::fs::write(&fixture, json!({"fingerprint":{"method":"GET","path":"/items","query":"","headers":{},"body_hash":null},
        "timestamp":"2026-10-20T00:00:00Z","status_code":200,"response_headers":{"content-type":"text/plain"},
        "response_body":"unverified text","metadata":{}}).to_string()).unwrap();
    handle_verify_mocks_command(VerifyMocksArgs {
        spec,
        capture_db: None,
        mock_spec: None,
        fixtures: Some(fixture),
        limit: 500,
        fail_on_drift: false,
        output: Some(output.clone()),
        record: false,
    })
    .await
    .unwrap();
    let report: Value = serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    assert_eq!(report["endpoints_with_drift"], 0);
    assert_eq!(report["endpoints_unverified"], 1);
    assert_eq!(report["findings"][0]["kind"], "fixture-unverified");
    assert_eq!(report["fixtures"]["routes"][0]["status"], "not-verified");
}

#[tokio::test]
async fn recorded_response_media_is_not_inferred_from_an_unrelated_json_schema() {
    let dir = tempfile::tempdir().unwrap();
    let spec = dir.path().join("api.json");
    let capture = dir.path().join("capture.sqlite");
    let output = dir.path().join("report.json");
    std::fs::write(&spec, json!({"openapi":"3.0.3","info":{"title":"media","version":"1"},
        "paths":{"/items":{"get":{"responses":{"200":{"description":"ok","content":{
            "application/json":{"schema":{"type":"integer"}},"text/plain":{"schema":{"type":"string"}}
        }}}}}}}).to_string()).unwrap();
    let db = mockforge_recorder::RecorderDatabase::new(&capture).await.unwrap();
    let request = serde_json::from_value(json!({"id":"sample", "protocol":"Http",
        "timestamp":"2026-10-20T00:00:00Z", "method":"GET", "path":"/items",
        "headers":"{}", "body_encoding":"utf8"}))
    .unwrap();
    db.insert_request(&request).await.unwrap();
    let response = serde_json::from_value(json!({"request_id":"sample", "status_code":200,
        "timestamp":"2026-10-20T00:00:00Z", "headers":r#"{"Content-Type":"text/plain"}"#,
        "body":"1", "body_encoding":"utf8", "size_bytes":1}))
    .unwrap();
    db.insert_response(&response).await.unwrap();
    drop(db);
    handle_verify_mocks_command(VerifyMocksArgs {
        spec,
        capture_db: Some(capture),
        mock_spec: None,
        fixtures: None,
        limit: 500,
        fail_on_drift: false,
        output: Some(output.clone()),
        record: false,
    })
    .await
    .unwrap();
    let report: Value = serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    assert_eq!(report["exchanges_checked"], 1);
    assert_eq!(report["endpoints_with_drift"], 0);
    assert_eq!(report["endpoints_unverified"], 1);
    assert_eq!(report["findings"][0]["kind"], "response-unverified");
}

#[test]
fn persisted_verdict_distinguishes_unverified_evidence_and_prioritizes_proven_drift() {
    let mut report = empty_capture_report(std::path::Path::new("api.json"));
    report.endpoints_unverified = 1;
    report.findings.push(DriftFinding {
        endpoint: "GET /items".into(),
        kind: "response-unverified".into(),
        detail: "Unsupported media".into(),
    });
    let record = verification_record(&report, None);
    assert_eq!(record.outcome, crate::status_commands::VerificationOutcome::Unverified);
    assert_eq!(record.endpoints_unverified, 1);
    assert_eq!(record.endpoints_with_drift, 0);
    report.endpoints_with_drift = 1;
    assert_eq!(
        verification_record(&report, None).outcome,
        crate::status_commands::VerificationOutcome::Drift
    );
}
