use super::*;
use serde_json::json;

fn fixture(schema: Value, schemas: Value) -> (OpenApiSpec, openapiv3::ParameterData) {
    let parameter = json!({"name":"value","in":"query","required":false,"schema":schema});
    let spec = OpenApiSpec::from_json(json!({"openapi":"3.0.3","info":{"title":"parameter-test","version":"1"},
        "components":{"schemas":schemas},"paths":{"/items":{"get":{"parameters":[parameter.clone()],"responses":{"200":{"description":"ok"}}}}}})).unwrap();
    let parameter: openapiv3::Parameter = serde_json::from_value(parameter).unwrap();
    let openapiv3::Parameter::Query { parameter_data, .. } = parameter else {
        unreachable!()
    };
    (spec, parameter_data)
}

fn check(schema: Value, schemas: Value, value: Option<Value>, expected_valid: bool) {
    let (spec, parameter) = fixture(schema, schemas);
    check_parameter(&spec, &parameter, value, expected_valid);
}

fn check_parameter(
    spec: &OpenApiSpec,
    parameter: &openapiv3::ParameterData,
    value: Option<Value>,
    expected_valid: bool,
) {
    let values = value.clone().map(|v| json!({"value":v})).unwrap_or(json!({}));
    let map = values.as_object().unwrap();
    let mut errors = Vec::new();
    let mut details = Vec::new();
    validate_parameter(parameter, map, "query", true, &mut errors, &mut details, spec);
    assert_eq!(errors.is_empty(), expected_valid, "normal: {errors:?}");
    let mut detailed = Vec::new();
    validate_parameter_detailed(parameter, map, "query", "query parameter", &mut detailed, spec);
    assert_eq!(detailed.is_empty(), expected_valid, "detailed: {detailed:?}");
    errors.clear();
    details.clear();
    detailed.clear();
    validate_parameter_with_deep_object(
        parameter,
        map,
        "query",
        value.clone(),
        Some("form"),
        true,
        &mut errors,
        &mut details,
        spec,
    );
    assert_eq!(errors.is_empty(), expected_valid, "style: {errors:?}");
    validate_parameter_detailed_with_deep(
        parameter,
        map,
        "query",
        "query parameter",
        value,
        &mut detailed,
        Some("form"),
        spec,
    );
    assert_eq!(detailed.is_empty(), expected_valid, "detailed style: {detailed:?}");
}

#[test]
fn referenced_enums_and_aliases_are_checked_by_every_parameter_validator() {
    let schemas = json!({"Kind":{"type":"string","enum":["one"]},"Alias":{"$ref":"#/components/schemas/Kind"}});
    for name in ["Kind", "Alias"] {
        let schema = json!({"$ref":format!("#/components/schemas/{name}")});
        check(schema.clone(), schemas.clone(), Some(json!("one")), true);
        check(schema, schemas.clone(), Some(json!("wrong")), false);
    }
}

#[test]
fn unresolved_cyclic_and_remote_root_references_fail_even_without_optional_values() {
    for reference in [
        "#/components/schemas/Missing",
        "#/components/schemas/Loop",
        "https://example.invalid/schema",
    ] {
        check(
            json!({"$ref":reference}),
            json!({"Loop":{"$ref":"#/components/schemas/Loop"}}),
            None,
            false,
        );
    }
}

#[test]
fn referenced_array_items_keep_wire_coercion_and_string_values() {
    let schemas = json!({"Count":{"type":"integer","minimum":1},"List":{"type":"array","items":{"$ref":"#/components/schemas/Count"}}});
    check(
        json!({"$ref":"#/components/schemas/List"}),
        schemas.clone(),
        Some(json!("1,2")),
        true,
    );
    check(
        json!({"$ref":"#/components/schemas/List"}),
        schemas,
        Some(json!("1,wrong")),
        false,
    );
    check(
        json!({"type":"array","items":{"type":"string","enum":["1","true"]}}),
        json!({}),
        Some(json!("1,true")),
        true,
    );
}

#[test]
fn nested_and_recursive_component_schemas_validate_after_coercion() {
    let schemas = json!({"Count":{"type":"integer","minimum":1},"Node":{"type":"object","required":["id"],"properties":{
        "id":{"$ref":"#/components/schemas/Count"},"child":{"$ref":"#/components/schemas/Node"}
    }}});
    check(
        json!({"$ref":"#/components/schemas/Node"}),
        schemas.clone(),
        Some(json!({"id":"1","child":{"id":"2"}})),
        true,
    );
    check(
        json!({"$ref":"#/components/schemas/Node"}),
        schemas,
        Some(json!({"id":"1","child":{"id":"bad"}})),
        false,
    );
}

#[test]
fn nested_external_files_are_not_loaded() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    use std::io::Write;
    file.write_all(br#"{"type":"integer"}"#).unwrap();
    let schema = json!({"type":"object","properties":{"id":{"$ref":format!("file://{}",file.path().display())}}});
    check(schema.clone(), json!({}), Some(json!({"id":1})), false);
    check(schema, json!({}), Some(json!({})), false);
}

