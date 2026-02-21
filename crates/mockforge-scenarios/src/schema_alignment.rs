//! Schema and route alignment for scenarios
//!
//! Provides functionality to automatically align schemas and routes when
//! applying scenarios to workspaces with existing configurations.

use crate::error::{Result, ScenarioError};
use serde_json::{json, Map, Value};

/// Merge strategy for schema alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStrategy {
    /// Prefer existing schemas/routes (keep existing, ignore scenario)
    PreferExisting,

    /// Prefer scenario schemas/routes (replace existing with scenario)
    PreferScenario,

    /// Interactive mode (prompt user for conflicts)
    Interactive,

    /// Merge intelligently (combine both, resolve conflicts automatically)
    Intelligent,
}

/// Schema alignment configuration
#[derive(Debug, Clone)]
pub struct SchemaAlignmentConfig {
    /// Merge strategy
    pub merge_strategy: MergeStrategy,

    /// Whether to validate merged schemas
    pub validate_merged: bool,

    /// Whether to backup existing files before merging
    pub backup_existing: bool,
}

impl Default for SchemaAlignmentConfig {
    fn default() -> Self {
        Self {
            merge_strategy: MergeStrategy::PreferExisting,
            validate_merged: true,
            backup_existing: true,
        }
    }
}

/// OpenAPI spec alignment result
#[derive(Debug, Clone)]
pub struct OpenApiAlignmentResult {
    /// Whether alignment was successful
    pub success: bool,

    /// Merged OpenAPI spec (if successful)
    pub merged_spec: Option<Value>,

    /// Conflicts found during alignment
    pub conflicts: Vec<SchemaConflict>,

    /// Warnings during alignment
    pub warnings: Vec<String>,
}

/// Schema conflict information
#[derive(Debug, Clone)]
pub struct SchemaConflict {
    /// Conflict type
    pub conflict_type: ConflictType,

    /// Path or location of conflict
    pub path: String,

    /// Existing value
    pub existing: Option<Value>,

    /// Scenario value
    pub scenario: Option<Value>,

    /// Resolution applied (if any)
    pub resolution: Option<Value>,
}

/// Type of schema conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictType {
    /// Duplicate path with different operations
    DuplicatePath,

    /// Conflicting schema definitions
    ConflictingSchema,

    /// Conflicting operation definitions
    ConflictingOperation,

    /// Missing required component
    MissingComponent,
}

