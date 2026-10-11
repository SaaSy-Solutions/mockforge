//! End-to-end loading and route generation for a representative legacy contract.

use mockforge_openapi::{
    openapi_routes::{OpenApiRouteRegistry, ValidationOptions},
    spec::OpenApiSpec,
};
use serde_json::{json, Value};
use std::error::Error;

const FIXTURE: &str = include_str!("../../../examples/swagger2-legacy.json");

type TestResult = Result<(), Box<dyn Error>>;

fn assert_routes(spec: OpenApiSpec) {
    let registry = OpenApiRouteRegistry::new_with_options(spec, ValidationOptions::default());
    assert_eq!(registry.routes().len(), 2);
    for (method, path, status) in [("GET", "/pets/{petId}", 200), ("POST", "/pets", 201)] {
        let route = registry
            .routes()
            .iter()
            .find(|route| route.method == method && route.path == path)
            .expect("Legacy operation must produce a mock route");
        let (actual_status, body) = route.mock_response_with_status();
        assert_eq!(actual_status, status);
        assert_eq!(body, json!({"id": 42, "name": "Fido", "owner": {"name": "Sam"}}));
    }
}

#[tokio::test]
async fn legacy_json_and_yaml_generate_routes_and_survive_normalized_round_trip() -> TestResult {
    let source: Value = serde_json::from_str(FIXTURE)?;
    for (suffix, content) in [
        (".json", FIXTURE.to_string()),
        (".yaml", serde_yaml::to_string(&source)?),
    ] {
        let file = tempfile::Builder::new().suffix(suffix).tempfile()?;
        std::fs::write(file.path(), content)?;
        let spec = OpenApiSpec::from_file(file.path()).await?;
        let raw = spec.raw_document.as_ref().expect("Converted document must be retained");
        assert_eq!(raw["openapi"], "3.0.3");
        assert_eq!(raw["servers"][0]["url"], "https://legacy.example.test/v1");
        assert_eq!(
            raw["components"]["schemas"]["Pet"]["properties"]["owner"]["$ref"],
            "#/components/schemas/Owner"
        );
        assert_eq!(
            raw["components"]["schemas"]["Pet"]["properties"]["metadata"]["example"]["$ref"],
            "#/definitions/literal-data"
        );
        assert_eq!(
            raw["paths"]["/pets"]["post"]["requestBody"]["content"]["application/json"]["schema"]
                ["$ref"],
            "#/components/schemas/Pet"
        );
        let normalized = serde_json::to_value(&spec.spec)?;
        assert_routes(OpenApiSpec::from_json(normalized)?);
        assert_routes(spec);
    }
    assert_routes(OpenApiSpec::from_json(source)?);
    Ok(())
}

#[test]
fn inherited_body_parameters_obey_operation_overrides() -> TestResult {
    let mut source: Value = serde_json::from_str(FIXTURE)?;
    source["paths"]["/pets"]["parameters"] = json!([{"$ref": "#/parameters/petBody"}]);
    source["paths"]["/pets"]["post"]["parameters"] = json!([]);
    let inherited = OpenApiSpec::from_json(source.clone())?;
    let raw = inherited.raw_document.as_ref().expect("Normalized document");
    assert_eq!(raw["paths"]["/pets"]["parameters"], json!([]));
    assert_eq!(
        raw["paths"]["/pets"]["post"]["requestBody"]["content"]["application/json"]["schema"]
            ["$ref"],
        "#/components/schemas/Pet"
    );
    source["paths"]["/pets"]["post"]["parameters"] = json!([{
        "name": "pet", "in": "body", "required": true, "schema": {"$ref": "#/definitions/Owner"}
    }]);
    let overridden = OpenApiSpec::from_json(source)?;
    let raw = overridden.raw_document.as_ref().expect("Normalized document");
    assert_eq!(
        raw["paths"]["/pets"]["post"]["requestBody"]["content"]["application/json"]["schema"]
            ["$ref"],
        "#/components/schemas/Owner"
    );
    Ok(())
}

