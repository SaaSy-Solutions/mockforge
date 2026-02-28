//! Cross-spec dependency detection and configuration for multi-spec benchmarking
//!
//! This module provides:
//! - Auto-detection of dependencies between specs based on schema references
//! - Manual dependency configuration via YAML/JSON files
//! - Topological sorting for correct execution order
//! - Value extraction and injection between spec groups

use crate::error::{BenchError, Result};
use mockforge_core::openapi::spec::OpenApiSpec;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Cross-spec dependency configuration (optional override)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpecDependencyConfig {
    /// Ordered list of spec groups to execute
    #[serde(default)]
    pub execution_order: Vec<SpecGroup>,
    /// Disable auto-detection of dependencies
    #[serde(default)]
    pub disable_auto_detect: bool,
}

impl SpecDependencyConfig {
    /// Load dependency configuration from a file (YAML or JSON)
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| BenchError::Other(format!("Failed to read dependency config: {}", e)))?;

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match ext {
            "yaml" | "yml" => serde_yaml::from_str(&content).map_err(|e| {
                BenchError::Other(format!("Failed to parse YAML dependency config: {}", e))
            }),
            "json" => serde_json::from_str(&content).map_err(|e| {
                BenchError::Other(format!("Failed to parse JSON dependency config: {}", e))
            }),
            _ => Err(BenchError::Other(format!(
                "Unsupported dependency config format: {}. Use .yaml, .yml, or .json",
                ext
            ))),
        }
    }
}

/// A group of specs to execute together
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecGroup {
    /// Name for this group (e.g., "infrastructure", "services")
    pub name: String,
    /// Spec files in this group
    pub specs: Vec<PathBuf>,
    /// Fields to extract from responses (JSONPath-like syntax)
    #[serde(default)]
    pub extract: HashMap<String, String>,
    /// Fields to inject into next group's requests
    #[serde(default)]
    pub inject: HashMap<String, String>,
}

/// Detected dependency between two specs
#[derive(Debug, Clone)]
pub struct SpecDependency {
    /// The spec that depends on another
    pub dependent_spec: PathBuf,
    /// The spec that is depended upon
    pub dependency_spec: PathBuf,
    /// Field name that creates the dependency (e.g., "pool_ref")
    pub field_name: String,
    /// Schema name being referenced (e.g., "Pool")
    pub referenced_schema: String,
    /// Extraction path for the dependency value
    pub extraction_path: String,
}

/// Dependency detector for analyzing specs
pub struct DependencyDetector {
    /// Schemas available in each spec (spec_path -> schema_names)
    schema_registry: HashMap<PathBuf, HashSet<String>>,
    /// Detected dependencies
    dependencies: Vec<SpecDependency>,
}

impl DependencyDetector {
    /// Create a new dependency detector
    pub fn new() -> Self {
        Self {
            schema_registry: HashMap::new(),
            dependencies: Vec::new(),
        }
    }

    /// Detect dependencies between specs by analyzing schema references
    pub fn detect_dependencies(&mut self, specs: &[(PathBuf, OpenApiSpec)]) -> Vec<SpecDependency> {
        // Build schema registry - collect all schemas from each spec
        for (path, spec) in specs {
            let schemas = self.extract_schema_names(spec);
            self.schema_registry.insert(path.clone(), schemas);
        }

        // Analyze each spec's request bodies for references to other specs' schemas
        for (path, spec) in specs {
            self.analyze_spec_references(path, spec, specs);
        }

        self.dependencies.clone()
    }

    /// Extract all schema names from a spec
    fn extract_schema_names(&self, spec: &OpenApiSpec) -> HashSet<String> {
        let mut schemas = HashSet::new();

        if let Some(components) = &spec.spec.components {
            for (name, _) in &components.schemas {
                schemas.insert(name.clone());
                // Also add common variations
                schemas.insert(name.to_lowercase());
                schemas.insert(to_snake_case(name));
            }
        }

        schemas
    }

    /// Analyze a spec's references to detect dependencies
    fn analyze_spec_references(
        &mut self,
        current_path: &PathBuf,
        spec: &OpenApiSpec,
        all_specs: &[(PathBuf, OpenApiSpec)],
    ) {
        // Analyze request body schemas for reference patterns
        for (path, path_item) in &spec.spec.paths.paths {
            if let openapiv3::ReferenceOr::Item(item) = path_item {
                // Check POST operations (most common for creating resources with refs)
                if let Some(op) = &item.post {
                    self.analyze_operation_refs(current_path, op, all_specs, path);
                }
                if let Some(op) = &item.put {
                    self.analyze_operation_refs(current_path, op, all_specs, path);
                }
                if let Some(op) = &item.patch {
                    self.analyze_operation_refs(current_path, op, all_specs, path);
                }
            }
        }
    }

