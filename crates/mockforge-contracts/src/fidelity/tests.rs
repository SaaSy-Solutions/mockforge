use super::*;
use serde_json::json;

fn spec(schema: Value) -> Value {
    json!({"openapi":"3.0.3","info":{"title":"test","version":"1"},"paths":{
        "/items":{"get":{"responses":{"200":{"description":"ok","content":{
            "application/json":{"schema":schema}
        }}}}}
    }})
}

#[test]
fn reports_added_removed_and_changed_routes_including_head_and_trace() {
    let mut mock = spec(json!({"type":"string"}));
    let mut source = spec(json!({"type":"integer"}));
    mock["paths"]["/old"] = json!({"head":{"responses":{}}});
    source["paths"]["/new"] = json!({"trace":{"responses":{}}});
    let report = compare_mock_spec(&mock, &source).unwrap();
    assert_eq!(report.routes_with_drift, 3);
    assert_eq!(
        report
            .routes
            .iter()
            .map(|r| (r.endpoint.as_str(), r.change.as_str()))
            .collect::<Vec<_>>(),
        [
            ("GET /items", "changed"),
            ("HEAD /old", "removed"),
            ("TRACE /new", "added")
        ]
    );
    let delta = &report.routes[0].deltas[0];
    assert_eq!(delta.location, "/responses/200/content/application~1json/schema/type");
    assert_eq!(delta.mock, Some(json!("string")));
    assert_eq!(delta.source, Some(json!("integer")));
}

#[test]
fn nested_schema_fields_and_constraints_produce_explicit_deltas() {
    let mock = spec(json!({"type":"object","required":["old"],"properties":{
        "old":{"type":"string"},"a/b~c":{"type":"integer","minimum":0}
    }}));
    let source = spec(json!({"type":"object","required":["new"],"properties":{
        "new":{"type":"boolean"},"a/b~c":{"type":"string","minLength":1}
    }}));
    let report = compare_mock_spec(&mock, &source).unwrap();
    let deltas = &report.routes[0].deltas;
    assert!(deltas
        .iter()
        .any(|d| d.location.ends_with("/properties/old") && d.change == "removed"));
    assert!(deltas
        .iter()
        .any(|d| d.location.ends_with("/properties/new") && d.change == "added"));
    assert!(deltas
        .iter()
        .any(|d| d.location.ends_with("/properties/a~1b~0c/type") && d.change == "changed"));
    assert!(deltas.iter().any(|d| d.location.ends_with("/required")));
}

#[test]
fn resolves_references_and_recursive_schema_renames_without_losing_field_changes() {
    fn recursive(name: &str, kind: &str) -> Value {
        let mut s = spec(json!({"$ref":format!("#/components/schemas/{name}")}));
        s["components"] = json!({"schemas":{name:{"type":"object","properties":{
            "next":{"$ref":format!("#/components/schemas/{name}")},"id":{"type":kind}
        }}}});
        s
    }
    let mock = recursive("Old", "integer");
    let mut alias = mock.clone();
    alias["components"]["schemas"]["Alias"] = json!({"$ref":"#/components/schemas/Old"});
    alias["paths"]["/items"]["get"]["responses"]["200"]["content"]["application/json"]["schema"] =
        json!({"$ref":"#/components/schemas/Alias"});
    assert_eq!(compare_mock_spec(&alias, &mock).unwrap().routes_with_drift, 0);
    assert_eq!(
        compare_mock_spec(&mock, &recursive("New", "integer"))
            .unwrap()
            .routes_with_drift,
        0
    );
    let report = compare_mock_spec(&mock, &recursive("New", "string")).unwrap();
    assert_eq!(report.routes_with_drift, 1);
    assert!(report.routes[0]
        .deltas
        .iter()
        .any(|d| d.location.ends_with("/properties/id/type")));
}

