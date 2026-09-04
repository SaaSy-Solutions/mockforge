//! Request template generation from OpenAPI operations

use crate::error::Result;
use crate::param_overrides::OperationOverrides;
use crate::spec_parser::ApiOperation;
use openapiv3::{
    MediaType, Parameter, ParameterData, ParameterSchemaOrContent, ReferenceOr, RequestBody,
    Schema, SchemaKind, Type,
};
use serde_json::{json, Value};
use std::collections::HashMap;

/// A request template for load testing
#[derive(Debug, Clone)]
pub struct RequestTemplate {
    pub operation: ApiOperation,
    pub path_params: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: Option<Value>,
}

impl RequestTemplate {
    /// Generate the full URL path with parameters substituted
    pub fn generate_path(&self) -> String {
        let mut path = self.operation.path.clone();

        for (key, value) in &self.path_params {
            path = path.replace(&format!("{{{}}}", key), value);
        }

        if !self.query_params.is_empty() {
            let query_string: Vec<String> =
                self.query_params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
            path = format!("{}?{}", path, query_string.join("&"));
        }

        path
    }

    /// Get all headers including content-type
    pub fn get_headers(&self) -> HashMap<String, String> {
        let mut headers = self.headers.clone();

        if self.body.is_some() {
            headers
                .entry("Content-Type".to_string())
                .or_insert_with(|| "application/json".to_string());
        }

        headers
    }
}

/// Request template generator
pub struct RequestGenerator;

impl RequestGenerator {
    /// Generate a request template from an API operation
    pub fn generate_template(operation: &ApiOperation) -> Result<RequestTemplate> {
        Self::generate_template_with_overrides(operation, None)
    }

    /// Generate a request template with optional parameter overrides
    ///
    /// When overrides are provided, they take precedence over auto-generated values.
    /// This allows users to provide realistic test data instead of placeholder values.
    pub fn generate_template_with_overrides(
        operation: &ApiOperation,
        overrides: Option<&OperationOverrides>,
    ) -> Result<RequestTemplate> {
        let mut template = RequestTemplate {
            operation: operation.clone(),
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: None,
        };

        // Extract parameters from OpenAPI spec
        for param_ref in &operation.operation.parameters {
            if let ReferenceOr::Item(param) = param_ref {
                Self::process_parameter_with_overrides(param, &mut template, overrides)?;
            }
        }

        // Apply any additional overridden parameters not in the spec
        if let Some(ovr) = overrides {
            // Add overridden path params that weren't in the spec
            for (name, value) in &ovr.path_params {
                template.path_params.entry(name.clone()).or_insert_with(|| value.clone());
            }
            // Add overridden query params that weren't in the spec
            for (name, value) in &ovr.query_params {
                template.query_params.entry(name.clone()).or_insert_with(|| value.clone());
            }
            // Add overridden headers that weren't in the spec
            for (name, value) in &ovr.headers {
                template.headers.entry(name.clone()).or_insert_with(|| value.clone());
            }
        }

        // Extract request body (override takes precedence)
        if let Some(ovr) = overrides {
            if let Some(body) = ovr.get_body() {
                template.body = Some(body.clone());
            } else if let Some(ReferenceOr::Item(request_body)) = &operation.operation.request_body
            {
                template.body = Self::generate_body(request_body)?;
            }
        } else if let Some(ReferenceOr::Item(request_body)) = &operation.operation.request_body {
            template.body = Self::generate_body(request_body)?;
        }

        Ok(template)
    }

    /// Process a parameter with optional overrides
    fn process_parameter_with_overrides(
        param: &Parameter,
        template: &mut RequestTemplate,
        overrides: Option<&OperationOverrides>,
    ) -> Result<()> {
        let (param_type, param_data) = match param {
            Parameter::Query { parameter_data, .. } => ("query", parameter_data),
            Parameter::Path { parameter_data, .. } => ("path", parameter_data),
            Parameter::Header { parameter_data, .. } => ("header", parameter_data),
            Parameter::Cookie { parameter_data, .. } => ("cookie", parameter_data),
        };

        // Check for override first, then fall back to generated value
        let value = if let Some(ovr) = overrides {
            match param_type {
                "path" => ovr.get_path_param(&param_data.name).cloned(),
                "query" => ovr.get_query_param(&param_data.name).cloned(),
                "header" => ovr.get_header(&param_data.name).cloned(),
                _ => None,
            }
        } else {
            None
        }
        .unwrap_or_else(|| Self::generate_param_value(param_data).unwrap_or_default());

        match param_type {
            "query" => {
                template.query_params.insert(param_data.name.clone(), value);
            }
            "path" => {
                template.path_params.insert(param_data.name.clone(), value);
            }
            "header" => {
                // Issue #79 (f) — S3 (and others) list Content-Length / Host as
                // header parameters. Filling those from the schema invents
                // `Content-Length: 42` on a 0-byte body. k6 then drops the
                // request: the declared length does not match the body.
                // Skip hop-by-hop / transport-owned names unless the user
                // overrode them on purpose (WAF CL-mismatch cases).
                let from_override =
                    overrides.and_then(|o| o.get_header(&param_data.name)).is_some();
                if Self::is_transport_owned_header(&param_data.name) && !from_override {
                    // Leave it to k6 / the HTTP client.
                } else {
                    template.headers.insert(param_data.name.clone(), value);
                }
            }
            "cookie" => {
                // Append cookie to existing Cookie header or create new one
                let cookie_pair = format!("{}={}", param_data.name, value);
                template
                    .headers
                    .entry("Cookie".to_string())
                    .and_modify(|existing| {
                        existing.push_str("; ");
                        existing.push_str(&cookie_pair);
                    })
                    .or_insert(cookie_pair);
            }
            _ => {}
        }

        Ok(())
    }

