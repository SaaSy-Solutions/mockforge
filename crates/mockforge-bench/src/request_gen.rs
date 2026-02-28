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

    /// Process a parameter and add it to the template
    #[allow(dead_code)]
    fn process_parameter(param: &Parameter, template: &mut RequestTemplate) -> Result<()> {
        Self::process_parameter_with_overrides(param, template, None)
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
                template.headers.insert(param_data.name.clone(), value);
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
            SchemaKind::Type(Type::Number(_)) => json!(42.0),
            SchemaKind::Type(Type::Integer(_)) => json!(42),
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
}
