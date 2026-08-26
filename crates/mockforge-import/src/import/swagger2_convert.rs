//! Swagger / OpenAPI 2.0 → OpenAPI 3.0 up-conversion (#838).
//!
//! The importer's parse path is openapiv3-based, so a 2.0 spec is
//! converted to a 3.0 document here and the rest of the pipeline runs
//! unchanged. The transform covers the constructs that appear in the
//! wild (host/basePath/schemes → servers, definitions → schemas,
//! body/formData parameters → requestBody, response schemas → content,
//! securityDefinitions → securitySchemes). Exotic vendor extensions are
//! passed through verbatim so nothing is silently dropped.

use serde_json::{json, Map, Value};

const DEFAULT_CONSUMES: &str = "application/json";
const DEFAULT_PRODUCES: &str = "application/json";

/// Convert a parsed Swagger 2.0 document to OpenAPI 3.0.
pub fn convert_swagger2_to_openapi3(spec: &Value) -> Result<Value, String> {
    let obj = spec
        .as_object()
        .ok_or_else(|| "Swagger spec must be a JSON object".to_string())?;
    if obj.get("swagger").and_then(Value::as_str) != Some("2.0") {
        return Err("not a Swagger 2.0 document (missing \"swagger\": \"2.0\")".to_string());
    }

    let mut out = Map::new();
    out.insert("openapi".into(), json!("3.0.0"));

    // Top-level pass-throughs.
    for key in ["info", "tags", "externalDocs", "x-"] {
        copy_matching(obj, key, &mut out);
    }
    if let Some(title) = obj.get("info").and_then(|i| i.get("title")) {
        let _ = title; // already copied above; kept for readability
    }

    // servers ← host + basePath + schemes.
    out.insert("servers".into(), json!(build_servers(obj)));

    // definitions → components.schemas ; securityDefinitions → securitySchemes.
    let mut components = Map::new();
    if let Some(Value::Object(defs)) = obj.get("definitions") {
        components.insert("schemas".into(), Value::Object(defs.clone()));
    }
    if let Some(sec) = obj.get("securityDefinitions") {
        components.insert("securitySchemes".into(), convert_security_schemes(sec));
    }
    if !components.is_empty() {
        out.insert("components".into(), Value::Object(components));
    }
    if let Some(security) = obj.get("security") {
        out.insert("security".into(), security.clone());
    }

    // Global media types, overridable per operation.
    let global_consumes = string_array(obj.get("consumes"));
    let global_produces = string_array(obj.get("produces"));

    // paths.
    let empty_map = Map::new();
    let paths = obj.get("paths").and_then(Value::as_object).unwrap_or(&empty_map);
    let mut new_paths = Map::new();
    for (path, item) in paths {
        new_paths.insert(path.clone(), convert_path_item(item, &global_consumes, &global_produces));
    }
    out.insert("paths".into(), Value::Object(new_paths));

    Ok(Value::Object(out))
}

fn copy_matching(src: &Map<String, Value>, prefix: &str, dst: &mut Map<String, Value>) {
    for (k, v) in src {
        if k == prefix
            || k.starts_with(&format!("{prefix}-"))
            || k.starts_with(prefix) && prefix == "x-"
        {
            dst.insert(k.clone(), v.clone());
        }
    }
}

fn build_servers(obj: &Map<String, Value>) -> Vec<Value> {
    let host = obj.get("host").and_then(Value::as_str).unwrap_or("localhost");
    let base_path = obj
        .get("basePath")
        .and_then(Value::as_str)
        .filter(|b| !b.is_empty())
        .unwrap_or("/");
    let schemes = string_array(obj.get("schemes"));
    let schemes: Vec<&str> = if schemes.is_empty() {
        vec!["https"]
    } else {
        schemes
    };
    schemes
        .iter()
        .map(|s| json!({ "url": format!("{s}://{host}{base_path}") }))
        .collect()
}