    /// Headers the HTTP client computes. Auto-filling them from an OpenAPI
    /// schema is how Srikanth's Amazon S3 spec produced `Content-Length: 42`
    /// on every empty POST and sent no useful traffic (#79 (f)).
    pub(crate) fn is_transport_owned_header(name: &str) -> bool {
        matches!(
            name.to_ascii_lowercase().as_str(),
            "content-length"
                | "transfer-encoding"
                | "host"
                | "connection"
                | "keep-alive"
                | "te"
                | "trailer"
                | "upgrade"
                | "proxy-connection"
        )
    }

    /// Generate a value for a parameter
    fn generate_param_value(param_data: &ParameterData) -> Result<String> {
        // Try to use example first
        if let Some(example) = &param_data.example {
            return Ok(example.to_string().trim_matches('"').to_string());
        }

        // Generate from schema
        if let ParameterSchemaOrContent::Schema(ReferenceOr::Item(schema)) = &param_data.format {
            return Ok(Self::generate_value_from_schema(schema));
        }

        // Default value based on parameter name
        Ok(Self::default_param_value(&param_data.name))
    }

    /// Generate a default value based on parameter name
    fn default_param_value(name: &str) -> String {
        match name.to_lowercase().as_str() {
            "id" => "1".to_string(),
            "limit" => "10".to_string(),
            "offset" => "0".to_string(),
            "page" => "1".to_string(),
            "sort" => "name".to_string(),
            _ => "test-value".to_string(),
        }
    }

    /// Generate a request body from a RequestBody definition
    fn generate_body(request_body: &RequestBody) -> Result<Option<Value>> {
        // Look for application/json content
        if let Some(content) = request_body.content.get("application/json") {
            return Ok(Some(Self::generate_json_body(content)));
        }

        Ok(None)
    }

    /// Generate JSON body from media type
    fn generate_json_body(media_type: &MediaType) -> Value {
        // Try to use example first
        if let Some(example) = &media_type.example {
            return example.clone();
        }

        // Generate from schema
        if let Some(ReferenceOr::Item(schema)) = &media_type.schema {
            return Self::generate_json_from_schema(schema);
        }

        json!({})
    }

    /// Generate JSON from schema
    fn generate_json_from_schema(schema: &Schema) -> Value {
        match &schema.schema_kind {
            SchemaKind::Type(Type::Object(obj)) => {
                let mut map = serde_json::Map::new();

                for (key, schema_ref) in &obj.properties {
                    if let ReferenceOr::Item(prop_schema) = schema_ref {
                        map.insert(key.clone(), Self::generate_json_from_schema(prop_schema));
                    }
                }

                Value::Object(map)
            }
            SchemaKind::Type(Type::Array(arr)) => {
                if let Some(ReferenceOr::Item(item_schema)) = &arr.items {
                    return json!([Self::generate_json_from_schema(item_schema)]);
                }
                json!([])
            }
            SchemaKind::Type(Type::String(_)) => Self::generate_string_value(schema),
            SchemaKind::Type(Type::Number(n)) => n
                .enumeration
                .iter()
                .flatten()
                .next()
                .map(|v| json!(v))
                .unwrap_or_else(|| json!(42.0)),
            SchemaKind::Type(Type::Integer(i)) => i
                .enumeration
                .iter()
                .flatten()
                .next()
                .map(|v| json!(v))
                .unwrap_or_else(|| json!(42)),
            SchemaKind::Type(Type::Boolean(_)) => json!(true),
            _ => json!(null),
        }
    }

    /// Generate a string value from schema
    fn generate_string_value(schema: &Schema) -> Value {
        // Use example if available
        if let Some(example) = &schema.schema_data.example {
            return example.clone();
        }
        // Round 51 (#79) — respect an enum so the positive body is spec-VALID
        // (Srikanth on 0.3.196: `billingType` was filled with the invalid
        // literal "test-string", tripping a body enum violation on every
        // probe). Negative body probes still override the value they attack.
        if let SchemaKind::Type(Type::String(s)) = &schema.schema_kind {
            if let Some(first) = s.enumeration.iter().flatten().next() {
                return json!(first);
            }
        }

        json!("test-string")
    }