    /// Analyze operation request body for reference fields
    fn analyze_operation_refs(
        &mut self,
        current_path: &PathBuf,
        operation: &openapiv3::Operation,
        all_specs: &[(PathBuf, OpenApiSpec)],
        _api_path: &str,
    ) {
        if let Some(openapiv3::ReferenceOr::Item(body)) = &operation.request_body {
            // Check JSON content
            if let Some(media_type) = body.content.get("application/json") {
                if let Some(schema_ref) = &media_type.schema {
                    self.analyze_schema_for_refs(current_path, schema_ref, all_specs, "");
                }
            }
        }
    }

    /// Recursively analyze schema for reference patterns
    fn analyze_schema_for_refs(
        &mut self,
        current_path: &PathBuf,
        schema_ref: &openapiv3::ReferenceOr<openapiv3::Schema>,
        all_specs: &[(PathBuf, OpenApiSpec)],
        field_prefix: &str,
    ) {
        match schema_ref {
            openapiv3::ReferenceOr::Item(schema) => {
                self.analyze_schema(current_path, schema, all_specs, field_prefix);
            }
            openapiv3::ReferenceOr::Reference { reference } => {
                // Could analyze $ref to other schemas here
                let _ = reference; // Silence unused warning for now
            }
        }
    }

    /// Analyze schema for reference patterns (handles both Box<Schema> and Schema)
    fn analyze_schema(
        &mut self,
        current_path: &PathBuf,
        schema: &openapiv3::Schema,
        all_specs: &[(PathBuf, OpenApiSpec)],
        field_prefix: &str,
    ) {
        match &schema.schema_kind {
            openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj)) => {
                for (prop_name, prop_schema) in &obj.properties {
                    let full_path = if field_prefix.is_empty() {
                        prop_name.clone()
                    } else {
                        format!("{}.{}", field_prefix, prop_name)
                    };

                    // Check for reference patterns in field names
                    if let Some(dep) = self.detect_ref_field(current_path, prop_name, all_specs) {
                        self.dependencies.push(SpecDependency {
                            dependent_spec: current_path.clone(),
                            dependency_spec: dep.0,
                            field_name: prop_name.clone(),
                            referenced_schema: dep.1,
                            extraction_path: format!("$.{}", full_path),
                        });
                    }

                    // Recursively check nested schemas
                    self.analyze_boxed_schema_ref(current_path, prop_schema, all_specs, &full_path);
                }
            }
            openapiv3::SchemaKind::AllOf { all_of } => {
                for sub_schema in all_of {
                    self.analyze_schema_for_refs(current_path, sub_schema, all_specs, field_prefix);
                }
            }
            openapiv3::SchemaKind::OneOf { one_of } => {
                for sub_schema in one_of {
                    self.analyze_schema_for_refs(current_path, sub_schema, all_specs, field_prefix);
                }
            }
            openapiv3::SchemaKind::AnyOf { any_of } => {
                for sub_schema in any_of {
                    self.analyze_schema_for_refs(current_path, sub_schema, all_specs, field_prefix);
                }
            }
            _ => {}
        }
    }

    /// Handle ReferenceOr<Box<Schema>> which is used in object properties
    fn analyze_boxed_schema_ref(
        &mut self,
        current_path: &PathBuf,
        schema_ref: &openapiv3::ReferenceOr<Box<openapiv3::Schema>>,
        all_specs: &[(PathBuf, OpenApiSpec)],
        field_prefix: &str,
    ) {
        match schema_ref {
            openapiv3::ReferenceOr::Item(boxed_schema) => {
                self.analyze_schema(current_path, boxed_schema.as_ref(), all_specs, field_prefix);
            }
            openapiv3::ReferenceOr::Reference { reference } => {
                let _ = reference; // Could analyze $ref here
            }
        }
    }

    /// Detect if a field name references another spec's schema
    fn detect_ref_field(
        &self,
        current_path: &PathBuf,
        field_name: &str,
        all_specs: &[(PathBuf, OpenApiSpec)],
    ) -> Option<(PathBuf, String)> {
        // Common patterns for reference fields
        let ref_patterns = [
            ("_ref", ""),       // pool_ref -> Pool
            ("_id", ""),        // pool_id -> Pool
            ("Id", ""),         // poolId -> pool
            ("_uuid", ""),      // pool_uuid -> Pool
            ("Uuid", ""),       // poolUuid -> pool
            ("_reference", ""), // pool_reference -> Pool
        ];

        for (suffix, _) in ref_patterns.iter() {
            if field_name.ends_with(suffix) {
                // Extract the schema name from the field
                let schema_base = field_name.trim_end_matches(suffix).trim_end_matches('_');

                // Search for this schema in other specs
                for (other_path, _) in all_specs {
                    if other_path == current_path {
                        continue;
                    }

                    if let Some(schemas) = self.schema_registry.get(other_path) {
                        // Check various name formats
                        let schema_pascal = to_pascal_case(schema_base);
                        let schema_lower = schema_base.to_lowercase();

                        for schema_name in schemas {
                            if schema_name == &schema_pascal
                                || schema_name == &schema_lower
                                || schema_name.to_lowercase() == schema_lower
                            {
                                return Some((other_path.clone(), schema_name.clone()));
                            }
                        }
                    }
                }
            }
        }

        None
    }
}