#[test]
fn deep_object_references_work_in_registry_and_enhanced_error_paths() {
    let (mut spec, _) = fixture(
        json!({"$ref":"#/components/schemas/Filter"}),
        json!({
            "Count":{"type":"integer","minimum":1},"Filter":{"type":"object","required":["age"],"properties":{"age":{"$ref":"#/components/schemas/Count"}}}
        }),
    );
    let openapiv3::ReferenceOr::Item(path) = spec.spec.paths.paths.get_mut("/items").unwrap()
    else {
        unreachable!()
    };
    let operation = path.get.as_mut().unwrap();
    let openapiv3::ReferenceOr::Item(openapiv3::Parameter::Query { style, .. }) =
        &mut operation.parameters[0]
    else {
        unreachable!()
    };
    *style = openapiv3::QueryStyle::DeepObject;
    let registry = OpenApiRouteRegistry::new_for_contract_verification(spec);
    for (value, valid) in [("3", true), ("bad", false)] {
        let query = json!({"value[age]":value});
        let query = query.as_object().unwrap();
        let result = registry.validate_request_with_all_ex(
            "/items",
            "GET",
            &Map::new(),
            query,
            &Map::new(),
            &Map::new(),
            None,
            false,
        );
        assert_eq!(result.is_ok(), valid, "{result:?}");
        let detailed = generate_enhanced_422_response(
            &registry,
            "/items",
            "GET",
            None,
            &Map::new(),
            query,
            &Map::new(),
            &Map::new(),
        );
        assert_eq!(detailed["details"].as_array().unwrap().is_empty(), valid, "{detailed}");
    }
}

#[test]
fn composed_and_additional_property_references_preserve_constraints() {
    let schemas = json!({"Count":{"type":"integer","minimum":1},"Base":{"type":"object","properties":{"age":{"$ref":"#/components/schemas/Count"}}}});
    check(
        json!({"allOf":[{"$ref":"#/components/schemas/Base"},{"type":"object","required":["age"]}]}),
        schemas.clone(),
        Some(json!({"age":"2"})),
        true,
    );
    check(
        json!({"allOf":[{"$ref":"#/components/schemas/Base"},{"type":"object","required":["age"]}]}),
        schemas.clone(),
        Some(json!({"age":"bad"})),
        false,
    );
    check(
        json!({"type":"object","additionalProperties":{"$ref":"#/components/schemas/Count"}}),
        schemas.clone(),
        Some(json!({"age":"2"})),
        true,
    );
    check(
        json!({"anyOf":[{"$ref":"#/components/schemas/Count"},{"type":"string","enum":["none"]}]}),
        schemas,
        Some(json!("none")),
        true,
    );
}

#[test]
fn referenced_integer_parameters_keep_precision_above_float_integer_range() {
    let schemas = json!({"Id":{"type":"integer","enum":[9007199254740993_u64]}});
    check(
        json!({"$ref":"#/components/schemas/Id"}),
        schemas.clone(),
        Some(json!("9007199254740993")),
        true,
    );
    check(
        json!({"$ref":"#/components/schemas/Id"}),
        schemas,
        Some(json!("9007199254740992")),
        false,
    );
}

fn check_content(
    media: &str,
    schema: Option<Value>,
    schemas: Value,
    value: Option<Value>,
    valid: bool,
) {
    let (spec, mut parameter) = fixture(json!({}), schemas);
    let declaration = match schema {
        Some(schema) => json!({media:{"schema":schema}}),
        None => json!({media:{}}),
    };
    parameter.format =
        openapiv3::ParameterSchemaOrContent::Content(serde_json::from_value(declaration).unwrap());
    check_parameter(&spec, &parameter, value, valid);
}

#[test]
fn json_content_preserves_json_types_and_rejects_malformed_values() {
    for (text, valid) in [
        ("2", true),
        ("0", false),
        ("wrong", false),
        ("\"2\"", false),
        ("null", false),
        ("2 trailing", false),
    ] {
        check_content(
            "application/json",
            Some(json!({"type":"integer","minimum":1})),
            json!({}),
            Some(json!(text)),
            valid,
        );
    }
    check_content(
        "application/json",
        None,
        json!({}),
        Some(json!("{\"anything\":[1,true,null]}")),
        true,
    );
    check_content("application/json", None, json!({}), Some(json!("wrong")), false);
    check_content(
        "application/json",
        Some(json!({"type":"string"})),
        json!({}),
        Some(json!("\"2\"")),
        true,
    );
    check_content(
        "application/json",
        Some(json!({"type":"string"})),
        json!({}),
        Some(json!("2")),
        false,
    );
}

#[test]
fn json_content_resolves_root_aliases_and_nested_components_without_coercion() {
    let schemas = json!({"Count":{"type":"integer","minimum":1},"Object":{"type":"object","required":["count"],"properties":{"count":{"$ref":"#/components/schemas/Count"}}},"Alias":{"$ref":"#/components/schemas/Object"}});
    for media in ["application/json", "application/vnd.example+json"] {
        for (text, valid) in [
            ("{\"count\":2}", true),
            ("{\"count\":\"2\"}", false),
            ("{}", false),
        ] {
            check_content(
                media,
                Some(json!({"$ref":"#/components/schemas/Alias"})),
                schemas.clone(),
                Some(json!(text)),
                valid,
            );
        }
    }
    check_content(
        "application/json",
        Some(json!({"$ref":"https://example.invalid/schema"})),
        json!({}),
        None,
        false,
    );
}

#[test]
fn unsupported_parameter_media_cannot_silently_pass() {
    for value in [None, Some(json!("text"))] {
        check_content("text/plain", Some(json!({"type":"string"})), json!({}), value, false);
    }
}
