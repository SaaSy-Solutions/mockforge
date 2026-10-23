use super::*;
use serde_json::json;

fn schema() -> Value {
    json!({"description":"OK","content":{"application/json":{"schema":{
        "type":"object","required":["id"],"properties":{"id":{"type":"integer"}}
    }}}})
}
fn check(responses: Value, components: Value, status: u16, body: &[u8]) -> Option<String> {
    let spec = json!({"openapi":"3.0.3","info":{"title":"fixture","version":"1"},
        "paths":{"/items":{"get":{"responses":responses}}},"components":components});
    let registry = OpenApiRouteRegistry::new(OpenApiSpec::from_json(spec.clone()).unwrap());
    check_response_drift(&registry, &spec, "GET", "/items", status, Some(body))
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
        vec!["--capture-db", "capture.sqlite"],
        vec!["--mock-spec", "mock.json", "--capture-db", "capture.sqlite"],
    ] {
        let mut args = vec!["verify", "--spec", "source.json"];
        args.extend(flags);
        assert!(command().try_get_matches_from(args).is_ok());
    }
}