#[test]
fn documentation_and_set_order_do_not_drift_but_named_description_fields_do() {
    let mock = spec(json!({"type":"object","description":"before","required":["id","description"],
        "properties":{"id":{"type":"integer"},"description":{"type":"string","enum":["b","a"]}}}));
    let mut source = mock.clone();
    let schema = &mut source["paths"]["/items"]["get"]["responses"]["200"]["content"]
        ["application/json"]["schema"];
    schema["description"] = json!("after");
    schema["required"] = json!(["description", "id"]);
    schema["properties"]["description"]["enum"] = json!(["a", "b"]);
    assert_eq!(compare_mock_spec(&mock, &source).unwrap().routes_with_drift, 0);
    source["paths"]["/items"]["get"]["responses"]["200"]["content"]["application/json"]["schema"]
        ["properties"]["description"]["type"] = json!("integer");
    assert_eq!(compare_mock_spec(&mock, &source).unwrap().routes_with_drift, 1);
}

#[test]
fn inherited_parameters_are_overridden_by_operation_identity() {
    let mut mock = spec(json!({"type":"string"}));
    let parameter = json!({"name":"id","in":"query","schema":{"type":"integer"}});
    mock["paths"]["/items"]["parameters"] = json!([parameter]);
    let mut source = mock.clone();
    source["paths"]["/items"].as_object_mut().unwrap().remove("parameters");
    source["paths"]["/items"]["get"]["parameters"] = json!([parameter]);
    assert_eq!(compare_mock_spec(&mock, &source).unwrap().routes_with_drift, 0);
    source["paths"]["/items"]["get"]["parameters"][0]["required"] = json!(true);
    assert_eq!(compare_mock_spec(&mock, &source).unwrap().routes_with_drift, 1);
}

#[test]
fn missing_external_and_alias_cycle_references_fail_explicitly() {
    for reference in [
        "#/components/schemas/Missing",
        "https://example.invalid/spec.json#/Thing",
        "#/components/schemas/Loop",
    ] {
        let mut value = spec(json!({"$ref":reference}));
        value["components"] = json!({"schemas":{"Loop":{"$ref":"#/components/schemas/Loop"}}});
        assert!(compare_mock_spec(&value, &value).is_err());
    }
    assert!(compare_mock_spec(&json!({"paths":{}}), &json!({"paths":{}})).is_err());
}

#[test]
fn inherited_security_compares_the_referenced_scheme_and_scope_sets() {
    let mut mock = spec(json!({"type":"string"}));
    mock["security"] = json!([{"ApiKey":[]}]);
    mock["components"] = json!({"securitySchemes":{"ApiKey":{
        "type":"apiKey","in":"header","name":"X-Old-Key"
    }}});
    let mut source = mock.clone();
    source["paths"]["/items"]["get"]["security"] = json!([{"ApiKey":[]}]);
    source.as_object_mut().unwrap().remove("security");
    assert_eq!(compare_mock_spec(&mock, &source).unwrap().routes_with_drift, 0);
    source["components"]["securitySchemes"]["ApiKey"]["name"] = json!("X-New-Key");
    let report = compare_mock_spec(&mock, &source).unwrap();
    assert!(report.routes[0]
        .deltas
        .iter()
        .any(|d| d.location == "/securitySchemes/ApiKey/name"));
    source["components"]["securitySchemes"].as_object_mut().unwrap().clear();
    assert!(compare_mock_spec(&mock, &source).is_err());
}

#[test]
fn server_variable_names_that_match_metadata_keys_preserve_their_contract() {
    for name in [
        "description",
        "summary",
        "example",
        "examples",
        "operationId",
        "tags",
    ] {
        let mut mock = spec(json!({"type":"string"}));
        mock["servers"] = json!([{
            "url": format!("https://{{{name}}}.example.invalid"),
            "variables": {name: {"default":"before", "enum":["before","after"]}}
        }]);
        let mut source = mock.clone();
        source["servers"][0]["variables"][name]["default"] = json!("after");
        assert_eq!(
            compare_mock_spec(&mock, &source).unwrap().routes_with_drift,
            1,
            "server variable {name} is a named contract, not documentation"
        );
        source = mock.clone();
        source["servers"][0]["variables"][name]["enum"] = json!(["before"]);
        assert_eq!(compare_mock_spec(&mock, &source).unwrap().routes_with_drift, 1);
        source = mock.clone();
        source["servers"][0]["variables"][name]["description"] = json!("documentation only");
        assert_eq!(compare_mock_spec(&mock, &source).unwrap().routes_with_drift, 0);
    }
}
