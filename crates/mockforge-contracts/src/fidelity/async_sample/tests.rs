use super::*;
use serde_json::json;

fn fixture(protocol: &str) -> Value {
    json!({"asyncapi":"2.6.0","info":{"title":"events","version":"1"},"defaultContentType":"application/json",
        "servers":{"local":{"url":"localhost:1","protocol":protocol}},
        "components":{"schemas":{"Order":{"type":"object","required":["id"],"properties":{"id":{"type":"integer","minimum":1},"child":{"$ref":"#/components/schemas/Order"}}}},"messages":{"Order":{"payload":{"$ref":"#/components/schemas/Order"}}}},
        "channels":{"orders/{id}":{"publish":{"message":{"$ref":"#/components/messages/Order"}}}}})
}
fn captured(protocol: &str, operation: &str, text: &str) -> AsyncMessage {
    AsyncMessage {
        protocol: protocol.into(),
        operation: operation.into(),
        address: Some("orders/one".into()),
        headers: None,
        payload: Some(text.as_bytes().to_vec()),
        timestamp: DateTime::parse_from_rfc3339("2026-10-20T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
    }
}
fn status(report: &AsyncSampleReport, endpoint: &str) -> &'static str {
    report.routes.iter().find(|r| r.endpoint == endpoint).unwrap().status
}
fn v3() -> Value {
    json!({"asyncapi":"3.0.0","info":{"title":"events","version":"1"},"defaultContentType":"application/json","servers":{"local":{"host":"localhost","protocol":"mqtt"}},"channels":{"orders":{"address":"orders/{id}","messages":{"Order":{"payload":{"type":"object","required":["id"],"properties":{"id":{"type":"integer"}}}}}}},"operations":{"receive":{"action":"receive","channel":{"$ref":"#/channels/orders"}}}})
}

#[test]
fn all_four_protocols_check_inbound_payloads_with_exact_types_and_recursive_refs() {
    for (protocol, server, operation) in [
        ("mqtt", "mqtts", "publish"),
        ("kafka", "kafka-secure", "produce"),
        ("amqp", "amqps", "publish"),
        ("websocket", "wss", "inbound"),
    ] {
        let source = fixture(server);
        let report = verify(
            &source,
            &[captured(
                protocol,
                operation,
                r#"{"id":1,"child":{"id":2}}"#,
            )],
            16 * 1024 * 1024,
        )
        .unwrap();
        assert_eq!(status(&report, "RECEIVE orders/{id}"), "verified");
        assert_eq!(status(&report, "CHANNEL orders/{id}"), "verified");
        for bad in [
            r#"{"id":"1"}"#,
            r#"{"id":0}"#,
            r#"{"id":1,"child":{"id":"2"}}"#,
            "invalid JSON",
        ] {
            let report =
                verify(&source, &[captured(protocol, operation, bad)], 16 * 1024 * 1024).unwrap();
            assert_eq!(status(&report, "RECEIVE orders/{id}"), "drift-detected");
        }
    }
}

