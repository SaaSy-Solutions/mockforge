//! OpenAPI specification loading and parsing
//!
//! This module handles loading OpenAPI specifications from files,
//! parsing them, and providing basic operations on the specs.

use crate::{Error, Result};
use openapiv3::{OpenAPI, ReferenceOr, Schema};
use std::collections::HashSet;
use std::path::Path;
use tokio::fs;

/// OpenAPI specification loader and parser
#[derive(Debug, Clone)]
pub struct OpenApiSpec {
    /// The parsed OpenAPI specification
    pub spec: OpenAPI,
    /// Path to the original spec file
    pub file_path: Option<String>,
    /// Raw OpenAPI document preserved as JSON for resolving unsupported constructs
    pub raw_document: Option<serde_json::Value>,
}

impl OpenApiSpec {
    /// Load OpenAPI spec from a file path
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        let content = fs::read_to_string(path_ref)
            .await
            .map_err(|e| Error::generic(format!("Failed to read OpenAPI spec file: {}", e)))?;

        let (raw_document, spec) = if path_ref.extension().and_then(|s| s.to_str()) == Some("yaml")
            || path_ref.extension().and_then(|s| s.to_str()) == Some("yml")
        {
            let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)
                .map_err(|e| Error::generic(format!("Failed to parse YAML OpenAPI spec: {}", e)))?;
            let raw = serde_json::to_value(&yaml_value).map_err(|e| {
                Error::generic(format!("Failed to convert YAML OpenAPI spec to JSON: {}", e))
            })?;
            let spec = serde_json::from_value(raw.clone())
                .map_err(|e| Error::generic(format!("Failed to read OpenAPI spec: {}", e)))?;
            (raw, spec)
        } else {
            let raw: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| Error::generic(format!("Failed to parse JSON OpenAPI spec: {}", e)))?;
            let spec = serde_json::from_value(raw.clone())
                .map_err(|e| Error::generic(format!("Failed to read OpenAPI spec: {}", e)))?;
            (raw, spec)
        };

        Ok(Self {
            spec,
            file_path: path_ref.to_str().map(|s| s.to_string()),
            raw_document: Some(raw_document),
        })
    }

    /// Load OpenAPI spec from string content
    pub fn from_string(content: &str, format: Option<&str>) -> Result<Self> {
        let (raw_document, spec) = if format == Some("yaml") || format == Some("yml") {
            let yaml_value: serde_yaml::Value = serde_yaml::from_str(content)
                .map_err(|e| Error::generic(format!("Failed to parse YAML OpenAPI spec: {}", e)))?;
            let raw = serde_json::to_value(&yaml_value).map_err(|e| {
                Error::generic(format!("Failed to convert YAML OpenAPI spec to JSON: {}", e))
            })?;
            let spec = serde_json::from_value(raw.clone())
                .map_err(|e| Error::generic(format!("Failed to read OpenAPI spec: {}", e)))?;
            (raw, spec)
        } else {
            let raw: serde_json::Value = serde_json::from_str(content)
                .map_err(|e| Error::generic(format!("Failed to parse JSON OpenAPI spec: {}", e)))?;
            let spec = serde_json::from_value(raw.clone())
                .map_err(|e| Error::generic(format!("Failed to read OpenAPI spec: {}", e)))?;
            (raw, spec)
        };

        Ok(Self {
            spec,
            file_path: None,
            raw_document: Some(raw_document),
        })
    }

    /// Load OpenAPI spec from JSON value
    pub fn from_json(json: serde_json::Value) -> Result<Self> {
        // Deserialize the spec - this consumes the JSON value
        // We need to clone before deserialization to keep raw_document, but we optimize
        // by only cloning if deserialization succeeds (early return on error avoids clone)
        let json_for_doc = json.clone();
        let spec: OpenAPI = serde_json::from_value(json)
            .map_err(|e| Error::generic(format!("Failed to parse JSON OpenAPI spec: {}", e)))?;

        Ok(Self {
            spec,
            file_path: None,
            raw_document: Some(json_for_doc),
        })
    }

    /// Validate the OpenAPI specification
    ///
    /// This method provides basic validation. For comprehensive validation
    /// with detailed error messages, use `spec_parser::OpenApiValidator::validate()`.
    pub fn validate(&self) -> Result<()> {
        // Basic validation - check that we have at least one path
        if self.spec.paths.paths.is_empty() {
            return Err(Error::generic("OpenAPI spec must contain at least one path"));
        }

        // Check that info section has required fields
        if self.spec.info.title.is_empty() {
            return Err(Error::generic("OpenAPI spec info must have a title"));
        }

        if self.spec.info.version.is_empty() {
            return Err(Error::generic("OpenAPI spec info must have a version"));
        }

        Ok(())
    }

    /// Enhanced validation with detailed error reporting
    pub fn validate_enhanced(&self) -> crate::spec_parser::ValidationResult {
        // Convert to JSON value for enhanced validator
        if let Some(raw) = &self.raw_document {
            let format = if raw.get("swagger").is_some() {
                crate::spec_parser::SpecFormat::OpenApi20
            } else if let Some(version) = raw.get("openapi").and_then(|v| v.as_str()) {
                if version.starts_with("3.1") {
                    crate::spec_parser::SpecFormat::OpenApi31
                } else {
                    crate::spec_parser::SpecFormat::OpenApi30
                }
            } else {
                // Default to 3.0 if we can't determine
                crate::spec_parser::SpecFormat::OpenApi30
            };
            crate::spec_parser::OpenApiValidator::validate(raw, format)
        } else {
            // Fallback to basic validation if no raw document
            crate::spec_parser::ValidationResult::failure(vec![
                crate::spec_parser::ValidationError::new(
                    "Cannot perform enhanced validation without raw document".to_string(),
                ),
            ])
        }
    }

    /// Get the OpenAPI version
    pub fn version(&self) -> &str {
        &self.spec.openapi
    }

    /// Get the API title
    pub fn title(&self) -> &str {
        &self.spec.info.title
    }

    /// Get the API description
    pub fn description(&self) -> Option<&str> {
        self.spec.info.description.as_deref()
    }

    /// Get the API version
    pub fn api_version(&self) -> &str {
        &self.spec.info.version
    }

    /// Get the server URLs
    pub fn servers(&self) -> &[openapiv3::Server] {
        &self.spec.servers
    }

    /// Get all paths defined in the spec
    pub fn paths(&self) -> &openapiv3::Paths {
        &self.spec.paths
    }

    /// Get all schemas defined in the spec
    pub fn schemas(
        &self,
    ) -> Option<&indexmap::IndexMap<String, openapiv3::ReferenceOr<openapiv3::Schema>>> {
        self.spec.components.as_ref().map(|c| &c.schemas)
    }

    /// Get all security schemes defined in the spec
    pub fn security_schemes(
        &self,
    ) -> Option<&indexmap::IndexMap<String, openapiv3::ReferenceOr<openapiv3::SecurityScheme>>>
    {
        self.spec.components.as_ref().map(|c| &c.security_schemes)
    }

    /// Get all operations for a given path
    pub fn operations_for_path(
        &self,
        path: &str,
    ) -> std::collections::HashMap<String, openapiv3::Operation> {
        let mut operations = std::collections::HashMap::new();

        if let Some(path_item_ref) = self.spec.paths.paths.get(path) {
            // Handle the ReferenceOr<PathItem> case
            if let Some(path_item) = path_item_ref.as_item() {
                if let Some(op) = &path_item.get {
                    operations.insert("GET".to_string(), op.clone());
                }
                if let Some(op) = &path_item.post {
                    operations.insert("POST".to_string(), op.clone());
                }
                if let Some(op) = &path_item.put {
                    operations.insert("PUT".to_string(), op.clone());
                }
                if let Some(op) = &path_item.delete {
                    operations.insert("DELETE".to_string(), op.clone());
                }
                if let Some(op) = &path_item.patch {
                    operations.insert("PATCH".to_string(), op.clone());
                }
                if let Some(op) = &path_item.head {
                    operations.insert("HEAD".to_string(), op.clone());
                }
                if let Some(op) = &path_item.options {
                    operations.insert("OPTIONS".to_string(), op.clone());
                }
                if let Some(op) = &path_item.trace {
                    operations.insert("TRACE".to_string(), op.clone());
                }
            }
        }

        operations
    }

    /// Get all paths with their operations
    pub fn all_paths_and_operations(
        &self,
    ) -> std::collections::HashMap<String, std::collections::HashMap<String, openapiv3::Operation>>
    {
        self.spec
            .paths
            .paths
            .iter()
            .map(|(path, _)| (path.clone(), self.operations_for_path(path)))
            .collect()
    }

    /// Get a schema by reference
    pub fn get_schema(&self, reference: &str) -> Option<crate::openapi::schema::OpenApiSchema> {
        self.resolve_schema(reference).map(crate::openapi::schema::OpenApiSchema::new)
    }

    /// Validate security requirements
    pub fn validate_security_requirements(
        &self,
        security_requirements: &[openapiv3::SecurityRequirement],
        auth_header: Option<&str>,
        api_key: Option<&str>,
    ) -> Result<()> {
        if security_requirements.is_empty() {
            return Ok(());
        }

        // Security requirements are OR'd - if any requirement is satisfied, pass
        for requirement in security_requirements {
            if self.is_security_requirement_satisfied(requirement, auth_header, api_key)? {
                return Ok(());
            }
        }

        Err(Error::generic("Security validation failed: no valid authentication provided"))
    }

    fn resolve_schema(&self, reference: &str) -> Option<Schema> {
        let mut visited = HashSet::new();
        self.resolve_schema_recursive(reference, &mut visited)
    }

    fn resolve_schema_recursive(
        &self,
        reference: &str,
        visited: &mut HashSet<String>,
    ) -> Option<Schema> {
        if !visited.insert(reference.to_string()) {
            tracing::warn!("Detected recursive schema reference: {}", reference);
            return None;
        }

        let schema_name = reference.strip_prefix("#/components/schemas/")?;
        let components = self.spec.components.as_ref()?;
        let schema_ref = components.schemas.get(schema_name)?;

        match schema_ref {
            ReferenceOr::Item(schema) => Some(schema.clone()),
            ReferenceOr::Reference { reference: nested } => {
                self.resolve_schema_recursive(nested, visited)
            }
        }
    }

    /// Check if a single security requirement is satisfied
    fn is_security_requirement_satisfied(
        &self,
        requirement: &openapiv3::SecurityRequirement,
        auth_header: Option<&str>,
        api_key: Option<&str>,
    ) -> Result<bool> {
        // All schemes in the requirement must be satisfied (AND)
        for (scheme_name, _scopes) in requirement {
            if !self.is_security_scheme_satisfied(scheme_name, auth_header, api_key)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Check if a security scheme is satisfied
    fn is_security_scheme_satisfied(
        &self,
        scheme_name: &str,
        auth_header: Option<&str>,
        api_key: Option<&str>,
    ) -> Result<bool> {
        let security_schemes = match self.security_schemes() {
            Some(schemes) => schemes,
            None => return Ok(false),
        };

        let scheme = match security_schemes.get(scheme_name) {
            Some(scheme) => scheme,
            None => {
                return Err(Error::generic(format!("Security scheme '{}' not found", scheme_name)))
            }
        };

        let scheme = match scheme {
            openapiv3::ReferenceOr::Item(s) => s,
            openapiv3::ReferenceOr::Reference { .. } => {
                return Err(Error::generic("Referenced security schemes not supported"))
            }
        };

        match scheme {
            openapiv3::SecurityScheme::HTTP { scheme, .. } => {
                match scheme.as_str() {
                    "bearer" => match auth_header {
                        Some(header) if header.starts_with("Bearer ") => Ok(true),
                        _ => Ok(false),
                    },
                    "basic" => match auth_header {
                        Some(header) if header.starts_with("Basic ") => Ok(true),
                        _ => Ok(false),
                    },
                    _ => Ok(false), // Unsupported scheme
                }
            }
            openapiv3::SecurityScheme::APIKey { location, .. } => {
                match location {
                    openapiv3::APIKeyLocation::Header => Ok(auth_header.is_some()),
                    openapiv3::APIKeyLocation::Query => Ok(api_key.is_some()),
                    _ => Ok(false), // Cookie not supported
                }
            }
            openapiv3::SecurityScheme::OpenIDConnect { .. } => Ok(false), // Not implemented
            openapiv3::SecurityScheme::OAuth2 { .. } => {
                // For OAuth2, check if Bearer token is provided
                match auth_header {
                    Some(header) if header.starts_with("Bearer ") => Ok(true),
                    _ => Ok(false),
                }
            }
        }
    }

    /// Get global security requirements
    pub fn get_global_security_requirements(&self) -> Vec<openapiv3::SecurityRequirement> {
        self.spec.security.clone().unwrap_or_default()
    }

    /// Resolve a request body reference
    pub fn get_request_body(&self, reference: &str) -> Option<&openapiv3::RequestBody> {
        if let Some(components) = &self.spec.components {
            if let Some(param_name) = reference.strip_prefix("#/components/requestBodies/") {
                if let Some(request_body_ref) = components.request_bodies.get(param_name) {
                    return request_body_ref.as_item();
                }
            }
        }
        None
    }

    /// Resolve a response reference
    pub fn get_response(&self, reference: &str) -> Option<&openapiv3::Response> {
        if let Some(components) = &self.spec.components {
            if let Some(response_name) = reference.strip_prefix("#/components/responses/") {
                if let Some(response_ref) = components.responses.get(response_name) {
                    return response_ref.as_item();
                }
            }
        }
        None
    }

    /// Resolve an example reference
    pub fn get_example(&self, reference: &str) -> Option<&openapiv3::Example> {
        if let Some(components) = &self.spec.components {
            if let Some(example_name) = reference.strip_prefix("#/components/examples/") {
                if let Some(example_ref) = components.examples.get(example_name) {
                    return example_ref.as_item();
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openapiv3::{SchemaKind, Type};

    #[test]
    fn resolves_nested_schema_references() {
        let yaml = r#"
openapi: 3.0.3
info:
  title: Test API
  version: "1.0.0"
paths: {}
components:
  schemas:
    Apiary:
      type: object
      properties:
        id:
          type: string
        hive:
          $ref: '#/components/schemas/Hive'
    Hive:
      type: object
      properties:
        name:
          type: string
    HiveWrapper:
      $ref: '#/components/schemas/Hive'
        "#;

        let spec = OpenApiSpec::from_string(yaml, Some("yaml")).expect("spec parses");

        let apiary = spec.get_schema("#/components/schemas/Apiary").expect("resolve apiary schema");
        assert!(matches!(apiary.schema.schema_kind, SchemaKind::Type(Type::Object(_))));

        let wrapper = spec
            .get_schema("#/components/schemas/HiveWrapper")
            .expect("resolve wrapper schema");
        assert!(matches!(wrapper.schema.schema_kind, SchemaKind::Type(Type::Object(_))));
    }
}