    /// Generate a value from schema (for parameters)
    fn generate_value_from_schema(schema: &Schema) -> String {
        match &schema.schema_kind {
            SchemaKind::Type(Type::String(_)) => "test-value".to_string(),
            SchemaKind::Type(Type::Number(_)) => "42.0".to_string(),
            SchemaKind::Type(Type::Integer(_)) => "42".to_string(),
            SchemaKind::Type(Type::Boolean(_)) => "true".to_string(),
            _ => "test-value".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openapiv3::Operation;

    // Round 51 (#79) — a string property with an enum must generate a VALID
    // member, not the invalid literal "test-string".
    #[test]
    fn generate_string_value_uses_enum_member() {
        use openapiv3::{Schema, SchemaData, SchemaKind, StringType, Type};
        let st = StringType {
            enumeration: vec![Some("EVALUATION".into()), Some("PAYG".into())],
            ..Default::default()
        };
        let schema = Schema {
            schema_data: SchemaData::default(),
            schema_kind: SchemaKind::Type(Type::String(st)),
        };
        assert_eq!(RequestGenerator::generate_string_value(&schema), json!("EVALUATION"));

        // No enum -> the generic filler is fine.
        let plain = Schema {
            schema_data: SchemaData::default(),
            schema_kind: SchemaKind::Type(Type::String(StringType::default())),
        };
        assert_eq!(RequestGenerator::generate_string_value(&plain), json!("test-string"));
    }

    #[test]
    fn test_generate_path() {
        let op = ApiOperation {
            method: "get".to_string(),
            path: "/users/{id}".to_string(),
            operation: Operation::default(),
            operation_id: None,
        };

        let mut template = RequestTemplate {
            operation: op,
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: None,
        };

        template.path_params.insert("id".to_string(), "123".to_string());
        template.query_params.insert("limit".to_string(), "10".to_string());

        let path = template.generate_path();
        assert_eq!(path, "/users/123?limit=10");
    }

    #[test]
    fn test_default_param_value() {
        assert_eq!(RequestGenerator::default_param_value("id"), "1");
        assert_eq!(RequestGenerator::default_param_value("limit"), "10");
        assert_eq!(RequestGenerator::default_param_value("unknown"), "test-value");
    }

    /// #79 (f): an integer `Content-Length` header param must not be
    /// invented as `"42"`. k6 owns that header.
    #[test]
    fn spec_content_length_header_is_not_invented() {
        use openapiv3::{HeaderStyle, IntegerType, ParameterData, Schema, SchemaData, SchemaKind};

        let mut operation = Operation::default();
        operation.parameters.push(ReferenceOr::Item(Parameter::Header {
            parameter_data: ParameterData {
                name: "Content-Length".to_string(),
                description: None,
                required: false,
                deprecated: None,
                format: ParameterSchemaOrContent::Schema(ReferenceOr::Item(Schema {
                    schema_data: SchemaData::default(),
                    schema_kind: SchemaKind::Type(Type::Integer(IntegerType::default())),
                })),
                example: None,
                examples: Default::default(),
                explode: None,
                extensions: Default::default(),
            },
            style: HeaderStyle::Simple,
        }));
        // A normal header still comes through so we know the loop ran.
        operation.parameters.push(ReferenceOr::Item(Parameter::Header {
            parameter_data: ParameterData {
                name: "x-amz-request-route".to_string(),
                description: None,
                required: false,
                deprecated: None,
                format: ParameterSchemaOrContent::Schema(ReferenceOr::Item(Schema {
                    schema_data: SchemaData::default(),
                    schema_kind: SchemaKind::Type(Type::String(openapiv3::StringType::default())),
                })),
                example: None,
                examples: Default::default(),
                explode: None,
                extensions: Default::default(),
            },
            style: HeaderStyle::Simple,
        }));

        let api_op = ApiOperation {
            method: "post".to_string(),
            path: "/WriteGetObjectResponse".to_string(),
            operation,
            operation_id: Some("WriteGetObjectResponse".to_string()),
        };
        let template = RequestGenerator::generate_template(&api_op).expect("template");
        let headers = template.get_headers();
        assert!(
            !headers.keys().any(|k| k.eq_ignore_ascii_case("content-length")),
            "invented Content-Length={:?} would make k6 drop empty-body POSTs",
            headers.get("Content-Length")
        );
        assert_eq!(headers.get("x-amz-request-route").map(String::as_str), Some("test-value"));
        assert!(RequestGenerator::is_transport_owned_header("Content-Length"));
        assert!(RequestGenerator::is_transport_owned_header("HOST"));
        assert!(!RequestGenerator::is_transport_owned_header("x-amz-request-route"));
    }
}
