//! Relationship Inference for Smart Personas
//!
//! This module provides functionality to automatically detect and infer
//! relationships between entities in OpenAPI specifications, enabling
//! automatic generation of related data.

use crate::{OpenApiSpec, Result};
use openapiv3::{ReferenceOr, Schema};

/// Represents a relationship between two entities
#[derive(Debug, Clone)]
pub struct Relationship {
    /// Parent entity name (e.g., "apiary")
    pub parent_entity: String,
    /// Child entity name (e.g., "hive")
    pub child_entity: String,
    /// Field in parent that indicates count (e.g., "hive_count")
    pub count_field: Option<String>,
    /// Field in child that references parent (e.g., "apiary_id")
    pub foreign_key_field: Option<String>,
    /// API path for the relationship (e.g., "/api/apiaries/{id}/hives")
    pub relationship_path: Option<String>,
    /// HTTP method for the relationship endpoint
    pub method: String,
}

impl Relationship {
    /// Create a new relationship
    pub fn new(parent_entity: String, child_entity: String) -> Self {
        Self {
            parent_entity,
            child_entity,
            count_field: None,
            foreign_key_field: None,
            relationship_path: None,
            method: "GET".to_string(),
        }
    }

    /// Set the count field
    pub fn with_count_field(mut self, field: String) -> Self {
        self.count_field = Some(field);
        self
    }

    /// Set the foreign key field
    pub fn with_foreign_key_field(mut self, field: String) -> Self {
        self.foreign_key_field = Some(field);
        self
    }

    /// Set the relationship path
    pub fn with_path(mut self, path: String) -> Self {
        self.relationship_path = Some(path);
        self.method = "GET".to_string();
        self
    }
}

/// Relationship inference engine
pub struct RelationshipInference {
    /// Detected relationships
    relationships: Vec<Relationship>,
}

impl RelationshipInference {
    /// Create a new relationship inference engine
    pub fn new() -> Self {
        Self {
            relationships: Vec::new(),
        }
    }

    /// Infer relationships from an OpenAPI specification
    pub fn infer_relationships(&mut self, spec: &OpenApiSpec) -> Result<Vec<Relationship>> {
        self.relationships.clear();

        // Strategy 1: Path-based inference
        // Look for patterns like /api/{parent}/{id}/{child}
        self.infer_from_paths(spec)?;

        // Strategy 2: Schema-based inference
        // Look for foreign key patterns and count fields in schemas
        self.infer_from_schemas(spec)?;

        Ok(self.relationships.clone())
    }

