//! OpenAPI specification handling for MockForge
//!
//! This module provides comprehensive OpenAPI 3.0 specification support including:
//! - Loading and parsing OpenAPI specs from files (JSON/YAML)
//! - Route generation from OpenAPI paths
//! - Schema validation against OpenAPI definitions
//! - Mock response generation based on OpenAPI schemas

use crate::{Error, Result};
use chrono::{DateTime, NaiveDate};
use once_cell::sync::Lazy;
use openapiv3::{
    OpenAPI, Operation, Parameter, ParameterSchemaOrContent, ReferenceOr, Response, Schema,
};
use openapiv3::{SchemaKind, StringFormat, Type, VariantOrUnknownOrEmpty};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::Path;
use tokio::fs;
use url::Url;
use uuid::Uuid;

// Simple email regex for practical validation (not full RFC 5322)
// Accepts forms like local@domain.tld and rejects obvious invalids
static EMAIL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[^@\s]+@[^@\s]+\.[^@\s]+$").expect("valid email regex"));

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
        let entry = self.spec.components.as_ref()?.schemas.get(name)?;
        match entry {
            ReferenceOr::Item(schema) => Some(schema),
            ReferenceOr::Reference { reference } => self.get_schema(reference),
        }
    }

    /// Get a parameter by reference
    pub fn get_parameter(&self, reference: &str) -> Option<&Parameter> {
        let name = reference.trim_start_matches("#/components/parameters/");
        let entry = self.spec.components.as_ref()?.parameters.get(name)?;
        match entry {
            ReferenceOr::Item(param) => Some(param),
            ReferenceOr::Reference { reference } => self.get_parameter(reference),
        }
    }

    /// Get a request body by reference
    pub fn get_request_body(&self, reference: &str) -> Option<&openapiv3::RequestBody> {
        let name = reference.trim_start_matches("#/components/requestBodies/");
        let entry = self.spec.components.as_ref()?.request_bodies.get(name)?;
        match entry {
            ReferenceOr::Item(rb) => Some(rb),
            ReferenceOr::Reference { reference } => self.get_request_body(reference),
        }
    }

    /// Get a response by reference
    pub fn get_response(&self, reference: &str) -> Option<&Response> {
        let name = reference.trim_start_matches("#/components/responses/");
        let entry = self.spec.components.as_ref()?.responses.get(name)?;
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

    /// Get a security scheme by name
    pub fn get_security_scheme(&self, name: &str) -> Option<&openapiv3::SecurityScheme> {
        self.spec.components.as_ref()?.security_schemes.get(name)
            .and_then(|scheme| match scheme {
                ReferenceOr::Item(s) => Some(s),
                ReferenceOr::Reference { .. } => None, // TODO: Handle references
            })
    }

    /// Validate security requirements for an operation
    pub fn validate_security_requirements(
        &self,
        security_requirements: &[OpenApiSecurityRequirement],
        auth_header: Option<&str>,
        api_key: Option<&str>,
    ) -> Result<()> {
        if security_requirements.is_empty() {
            // No security requirements, so validation passes
            return Ok(());
        }

        for requirement in security_requirements {
            self.validate_single_security_requirement(requirement, auth_header, api_key)?;
        }

        Ok(())
    }

    /// Validate a single security requirement
    fn validate_single_security_requirement(
        &self,
        requirement: &OpenApiSecurityRequirement,
        auth_header: Option<&str>,
        api_key: Option<&str>,
    ) -> Result<()> {
        let scheme = self.get_security_scheme(&requirement.scheme)
            .ok_or_else(|| Error::generic(format!("Security scheme '{}' not found", requirement.scheme)))?;

        match scheme {
            openapiv3::SecurityScheme::APIKey { location, name, .. } => {
                match location {
                    openapiv3::APIKeyLocation::Header => {
                        if let Some(header_value) = auth_header {
                            // For API key in header, just check if header is present
                            // In a real implementation, you might want to validate the key format
                            if header_value.is_empty() {
                                return Err(Error::generic(format!(
                                    "API key header '{}' is required but empty", name
                                )));
                            }
                        } else {
                            return Err(Error::generic(format!(
                                "API key header '{}' is required", name
                            )));
                        }
                    }
                    openapiv3::APIKeyLocation::Query => {
                        // Query parameter validation would require request parsing
                        // For now, we'll assume it's present if we have an API key
                        if api_key.is_none() || api_key.unwrap().is_empty() {
                            return Err(Error::generic(format!(
                                "API key query parameter '{}' is required", name
                            )));
                        }
                    }
                    openapiv3::APIKeyLocation::Cookie => {
                        // Cookie validation would require cookie parsing
                        return Err(Error::generic("Cookie-based API key validation not implemented"));
                    }
                }
            }
            openapiv3::SecurityScheme::HTTP { scheme, .. } => {
                match scheme.to_lowercase().as_str() {
                    "bearer" | "basic" => {
                        if let Some(header_value) = auth_header {
                            if header_value.is_empty() {
                                return Err(Error::generic(format!(
                                    "HTTP {} authentication header is required but empty", scheme
                                )));
                            }
                            // For Bearer tokens, check if it starts with "Bearer "
                            if scheme == "bearer" && !header_value.to_lowercase().starts_with("bearer ") {
                                return Err(Error::generic(
                                    "Bearer token must start with 'Bearer '"
                                ));
                            }
                        } else {
                            return Err(Error::generic(format!(
                                "HTTP {} authentication header is required", scheme
                            )));
                        }
                    }
                    _ => {
                        return Err(Error::generic(format!(
                            "HTTP authentication scheme '{}' not supported", scheme
                        )));
                    }
                }
            }
            openapiv3::SecurityScheme::OAuth2 { .. } => {
                // OAuth2 validation would require token introspection
                // For now, just check if we have some form of authentication
                if auth_header.is_none() && api_key.is_none() {
                    return Err(Error::generic("OAuth2 authentication is required"));
                }
            }
            openapiv3::SecurityScheme::OpenIDConnect { .. } => {
                // OpenID Connect validation would require token validation
                // For now, just check if we have some form of authentication
                if auth_header.is_none() && api_key.is_none() {
                    return Err(Error::generic("OpenID Connect authentication is required"));
                }
            }
        }

        Ok(())
    }

    /// Extract security requirements from global spec security
    pub fn get_global_security_requirements(&self) -> Vec<OpenApiSecurityRequirement> {
        self.spec.security.iter()
            .flatten()
            .map(|req| OpenApiSecurityRequirement::from_security_requirement(req))
            .collect()
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
    pub fn from_operation(
        method: String,
        path: String,
        operation: &Operation,
        spec: &OpenApiSpec,
    ) -> Self {
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
            responses: {
                let mut responses: HashMap<String, OpenApiResponse> = operation
                    .responses
                    .responses
                    .iter()
                    .filter_map(|(code, resp)| {
                        OpenApiResponse::from_response(resp, spec).map(|r| (code.to_string(), r))
                    })
                    .collect();

                // Add the default response if it exists
                if let Some(default_resp) = &operation.responses.default {
                    if let Some(openapi_resp) = OpenApiResponse::from_response(default_resp, spec) {
                        responses.insert("default".to_string(), openapi_resp);
                    }
                }

                responses
            },
            security: operation
                .security
                .iter()
                .flatten()
                .map(|req| OpenApiSecurityRequirement::from_security_requirement(req))
                .collect(),
        }
    }
}