fn string_array(v: Option<&Value>) -> Vec<&str> {
    v.and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

fn convert_path_item(item: &Value, consumes: &[&str], produces: &[&str]) -> Value {
    match item {
        Value::Object(fields) => {
            let mut out = Map::new();
            // Path-level parameters merge into operations by name+in (2.0
            // semantics); we approximate by appending — consumers of the 3.0
            // doc treat both levels as cumulative anyway.
            let path_level_params = fields.get("parameters").cloned();
            for (k, v) in fields {
                if k == "parameters" {
                    continue;
                }
                if is_operation(k) {
                    out.insert(
                        k.clone(),
                        convert_operation(v, consumes, produces, path_level_params.as_ref()),
                    );
                } else {
                    out.insert(k.clone(), v.clone());
                }
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

fn is_operation(key: &str) -> bool {
    matches!(key, "get" | "put" | "post" | "delete" | "options" | "head" | "patch")
}

fn convert_operation(
    op: &Value,
    global_consumes: &[&str],
    global_produces: &[&str],
    path_level_params: Option<&Value>,
) -> Value {
    let fields = op.as_object().cloned().unwrap_or_default();

    let op_consumes = merged_media_types(&fields, "consumes", global_consumes);
    let op_produces = merged_media_types(&fields, "produces", global_produces);

    let all_params: Vec<Value> = {
        let mut p: Vec<Value> =
            path_level_params.and_then(Value::as_array).cloned().unwrap_or_default();
        if let Some(own) = fields.get("parameters").and_then(Value::as_array) {
            p.extend(own.iter().cloned());
        }
        p
    };

    let mut out = Map::new();
    for (k, v) in &fields {
        if k == "parameters"
            || k == "consumes"
            || k == "produces"
            || k == "responses"
            || k == "schemes"
        {
            continue;
        }
        out.insert(k.clone(), v.clone());
    }

    // Split parameters.
    let mut plain_params: Vec<Value> = Vec::new();
    let mut body_schema: Option<&Value> = None;
    let mut form_fields: Vec<Value> = Vec::new();
    let mut has_file = false;
    for p in &all_params {
        let location = p.get("in").and_then(Value::as_str).unwrap_or("");
        match location {
            "body" => body_schema = p.get("schema"),
            "formData" => {
                if p.get("type").and_then(Value::as_str) == Some("file") {
                    has_file = true;
                }
                form_fields.push(p.clone());
            }
            // query/header/path: 2.0 carried type inline; 3.0 wants a
            // `schema` object.
            _ => plain_params.push(upgrade_inline_schema(p)),
        }
    }
    if !plain_params.is_empty() {
        out.insert("parameters".into(), Value::Array(plain_params));
    }

    // requestBody ← body parameter or formData fields.
    if let Some(schema) = body_schema {
        let ct = op_consumes.first().copied().unwrap_or(DEFAULT_CONSUMES);
        out.insert("requestBody".into(), request_body(ct, schema));
    } else if !form_fields.is_empty() {
        let ct = if has_file {
            "multipart/form-data"
        } else {
            "application/x-www-form-urlencoded"
        };
        let mut properties = Map::new();
        let mut required_names: Vec<Value> = Vec::new();
        for (name, prop, required) in form_fields.iter().map(convert_form_param) {
            if required {
                required_names.push(json!(name));
            }
            properties.insert(name, prop);
        }
        let mut schema_obj = Map::new();
        schema_obj.insert("type".into(), json!("object"));
        schema_obj.insert("properties".into(), Value::Object(properties));
        if !required_names.is_empty() {
            schema_obj.insert("required".into(), Value::Array(required_names));
        }
        let schema = Value::Object(schema_obj);
        out.insert("requestBody".into(), request_body(ct, &schema));
    }

    // responses: wrap each response's `schema` in content.
    if let Some(responses) = fields.get("responses") {
        out.insert("responses".into(), convert_responses(responses, op_produces.first().copied()));
    }

    Value::Object(out)
}

fn merged_media_types<'a>(
    fields: &'a Map<String, Value>,
    key: &str,
    global: &'a [&'a str],
) -> Vec<&'a str> {
    let own = string_array(fields.get(key));
    if own.is_empty() {
        global.to_vec()
    } else {
        own
    }
}

fn upgrade_inline_schema(param: &Value) -> Value {
    let mut out = param.as_object().cloned().unwrap_or_default();
    // NOTE: Parameter.required stays boolean here — correct for 3.0
    // parameters. The boolean never reaches *schema* position because the
    // schema whitelist below excludes it; form properties strip it
    // explicitly in convert_form_param.
    if let (Some(typ), None) = (param.get("type").map(|v| v.clone()), param.get("schema")) {
        let mut schema = Map::new();
        for k in [
            "type",
            "format",
            "items",
            "enum",
            "maximum",
            "minimum",
            "maxLength",
            "minLength",
            "pattern",
            "default",
        ] {
            if let Some(v) = param.get(k) {
                schema.insert(k.to_string(), v.clone());
            }
        }
        let _ = typ;
        out.insert("schema".into(), Value::Object(schema));
    }
    Value::Object(out)
}

/// Returns `(name, schema_property)` for one formData parameter.
///
/// 2.0 carried `required: bool` on the parameter; in a 3.0 *schema*
/// property that must not survive as a boolean (`required` inside a
/// schema is an array of names), so the caller collects it instead.
fn convert_form_param(p: &Value) -> (String, Value, bool) {
    let name = p.get("name").and_then(Value::as_str).unwrap_or("field").to_string();
    let mut prop = upgrade_inline_schema(p);
    if let Some(o) = prop.as_object_mut() {
        o.remove("in");
        o.remove("name");
        let required = matches!(o.remove("required"), Some(Value::Bool(true)));
        return (name, Value::Object(o.clone()), required);
    }
    (name, prop, false)
}

fn request_body(content_type: &str, schema: &Value) -> Value {
    json!({
        "required": true,
        "content": { content_type: { "schema": schema } }
    })
}

fn convert_responses(responses: &Value, produce_ct: Option<&str>) -> Value {
    let ct = produce_ct.unwrap_or(DEFAULT_PRODUCES);
    let mut out = Map::new();
    if let Some(map) = responses.as_object() {
        for (code, resp) in map {
            let mut converted = resp.as_object().cloned().unwrap_or_default();
            if let Some(schema) = converted.remove("schema") {
                converted.insert("content".into(), json!({ ct: { "schema": schema } }));
            }
            // Headers in 2.0 carry type inline; wrap them too.
            if let Some(Value::Object(headers)) = converted.get_mut("headers") {
                for h in headers.values_mut() {
                    *h = upgrade_inline_schema(h);
                }
            }
            out.insert(code.clone(), Value::Object(converted));
        }
    }
    Value::Object(out)
}

fn convert_security_schemes(defs: &Value) -> Value {
    let mut out = Map::new();
    if let Some(map) = defs.as_object() {
        for (name, def) in map {
            let t = def.get("type").and_then(Value::as_str).unwrap_or("");
            let scheme = match t {
                "basic" => json!({ "type": "http", "scheme": "basic" }),
                "apiKey" => {
                    let mut o = Map::new();
                    o.insert("type".into(), json!("apiKey"));
                    o.insert("name".into(), def.get("name").cloned().unwrap_or(json!("api_key")));
                    o.insert("in".into(), def.get("in").cloned().unwrap_or(json!("header")));
                    Value::Object(o)
                }
                "oauth2" => {
                    let authorization_url = def
                        .get("authorizationUrl")
                        .cloned()
                        .unwrap_or_else(|| json!("https://example.com/oauth/authorize"));
                    let token_url = def
                        .get("tokenUrl")
                        .cloned()
                        .unwrap_or_else(|| json!("https://example.com/oauth/token"));
                    let scopes = def.get("scopes").cloned().unwrap_or_else(|| json!({}));
                    json!({
                        "type": "oauth2",
                        "flows": {
                            "implicit": {
                                "authorizationUrl": authorization_url,
                                "tokenUrl": token_url,
                                "scopes": scopes,
                            }
                        }
                    })
                }
                other => {
                    // Unknown type — preserve verbatim so nothing is lost.
                    let _ = other;
                    def.clone()
                }
            };
            out.insert(name.clone(), scheme);
        }
    }
    Value::Object(out)
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
        assert_eq!(out["openapi"], "3.0.0");
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
        assert_eq!(resp_schema["$ref"], "#/definitions/Pet");
    }

    #[test]
    fn body_parameter_becomes_request_body() {
        let out = convert_swagger2_to_openapi3(&petstore_20()).unwrap();
        let post = &out["paths"]["/pets/{id}"]["post"];
        assert_eq!(
            post["requestBody"]["content"]["application/json"]["schema"]["$ref"],
            "#/definitions/Pet"
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