impl Default for DependencyDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Topologically sort specs based on dependencies
pub fn topological_sort(
    specs: &[(PathBuf, OpenApiSpec)],
    dependencies: &[SpecDependency],
) -> Result<Vec<PathBuf>> {
    let spec_paths: Vec<PathBuf> = specs.iter().map(|(p, _)| p.clone()).collect();

    // Build adjacency list (dependency -> dependent)
    let mut adj: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    let mut in_degree: HashMap<PathBuf, usize> = HashMap::new();

    for path in &spec_paths {
        adj.insert(path.clone(), Vec::new());
        in_degree.insert(path.clone(), 0);
    }

    for dep in dependencies {
        adj.entry(dep.dependency_spec.clone())
            .or_default()
            .push(dep.dependent_spec.clone());
        *in_degree.entry(dep.dependent_spec.clone()).or_insert(0) += 1;
    }

    // Kahn's algorithm
    let mut queue: Vec<PathBuf> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(path, _)| path.clone())
        .collect();

    let mut result = Vec::new();

    while let Some(path) = queue.pop() {
        result.push(path.clone());

        if let Some(dependents) = adj.get(&path) {
            for dependent in dependents {
                if let Some(deg) = in_degree.get_mut(dependent) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(dependent.clone());
                    }
                }
            }
        }
    }

    if result.len() != spec_paths.len() {
        return Err(BenchError::Other("Circular dependency detected between specs".to_string()));
    }

    Ok(result)
}

/// Convert string to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert string to PascalCase
fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c == '_' || c == '-' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

/// Extracted values from spec execution for passing to dependent specs
#[derive(Debug, Clone, Default)]
pub struct ExtractedValues {
    /// Values extracted by variable name
    pub values: HashMap<String, serde_json::Value>,
}

impl ExtractedValues {
    /// Create new empty extracted values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a value
    pub fn set(&mut self, key: String, value: serde_json::Value) {
        self.values.insert(key, value);
    }

    /// Get a value
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.values.get(key)
    }

    /// Merge values from another ExtractedValues
    pub fn merge(&mut self, other: &ExtractedValues) {
        for (key, value) in &other.values {
            self.values.insert(key.clone(), value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("PascalCase"), "pascal_case");
        assert_eq!(to_snake_case("camelCase"), "camel_case");
        assert_eq!(to_snake_case("Pool"), "pool");
        assert_eq!(to_snake_case("VirtualService"), "virtual_service");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("snake_case"), "SnakeCase");
        assert_eq!(to_pascal_case("pool"), "Pool");
        assert_eq!(to_pascal_case("virtual_service"), "VirtualService");
    }

    #[test]
    fn test_extracted_values() {
        let mut values = ExtractedValues::new();
        values.set("pool_id".to_string(), serde_json::json!("abc123"));
        values.set("name".to_string(), serde_json::json!("test-pool"));

        assert_eq!(values.get("pool_id"), Some(&serde_json::json!("abc123")));
        assert_eq!(values.get("missing"), None);
    }

    #[test]
    fn test_spec_dependency_config_default() {
        let config = SpecDependencyConfig::default();
        assert!(config.execution_order.is_empty());
        assert!(!config.disable_auto_detect);
    }
}
