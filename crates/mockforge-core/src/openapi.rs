//! OpenAPI specification handling for MockForge
//!
//! This module provides comprehensive OpenAPI 3.0 specification support including:
//! - Loading and parsing OpenAPI specs from files (JSON/YAML)
//! - Route generation from OpenAPI paths
//! - Schema validation against OpenAPI definitions
//! - Mock response generation based on OpenAPI schemas

use crate::{Error, Result};
use openapiv3::{OpenAPI, Operation, Parameter, ParameterSchemaOrContent, ReferenceOr, Response, Schema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use uuid::Uuid;
use chrono::{NaiveDate, DateTime};
use url::Url;
use regex::Regex;
use once_cell::sync::Lazy;
use openapiv3::{SchemaKind, Type, StringFormat, VariantOrUnknownOrEmpty};

// Simple email regex for practical validation (not full RFC 5322)
// Accepts forms like local@domain.tld and rejects obvious invalids
static EMAIL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[^@\s]+@[^@\s]+\.[^@\s]+$").expect("valid email regex")
});

/// OpenAPI specification loader and parser
#[derive(Debug, Clone)]
pub struct OpenApiSpec {
    /// The parsed OpenAPI specification
    pub spec: OpenAPI,
    /// Path to the original spec file
    pub file_path: Option<String>,
}

impl OpenApiSpec {
    /// Load OpenAPI spec from a file path
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| Error::generic(format!("Failed to read OpenAPI spec file: {}", e)))?;

        let spec: OpenAPI = if path.as_ref().extension().and_then(|s| s.to_str()) == Some("yaml")
            || path.as_ref().extension().and_then(|s| s.to_str()) == Some("yml")
        {
            serde_yaml::from_str(&content)
                .map_err(|e| Error::generic(format!("Failed to parse YAML OpenAPI spec: {}", e)))?
        } else {
            serde_json::from_str(&content)
                .map_err(|e| Error::generic(format!("Failed to parse JSON OpenAPI spec: {}", e)))?
        };

        Ok(Self {
            spec,
            file_path: Some(path.as_ref().to_string_lossy().to_string()),
        })
    }

    /// Load OpenAPI spec from a JSON value
    pub fn from_json(value: Value) -> Result<Self> {
        let spec: OpenAPI = serde_json::from_value(value).map_err(|e| {
            Error::generic(format!("Failed to parse OpenAPI spec from JSON: {}", e))
        })?;

        Ok(Self {
            spec,
            file_path: None,
        })
    }

    /// Get all paths defined in the OpenAPI spec
    pub fn paths(&self) -> &openapiv3::Paths {
        &self.spec.paths
    }

    /// Get all operations for a given path
    pub fn operations_for_path(&self, path: &str) -> Vec<(String, &Operation)> {
        let mut operations = Vec::new();

        if let Some(path_item_ref) = self.spec.paths.paths.get(path) {
            // Handle the ReferenceOr<PathItem> case
            if let Some(path_item) = path_item_ref.as_item() {
                if let Some(op) = &path_item.get {
                    operations.push(("GET".to_string(), op));
                }
                if let Some(op) = &path_item.post {
                    operations.push(("POST".to_string(), op));
                }
                if let Some(op) = &path_item.put {
                    operations.push(("PUT".to_string(), op));
                }
                if let Some(op) = &path_item.delete {
                    operations.push(("DELETE".to_string(), op));
                }
                if let Some(op) = &path_item.patch {
                    operations.push(("PATCH".to_string(), op));
                }
                if let Some(op) = &path_item.head {
                    operations.push(("HEAD".to_string(), op));
                }
                if let Some(op) = &path_item.options {
                    operations.push(("OPTIONS".to_string(), op));
                }
                if let Some(op) = &path_item.trace {
                    operations.push(("TRACE".to_string(), op));
                }
            }
        }

        operations
    }

    /// Get all paths with their operations
    pub fn all_paths_and_operations(&self) -> Vec<(String, Vec<(String, &Operation)>)> {
        self.spec
            .paths
            .paths
            .iter()
            .map(|(path, _)| (path.clone(), self.operations_for_path(path)))
            .collect()
    }

    /// Get a schema by reference
    pub fn get_schema(&self, reference: &str) -> Option<&Schema> {
        let name = reference.trim_start_matches("#/components/schemas/");
        let entry = self
            .spec
            .components
            .as_ref()?
            .schemas
            .get(name)?;
        match entry {
            ReferenceOr::Item(schema) => Some(schema),
            ReferenceOr::Reference { reference } => self.get_schema(reference),
        }
    }

    /// Get a parameter by reference
    pub fn get_parameter(&self, reference: &str) -> Option<&Parameter> {
        let name = reference.trim_start_matches("#/components/parameters/");
        let entry = self
            .spec
            .components
            .as_ref()?
            .parameters
            .get(name)?;
        match entry {
            ReferenceOr::Item(param) => Some(param),
            ReferenceOr::Reference { reference } => self.get_parameter(reference),
        }
    }

    /// Get a request body by reference
    pub fn get_request_body(&self, reference: &str) -> Option<&openapiv3::RequestBody> {
        let name = reference.trim_start_matches("#/components/requestBodies/");
        let entry = self
            .spec
            .components
            .as_ref()?
            .request_bodies
            .get(name)?;
        match entry {
            ReferenceOr::Item(rb) => Some(rb),
            ReferenceOr::Reference { reference } => self.get_request_body(reference),
        }
    }

    /// Get a response by reference
    pub fn get_response(&self, reference: &str) -> Option<&Response> {
        let name = reference.trim_start_matches("#/components/responses/");
        let entry = self
            .spec
            .components
            .as_ref()?
            .responses
            .get(name)?;
        match entry {
            ReferenceOr::Item(resp) => Some(resp),
            ReferenceOr::Reference { reference } => self.get_response(reference),
        }
    }

    /// Validate that this is a valid OpenAPI 3.0 spec
    pub fn validate(&self) -> Result<()> {
        // Basic validation - check required fields
        if self.spec.openapi.is_empty() {
            return Err(Error::generic("OpenAPI version is required"));
        }

        if !self.spec.openapi.starts_with("3.") {
            return Err(Error::generic(format!(
                "Unsupported OpenAPI version: {}. Only 3.x is supported",
                self.spec.openapi
            )));
        }

        if self.spec.info.title.is_empty() {
            return Err(Error::generic("API title is required"));
        }

        Ok(())
    }

    /// Get the API title
    pub fn title(&self) -> &str {
        &self.spec.info.title
    }

    /// Get the API version
    pub fn version(&self) -> &str {
        &self.spec.info.version
    }

    /// Get the API description
    pub fn description(&self) -> Option<&str> {
        self.spec.info.description.as_deref()
    }
}

