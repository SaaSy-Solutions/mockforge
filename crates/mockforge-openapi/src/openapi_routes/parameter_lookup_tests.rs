use super::*;
use serde_json::json;

fn parameter() -> openapiv3::ParameterData {
    let parameter: openapiv3::Parameter = serde_json::from_value(json!({
        "name":"X-Trace", "in":"header", "required":true,
        "schema":{"type":"string","enum":["present"]}
    }))
    .unwrap();
    let openapiv3::Parameter::Header { parameter_data, .. } = parameter else {
        unreachable!()
    };
    parameter_data
}

fn check(values: &Map<String, Value>, location: &str, expected_valid: bool) {
    let parameter = parameter();
    let spec = OpenApiSpec::from_json(
        json!({"openapi":"3.0.3","info":{"title":"test","version":"1"},"paths":{}}),
    )
    .unwrap();
    let mut errors = Vec::new();
    let mut details = Vec::new();
    validate_parameter(&parameter, values, location, true, &mut errors, &mut details, &spec);
    assert_eq!(errors.is_empty(), expected_valid);
    assert_eq!(details.is_empty(), expected_valid);
    let mut field_errors = Vec::new();
    validate_parameter_detailed(
        &parameter,
        values,
        location,
        "parameter",
        &mut field_errors,
        &spec,
    );
    assert_eq!(field_errors.is_empty(), expected_valid);
}

#[test]
fn header_spelling_does_not_change_normal_or_detailed_validation() {
    for name in ["X-Trace", "x-trace", "X-TRACE", "x-TrAcE"] {
        check(json!({name:"present"}).as_object().unwrap(), "header", true);
    }
}

#[test]
fn header_values_and_required_presence_still_enforce_the_schema() {
    check(json!({"x-trace":"wrong"}).as_object().unwrap(), "header", false);
    check(&Map::new(), "header", false);
}

#[test]
fn other_parameter_names_remain_case_sensitive() {
    for location in ["cookie", "query", "path"] {
        check(json!({"x-trace":"present"}).as_object().unwrap(), location, false);
        check(json!({"X-Trace":"present"}).as_object().unwrap(), location, true);
    }
}
