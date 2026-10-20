use super::*;
use crate::fidelity::{compare_mock_spec, compare_mock_spec_with_budget};

fn v2() -> Value {
    json!({"asyncapi":"2.6.0","info":{"title":"events","version":"1"},"defaultContentType":"application/json",
        "servers":{"local":{"url":"localhost:1883","protocol":"mqtt","security":[{"auth":[]}]}},
        "components":{"securitySchemes":{"auth":{"type":"userPassword"}},"schemas":{"Order":{"type":"object","required":["id"],"properties":{"id":{"type":"integer"},"description":{"type":"string"},"traits":{"type":"string"}}}},"messages":{"Order":{"payload":{"$ref":"#/components/schemas/Order"},"headers":{"type":"object","properties":{"trace":{"type":"string"}}}}}},
        "channels":{"orders":{"publish":{"message":{"$ref":"#/components/messages/Order"}},"bindings":{"mqtt":{"qos":1}}}}})
}

fn v3() -> Value {
    json!({"asyncapi":"3.0.0","info":{"title":"events","version":"1"},"defaultContentType":"application/json",
        "servers":{"local":{"host":"localhost:1883","protocol":"mqtt"}},
        "channels":{"orders":{"address":"orders/created","messages":{"description":{"payload":{"type":"object","required":["id"],"properties":{"id":{"type":"integer"}}}}}}},
        "operations":{"receiveOrder":{"action":"receive","channel":{"$ref":"#/channels/orders"},"messages":[{"$ref":"#/channels/orders/messages/description"}]}}})
}

#[test]
fn identical_asyncapi_documents_and_unreferenced_components_are_verified() {
    for mut source in [v2(), v3()] {
        let mock = source.clone();
        source["components"]["unused"] = json!({"type":"string"});
        let report = compare_mock_spec(&mock, &source).unwrap();
        assert_eq!(report.routes_with_drift, 0);
        assert_eq!(report.routes.len(), 2);
    }
}

#[test]
fn v2_publish_means_receive_and_subscribe_means_send() {
    let mut source = v2();
    source["channels"]["orders"]["subscribe"] = source["channels"]["orders"]["publish"].clone();
    let report = compare_mock_spec(&source, &source).unwrap();
    assert!(report.routes.iter().any(|r| r.endpoint == "RECEIVE orders"));
    assert!(report.routes.iter().any(|r| r.endpoint == "SEND orders"));
}

#[test]
fn v3_operations_on_same_channel_are_never_overwritten() {
    let mut source = v3();
    source["operations"]["another"] = source["operations"]["receiveOrder"].clone();
    source["operations"]["sendOrder"] = source["operations"]["receiveOrder"].clone();
    source["operations"]["sendOrder"]["action"] = json!("send");
    let report = compare_mock_spec(&source, &source).unwrap();
    assert_eq!(report.routes.len(), 4);
    let mock = source.clone();
    source["operations"].as_object_mut().unwrap().remove("another");
    let report = compare_mock_spec(&mock, &source).unwrap();
    assert!(report
        .routes
        .iter()
        .any(|r| r.change == "removed" && r.endpoint.contains("[another]")));
}

#[test]
fn nested_schema_headers_and_protocol_bindings_produce_field_deltas() {
    let mock = v2();
    for pointer in [
        "/components/schemas/Order/properties/id/type",
        "/components/schemas/Order/properties/description/type",
        "/components/schemas/Order/properties/traits/type",
        "/components/messages/Order/headers/properties/trace/type",
    ] {
        let mut source = mock.clone();
        *source.pointer_mut(pointer).unwrap() = json!("boolean");
        let report = compare_mock_spec(&mock, &source).unwrap();
        assert!(report
            .routes
            .iter()
            .any(|r| r.deltas.iter().any(|d| d.location.ends_with("/type"))));
    }
    let mut source = mock.clone();
    source["channels"]["orders"]["bindings"]["mqtt"]["qos"] = json!(2);
    assert!(compare_mock_spec(&mock, &source).unwrap().routes_with_drift > 0);
}