fn query_style_to_str(s: &openapiv3::QueryStyle) -> String {
    use openapiv3::QueryStyle::*;
    match s {
        Form => "form",
        SpaceDelimited => "spaceDelimited",
        PipeDelimited => "pipeDelimited",
        DeepObject => "deepObject",
    }
    .to_string()
}
fn path_style_to_str(s: &openapiv3::PathStyle) -> String {
    use openapiv3::PathStyle::*;
    match s {
        Simple => "simple",
        Label => "label",
        Matrix => "matrix",
    }
    .to_string()
}
fn header_style_to_str(_: &openapiv3::HeaderStyle) -> String {
    "simple".to_string()
}
fn cookie_style_to_str(_: &openapiv3::CookieStyle) -> String {
    "form".to_string()
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
    /// Parameter style hint (query/header/path)
    pub style: Option<String>,
    /// Explode behavior
    pub explode: Option<bool>,
}

impl OpenApiParameter {
    /// Create from OpenAPI parameter
    pub fn from_parameter(param_ref: &ReferenceOr<Parameter>, spec: &OpenApiSpec) -> Option<Self> {
        match param_ref {
            ReferenceOr::Item(param) => {
                let (param_data, location, style, explode) = match param {
                    Parameter::Query {
                        parameter_data,
                        style,
                        allow_reserved: _,
                        allow_empty_value: _,
                    } => (
                        parameter_data,
                        "query",
                        Some(query_style_to_str(style)),
                        Some(parameter_data.explode),
                    ),
                    Parameter::Path {
                        parameter_data,
                        style,
                    } => (
                        parameter_data,
                        "path",
                        Some(path_style_to_str(style)),
                        Some(parameter_data.explode),
                    ),
                    Parameter::Header {
                        parameter_data,
                        style,
                    } => (
                        parameter_data,
                        "header",
                        Some(header_style_to_str(style)),
                        Some(parameter_data.explode),
                    ),
                    Parameter::Cookie {
                        parameter_data,
                        style,
                    } => (
                        parameter_data,
                        "cookie",
                        Some(cookie_style_to_str(style)),
                        Some(parameter_data.explode),
                    ),
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
                    style,
                    explode: explode.flatten(),
                })
            }
            ReferenceOr::Reference { reference } => spec.get_parameter(reference).and_then(|p| {
                OpenApiParameter::from_parameter(&ReferenceOr::Item(p.clone()), spec)
            }),
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
    /// Regex pattern (for strings)
    pub pattern: Option<String>,
    /// Example value (if provided)
    pub example: Option<Value>,
    /// Default value (if provided)
    pub default_value: Option<Value>,
    /// Minimum length
    pub min_length: Option<usize>,
    /// Maximum length
    pub max_length: Option<usize>,
    /// Minimum items (arrays)
    pub min_items: Option<usize>,
    /// Maximum items (arrays)
    pub max_items: Option<usize>,
    /// oneOf variants
    pub one_of: Vec<OpenApiSchema>,
    /// anyOf variants
    pub any_of: Vec<OpenApiSchema>,
    /// allOf merged components
    pub all_of: Vec<OpenApiSchema>,
    /// Whether additionalProperties are allowed (if boolean specified)
    pub additional_properties_allowed: Option<bool>,
    /// Schema for additionalProperties values
    pub additional_properties_schema: Option<Box<OpenApiSchema>>,
}

impl OpenApiSchema {
    /// Create from OpenAPI schema
    pub fn from_schema(schema_ref: &ReferenceOr<Schema>, spec: &OpenApiSpec) -> Option<Self> {
        match schema_ref {
            ReferenceOr::Item(schema) => Self::from_schema_data(schema, spec),
            ReferenceOr::Reference { reference } => {
                spec.get_schema(reference).and_then(|s| Self::from_schema_data(s, spec))
            }
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
            pattern: None,
            example: None,
            default_value: None,
            min_length: None,
            max_length: None,
            min_items: None,
            max_items: None,
            one_of: Vec::new(),
            any_of: Vec::new(),
            all_of: Vec::new(),
            additional_properties_allowed: None,
            additional_properties_schema: None,
        };

        match &schema.schema_kind {
            SchemaKind::Type(ty) => match ty {
                Type::String(st) => {
                    out.schema_type = Some("string".to_string());
                    // map format
                    out.format = match &st.format {
                        VariantOrUnknownOrEmpty::Item(f) => Some(
                            match f {
                                StringFormat::Byte => "byte",
                                StringFormat::Binary => "binary",
                                StringFormat::Date => "date",
                                StringFormat::DateTime => "date-time",
                                StringFormat::Password => "password",
                            }
                            .to_string(),
                        ),
                        VariantOrUnknownOrEmpty::Unknown(s) => Some(s.clone()),
                        VariantOrUnknownOrEmpty::Empty => None,
                    };
                    out.min_length = st.min_length;
                    out.max_length = st.max_length;
                    out.pattern = st.pattern.clone();
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
                            .map(|opt| opt.map(Value::from).unwrap_or(Value::Null))
                            .collect();
                        out.enum_values = Some(vals);
                    }
                }
                Type::Boolean(_) => {
                    out.schema_type = Some("boolean".to_string());
                }
                Type::Array(at) => {
                    out.schema_type = Some("array".to_string());
                    out.min_items = at.min_items;
                    out.max_items = at.max_items;
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
                    // additionalProperties
                    if let Some(ap) = &ot.additional_properties {
                        match ap {
                            openapiv3::AdditionalProperties::Any(b) => {
                                out.additional_properties_allowed = Some(*b);
                            }
                            openapiv3::AdditionalProperties::Schema(s) => {
                                if let Some(mapped) = Self::from_schema(s, spec) {
                                    out.additional_properties_schema = Some(Box::new(mapped));
                                }
                            }
                        }
                    }
                }
            },
            SchemaKind::OneOf { one_of } => {
                for s in one_of {
                    if let Some(schema) = Self::from_schema(s, spec) {
                        out.one_of.push(schema);
                    }
                }
            }
            SchemaKind::AnyOf { any_of } => {
                for s in any_of {
                    if let Some(schema) = Self::from_schema(s, spec) {
                        out.any_of.push(schema);
                    }
                }
            }
            SchemaKind::AllOf { all_of } => {
                for s in all_of {
                    if let Some(schema) = Self::from_schema(s, spec) {
                        // store for validation; we also attempt a shallow merge for object props/required
                        if let Some("object") = schema.schema_type.as_deref() {
                            for (k, v) in &schema.properties {
                                out.properties.entry(k.clone()).or_insert_with(|| v.clone());
                            }
                            for r in &schema.required {
                                if !out.required.contains(r) {
                                    out.required.push(r.clone());
                                }
                            }
                        }
                        if out.items.is_none() && schema.items.is_some() {
                            out.items = schema.items.clone();
                        }
                        if out.schema_type.is_none() {
                            out.schema_type = schema.schema_type.clone();
                        }
                        if out.format.is_none() {
                            out.format = schema.format.clone();
                        }
                        // numeric/string constraints: keep existing if present, else take from child
                        if out.minimum.is_none() {
                            out.minimum = schema.minimum;
                        }
                        if out.maximum.is_none() {
                            out.maximum = schema.maximum;
                        }
                        if out.min_length.is_none() {
                            out.min_length = schema.min_length;
                        }
                        if out.max_length.is_none() {
                            out.max_length = schema.max_length;
                        }
                        out.all_of.push(schema);
                    }
                }
            }
            SchemaKind::Any(any) => {
                // Map basic type hints
                if let Some(t) = &any.typ {
                    out.schema_type = Some(t.clone());
                }
                // String constraints
                out.min_length = any.min_length;
                out.max_length = any.max_length;
                if let Some(p) = &any.pattern {
                    out.pattern = Some(p.clone());
                }
                if let Some(fmt) = &any.format {
                    out.format = Some(fmt.clone());
                }
                // Numeric constraints
                out.minimum = any.minimum;
                out.maximum = any.maximum;
                // Enumeration values
                if !any.enumeration.is_empty() {
                    out.enum_values = Some(any.enumeration.clone());
                }
                // Items
                if let Some(items) = &any.items {
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
                // Object properties
                for (name, prop_schema) in &any.properties {
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
                out.required = any.required.clone();
                // Composition
                for s in &any.one_of {
                    if let Some(schema) = Self::from_schema(s, spec) {
                        out.one_of.push(schema);
                    }
                }
                for s in &any.any_of {
                    if let Some(schema) = Self::from_schema(s, spec) {
                        out.any_of.push(schema);
                    }
                }
                for s in &any.all_of {
                    if let Some(schema) = Self::from_schema(s, spec) {
                        out.all_of.push(schema.clone());
                        if let Some("object") = schema.schema_type.as_deref() {
                            for (k, v) in &schema.properties {
                                out.properties.entry(k.clone()).or_insert_with(|| v.clone());
                            }
                            for r in &schema.required {
                                if !out.required.contains(r) {
                                    out.required.push(r.clone());
                                }
                            }
                        }
                    }
                }
            }
            SchemaKind::Not { .. } => { /* ignore */ }
        }

        // Capture example/default if present on schema data
        out.example = schema.schema_data.example.clone();
        out.default_value = schema.schema_data.default.clone();

        Some(out)
    }

    /// Create from request body
    pub fn from_request_body(
        request_body: &ReferenceOr<openapiv3::RequestBody>,
        spec: &OpenApiSpec,
    ) -> Option<Self> {
        match request_body {
            ReferenceOr::Item(rb) => rb
                .content
                .get("application/json")
                .or_else(|| rb.content.get("*/*"))
                .and_then(|media| media.schema.as_ref())
                .and_then(|s| Self::from_schema(s, spec)),
            ReferenceOr::Reference { reference } => {
                spec.get_request_body(reference).and_then(|rb| {
                    rb.content
                        .get("application/json")
                        .or_else(|| rb.content.get("*/*"))
                        .and_then(|media| media.schema.as_ref())
                        .and_then(|s| Self::from_schema(s, spec))
                })
            }
        }
    }

    /// Generate a mock value for this schema
    pub fn generate_mock_value(&self) -> Value {
        if let Some(ex) = &self.example {
            return ex.clone();
        }
        if let Some(def) = &self.default_value {
            return def.clone();
        }
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
        // composition checks first
        if !self.one_of.is_empty() {
            let mut matches = 0usize;
            for s in &self.one_of {
                // Heuristic: for object schemas, require some structural signal to avoid vacuous matches
                if let (Some("object"), Some(obj)) = (s.schema_type.as_deref(), value.as_object()) {
                    if !s.required.is_empty() {
                        if !s.required.iter().all(|k| obj.contains_key(k)) {
                            continue;
                        }
                    } else if !s.properties.is_empty()
                        && !s.properties.keys().any(|k| obj.contains_key(k))
                    {
                        continue;
                    }
                }
                if s.validate_value(value, path).is_ok() {
                    matches += 1;
                }
            }
            if matches != 1 {
                return Err(Error::validation(format!(
                    "{}: oneOf expected exactly one schema to match (got {})",
                    path, matches
                )));
            }
        }
        if !self.any_of.is_empty() {
            let mut matches = 0usize;
            for s in &self.any_of {
                if let (Some("object"), Some(obj)) = (s.schema_type.as_deref(), value.as_object()) {
                    if !s.required.is_empty() {
                        if !s.required.iter().all(|k| obj.contains_key(k)) {
                            continue;
                        }
                    } else if !s.properties.is_empty()
                        && !s.properties.keys().any(|k| obj.contains_key(k))
                    {
                        continue;
                    }
                }
                if s.validate_value(value, path).is_ok() {
                    matches += 1;
                }
            }
            if matches == 0 {
                return Err(Error::validation(format!(
                    "{}: anyOf expected at least one schema to match",
                    path
                )));
            }
        }
        if !self.all_of.is_empty() {
            for s in &self.all_of {
                s.validate_value(value, path)?;
            }
        }
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
                // regex pattern
                if let Some(pat) = &self.pattern {
                    if let Ok(re) = Regex::new(pat) {
                        if !re.is_match(s) {
                            return Err(Error::validation(format!(
                                "{}: pattern '{}' not matched",
                                path, pat
                            )));
                        }
                    }
                }
                if let Some(fmt) = &self.format {
                    match fmt.as_str() {
                        "ipv4" => {
                            let ip: IpAddr = s.parse().map_err(|_| {
                                Error::validation(format!("{}: invalid ipv4", path))
                            })?;
                            if !ip.is_ipv4() {
                                return Err(Error::validation(format!("{}: invalid ipv4", path)));
                            }
                        }
                        "ipv6" => {
                            let ip: IpAddr = s.parse().map_err(|_| {
                                Error::validation(format!("{}: invalid ipv6", path))
                            })?;
                            if !ip.is_ipv6() {
                                return Err(Error::validation(format!("{}: invalid ipv6", path)));
                            }
                        }
                        "hostname" => {
                            // simple hostname regex (no underscores, labels 1-63, total <=253)
                            if s.len() > 253 {
                                return Err(Error::validation(format!(
                                    "{}: invalid hostname",
                                    path
                                )));
                            }
                            static HOST_RE: Lazy<Regex> = Lazy::new(|| {
                                Regex::new(r"^([A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?)(?:\.[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?)*$").unwrap()
                            });
                            if !HOST_RE.is_match(s) {
                                return Err(Error::validation(format!(
                                    "{}: invalid hostname",
                                    path
                                )));
                            }
                        }
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
                                Error::validation(format!("{}: invalid uri format", path))
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
                                Error::validation(format!(
                                    "{}: invalid date format (YYYY-MM-DD)",
                                    path
                                ))
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
                if let Some(min) = self.min_items {
                    if arr.len() < min {
                        return Err(Error::validation(format!(
                            "{}: minItems {} not satisfied",
                            path, min
                        )));
                    }
                }
                if let Some(max) = self.max_items {
                    if arr.len() > max {
                        return Err(Error::validation(format!(
                            "{}: maxItems {} exceeded",
                            path, max
                        )));
                    }
                }
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
                // additionalProperties handling
                if let Some(allowed) = self.additional_properties_allowed {
                    if !allowed {
                        for key in obj.keys() {
                            if !self.properties.contains_key(key) {
                                return Err(Error::validation(format!(
                                    "{}: additional property '{}' not allowed",
                                    path, key
                                )));
                            }
                        }
                    }
                }
                if let Some(schema) = &self.additional_properties_schema {
                    for (key, val) in obj.iter() {
                        if !self.properties.contains_key(key) {
                            schema.validate_value(val, &format!("{}/{}", path, key))?;
                        }
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Collect validation errors into `errors` rather than returning early.
    pub fn validate_collect(&self, value: &Value, path: &str, errors: &mut Vec<String>) {
        // composition
        if !self.one_of.is_empty() {
            let mut matches = 0usize;
            for s in &self.one_of {
                // avoid vacuous matches for object schemas
                if let (Some("object"), Some(obj)) = (s.schema_type.as_deref(), value.as_object()) {
                    if !s.required.is_empty() && !s.required.iter().all(|k| obj.contains_key(k)) {
                        continue;
                    }
                    if s.required.is_empty()
                        && !s.properties.is_empty()
                        && !s.properties.keys().any(|k| obj.contains_key(k))
                    {
                        continue;
                    }
                }
                let mut sub = Vec::new();
                s.validate_collect(value, path, &mut sub);
                if sub.is_empty() {
                    matches += 1;
                }
            }
            if matches != 1 {
                errors.push(format!(
                    "{}: oneOf expected exactly one schema to match (got {})",
                    path, matches
                ));
            }
        }
        if !self.any_of.is_empty() {
            let mut matches = 0usize;
            for s in &self.any_of {
                if let (Some("object"), Some(obj)) = (s.schema_type.as_deref(), value.as_object()) {
                    if !s.required.is_empty() && !s.required.iter().all(|k| obj.contains_key(k)) {
                        continue;
                    }
                    if s.required.is_empty()
                        && !s.properties.is_empty()
                        && !s.properties.keys().any(|k| obj.contains_key(k))
                    {
                        continue;
                    }
                }
                let mut sub = Vec::new();
                s.validate_collect(value, path, &mut sub);
                if sub.is_empty() {
                    matches += 1;
                }
            }
            if matches == 0 {
                errors.push(format!("{}: anyOf expected at least one schema to match", path));
            }
        }
        if !self.all_of.is_empty() {
            for s in &self.all_of {
                s.validate_collect(value, path, errors);
            }
        }

        match self.schema_type.as_deref() {
            Some("string") => {
                if let Some(sv) = value.as_str() {
                    if let Some(min) = self.min_length {
                        if sv.len() < min {
                            errors.push(format!("{}: minLength {} not satisfied", path, min));
                        }
                    }
                    if let Some(max) = self.max_length {
                        if sv.len() > max {
                            errors.push(format!("{}: maxLength {} exceeded", path, max));
                        }
                    }
                    if self.pattern.is_some() || self.format.is_some() {
                        if let Err(e) = self.validate_value(value, path) {
                            errors.push(format!("{}", e));
                        }
                    }
                } else {
                    errors.push(format!("{}: expected string, got {}", path, value));
                }
            }
            Some("number") | Some("integer") => {
                if let Some(n) = value.as_f64() {
                    if let Some(min) = self.minimum {
                        if n < min {
                            errors.push(format!("{}: minimum {} not satisfied", path, min));
                        }
                    }
                    if let Some(max) = self.maximum {
                        if n > max {
                            errors.push(format!("{}: maximum {} exceeded", path, max));
                        }
                    }
                } else {
                    errors.push(format!("{}: expected number, got {}", path, value));
                }
            }
            Some("boolean") => {
                if !value.is_boolean() {
                    errors.push(format!("{}: expected boolean, got {}", path, value));
                }
            }
            Some("array") => {
                if let Some(arr) = value.as_array() {
                    if let Some(min) = self.min_items {
                        if arr.len() < min {
                            errors.push(format!("{}: minItems {} not satisfied", path, min));
                        }
                    }
                    if let Some(max) = self.max_items {
                        if arr.len() > max {
                            errors.push(format!("{}: maxItems {} exceeded", path, max));
                        }
                    }
                    if let Some(items) = &self.items {
                        for (i, v) in arr.iter().enumerate() {
                            items.validate_collect(v, &format!("{}[{}]", path, i), errors);
                        }
                    }
                } else {
                    errors.push(format!("{}: expected array, got {}", path, value));
                }
            }
            Some("object") => {
                if let Some(obj) = value.as_object() {
                    for req in &self.required {
                        if !obj.contains_key(req) {
                            errors.push(format!("{}: missing required property '{}'", path, req));
                        }
                    }
                    for (k, s) in &self.properties {
                        if let Some(v) = obj.get(k) {
                            s.validate_collect(v, &format!("{}/{}", path, k), errors);
                        }
                    }
                    if let Some(allowed) = self.additional_properties_allowed {
                        if !allowed {
                            for k in obj.keys() {
                                if !self.properties.contains_key(k) {
                                    errors.push(format!(
                                        "{}: additional property '{}' not allowed",
                                        path, k
                                    ));
                                }
                            }
                        }
                    }
                    if let Some(schema) = &self.additional_properties_schema {
                        for (k, v) in obj.iter() {
                            if !self.properties.contains_key(k) {
                                schema.validate_collect(v, &format!("{}/{}", path, k), errors);
                            }
                        }
                    }
                } else {
                    errors.push(format!("{}: expected object, got {}", path, value));
                }
            }
            _ => {}
        }
    }

    pub fn validate_collect_detailed(
        &self,
        value: &Value,
        path: &str,
        details: &mut Vec<serde_json::Value>,
    ) {
        // Use validate_collect to get semantics, then enrich with expectations
        let mut msgs = Vec::new();
        self.validate_collect(value, path, &mut msgs);
        for m in msgs {
            let code = if m.contains("minLength") {
                "minLength"
            } else if m.contains("maxLength") {
                "maxLength"
            } else if m.contains("minItems") {
                "minItems"
            } else if m.contains("maxItems") {
                "maxItems"
            } else if m.contains("pattern") {
                "pattern"
            } else if m.contains("invalid ") {
                "format"
            } else if m.contains("expected string")
                || m.contains("expected number")
                || m.contains("expected object")
                || m.contains("expected array")
                || m.contains("expected boolean")
            {
                "type"
            } else if m.contains("missing required") {
                "required"
            } else if m.contains("additional property") {
                "additionalProperties"
            } else if m.contains("oneOf") {
                "oneOf"
            } else if m.contains("anyOf") {
                "anyOf"
            } else {
                "validation"
            };

            let mut obj = serde_json::Map::new();
            obj.insert("path".into(), serde_json::Value::String(path.to_string()));
            obj.insert("code".into(), serde_json::Value::String(code.to_string()));
            obj.insert("message".into(), serde_json::Value::String(m.clone()));
            obj.insert("value".into(), value.clone());

            // Expected fields based on schema
            if let Some(t) = &self.schema_type {
                obj.insert("expected_type".into(), serde_json::Value::String(t.clone()));
            }
            if let Some(fmt) = &self.format {
                obj.insert("expected_format".into(), serde_json::Value::String(fmt.clone()));
            }
            if let Some(min) = self.minimum {
                obj.insert("expected_min".into(), serde_json::json!(min));
            }
            if let Some(max) = self.maximum {
                obj.insert("expected_max".into(), serde_json::json!(max));
            }
            if let Some(minl) = self.min_length {
                obj.insert("expected_minLength".into(), serde_json::json!(minl));
            }
            if let Some(maxl) = self.max_length {
                obj.insert("expected_maxLength".into(), serde_json::json!(maxl));
            }
            if let Some(p) = &self.pattern {
                obj.insert("expected_pattern".into(), serde_json::Value::String(p.clone()));
            }

            // Enum expectations
            if let Some(ev) = &self.enum_values {
                if !ev.is_empty() {
                    obj.insert("expected_enum".into(), serde_json::Value::Array(ev.clone()));
                }
            }

            // Array item hints
            if let Some("array") = self.schema_type.as_deref() {
                if let Some(items) = &self.items {
                    if let Some(t) = &items.schema_type {
                        obj.insert(
                            "items_expected_type".into(),
                            serde_json::Value::String(t.clone()),
                        );
                    }
                    if let Some(fmt) = &items.format {
                        obj.insert(
                            "items_expected_format".into(),
                            serde_json::Value::String(fmt.clone()),
                        );
                    }
                }
                if let Some(mi) = self.min_items {
                    obj.insert("expected_minItems".into(), serde_json::json!(mi));
                }
                if let Some(ma) = self.max_items {
                    obj.insert("expected_maxItems".into(), serde_json::json!(ma));
                }
            }

            // Object property hints
            if let Some("object") = self.schema_type.as_deref() {
                if !self.properties.is_empty() {
                    obj.insert(
                        "object_properties".into(),
                        serde_json::Value::Array(
                            self.properties
                                .keys()
                                .cloned()
                                .map(serde_json::Value::String)
                                .collect(),
                        ),
                    );
                }
                if !self.required.is_empty() {
                    obj.insert(
                        "required_properties".into(),
                        serde_json::Value::Array(
                            self.required.iter().cloned().map(serde_json::Value::String).collect(),
                        ),
                    );
                }
            }

            details.push(serde_json::Value::Object(obj));
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
    /// Inline example (from media.example or first media.examples entry)
    pub example: Option<serde_json::Value>,
}

impl OpenApiResponse {
    /// Create from OpenAPI response
    pub fn from_response(response_ref: &ReferenceOr<Response>, spec: &OpenApiSpec) -> Option<Self> {
        match response_ref {
            ReferenceOr::Item(response) => {
                let media = response
                    .content
                    .get("application/json")
                    .or_else(|| response.content.get("*/*"));
                let schema = media
                    .and_then(|m| m.schema.as_ref())
                    .and_then(|s| OpenApiSchema::from_schema(s, spec));
                let mut example = media.and_then(|m| m.example.clone());
                if example.is_none() {
                    if let Some(m) = media {
                        if let Some((_, exref)) = m.examples.iter().next() {
                            if let Some(ex) = exref.as_item() {
                                example = ex.value.clone();
                            }
                        }
                    }
                }
                Some(Self {
                    description: response.description.clone(),
                    schema,
                    example,
                })
            }
            ReferenceOr::Reference { reference } => spec.get_response(reference).map(|r| {
                let media = r.content.get("application/json").or_else(|| r.content.get("*/*"));
                let schema = media
                    .and_then(|m| m.schema.as_ref())
                    .and_then(|s| OpenApiSchema::from_schema(s, spec));
                let mut example = media.and_then(|m| m.example.clone());
                if example.is_none() {
                    if let Some(m) = media {
                        if let Some((_, exref)) = m.examples.iter().next() {
                            if let Some(ex) = exref.as_item() {
                                example = ex.value.clone();
                            }
                        }
                    }
                }
                Self {
                    description: r.description.clone(),
                    schema,
                    example,
                }
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
    pub fn from_operation(
        method: String,
        path: String,
        operation: &Operation,
        spec: &OpenApiSpec,
    ) -> Self {
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

    /// Select the best 2xx response from the OpenAPI spec
    /// Returns the response status code and the response object
    pub fn select_best_2xx_response(&self) -> Option<(u16, &OpenApiResponse)> {
        self.operation.select_best_2xx_response()
    }

    /// Get (status, body) for this route using 2xx selection
    pub fn mock_response_with_status(&self) -> (u16, Value) {
        if let Some((code, response)) = self.select_best_2xx_response() {
            if let Some(ex) = &response.example {
                return (code, ex.clone());
            }
            if let Some(schema) = &response.schema {
                return (code, schema.generate_mock_value());
            }
            return (code, Value::Object(serde_json::Map::new()));
        }
        (200, Value::Object(serde_json::Map::new()))
    }

    /// Back-compat: return only the mock body
    pub fn mock_response(&self) -> Value {
        let (_, body) = self.mock_response_with_status();
        body
    }

    /// Back-compat: return just the selected status code
    pub fn get_response_status_code(&self) -> u16 {
        self.select_best_2xx_response().map(|(c, _)| c).unwrap_or(200)
    }
}

impl OpenApiOperation {
    /// Select the best 2xx response from the OpenAPI spec
    /// Returns the response status code and the response object
    pub fn select_best_2xx_response(&self) -> Option<(u16, &OpenApiResponse)> {
        // Define 2xx status codes in priority order
        let priority_codes = [
            "200", "201", "202", "204", "203", "205", "206", "207", "208", "226",
        ];

        // First try explicit 2xx codes in priority order
        for code in &priority_codes {
            if let Some(response) = self.responses.get(*code) {
                return Some((code.parse().unwrap_or(200), response));
            }
        }

        // Then try any other numeric 2xx codes (sorted ascending)
        let mut others: Vec<u16> = self
            .responses
            .keys()
            .filter_map(|k| k.parse::<u16>().ok())
            .filter(|c| {
                (200..=299).contains(c) && !priority_codes.contains(&c.to_string().as_str())
            })
            .collect();
        others.sort_unstable();
        if let Some(code) = others.first() {
            if let Some(resp) = self.responses.get(&code.to_string()) {
                return Some((*code, resp));
            }
        }

        // 2XX wildcard
        if let Some(response) = self.responses.get("2XX").or_else(|| self.responses.get("2xx")) {
            return Some((200, response));
        }

        // Then try "default" response
        if let Some(response) = self.responses.get("default") {
            return Some((200, response)); // Default to 200 for default responses
        }

        None
    }

    // Legacy helpers removed in favor of route.mock_response_with_status
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
                        pattern: None,
                        example: None,
                        default_value: None,
                        min_length: None,
                        max_length: None,
                        min_items: None,
                        max_items: None,
                        one_of: Vec::new(),
                        any_of: Vec::new(),
                        all_of: Vec::new(),
                        additional_properties_allowed: None,
                        additional_properties_schema: None,
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
                        pattern: None,
                        example: None,
                        default_value: None,
                        min_length: None,
                        max_length: None,
                        min_items: None,
                        max_items: None,
                        one_of: Vec::new(),
                        any_of: Vec::new(),
                        all_of: Vec::new(),
                        additional_properties_allowed: None,
                        additional_properties_schema: None,
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
            pattern: None,
            example: None,
            default_value: None,
            min_length: None,
            max_length: None,
            min_items: None,
            max_items: None,
            one_of: Vec::new(),
            any_of: Vec::new(),
            all_of: Vec::new(),
            additional_properties_allowed: None,
            additional_properties_schema: None,
        };

        let mock_value = schema.generate_mock_value();
        assert!(mock_value.is_object());

        let obj = mock_value.as_object().unwrap();
        assert!(obj.contains_key("id"));
        assert!(obj.contains_key("name"));
    }

    #[test]
    fn test_response_selection_priority() {
        // Test comprehensive 2xx response selection with priority ordering
        let spec_json = json!({
            "openapi": "3.0.0",
            "info": {"title": "Response Test", "version": "1.0.0"},
            "paths": {
                "/test": {
                    "get": {
                        "responses": {
                            // Test all common 2xx codes in non-priority order
                            "202": {"description": "Accepted"},
                            "200": {"description": "OK"},
                            "201": {"description": "Created"},
                            "204": {"description": "No Content"},
                            "203": {"description": "Non-Authoritative Information"}
                        }
                    }
                }
            }
        });

        let spec = OpenApiSpec::from_json(spec_json).unwrap();
        let operations = spec.operations_for_path("/test");
        let operation = OpenApiOperation::from_operation(
            operations[0].0.clone(),
            "/test".to_string(),
            operations[0].1,
            &spec,
        );

        // Test response selection priority (200 should be selected first)
        let (status_code, response) = operation.select_best_2xx_response().unwrap();
        assert_eq!(status_code, 200);
        assert_eq!(response.description, "OK");

        // Test status code getter
        let route = OpenApiRoute {
            method: "GET".to_string(),
            path: "/test".to_string(),
            operation: operation.clone(),
        };
        assert_eq!(route.get_response_status_code(), 200);
    }

    #[test]
    fn test_response_selection_fallback_to_default() {
        // Test fallback to "default" response when no 2xx codes exist
        let spec_json = json!({
            "openapi": "3.0.0",
            "info": {"title": "Default Test", "version": "1.0.0"},
            "paths": {
                "/test": {
                    "get": {
                        "responses": {
                            "400": {"description": "Bad Request"},
                            "500": {"description": "Server Error"},
                            "default": {"description": "Default Response"}
                        }
                    }
                }
            }
        });

        let spec = OpenApiSpec::from_json(spec_json).unwrap();
        let operations = spec.operations_for_path("/test");
        let operation = OpenApiOperation::from_operation(
            operations[0].0.clone(),
            "/test".to_string(),
            operations[0].1,
            &spec,
        );

        // Should fallback to default and use 200 as status code
        let (status_code, response) = operation.select_best_2xx_response().unwrap();
        assert_eq!(status_code, 200);
        assert_eq!(response.description, "Default Response");
    }

    #[test]
    fn test_response_selection_all_2xx_codes() {
        // Test that all 2xx codes are supported
        let spec_json = json!({
            "openapi": "3.0.0",
            "info": {"title": "2xx Test", "version": "1.0.0"},
            "paths": {
                "/test": {
                    "get": {
                        "responses": {
                            "226": {"description": "IM Used"},           // Should be selected (lowest priority)
                            "208": {"description": "Already Reported"},
                            "207": {"description": "Multi-Status"},
                            "206": {"description": "Partial Content"},
                            "205": {"description": "Reset Content"},     // Should be selected over 226
                            "204": {"description": "No Content"},        // Should be selected over 205
                            "203": {"description": "Non-Authoritative"}, // Should be selected over 204
                            "202": {"description": "Accepted"},          // Should be selected over 203
                            "201": {"description": "Created"},           // Should be selected over 202
                            "200": {"description": "OK"}                 // Should be selected first
                        }
                    }
                }
            }
        });

        let spec = OpenApiSpec::from_json(spec_json).unwrap();
        let operations = spec.operations_for_path("/test");
        let operation = OpenApiOperation::from_operation(
            operations[0].0.clone(),
            "/test".to_string(),
            operations[0].1,
            &spec,
        );

        // Should select 200 (highest priority)
        let (status_code, response) = operation.select_best_2xx_response().unwrap();
        assert_eq!(status_code, 200);
        assert_eq!(response.description, "OK");
    }

    #[test]
    fn test_response_selection_no_responses() {
        // Test behavior when no responses are defined
        let spec_json = json!({
            "openapi": "3.0.0",
            "info": {"title": "No Response Test", "version": "1.0.0"},
            "paths": {
                "/test": {
                    "get": {
                        "responses": {}
                    }
                }
            }
        });

        let spec = OpenApiSpec::from_json(spec_json).unwrap();
        let operations = spec.operations_for_path("/test");
        let operation = OpenApiOperation::from_operation(
            operations[0].0.clone(),
            "/test".to_string(),
            operations[0].1,
            &spec,
        );

        // Should return None when no responses exist
        assert!(operation.select_best_2xx_response().is_none());

        // Status code getter should default to 200
        let route = OpenApiRoute {
            method: "GET".to_string(),
            path: "/test".to_string(),
            operation: operation.clone(),
        };
        assert_eq!(route.get_response_status_code(), 200);

        // Mock response should return empty object
        let mock_response = route.mock_response();
        assert!(mock_response.is_object());
        assert_eq!(mock_response.as_object().unwrap().len(), 0);
    }
}