/// Align OpenAPI specifications
///
/// Merges a scenario's OpenAPI spec with an existing OpenAPI spec
/// according to the specified merge strategy.
pub fn align_openapi_specs(
    existing_spec: &Value,
    scenario_spec: &Value,
    config: &SchemaAlignmentConfig,
) -> Result<OpenApiAlignmentResult> {
    let mut conflicts = Vec::new();
    let mut warnings = Vec::new();
    let mut merged = existing_spec.clone();

    // Extract paths from both specs
    let _existing_paths: Map<String, Value> = existing_spec
        .get("paths")
        .and_then(|p| p.as_object())
        .cloned()
        .unwrap_or_default();

    let scenario_paths: Map<String, Value> = scenario_spec
        .get("paths")
        .and_then(|p| p.as_object())
        .cloned()
        .unwrap_or_default();

    // Merge paths based on strategy
    match config.merge_strategy {
        MergeStrategy::PreferExisting => {
            // Keep existing paths, only add new paths from scenario
            if let Some(merged_paths) = merged.get_mut("paths").and_then(|p| p.as_object_mut()) {
                for (path, scenario_path_item) in scenario_paths {
                    if !merged_paths.contains_key(&path) {
                        merged_paths.insert(path.clone(), scenario_path_item);
                    } else {
                        warnings.push(format!(
                            "Path '{}' already exists, keeping existing (prefer existing)",
                            path
                        ));
                    }
                }
            }
        }
        MergeStrategy::PreferScenario => {
            // Replace existing paths with scenario paths
            if let Some(merged_paths) = merged.get_mut("paths").and_then(|p| p.as_object_mut()) {
                for (path, scenario_path_item) in scenario_paths {
                    if merged_paths.contains_key(&path) {
                        warnings.push(format!(
                            "Replacing existing path '{}' with scenario path (prefer scenario)",
                            path
                        ));
                    }
                    merged_paths.insert(path.clone(), scenario_path_item);
                }
            }
        }
        MergeStrategy::Intelligent => {
            // Merge intelligently: combine operations, resolve conflicts
            if let Some(merged_paths) = merged.get_mut("paths").and_then(|p| p.as_object_mut()) {
                for (path, scenario_path_item) in scenario_paths {
                    if let Some(existing_path_item) = merged_paths.get_mut(&path) {
                        // Path exists, merge operations
                        if let (Some(existing_obj), Some(scenario_obj)) =
                            (existing_path_item.as_object_mut(), scenario_path_item.as_object())
                        {
                            for (method, scenario_op) in scenario_obj {
                                if existing_obj.contains_key(method) {
                                    // Conflict: same path and method
                                    conflicts.push(SchemaConflict {
                                        conflict_type: ConflictType::ConflictingOperation,
                                        path: format!("{} {}", method.to_uppercase(), path),
                                        existing: existing_obj.get(method).cloned(),
                                        scenario: Some(scenario_op.clone()),
                                        resolution: None,
                                    });
                                } else {
                                    // New operation, add it
                                    existing_obj.insert(method.clone(), scenario_op.clone());
                                }
                            }
                        }
                    } else {
                        // New path, add it
                        merged_paths.insert(path.clone(), scenario_path_item);
                    }
                }
            }
        }
        MergeStrategy::Interactive => {
            // For interactive mode, collect all conflicts
            if let Some(merged_paths) = merged.get_mut("paths").and_then(|p| p.as_object_mut()) {
                for (path, scenario_path_item) in scenario_paths {
                    if let Some(existing_path_item) = merged_paths.get(&path) {
                        // Check for operation conflicts
                        if let (Some(existing_obj), Some(scenario_obj)) =
                            (existing_path_item.as_object(), scenario_path_item.as_object())
                        {
                            for (method, _) in scenario_obj {
                                if existing_obj.contains_key(method) {
                                    conflicts.push(SchemaConflict {
                                        conflict_type: ConflictType::ConflictingOperation,
                                        path: format!("{} {}", method.to_uppercase(), path),
                                        existing: existing_obj.get(method).cloned(),
                                        scenario: scenario_obj.get(method).cloned(),
                                        resolution: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            // In interactive mode, don't merge automatically
            return Ok(OpenApiAlignmentResult {
                success: false,
                merged_spec: None,
                conflicts,
                warnings,
            });
        }
    }

    // Merge components/schemas
    let _existing_components: Map<String, Value> = existing_spec
        .get("components")
        .and_then(|c| c.get("schemas"))
        .and_then(|s| s.as_object())
        .cloned()
        .unwrap_or_default();

    let scenario_components: Map<String, Value> = scenario_spec
        .get("components")
        .and_then(|c| c.get("schemas"))
        .and_then(|s| s.as_object())
        .cloned()
        .unwrap_or_default();

    if !scenario_components.is_empty() {
        // Ensure merged is an object
        let merged_obj = merged.as_object_mut().ok_or_else(|| {
            ScenarioError::Generic("Invalid OpenAPI spec structure - must be an object".to_string())
        })?;

        // Ensure components exists
        if !merged_obj.contains_key("components") {
            merged_obj.insert("components".to_string(), json!({}));
        }

        let components = merged_obj.get_mut("components").and_then(|c| c.as_object_mut());

        if let Some(components_obj) = components {
            // Ensure schemas exists
            if !components_obj.contains_key("schemas") {
                components_obj.insert("schemas".to_string(), json!({}));
            }

            let schemas = components_obj
                .get_mut("schemas")
                .and_then(|s| s.as_object_mut())
                .ok_or_else(|| ScenarioError::Generic("Invalid schemas structure".to_string()))?;

            for (schema_name, scenario_schema) in scenario_components {
                match config.merge_strategy {
                    MergeStrategy::PreferExisting => {
                        if !schemas.contains_key(&schema_name) {
                            schemas.insert(schema_name.clone(), scenario_schema);
                        } else {
                            warnings.push(format!(
                                "Schema '{}' already exists, keeping existing",
                                schema_name
                            ));
                        }
                    }
                    MergeStrategy::PreferScenario => {
                        if schemas.contains_key(&schema_name) {
                            warnings.push(format!(
                                "Replacing existing schema '{}' with scenario schema",
                                schema_name
                            ));
                        }
                        schemas.insert(schema_name.clone(), scenario_schema);
                    }
                    MergeStrategy::Intelligent => {
                        if schemas.contains_key(&schema_name) {
                            // Try to merge schema properties
                            warnings.push(format!(
                                "Schema '{}' exists in both, merging properties",
                                schema_name
                            ));
                            // Simple merge: prefer scenario properties
                            schemas.insert(schema_name.clone(), scenario_schema);
                        } else {
                            schemas.insert(schema_name.clone(), scenario_schema);
                        }
                    }
                    MergeStrategy::Interactive => {
                        if schemas.contains_key(&schema_name) {
                            conflicts.push(SchemaConflict {
                                conflict_type: ConflictType::ConflictingSchema,
                                path: format!("components/schemas/{}", schema_name),
                                existing: schemas.get(&schema_name).cloned(),
                                scenario: Some(scenario_schema),
                                resolution: None,
                            });
                        }
                    }
                }
            }
        }
    }

    let success = conflicts.is_empty() || config.merge_strategy != MergeStrategy::Interactive;

    Ok(OpenApiAlignmentResult {
        success,
        merged_spec: if success { Some(merged) } else { None },
        conflicts,
        warnings,
    })
}

/// Align VBR entities
///
/// Merges scenario VBR entities with existing entities according to the merge strategy.
pub fn align_vbr_entities(
    existing_entities: &[crate::vbr_integration::VbrEntityDefinition],
    scenario_entities: &[crate::vbr_integration::VbrEntityDefinition],
    config: &SchemaAlignmentConfig,
) -> Result<Vec<SchemaConflict>> {
    let existing_by_name: std::collections::HashMap<_, _> =
        existing_entities.iter().map(|e| (e.name.as_str(), e)).collect();

    let mut conflicts = Vec::new();
    for scenario_entity in scenario_entities {
        if let Some(existing) = existing_by_name.get(scenario_entity.name.as_str()) {
            if existing.schema != scenario_entity.schema {
                let resolution = match config.merge_strategy {
                    MergeStrategy::PreferExisting => Some(existing.schema.clone()),
                    MergeStrategy::PreferScenario => Some(scenario_entity.schema.clone()),
                    MergeStrategy::Intelligent => {
                        if let (Some(existing_obj), Some(scenario_obj)) =
                            (existing.schema.as_object(), scenario_entity.schema.as_object())
                        {
                            let mut merged = existing_obj.clone();
                            for (k, v) in scenario_obj {
                                merged.insert(k.clone(), v.clone());
                            }
                            Some(Value::Object(merged))
                        } else {
                            Some(scenario_entity.schema.clone())
                        }
                    }
                    MergeStrategy::Interactive => None,
                };

                conflicts.push(SchemaConflict {
                    conflict_type: ConflictType::ConflictingSchema,
                    path: format!("entity:{}", scenario_entity.name),
                    existing: Some(existing.schema.clone()),
                    scenario: Some(scenario_entity.schema.clone()),
                    resolution,
                });
            }
        }
    }

    Ok(conflicts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_alignment_prefer_existing() {
        let existing = json!({
            "paths": {
                "/users": {
                    "get": {
                        "summary": "Get users"
                    }
                }
            }
        });

        let scenario = json!({
            "paths": {
                "/users": {
                    "post": {
                        "summary": "Create user"
                    }
                },
                "/products": {
                    "get": {
                        "summary": "Get products"
                    }
                }
            }
        });

        let config = SchemaAlignmentConfig {
            merge_strategy: MergeStrategy::PreferExisting,
            validate_merged: false,
            backup_existing: false,
        };

        let result = align_openapi_specs(&existing, &scenario, &config).unwrap();
        assert!(result.success);
        assert!(result.warnings.iter().any(|w| w.contains("/users")));
    }

    #[test]
    fn test_openapi_alignment_prefer_scenario() {
        let existing = json!({
            "paths": {
                "/users": {
                    "get": {
                        "summary": "Get users (old)"
                    }
                }
            }
        });

        let scenario = json!({
            "paths": {
                "/users": {
                    "get": {
                        "summary": "Get users (new)"
                    }
                }
            }
        });

        let config = SchemaAlignmentConfig {
            merge_strategy: MergeStrategy::PreferScenario,
            validate_merged: false,
            backup_existing: false,
        };

        let result = align_openapi_specs(&existing, &scenario, &config).unwrap();
        assert!(result.success);
        let merged = result.merged_spec.unwrap();
        let path = merged["paths"]["/users"]["get"]["summary"].as_str().unwrap();
        assert_eq!(path, "Get users (new)");
    }

    #[test]
    fn test_align_vbr_entities_detects_conflict() {
        let existing = vec![crate::vbr_integration::VbrEntityDefinition::new(
            "User".to_string(),
            json!({"base":{"name":"User","fields":[{"name":"id"}]}}),
        )];
        let scenario = vec![crate::vbr_integration::VbrEntityDefinition::new(
            "User".to_string(),
            json!({"base":{"name":"User","fields":[{"name":"id"},{"name":"email"}]}}),
        )];
        let config = SchemaAlignmentConfig {
            merge_strategy: MergeStrategy::PreferScenario,
            ..Default::default()
        };

        let conflicts = align_vbr_entities(&existing, &scenario, &config).unwrap();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].conflict_type, ConflictType::ConflictingSchema);
        assert!(conflicts[0].resolution.is_some());
    }
}