#[test]
fn server_security_changes_are_not_hidden_behind_unchanged_references() {
    let mock = v2();
    let mut source = mock.clone();
    source["components"]["securitySchemes"]["auth"]["type"] = json!("plain");
    assert!(compare_mock_spec(&mock, &source).unwrap().routes_with_drift > 0);
    let mock = v3();
    let mut source = mock.clone();
    source["components"]["securitySchemes"]["auth"] = json!({"type":"userPassword"});
    source["components"]["operationTraits"]["secure"] =
        json!({"security":[{"$ref":"#/components/securitySchemes/auth"}]});
    source["operations"]["receiveOrder"]["traits"] =
        json!([{"$ref":"#/components/operationTraits/secure"}]);
    let mock = source.clone();
    source["components"]["securitySchemes"]["auth"]["type"] = json!("plain");
    assert!(compare_mock_spec(&mock, &source).unwrap().routes_with_drift > 0);
}

#[test]
fn message_traits_fill_missing_fields_and_explicit_fields_win() {
    let mut source = v2();
    source["components"]["messageTraits"]["typed"] =
        json!({"contentType":"text/plain","headers":{"type":"object","required":["trace"]}});
    source["components"]["messages"]["Order"]["traits"] =
        json!([{"$ref":"#/components/messageTraits/typed"}]);
    source["components"]["messages"]["Order"]["contentType"] = json!("application/json");
    let mut mock = source.clone();
    mock["components"]["messageTraits"]["typed"]["contentType"] = json!("application/xml");
    assert_eq!(compare_mock_spec(&mock, &source).unwrap().routes_with_drift, 0);
    mock["components"]["messageTraits"]["typed"]["headers"]["required"] = json!(["other"]);
    assert!(compare_mock_spec(&mock, &source).unwrap().routes_with_drift > 0);
}

#[test]
fn schema_literal_traits_fields_are_never_applied_as_asyncapi_traits() {
    let mut source = v2();
    source["components"]["schemas"]["Order"]["properties"]["data"] =
        json!({"type":"object","default":{"traits":[{"x":1}],"x":2}});
    let mock = source.clone();
    source["components"]["schemas"]["Order"]["properties"]["data"]["default"]["traits"][0]["x"] =
        json!(9);
    assert!(compare_mock_spec(&mock, &source).unwrap().routes_with_drift > 0);
}

#[test]
fn content_type_inheritance_and_server_subset_are_effective_contracts() {
    let source = v2();
    let mut mock = source.clone();
    mock["components"]["messages"]["Order"]["contentType"] = json!("application/json");
    mock["servers"]["unused"] = json!({"url":"other:1883","protocol":"mqtt"});
    mock["channels"]["orders"]["servers"] = json!(["local"]);
    assert_eq!(compare_mock_spec(&mock, &source).unwrap().routes_with_drift, 0);
    mock["defaultContentType"] = json!("text/plain");
    assert_eq!(compare_mock_spec(&mock, &source).unwrap().routes_with_drift, 0);
    mock["components"]["messages"]["Order"]
        .as_object_mut()
        .unwrap()
        .remove("contentType");
    assert!(compare_mock_spec(&mock, &source).unwrap().routes_with_drift > 0);
}

#[test]
fn missing_cyclic_external_and_invalid_operation_references_fail_closed() {
    for reference in [
        "#/missing",
        "#/components/messages/Loop",
        "file:///never-read",
        "https://example.invalid/never-fetch",
    ] {
        let mut source = v2();
        source["components"]["messages"]["Loop"] = json!({"$ref":"#/components/messages/Loop"});
        source["channels"]["orders"]["publish"]["message"] = json!({"$ref":reference});
        assert!(compare_mock_spec(&source, &source).is_err());
    }
    let mut source = v3();
    source["operations"]["receiveOrder"]["messages"] =
        json!([{"$ref":"#/channels/orders/messages/description/payload"}]);
    assert!(compare_mock_spec(&source, &source).is_err());
}

#[test]
fn metadata_schema_order_and_comparison_budget_remain_consistent() {
    let source = v2();
    let mut mock = source.clone();
    mock["info"]["title"] = json!("Different documentation");
    mock["channels"]["orders"]["description"] = json!("Only prose");
    mock["components"]["schemas"]["Order"]["required"] = json!(["id", "description"]);
    let mut source = source;
    source["components"]["schemas"]["Order"]["required"] = json!(["description", "id"]);
    assert_eq!(compare_mock_spec(&mock, &source).unwrap().routes_with_drift, 0);
    assert!(compare_mock_spec_with_budget(&mock, &source, 100).is_err());
    assert!(compare_mock_spec(&mock, &json!({"openapi":"3.0.3","paths":{}})).is_err());
}