/// OpenAPI operation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiOperation {
    /// HTTP method
    pub method: String,
    /// Path template
    pub path: String,
    /// Operation ID
    pub operation_id: Option<String>,
    /// Summary
    pub summary: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Parameters
    pub parameters: Vec<OpenApiParameter>,
    /// Request body schema (if any)
    pub request_body: Option<OpenApiSchema>,
    /// Response schemas
    pub responses: HashMap<String, OpenApiResponse>,
    /// Security requirements
    pub security: Vec<OpenApiSecurityRequirement>,
}

impl OpenApiOperation {
    /// Create from OpenAPI operation
    pub fn from_operation(method: String, path: String, operation: &Operation, spec: &OpenApiSpec) -> Self {
        Self {
            method,
            path,
            operation_id: operation.operation_id.clone(),
            summary: operation.summary.clone(),
            description: operation.description.clone(),
            parameters: operation
                .parameters
                .iter()
                .filter_map(|p| OpenApiParameter::from_parameter(p, spec))
                .collect(),
            request_body: operation
                .request_body
                .as_ref()
                .and_then(|rb| OpenApiSchema::from_request_body(rb, spec)),
            responses: operation
                .responses
                .responses
                .iter()
                .filter_map(|(code, resp)| {
                    OpenApiResponse::from_response(resp, spec).map(|r| (code.to_string(), r))
                })
                .collect(),
            security: vec![], // TODO: Implement security requirement parsing
        }
    }
}

/// OpenAPI parameter information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiParameter {
    /// Parameter name
    pub name: String,
    /// Parameter location
    pub location: String,
    /// Whether parameter is required
    pub required: bool,
    /// Parameter schema
    pub schema: Option<OpenApiSchema>,
    /// Parameter description
    pub description: Option<String>,
}