    /// Infer relationships from API paths
    fn infer_from_paths(&mut self, spec: &OpenApiSpec) -> Result<()> {
        // Extract entity names from paths
        // Pattern: /api/{parent_entity}/{id}/{child_entity}
        // Example: /api/apiaries/{apiaryId}/hives

        let paths = &spec.spec.paths.paths;
        for (path, path_item) in paths.iter() {
            // Check if path matches nested resource pattern
            // Pattern: /api/{parent}/{id}/{child}
            let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

            if parts.len() >= 4 {
                // Check if we have a pattern like: /api/{parent}/{id}/{child}
                let parent_part = parts.get(1);
                let id_part = parts.get(2);
                let child_part = parts.get(3);

                if let (Some(parent), Some(id_param), Some(child)) =
                    (parent_part, id_part, child_part)
                {
                    // Check if middle part is an ID parameter (starts with { and ends with })
                    if id_param.starts_with('{') && id_param.ends_with('}') {
                        // Extract entity names
                        let parent_entity = parent.trim_end_matches('s'); // Remove plural
                        let child_entity = child.trim_end_matches('s'); // Remove plural

                        // Check if this path has a GET operation
                        let has_get = match path_item {
                            ReferenceOr::Item(item) => item.get.is_some() || item.post.is_some(),
                            ReferenceOr::Reference { .. } => false,
                        };

                        if has_get {
                            let relationship = Relationship::new(
                                parent_entity.to_string(),
                                child_entity.to_string(),
                            )
                            .with_path(path.clone())
                            .with_foreign_key_field(format!("{}_id", parent_entity));

                            tracing::debug!(
                                "Inferred relationship from path: {} -> {} (path: {})",
                                parent_entity,
                                child_entity,
                                path
                            );

                            self.relationships.push(relationship);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Infer relationships from schemas
    fn infer_from_schemas(&mut self, spec: &OpenApiSpec) -> Result<()> {
        // Look for count fields and foreign key patterns in schemas
        if let Some(components) = &spec.spec.components {
            let schemas = &components.schemas;
            for (schema_name, schema_ref) in schemas {
                if let ReferenceOr::Item(schema) = schema_ref {
                    self.analyze_schema_for_relationships(spec, schema_name, schema)?;
                }
            }
        }

        Ok(())
    }

    /// Analyze a schema for relationship indicators
    fn analyze_schema_for_relationships(
        &mut self,
        spec: &OpenApiSpec,
        schema_name: &str,
        schema: &Schema,
    ) -> Result<()> {
        // Extract entity name from schema name (e.g., "Apiary" -> "apiary")
        let entity_name = schema_name.to_lowercase();

        // Check if this schema has properties
        if let openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj)) = &schema.schema_kind {
            // Look for count fields (e.g., "hive_count", "apiary_count")
            for (prop_name, _prop_schema) in &obj.properties {
                let prop_lower = prop_name.to_lowercase();

                // Pattern: {entity}_count indicates relationship to {entity}
                if prop_lower.ends_with("_count") {
                    let related_entity =
                        prop_lower.strip_suffix("_count").unwrap_or("").to_string();

                    if !related_entity.is_empty() && related_entity != entity_name {
                        // Check if we already have this relationship
                        let exists = self.relationships.iter().any(|r| {
                            r.parent_entity == entity_name && r.child_entity == related_entity
                        });

                        if !exists {
                            let relationship =
                                Relationship::new(entity_name.clone(), related_entity.clone())
                                    .with_count_field(prop_name.clone())
                                    .with_foreign_key_field(format!("{}_id", entity_name));

                            tracing::debug!(
                                "Inferred relationship from count field: {} -> {} (count_field: {})",
                                entity_name,
                                related_entity,
                                prop_name
                            );

                            self.relationships.push(relationship);
                        }
                    }
                }

                // Pattern: {entity}_id indicates foreign key to {entity}
                if prop_lower.ends_with("_id") && prop_lower != "id" {
                    let parent_entity = prop_lower.strip_suffix("_id").unwrap_or("").to_string();

                    if !parent_entity.is_empty() && parent_entity != entity_name {
                        // This entity has a foreign key to parent_entity
                        // Check if we already have this relationship
                        let exists = self.relationships.iter().any(|r| {
                            r.parent_entity == parent_entity && r.child_entity == entity_name
                        });

                        if !exists {
                            let relationship =
                                Relationship::new(parent_entity.clone(), entity_name.clone())
                                    .with_foreign_key_field(prop_name.clone());

                            tracing::debug!(
                                "Inferred relationship from foreign key: {} -> {} (fk_field: {})",
                                parent_entity,
                                entity_name,
                                prop_name
                            );

                            self.relationships.push(relationship);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get relationships for a specific parent entity
    pub fn get_relationships_for_parent(&self, parent_entity: &str) -> Vec<&Relationship> {
        self.relationships.iter().filter(|r| r.parent_entity == parent_entity).collect()
    }

    /// Get all relationships
    pub fn get_all_relationships(&self) -> &[Relationship] {
        &self.relationships
    }
}

impl Default for RelationshipInference {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relationship_creation() {
        let rel = Relationship::new("apiary".to_string(), "hive".to_string())
            .with_count_field("hive_count".to_string())
            .with_foreign_key_field("apiary_id".to_string())
            .with_path("/api/apiaries/{id}/hives".to_string());

        assert_eq!(rel.parent_entity, "apiary");
        assert_eq!(rel.child_entity, "hive");
        assert_eq!(rel.count_field, Some("hive_count".to_string()));
        assert_eq!(rel.foreign_key_field, Some("apiary_id".to_string()));
        assert_eq!(rel.relationship_path, Some("/api/apiaries/{id}/hives".to_string()));
    }

    #[test]
    fn test_relationship_inference_new() {
        let inference = RelationshipInference::new();
        assert_eq!(inference.relationships.len(), 0);
    }
}
