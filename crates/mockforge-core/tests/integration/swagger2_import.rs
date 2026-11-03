//! The import command must accept the same legacy contracts as the spec loader.

// Exercise the deprecated compatibility API until its consumers migrate.
#![allow(deprecated)]

use mockforge_core::import::openapi_import::import_openapi_spec;
use serde_json::{json, Value};
use std::error::Error;

#[test]
fn legacy_json_and_yaml_import_into_serializable_mock_routes() -> Result<(), Box<dyn Error>> {
    let source = include_str!("../../../../examples/swagger2-legacy.json");
    let json: Value = serde_json::from_str(source)?;
    for content in [source.to_string(), serde_yaml::to_string(&json)?] {
        let imported = import_openapi_spec(&content, None)?;
        assert_eq!(imported.spec_info.openapi_version, "3.0.3");
        assert_eq!(imported.spec_info.servers, ["https://legacy.example.test/v1"]);
        assert_eq!(imported.routes.len(), 2);
        let routes = serde_json::to_value(&imported.routes)?;
        for (method, path, status) in [("GET", "/pets/:petId", 200), ("POST", "/pets", 201)] {
            let route = routes
                .as_array()
                .expect("Serialized route array")
                .iter()
                .find(|route| route["method"] == method && route["path"] == path)
                .expect("Legacy operation must import into a mock route");
            assert_eq!(route["response"]["status"], status);
            assert_eq!(
                route["response"]["body"],
                json!({"id":42,"name":"Fido","owner":{"name":"Sam"}})
            );
        }
    }
    Ok(())
}