impl OpenApiParameter {
    /// Create from OpenAPI parameter
    pub fn from_parameter(param_ref: &ReferenceOr<Parameter>, spec: &OpenApiSpec) -> Option<Self> {
        match param_ref {
            ReferenceOr::Item(param) => {
                let (param_data, location) = match param {
                    Parameter::Query { parameter_data, .. } => (parameter_data, "query"),
                    Parameter::Path { parameter_data, .. } => (parameter_data, "path"),
                    Parameter::Header { parameter_data, .. } => (parameter_data, "header"),
                    Parameter::Cookie { parameter_data, .. } => (parameter_data, "cookie"),
                };

                // Extract schema if present
                let schema = match &param_data.format {
                    ParameterSchemaOrContent::Schema(ref_or_schema) => match ref_or_schema {
                        ReferenceOr::Item(schema) => OpenApiSchema::from_schema_data(schema, spec),
                        ReferenceOr::Reference { reference } => spec
                            .get_schema(reference)
                            .and_then(|s| OpenApiSchema::from_schema_data(s, spec)),
                    },
                    ParameterSchemaOrContent::Content(_) => None,
                };

                Some(Self {
                    name: param_data.name.clone(),
                    location: location.to_string(),
                    required: param_data.required,
                    schema,
                    description: param_data.description.clone(),
                })
            }
            ReferenceOr::Reference { reference } => spec
                .get_parameter(reference)
                .and_then(|p| OpenApiParameter::from_parameter(&ReferenceOr::Item(p.clone()), spec)),
        }
    }
}

/// OpenAPI schema information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSchema {
    /// Schema type
    pub schema_type: Option<String>,
    /// Schema format
    pub format: Option<String>,
    /// Schema description
    pub description: Option<String>,
    /// Schema properties (for objects)
    pub properties: HashMap<String, Box<OpenApiSchema>>,
    /// Required properties
    pub required: Vec<String>,
    /// Items schema (for arrays)
    pub items: Option<Box<OpenApiSchema>>,
    /// Enum values
    pub enum_values: Option<Vec<Value>>,
    /// Minimum value
    pub minimum: Option<f64>,
    /// Maximum value
    pub maximum: Option<f64>,
    /// Minimum length
    pub min_length: Option<usize>,
    /// Maximum length
    pub max_length: Option<usize>,
}

impl OpenApiSchema {
    /// Create from OpenAPI schema
    pub fn from_schema(schema_ref: &ReferenceOr<Schema>, spec: &OpenApiSpec) -> Option<Self> {
        match schema_ref {
            ReferenceOr::Item(schema) => Self::from_schema_data(schema, spec),
            ReferenceOr::Reference { reference } => spec
                .get_schema(reference)
                .and_then(|s| Self::from_schema_data(s, spec)),
        }
    }