#[test]
fn outbound_direction_does_not_get_checked_against_inbound_operations() {
    for (protocol, operation) in [
        ("mqtt", "deliver"),
        ("kafka", "consume"),
        ("amqp", "deliver"),
        ("websocket", "outbound"),
    ] {
        let mut source = fixture(match protocol {
            "websocket" => "ws",
            p => p,
        });
        let channel = &mut source["channels"]["orders/{id}"];
        channel["subscribe"] = channel["publish"].take();
        channel.as_object_mut().unwrap().remove("publish");
        let report =
            verify(&source, &[captured(protocol, operation, r#"{"id":1}"#)], 16 * 1024 * 1024)
                .unwrap();
        assert_eq!(status(&report, "SEND orders/{id}"), "verified");
    }
    let report = verify(
        &fixture("mqtt"),
        &[captured("mqtt", "deliver", r#"{"id":1}"#)],
        16 * 1024 * 1024,
    )
    .unwrap();
    assert!(report.routes.iter().any(|r| r.status == "drift-detected"));
}

#[test]
fn v3_selected_message_sets_and_unused_operations_remain_distinct() {
    let mut source = v3();
    source["operations"]["send"] = source["operations"]["receive"].clone();
    source["operations"]["send"]["action"] = json!("send");
    let report =
        verify(&source, &[captured("mqtt", "publish", r#"{"id":1}"#)], 16 * 1024 * 1024).unwrap();
    assert_eq!(status(&report, "RECEIVE orders/{id} [receive]"), "verified");
    assert_eq!(status(&report, "SEND orders/{id} [send]"), "not-sampled");
}

#[test]
fn captured_headers_validate_and_missing_capture_is_unverified() {
    let mut source = fixture("mqtt");
    source["components"]["messages"]["Order"]["headers"] =
        json!({"type":"object","required":["trace"],"properties":{"trace":{"type":"string"}}});
    let mut record = captured("mqtt", "publish", r#"{"id":1}"#);
    let report = verify(&source, &[record], 16 * 1024 * 1024).unwrap();
    assert_eq!(status(&report, "RECEIVE orders/{id}"), "not-verified");
    record = captured("mqtt", "publish", r#"{"id":1}"#);
    record.headers = Some(json!({}));
    let report = verify(&source, &[record], 16 * 1024 * 1024).unwrap();
    assert_eq!(status(&report, "RECEIVE orders/{id}"), "drift-detected");
    let mut record = captured("mqtt", "publish", r#"{"id":1}"#);
    record.headers = Some(json!({"trace":"owned"}));
    let report = verify(&source, &[record], 16 * 1024 * 1024).unwrap();
    assert_eq!(status(&report, "RECEIVE orders/{id}"), "verified");
}

#[test]
fn unsupported_formats_missing_capture_and_unspecified_media_cannot_pass() {
    for (key, value) in [
        ("schemaFormat", json!("application/vnd.apache.avro;version=1.9.0")),
        ("contentType", json!("application/octet-stream")),
    ] {
        let mut source = fixture("mqtt");
        source["components"]["messages"]["Order"][key] = value;
        let report =
            verify(&source, &[captured("mqtt", "publish", r#"{"id":1}"#)], 16 * 1024 * 1024)
                .unwrap();
        assert_eq!(status(&report, "RECEIVE orders/{id}"), "not-verified");
    }
    let mut source = fixture("mqtt");
    source.as_object_mut().unwrap().remove("defaultContentType");
    let report =
        verify(&source, &[captured("mqtt", "publish", r#"{"id":1}"#)], 16 * 1024 * 1024).unwrap();
    assert_eq!(status(&report, "RECEIVE orders/{id}"), "not-verified");
    let mut record = captured("mqtt", "publish", r#"{"id":1}"#);
    record.payload = None;
    let report = verify(&fixture("mqtt"), &[record], 16 * 1024 * 1024).unwrap();
    assert_eq!(status(&report, "RECEIVE orders/{id}"), "not-verified");
}

#[test]
fn overlapping_message_alternatives_are_not_silently_accepted() {
    let mut source = fixture("mqtt");
    let message = source["channels"]["orders/{id}"]["publish"]["message"].clone();
    source["channels"]["orders/{id}"]["publish"]["message"] =
        json!({"oneOf":[message.clone(),message]});
    let report =
        verify(&source, &[captured("mqtt", "publish", r#"{"id":1}"#)], 16 * 1024 * 1024).unwrap();
    assert_eq!(status(&report, "RECEIVE orders/{id}"), "drift-detected");
}

#[test]
fn multiple_same_channel_operations_are_not_falsely_attributed() {
    let mut source = v3();
    source["operations"]["other"] = source["operations"]["receive"].clone();
    let report =
        verify(&source, &[captured("mqtt", "publish", r#"{"id":1}"#)], 16 * 1024 * 1024).unwrap();
    assert_eq!(status(&report, "RECEIVE orders/{id} [receive]"), "not-verified");
    assert_eq!(status(&report, "RECEIVE orders/{id} [other]"), "not-verified");
}

#[test]
fn unknown_channel_protocol_non_message_ops_and_dynamic_addresses_are_explicit() {
    let mut record = captured("mqtt", "publish", r#"{"id":1}"#);
    record.address = Some("removed".into());
    let report = verify(&fixture("mqtt"), &[record], 16 * 1024 * 1024).unwrap();
    assert!(report.routes.iter().any(|r| r.status == "drift-detected"));
    let mut source = fixture("mqtt");
    source.as_object_mut().unwrap().remove("servers");
    let report =
        verify(&source, &[captured("mqtt", "publish", r#"{"id":1}"#)], 16 * 1024 * 1024).unwrap();
    assert_eq!(status(&report, "RECEIVE orders/{id}"), "not-verified");
    let report =
        verify(&fixture("mqtt"), &[captured("mqtt", "subscribe", "")], 16 * 1024 * 1024).unwrap();
    assert!(report.routes.iter().any(|r| r.status == "not-verified"));
    let mut source = v3();
    source["channels"]["orders"]["address"] = Value::Null;
    let report =
        verify(&source, &[captured("mqtt", "publish", r#"{"id":1}"#)], 16 * 1024 * 1024).unwrap();
    assert!(report.routes.iter().any(|r| r.status == "not-verified"));
}

#[test]
fn text_messages_validate_as_strings_and_do_not_coerce_to_numbers() {
    let mut source = fixture("mqtt");
    source["components"]["messages"]["Order"] =
        json!({"contentType":"text/plain","payload":{"type":"string","enum":["2"]}});
    let report = verify(&source, &[captured("mqtt", "publish", "2")], 16 * 1024 * 1024).unwrap();
    assert_eq!(status(&report, "RECEIVE orders/{id}"), "verified");
    source["components"]["messages"]["Order"]["payload"]["type"] = json!("integer");
    let report = verify(&source, &[captured("mqtt", "publish", "2")], 16 * 1024 * 1024).unwrap();
    assert_eq!(status(&report, "RECEIVE orders/{id}"), "drift-detected");
}

#[test]
fn schema_literals_and_unused_external_references_are_not_fetched() {
    let mut source = fixture("mqtt");
    source["components"]["schemas"]["Order"]["properties"]["$id"] = json!({"type":"string"});
    source["components"]["schemas"]["Order"]["properties"]["literal"] =
        json!({"enum":[{"$ref":"not-a-schema-reference"}]});
    source["components"]["schemas"]["Unused"] =
        json!({"$ref":"https://example.invalid/never-fetch"});
    let report = verify(
        &source,
        &[captured(
            "mqtt",
            "publish",
            r#"{"id":1,"literal":{"$ref":"not-a-schema-reference"},"$id":"data"}"#,
        )],
        16 * 1024 * 1024,
    )
    .unwrap();
    assert_eq!(status(&report, "RECEIVE orders/{id}"), "verified");
    source["components"]["schemas"]["Order"]["properties"]["id"] =
        json!({"$ref":"file:///never-read"});
    assert!(
        verify(&source, &[captured("mqtt", "publish", r#"{"id":1}"#)], 16 * 1024 * 1024).is_err()
    );
    assert!(verify(&fixture("mqtt"), &[], 16 * 1024 * 1024).is_err());
}

#[test]
fn multi_format_payload_and_header_wrappers_do_not_skip_the_inner_schema() {
    let mut source = v3();
    source["channels"]["orders"]["messages"]["Order"]["payload"] = json!({"schemaFormat":"application/schema+json;version=draft-07","schema":{"type":"integer","minimum":1}});
    for (payload, expected) in [
        ("2", "verified"),
        ("\"2\"", "drift-detected"),
        ("0", "drift-detected"),
    ] {
        let report =
            verify(&source, &[captured("mqtt", "publish", payload)], 16 * 1024 * 1024).unwrap();
        assert_eq!(status(&report, "RECEIVE orders/{id} [receive]"), expected);
    }
    source["channels"]["orders"]["messages"]["Order"]["headers"] = json!({"schemaFormat":"application/schema+json;version=draft-07","schema":{"type":"object","required":["trace"]}});
    let mut record = captured("mqtt", "publish", "2");
    record.headers = Some(json!({}));
    let report = verify(&source, &[record], 16 * 1024 * 1024).unwrap();
    assert_eq!(status(&report, "RECEIVE orders/{id} [receive]"), "drift-detected");
}

#[test]
fn ambiguous_operations_count_channel_messages_once_and_detect_impossible_payloads() {
    let mut source = v3();
    source["operations"]["other"] = source["operations"]["receive"].clone();
    let report =
        verify(&source, &[captured("mqtt", "publish", r#"{"id":1}"#)], 16 * 1024 * 1024).unwrap();
    let channel = report.routes.iter().find(|r| r.endpoint == "CHANNEL orders").unwrap();
    assert_eq!(channel.exchanges_checked, 1);
    let report =
        verify(&source, &[captured("mqtt", "publish", r#"{"id":"wrong"}"#)], 16 * 1024 * 1024)
            .unwrap();
    assert_eq!(status(&report, "CHANNEL orders"), "drift-detected");
}

#[test]
fn operation_trait_message_selections_control_the_runtime_validator() {
    let mut source = v3();
    source["channels"]["orders"]["messages"]["other"] = json!({"payload":{"type":"integer"}});
    source["components"]["operationTraits"]["selection"] =
        json!({"messages":[{"$ref":"#/channels/orders/messages/other"}]});
    source["operations"]["receive"]["traits"] =
        json!([{"$ref":"#/components/operationTraits/selection"}]);
    let report = verify(&source, &[captured("mqtt", "publish", "2")], 16 * 1024 * 1024).unwrap();
    assert_eq!(status(&report, "RECEIVE orders/{id} [receive]"), "verified");
    let report =
        verify(&source, &[captured("mqtt", "publish", r#"{"id":1}"#)], 16 * 1024 * 1024).unwrap();
    assert_eq!(status(&report, "RECEIVE orders/{id} [receive]"), "drift-detected");
}

#[test]
fn null_constraints_survive_message_trait_resolution() {
    for with_trait in [false, true] {
        let mut source = fixture("mqtt");
        source["components"]["messages"]["Order"]["payload"] = json!({"const":null});
        if with_trait {
            source["components"]["messages"]["Order"]["traits"] =
                json!([{"contentType":"application/json"}]);
        }
        let report =
            verify(&source, &[captured("mqtt", "publish", "null")], 16 * 1024 * 1024).unwrap();
        assert_eq!(status(&report, "RECEIVE orders/{id}"), "verified");
        let report =
            verify(&source, &[captured("mqtt", "publish", "2")], 16 * 1024 * 1024).unwrap();
        assert_eq!(
            status(&report, "RECEIVE orders/{id}"),
            "drift-detected",
            "const:null was lost (trait={with_trait})"
        );
    }
}

#[test]
fn unsupported_explicit_dialects_are_unverified_in_inline_and_referenced_schemas() {
    let schema = json!({"$schema":"https://json-schema.org/draft/2020-12/schema","type":"array","prefixItems":[{"const":1}],"minItems":1,"maxItems":1});
    for referenced in [false, true] {
        let mut source = fixture("mqtt");
        if referenced {
            source["components"]["schemas"]["Order"] = schema.clone();
        } else {
            source["components"]["messages"]["Order"]["payload"] = schema.clone();
        }
        let report =
            verify(&source, &[captured("mqtt", "publish", "[2]")], 16 * 1024 * 1024).unwrap();
        assert_eq!(status(&report, "RECEIVE orders/{id}"), "not-verified");
        assert!(report.routes.iter().any(|r| r.notes.iter().any(|n| n.contains("dialect"))));
    }
}

#[test]
fn explicit_draft07_is_validated_and_schema_named_data_fields_remain_literal() {
    let mut source = fixture("mqtt");
    source["components"]["messages"]["Order"]["payload"] = json!({"$schema":"http://json-schema.org/draft-07/schema#","type":"array","items":{"const":1},"minItems":1});
    for (data, status_value) in [("[1]", "verified"), ("[2]", "drift-detected")] {
        let report =
            verify(&source, &[captured("mqtt", "publish", data)], 16 * 1024 * 1024).unwrap();
        assert_eq!(status(&report, "RECEIVE orders/{id}"), status_value);
    }
    source["components"]["messages"]["Order"]["payload"] = json!({"type":"object","properties":{"$schema":{"type":"string"},"literal":{"const":{"$schema":"data"}}}});
    let report = verify(
        &source,
        &[captured(
            "mqtt",
            "publish",
            r#"{"$schema":"data","literal":{"$schema":"data"}}"#,
        )],
        16 * 1024 * 1024,
    )
    .unwrap();
    assert_eq!(status(&report, "RECEIVE orders/{id}"), "verified");
}

#[test]
fn schema_resolution_bases_are_unverified_instead_of_compiled_with_wrong_scope() {
    let mut source = fixture("mqtt");
    source["components"]["messages"]["Order"]["payload"] =
        json!({"$id":"urn:owned-message","type":"integer"});
    let report = verify(&source, &[captured("mqtt", "publish", "2")], 16 * 1024 * 1024).unwrap();
    assert_eq!(status(&report, "RECEIVE orders/{id}"), "not-verified");
}