#[test]
fn x_prefixed_channel_names_are_preserved_as_real_topics() {
    let mut source = v2();
    source["channels"]["x-events"] = source["channels"]["orders"].take();
    source["channels"].as_object_mut().unwrap().remove("orders");
    let report = compare_mock_spec(&source, &source).unwrap();
    assert!(report.routes.iter().any(|r| r.endpoint == "RECEIVE x-events"));
    let mock = source.clone();
    source["channels"]["x-events"]["bindings"]["mqtt"]["qos"] = json!(2);
    assert!(compare_mock_spec(&mock, &source).unwrap().routes_with_drift > 0);
}

#[test]
fn v3_default_message_set_equals_an_explicit_reordered_selection() {
    let mut source = v3();
    source["channels"]["orders"]["messages"]["another"] = json!({"payload":{"type":"string"}});
    source["operations"]["receiveOrder"].as_object_mut().unwrap().remove("messages");
    let mut mock = source.clone();
    mock["operations"]["receiveOrder"]["messages"] = json!([
        {"$ref":"#/channels/orders/messages/description"},
        {"$ref":"#/channels/orders/messages/another"}
    ]);
    assert_eq!(compare_mock_spec(&mock, &source).unwrap().routes_with_drift, 0);
}

#[test]
fn protocol_binding_schemas_resolve_components_and_preserve_schema_fields() {
    let mut mock = v3();
    mock["components"]["schemas"]["Query"] =
        json!({"type":"object","properties":{"description":{"type":"string"}}});
    mock["channels"]["orders"]["bindings"] =
        json!({"ws":{"method":"GET","query":{"$ref":"#/components/schemas/Query"}}});
    let mut source = mock.clone();
    source["components"]["schemas"]["Query"]["properties"]["description"]["type"] =
        json!("integer");
    assert!(compare_mock_spec(&mock, &source).unwrap().routes_with_drift > 0);
    source["channels"]["orders"]["bindings"]["ws"]["query"] =
        json!({"$ref":"https://example.invalid/binding-schema"});
    assert!(compare_mock_spec(&source, &source).is_err());
}

#[test]
fn dynamic_channel_addresses_normalize_absence_and_null() {
    let mut mock = v3();
    mock["channels"]["orders"].as_object_mut().unwrap().remove("address");
    let mut source = mock.clone();
    source["channels"]["orders"]["address"] = Value::Null;
    assert_eq!(compare_mock_spec(&mock, &source).unwrap().routes_with_drift, 0);
    source["channels"]["orders"]["address"] = json!(12);
    assert!(compare_mock_spec(&source, &source).is_err());
}

#[test]
fn operation_trait_message_selections_are_not_overwritten_by_channel_defaults() {
    let mut mock = v3();
    mock["channels"]["orders"]["messages"]["other"] = json!({"payload":{"type":"integer"}});
    mock["operations"]["receiveOrder"].as_object_mut().unwrap().remove("messages");
    mock["components"]["operationTraits"]["selection"] =
        json!({"messages":[{"$ref":"#/channels/orders/messages/description"}]});
    mock["operations"]["receiveOrder"]["traits"] =
        json!([{"$ref":"#/components/operationTraits/selection"}]);
    let mut source = mock.clone();
    source["components"]["operationTraits"]["selection"]["messages"] =
        json!([{"$ref":"#/channels/orders/messages/other"}]);
    let report = compare_mock_spec(&mock, &source).unwrap();
    assert!(report
        .routes
        .iter()
        .any(|r| r.endpoint.contains("[receiveOrder]") && r.change == "changed"));
}

#[test]
fn null_constraints_survive_inline_contract_normalization() {
    let mut mock = v2();
    mock["channels"]["orders"]["publish"]["message"] = json!({"payload":{"const":null}});
    let mut source = mock.clone();
    source["channels"]["orders"]["publish"]["message"]["payload"] = json!({});
    assert!(
        compare_mock_spec(&mock, &source).unwrap().routes_with_drift > 0,
        "an explicit null constraint was erased"
    );
}