    /// Create from OpenAPI schema data
    pub fn from_schema_data(schema: &Schema, spec: &OpenApiSpec) -> Option<Self> {
        let mut out = Self {
            schema_type: None,
            format: None,
            description: schema.schema_data.description.clone(),
            properties: HashMap::new(),
            required: Vec::new(),
            items: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
        };

        match &schema.schema_kind {
            SchemaKind::Type(ty) => match ty {
                Type::String(st) => {
                    out.schema_type = Some("string".to_string());
                    // map format
                    out.format = match &st.format {
                        VariantOrUnknownOrEmpty::Item(f) => Some(match f {
                            StringFormat::Byte => "byte",
                            StringFormat::Binary => "binary",
                            StringFormat::Date => "date",
                            StringFormat::DateTime => "date-time",
                            StringFormat::Password => "password",
                            _ => "string",
                        }.to_string()),
                        VariantOrUnknownOrEmpty::Unknown(s) => Some(s.clone()),
                        VariantOrUnknownOrEmpty::Empty => None,
                    };
                    out.min_length = st.min_length;
                    out.max_length = st.max_length;
                    if !st.enumeration.is_empty() {
                        let vals = st
                            .enumeration
                            .iter()
                            .map(|opt| opt.clone().map(Value::String).unwrap_or(Value::Null))
                            .collect::<Vec<_>>();
                        out.enum_values = Some(vals);
                    }
                }
                Type::Number(nt) => {
                    out.schema_type = Some("number".to_string());
                    out.minimum = nt.minimum;
                    out.maximum = nt.maximum;
                    if !nt.enumeration.is_empty() {
                        let vals = nt
                            .enumeration
                            .iter()
                            .map(|opt| opt.map(Value::from).unwrap_or(Value::Null))
                            .collect();
                        out.enum_values = Some(vals);
                    }
                }
                Type::Integer(it) => {
                    out.schema_type = Some("integer".to_string());
                    out.minimum = it.minimum.map(|v| v as f64);
                    out.maximum = it.maximum.map(|v| v as f64);
                    if !it.enumeration.is_empty() {
                        let vals = it
                            .enumeration
                            .iter()
                            .map(|opt| opt.map(|v| Value::from(v as i64)).unwrap_or(Value::Null))
                            .collect();
                        out.enum_values = Some(vals);
                    }
                }
                Type::Boolean(_) => {
                    out.schema_type = Some("boolean".to_string());
                }
                Type::Array(at) => {
                    out.schema_type = Some("array".to_string());
                    if let Some(items) = &at.items {
                        match items {
                            ReferenceOr::Item(b) => {
                                if let Some(mapped) = Self::from_schema_data(b, spec) {
                                    out.items = Some(Box::new(mapped));
                                }
                            }
                            ReferenceOr::Reference { reference } => {
                                if let Some(resolved) = spec.get_schema(reference) {
                                    if let Some(mapped) = Self::from_schema_data(resolved, spec) {
                                        out.items = Some(Box::new(mapped));
                                    }
                                }
                            }
                        }
                    }
                }
                Type::Object(ot) => {
                    out.schema_type = Some("object".to_string());
                    // properties
                    for (name, prop_schema) in &ot.properties {
                        match prop_schema {
                            ReferenceOr::Item(b) => {
                                if let Some(mapped) = Self::from_schema_data(b, spec) {
                                    out.properties.insert(name.clone(), Box::new(mapped));
                                }
                            }
                            ReferenceOr::Reference { reference } => {
                                if let Some(resolved) = spec.get_schema(reference) {
                                    if let Some(mapped) = Self::from_schema_data(resolved, spec) {
                                        out.properties.insert(name.clone(), Box::new(mapped));
                                    }
                                }
                            }
                        }
                    }
                    out.required = ot.required.clone();
                }
            },
            // For composite/any, keep generic object
            _ => {}
        }

        Some(out)
    }

    /// Create from request body
    pub fn from_request_body(request_body: &ReferenceOr<openapiv3::RequestBody>, spec: &OpenApiSpec) -> Option<Self> {
        match request_body {
            ReferenceOr::Item(rb) => {
                rb.content
                    .get("application/json")
                    .or_else(|| rb.content.get("*/*"))
                    .and_then(|media| media.schema.as_ref())
                    .and_then(|s| Self::from_schema(s, spec))
            }
            ReferenceOr::Reference { reference } => spec
                .get_request_body(reference)
                .and_then(|rb| {
                    rb.content
                        .get("application/json")
                        .or_else(|| rb.content.get("*/*"))
                        .and_then(|media| media.schema.as_ref())
                        .and_then(|s| Self::from_schema(s, spec))
                }),
        }
    }

    /// Generate a mock value for this schema
    pub fn generate_mock_value(&self) -> Value {
        match self.schema_type.as_deref() {
            Some("string") => {
                if let Some(enum_vals) = &self.enum_values {
                    if !enum_vals.is_empty() {
                        return enum_vals[0].clone();
                    }
                }
                Value::String("mock_string".to_string())
            }
            Some("number") | Some("integer") => {
                if let Some(min) = self.minimum {
                    Value::Number(serde_json::Number::from_f64(min).unwrap_or(42.into()))
                } else {
                    Value::Number(42.into())
                }
            }
            Some("boolean") => Value::Bool(true),
            Some("object") => {
                let mut obj = serde_json::Map::new();
                for (name, prop) in &self.properties {
                    obj.insert(name.clone(), prop.generate_mock_value());
                }
                Value::Object(obj)
            }
            Some("array") => {
                if let Some(items) = &self.items {
                    Value::Array(vec![items.generate_mock_value(), items.generate_mock_value()])
                } else {
                    Value::Array(vec![Value::String("mock_item".to_string())])
                }
            }
            _ => Value::Null,
        }
    }