#[test]
fn reused_responses_obey_operation_media_types_and_preserve_extensions() -> TestResult {
    let mut source: Value = serde_json::from_str(FIXTURE)?;
    source["paths"]["/pets/{petId}"]["get"]["produces"] = json!(["application/xml"]);
    source["responses"]["PetResponse"]["examples"]["application/xml"] =
        json!("<pet name=\"Fido\"/>");
    source["responses"]["PetResponse"]["x-origin"] = json!({"system": "legacy"});
    let spec = OpenApiSpec::from_json(source)?;
    let raw = spec.raw_document.as_ref().expect("Normalized document");
    let response = &raw["paths"]["/pets/{petId}"]["get"]["responses"]["200"];
    assert_eq!(response["content"]["application/xml"]["example"], "<pet name=\"Fido\"/>");
    assert!(response["content"].get("application/json").is_none());
    assert_eq!(response["x-origin"], json!({"system": "legacy"}));
    Ok(())
}

#[test]
fn missing_and_cyclic_reusable_references_fail_explicitly() -> TestResult {
    let mut source: Value = serde_json::from_str(FIXTURE)?;
    source["parameters"]["petId"] = json!({"$ref": "#/parameters/missing"});
    let missing = OpenApiSpec::from_json(source.clone()).expect_err("Missing reference must fail");
    assert!(missing.to_string().contains("Missing Swagger reference"));
    source["parameters"]["petId"] = json!({"$ref": "#/parameters/petId"});
    let cyclic = OpenApiSpec::from_json(source).expect_err("Cycle must fail without recursion");
    assert!(cyclic.to_string().contains("Cyclic Swagger reference"));
    Ok(())
}

#[test]
fn path_parameters_are_inherited_once_and_operation_overrides_win() -> TestResult {
    let mut source: Value = serde_json::from_str(FIXTURE)?;
    source["paths"]["/pets/{petId}"]["parameters"] = json!([
        {"$ref": "#/parameters/petId"},
        {"name":"X-Client","in":"header","type":"string","minLength":2}
    ]);
    source["paths"]["/pets/{petId}"]["get"]["parameters"] = json!([
        {"name":"X-Client","in":"header","type":"integer","minimum":1}
    ]);
    let spec = OpenApiSpec::from_json(source)?;
    let raw = spec.raw_document.as_ref().expect("Normalized document");
    assert_eq!(raw["paths"]["/pets/{petId}"]["parameters"], json!([]));
    let parameters = raw["paths"]["/pets/{petId}"]["get"]["parameters"]
        .as_array()
        .expect("Operation parameters");
    assert_eq!(parameters.len(), 2);
    let header = parameters
        .iter()
        .find(|parameter| parameter["name"] == "X-Client")
        .expect("Inherited header override");
    assert_eq!(header["schema"]["type"], "integer");
    assert_eq!(header["schema"]["minimum"], 1);
    assert!(header["schema"].get("minLength").is_none());
    Ok(())
}

#[test]
fn required_multipart_files_and_form_constraints_survive_conversion() -> TestResult {
    let mut source: Value = serde_json::from_str(FIXTURE)?;
    source["paths"]["/pets"]["post"]["parameters"] = json!([
        {"name":"upload","in":"formData","type":"file","required":true},
        {"name":"label","in":"formData","type":"string","minLength":2,"maxLength":12}
    ]);
    let spec = OpenApiSpec::from_json(source)?;
    let raw = spec.raw_document.as_ref().expect("Normalized document");
    let body = &raw["paths"]["/pets"]["post"]["requestBody"];
    assert_eq!(body["required"], true);
    let schema = &body["content"]["multipart/form-data"]["schema"];
    assert_eq!(schema["required"], json!(["upload"]));
    assert_eq!(schema["properties"]["upload"]["type"], "string");
    assert_eq!(schema["properties"]["upload"]["format"], "binary");
    assert_eq!(schema["properties"]["label"]["minLength"], 2);
    assert_eq!(schema["properties"]["label"]["maxLength"], 12);
    Ok(())
}
