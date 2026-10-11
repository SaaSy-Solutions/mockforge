//! Swagger 2.0 conversion shared with direct specification loading.

use serde_json::Value;

/// Convert a parsed Swagger 2.0 document to OpenAPI 3.0.
///
/// The canonical converter also serves `mockforge serve --spec`, so import and
/// serving retain the same references, media examples, and parameter semantics.
pub fn convert_swagger2_to_openapi3(spec: &Value) -> Result<Value, String> {
    mockforge_openapi::swagger_convert::convert_swagger_to_openapi3(spec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn petstore_20() -> Value {
        json!({
            "swagger": "2.0",
            "info": { "title": "Pets", "version": "1.0.0" },
            "host": "pets.example.com",
            "basePath": "/v1",
            "schemes": ["https"],
            "consumes": ["application/json"],
            "produces": ["application/json"],
            "definitions": {
                "Pet": { "type": "object", "properties": { "name": { "type": "string" } } }
            },
            "securityDefinitions": {
                "api_key": { "type": "apiKey", "name": "api_key", "in": "header" },
                "basic": { "type": "basic" }
            },
            "paths": {
                "/pets/{id}": {
                    "get": {
                        "summary": "Find pet",
                        "parameters": [
                            { "name": "id", "in": "path", "required": true, "type": "string" },
                            { "name": "limit", "in": "query", "type": "integer" }
                        ],
                        "responses": {
                            "200": {
                                "description": "ok",
                                "schema": { "$ref": "#/definitions/Pet" }
                            }
                        }
                    },
                    "post": {
                        "summary": "Update pet",
                        "parameters": [
                            { "name": "id", "in": "path", "required": true, "type": "string" },
                            { "name": "body", "in": "body", "required": true,
                              "schema": { "$ref": "#/definitions/Pet" } }
                        ],
                        "responses": { "200": { "description": "ok" } }
                    }
                },
                "/pets": {
                    "post": {
                        "summary": "Add pet via form",
                        "parameters": [
                            { "name": "name", "in": "formData", "type": "string", "required": true },
                            { "name": "status", "in": "formData", "type": "string" }
                        ],
                        "responses": { "200": { "description": "ok" } }
                    }
                }
            }
        })
    }

    #[test]
    fn rejects_non_swagger_documents() {
        assert!(convert_swagger2_to_openapi3(&json!({"openapi": "3.0.0"})).is_err());
        assert!(convert_swagger2_to_openapi3(&json!("nope")).is_err());
    }

    #[test]
    fn converts_core_structure() {
        let out = convert_swagger2_to_openapi3(&petstore_20()).unwrap();
        assert_eq!(out["openapi"], "3.0.3");
        assert_eq!(out["info"]["title"], "Pets");
        assert_eq!(
            out["servers"][0]["url"], "https://pets.example.com/v1",
            "host+basePath+scheme fold into one server URL"
        );
        assert_eq!(out["components"]["schemas"]["Pet"]["type"], "object");
        assert_eq!(out["components"]["securitySchemes"]["api_key"]["in"], "header");
        assert_eq!(out["components"]["securitySchemes"]["basic"]["scheme"], "basic");
    }

    #[test]
    fn upgrades_parameters_and_responses() {
        let out = convert_swagger2_to_openapi3(&petstore_20()).unwrap();
        let get = &out["paths"]["/pets/{id}"]["get"];

        let id = get["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == "id")
            .expect("path param kept");
        assert_eq!(id["schema"]["type"], "string", "inline 2.0 type moves into schema");

        let resp_schema = &get["responses"]["200"]["content"]["application/json"]["schema"];
        assert_eq!(resp_schema["$ref"], "#/components/schemas/Pet");
    }

    #[test]
    fn body_parameter_becomes_request_body() {
        let out = convert_swagger2_to_openapi3(&petstore_20()).unwrap();
        let post = &out["paths"]["/pets/{id}"]["post"];
        assert_eq!(
            post["requestBody"]["content"]["application/json"]["schema"]["$ref"],
            "#/components/schemas/Pet"
        );
    }

    #[test]
    fn form_params_become_urlencoded_request_body() {
        let out = convert_swagger2_to_openapi3(&petstore_20()).unwrap();
        let post = &out["paths"]["/pets"]["post"];
        let props = &post["requestBody"]["content"]["application/x-www-form-urlencoded"]["schema"]
            ["properties"];
        assert!(props.get("name").is_some());
        assert!(props.get("status").is_some());
    }

    /// #838 round-trip acceptance: the converted document must be a valid
    /// OpenAPI 3.0 spec that the standard importer accepts.
    #[test]
    fn converted_spec_feeds_standard_openapi_import() {
        let converted = convert_swagger2_to_openapi3(&petstore_20()).unwrap();
        let text = serde_json::to_string(&converted).unwrap();
        let result = crate::import::openapi_import::import_openapi_spec(&text, None);
        assert!(result.is_ok(), "converted spec should import: {:?}", result.err());
        let imported = result.unwrap();
        assert_eq!(imported.spec_info.title, "Pets");
        let methods: Vec<_> = imported.routes.iter().map(|r| r.method.as_str()).collect();
        assert!(methods.contains(&"GET"));
        assert!(methods.contains(&"POST"));
    }
}