    /// Validate a JSON value against this simplified schema
    pub fn validate_value(&self, value: &Value, path: &str) -> Result<()> {
        // enum check first (applies to many types)
        if let Some(enum_vals) = &self.enum_values {
            if !enum_vals.is_empty() && !enum_vals.iter().any(|v| v == value) {
                return Err(Error::validation(format!(
                    "{}: value not in enum {:?}",
                    path, enum_vals
                )));
            }
        }

        match self.schema_type.as_deref() {
            Some("string") => {
                let s = value.as_str().ok_or_else(|| {
                    Error::validation(format!("{}: expected string, got {}", path, value))
                })?;
                if let Some(min) = self.min_length {
                    if s.len() < min {
                        return Err(Error::validation(format!(
                            "{}: minLength {} not satisfied",
                            path, min
                        )));
                    }
                }
                if let Some(max) = self.max_length {
                    if s.len() > max {
                        return Err(Error::validation(format!(
                            "{}: maxLength {} exceeded",
                            path, max
                        )));
                    }
                }
                        if let Some(fmt) = &self.format {
                            match fmt.as_str() {
                                "email" => {
                                    if !EMAIL_RE.is_match(s) {
                                        return Err(Error::validation(format!(
                                            "{}: invalid email format",
                                            path
                                        )));
                                    }
                                }
                                "uri" => {
                                    Url::parse(s).map_err(|_| {
                                        Error::validation(format!(
                                            "{}: invalid uri format",
                                            path
                                        ))
                                    })?;
                                }
                                "uuid" => {
                                    Uuid::parse_str(s).map_err(|_| {
                                        Error::validation(format!("{}: invalid uuid format", path))
                                    })?;
                                }
                        "date-time" => {
                            DateTime::parse_from_rfc3339(s).map_err(|_| {
                                Error::validation(format!("{}: invalid date-time format", path))
                            })?;
                        }
                        "date" => {
                            NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| {
                                Error::validation(format!("{}: invalid date format (YYYY-MM-DD)", path))
                            })?;
                        }
                        _ => {}
                    }
                }
                Ok(())
            }
            Some("number") | Some("integer") => {
                let n = value.as_f64().ok_or_else(|| {
                    Error::validation(format!("{}: expected number, got {}", path, value))
                })?;
                if let Some(min) = self.minimum {
                    if n < min {
                        return Err(Error::validation(format!(
                            "{}: minimum {} not satisfied",
                            path, min
                        )));
                    }
                }
                if let Some(max) = self.maximum {
                    if n > max {
                        return Err(Error::validation(format!(
                            "{}: maximum {} exceeded",
                            path, max
                        )));
                    }
                }
                Ok(())
            }
            Some("boolean") => {
                if !value.is_boolean() {
                    return Err(Error::validation(format!(
                        "{}: expected boolean, got {}",
                        path, value
                    )));
                }
                Ok(())
            }
            Some("array") => {
                let arr = value.as_array().ok_or_else(|| {
                    Error::validation(format!("{}: expected array, got {}", path, value))
                })?;
                if let Some(items_schema) = &self.items {
                    for (idx, item) in arr.iter().enumerate() {
                        items_schema.validate_value(item, &format!("{}[{}]", path, idx))?;
                    }
                }
                Ok(())
            }
            Some("object") => {
                let obj = value.as_object().ok_or_else(|| {
                    Error::validation(format!("{}: expected object, got {}", path, value))
                })?;
                // required
                for req in &self.required {
                    if !obj.contains_key(req) {
                        return Err(Error::validation(format!(
                            "{}: missing required property '{}'",
                            path, req
                        )));
                    }
                }
                // validate known properties
                for (name, schema) in &self.properties {
                    if let Some(val) = obj.get(name) {
                        schema.validate_value(val, &format!("{}/{}", path, name))?;
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

/// OpenAPI response information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiResponse {
    /// Response description
    pub description: String,
    /// Response schema
    pub schema: Option<OpenApiSchema>,
}

impl OpenApiResponse {
    /// Create from OpenAPI response
    pub fn from_response(response_ref: &ReferenceOr<Response>, spec: &OpenApiSpec) -> Option<Self> {
        match response_ref {
            ReferenceOr::Item(response) => {
                let schema = response
                    .content
                    .get("application/json")
                    .or_else(|| response.content.get("*/*"))
                    .and_then(|media| media.schema.as_ref())
                    .and_then(|s| OpenApiSchema::from_schema(s, spec));

                Some(Self {
                    description: response.description.clone(),
                    schema,
                })
            }
            ReferenceOr::Reference { reference } => spec.get_response(reference).and_then(|r| {
                let schema = r
                    .content
                    .get("application/json")
                    .or_else(|| r.content.get("*/*"))
                    .and_then(|media| media.schema.as_ref())
                    .and_then(|s| OpenApiSchema::from_schema(s, spec));
                Some(Self { description: r.description.clone(), schema })
            }),
        }
    }
}

/// OpenAPI security requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSecurityRequirement {
    /// Security scheme name
    pub scheme: String,
    /// Required scopes
    pub scopes: Vec<String>,
}

impl OpenApiSecurityRequirement {
    /// Create from OpenAPI security requirement
    pub fn from_security_requirement(sec: &openapiv3::SecurityRequirement) -> Self {
        // For simplicity, take the first security scheme
        if let Some((scheme, scopes)) = sec.iter().next() {
            Self {
                scheme: scheme.clone(),
                scopes: scopes.clone(),
            }
        } else {
            Self {
                scheme: "".to_string(),
                scopes: vec![],
            }
        }
    }
}

/// Route information extracted from OpenAPI spec
#[derive(Debug, Clone)]
pub struct OpenApiRoute {
    /// HTTP method
    pub method: String,
    /// Path template
    pub path: String,
    /// Operation details
    pub operation: OpenApiOperation,
}

impl OpenApiRoute {
    /// Create from OpenAPI operation
    pub fn from_operation(method: String, path: String, operation: &Operation, spec: &OpenApiSpec) -> Self {
        let operation_data =
            OpenApiOperation::from_operation(method.clone(), path.clone(), operation, spec);
        Self {
            method,
            path,
            operation: operation_data,
        }
    }

    /// Convert OpenAPI path to Axum-compatible path
    pub fn axum_path(&self) -> String {
        // Convert OpenAPI path parameters {param} to Axum :param
        self.path.replace("{", ":").replace("}", "")
    }

    /// Get mock response for this route
    pub fn mock_response(&self) -> Value {
        // Try to get the 200 response first, then any success response
        if let Some(response) = self
            .operation
            .responses
            .get("200")
            .or_else(|| self.operation.responses.get("201"))
            .or_else(|| self.operation.responses.get("default"))
        {
            if let Some(schema) = &response.schema {
                schema.generate_mock_value()
            } else {
                Value::Object(serde_json::Map::new())
            }
        } else {
            Value::Object(serde_json::Map::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_openapi_spec_from_json() {
        let spec_json = r#"
        {
            "openapi": "3.0.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/users": {
                    "get": {
                        "summary": "Get users",
                        "responses": {
                            "200": {
                                "description": "Success",
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "array",
                                            "items": {
                                                "type": "object",
                                                "properties": {
                                                    "id": {"type": "integer"},
                                                    "name": {"type": "string"}
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        "#;

        let value: Value = serde_json::from_str(spec_json).unwrap();
        let spec = OpenApiSpec::from_json(value).unwrap();

        assert_eq!(spec.title(), "Test API");
        assert_eq!(spec.version(), "1.0.0");

        let operations = spec.operations_for_path("/users");
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].0, "GET");
    }

    #[test]
    fn test_schema_generation() {
        let schema = OpenApiSchema {
            schema_type: Some("object".to_string()),
            format: None,
            description: None,
            properties: vec![
                (
                    "id".to_string(),
                    Box::new(OpenApiSchema {
                        schema_type: Some("integer".to_string()),
                        format: None,
                        description: None,
                        properties: HashMap::new(),
                        required: Vec::new(),
                        items: None,
                        enum_values: None,
                        minimum: None,
                        maximum: None,
                        min_length: None,
                        max_length: None,
                    }),
                ),
                (
                    "name".to_string(),
                    Box::new(OpenApiSchema {
                        schema_type: Some("string".to_string()),
                        format: None,
                        description: None,
                        properties: HashMap::new(),
                        required: Vec::new(),
                        items: None,
                        enum_values: None,
                        minimum: None,
                        maximum: None,
                        min_length: None,
                        max_length: None,
                    }),
                ),
            ]
            .into_iter()
            .collect(),
            required: Vec::new(),
            items: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
        };

        let mock_value = schema.generate_mock_value();
        assert!(mock_value.is_object());

        let obj = mock_value.as_object().unwrap();
        assert!(obj.contains_key("id"));
        assert!(obj.contains_key("name"));
    }
}
