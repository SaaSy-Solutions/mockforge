//! OpenAPI integration for VBR
//!
//! This module provides functionality to automatically generate VBR entities
//! and CRUD operations from OpenAPI 3.x specifications.

use crate::schema::{AutoGenerationRule, CascadeAction, ForeignKeyDefinition, VbrSchemaDefinition};
use crate::{Error, Result};
use mockforge_core::openapi::OpenApiSpec;
use mockforge_data::schema::{FieldDefinition, SchemaDefinition};
use openapiv3::{ReferenceOr, Schema, SchemaKind, Type};
use std::collections::HashMap;

/// Result of converting an OpenAPI spec to VBR entities
#[derive(Debug)]
pub struct OpenApiConversionResult {
    /// Successfully converted entities
    pub entities: Vec<(String, VbrSchemaDefinition)>,
    /// Warnings encountered during conversion
    pub warnings: Vec<String>,
}

/// Convert an OpenAPI specification to VBR entities
///
/// This function automatically:
/// - Extracts schemas from `components/schemas`
/// - Detects primary keys (fields named "id", "uuid", or marked as required)
/// - Detects foreign keys (fields ending in "_id" or following naming conventions)
/// - Converts OpenAPI schema types to VBR field definitions
///
/// # Arguments
/// * `spec` - The OpenAPI specification to convert
///
/// # Returns
/// Conversion result with entities and any warnings
pub fn convert_openapi_to_vbr(spec: &OpenApiSpec) -> Result<OpenApiConversionResult> {
    let mut entities = Vec::new();
    let mut warnings = Vec::new();

    // Extract schemas from components
    let schemas = extract_schemas_from_openapi(spec);

    if schemas.is_empty() {
        warnings.push("No schemas found in OpenAPI components/schemas".to_string());
        return Ok(OpenApiConversionResult { entities, warnings });
    }

    // Convert each schema to a VBR entity
    // Collect schema names first to avoid borrow issues
    let schema_names: Vec<String> = schemas.keys().cloned().collect();
    for schema_name in schema_names {
        let schema = schemas.get(&schema_name).unwrap().clone();
        match convert_schema_to_vbr(&schema_name, schema, &schemas) {
            Ok(vbr_schema) => {
                entities.push((schema_name.clone(), vbr_schema));
            }
            Err(e) => {
                warnings.push(format!("Failed to convert schema '{}': {}", schema_name, e));
            }
        }
    }

    // Auto-detect foreign keys based on field names and schema references
    // Collect entity names first to avoid borrow conflicts
    let entity_names: Vec<String> = entities.iter().map(|(n, _)| n.clone()).collect();
    for (entity_name, vbr_schema) in &mut entities {
        detect_foreign_keys(entity_name, vbr_schema, &entity_names, &mut warnings);
    }

    Ok(OpenApiConversionResult { entities, warnings })
}

/// Extract all schemas from OpenAPI components
fn extract_schemas_from_openapi(spec: &OpenApiSpec) -> HashMap<String, Schema> {
    let mut schemas = HashMap::new();

    if let Some(components) = &spec.spec.components {
        if !components.schemas.is_empty() {
            for (name, schema_ref) in &components.schemas {
                if let ReferenceOr::Item(schema) = schema_ref {
                    schemas.insert(name.clone(), schema.clone());
                }
            }
        }
    }

    schemas
}

/// Convert an OpenAPI schema to a VBR schema definition
fn convert_schema_to_vbr(
    schema_name: &str,
    schema: Schema,
    all_schemas: &HashMap<String, Schema>,
) -> Result<VbrSchemaDefinition> {
    let mut fields = Vec::new();
    let mut primary_key = Vec::new();
    let mut auto_generation = HashMap::new();

    // Extract fields from schema
    if let SchemaKind::Type(Type::Object(obj_type)) = &schema.schema_kind {
        // Process properties (properties is directly an IndexMap, not Option)
        for (field_name, field_schema_ref) in &obj_type.properties {
            match field_schema_ref {
                ReferenceOr::Item(field_schema) => {
                    let field_def = convert_field_to_definition(
                        field_name,
                        field_schema,
                        &obj_type.required,
                    )?;
                    fields.push(field_def.clone());

                    // Auto-detect primary key
                    if is_primary_key_field(field_name, &field_def) {
                        primary_key.push(field_name.clone());
                        // Auto-generate UUID for primary keys if not specified
                        if primary_key.len() == 1 && !auto_generation.contains_key(field_name) {
                            auto_generation.insert(field_name.clone(), AutoGenerationRule::Uuid);
                        }
                    }

                    // Auto-detect auto-generation rules
                    if let Some(rule) = detect_auto_generation(field_name, field_schema) {
                        auto_generation.insert(field_name.clone(), rule);
                    }
                }
                ReferenceOr::Reference { reference } => {
                    // Handle schema references - for now, treat as string
                    // TODO: Resolve references properly
                    let field_def = FieldDefinition::new(
                        field_name.clone(),
                        "string".to_string(),
                    )
                    .optional();
                    fields.push(field_def);
                }
            }
        }
    } else {
        return Err(Error::generic(format!(
            "Schema '{}' is not an object type, cannot convert to entity",
            schema_name
        )));
    }

    // Default primary key if none detected
    if primary_key.is_empty() {
        // Try to find an "id" field
        if fields.iter().any(|f| f.name == "id") {
            primary_key.push("id".to_string());
            auto_generation.insert("id".to_string(), AutoGenerationRule::Uuid);
        } else {
            // Create a default "id" field
            primary_key.push("id".to_string());
            fields.insert(
                0,
                FieldDefinition::new("id".to_string(), "string".to_string())
                    .with_description("Auto-generated primary key".to_string()),
            );
            auto_generation.insert("id".to_string(), AutoGenerationRule::Uuid);
        }
    }

    // Create base schema definition
    let base_schema = SchemaDefinition {
        name: schema_name.to_string(),
        fields,
        description: schema
            .schema_data
            .description
            .as_ref()
            .map(|s| s.clone()),
        metadata: HashMap::new(),
        relationships: HashMap::new(),
    };

    // Create VBR schema definition
    let vbr_schema = VbrSchemaDefinition::new(base_schema)
        .with_primary_key(primary_key);

    // Apply auto-generation rules
    let mut final_schema = vbr_schema;
    for (field, rule) in auto_generation {
        final_schema = final_schema.with_auto_generation(field, rule);
    }

    Ok(final_schema)
}

/// Convert an OpenAPI schema field to a FieldDefinition
fn convert_field_to_definition(
    field_name: &str,
    schema: &Schema,
    required_fields: &[String],
) -> Result<FieldDefinition> {
    let required = required_fields.contains(&field_name.to_string());
    let field_type = schema_type_to_string(schema)?;
    let description = schema.schema_data.description.clone();

    let mut field_def = FieldDefinition::new(field_name.to_string(), field_type);

    if !required {
        field_def = field_def.optional();
    }

    if let Some(desc) = description {
        field_def = field_def.with_description(desc);
    }

    // Extract constraints from schema
    if let SchemaKind::Type(Type::String(string_type)) = &schema.schema_kind {
        if let Some(max_length) = string_type.max_length {
            field_def = field_def.with_constraint("maxLength".to_string(), max_length.into());
        }
        if let Some(min_length) = string_type.min_length {
            field_def = field_def.with_constraint("minLength".to_string(), min_length.into());
        }
        if let Some(pattern) = &string_type.pattern {
            field_def = field_def.with_constraint("pattern".to_string(), pattern.clone().into());
        }
    } else if let SchemaKind::Type(Type::Integer(int_type)) = &schema.schema_kind {
        if let Some(maximum) = int_type.maximum {
            field_def = field_def.with_constraint("maximum".to_string(), maximum.into());
        }
        if let Some(minimum) = int_type.minimum {
            field_def = field_def.with_constraint("minimum".to_string(), minimum.into());
        }
    } else if let SchemaKind::Type(Type::Number(num_type)) = &schema.schema_kind {
        if let Some(maximum) = num_type.maximum {
            field_def = field_def.with_constraint("maximum".to_string(), maximum.into());
        }
        if let Some(minimum) = num_type.minimum {
            field_def = field_def.with_constraint("minimum".to_string(), minimum.into());
        }
    }

    Ok(field_def)
}

/// Convert OpenAPI schema type to string representation
fn schema_type_to_string(schema: &Schema) -> Result<String> {
    match &schema.schema_kind {
        SchemaKind::Type(Type::String(string_type)) => {
            // Check for format (format is VariantOrUnknownOrEmpty, not Option)
            match &string_type.format {
                openapiv3::VariantOrUnknownOrEmpty::Item(fmt) => match fmt {
                    openapiv3::StringFormat::Date => Ok("date".to_string()),
                    openapiv3::StringFormat::DateTime => Ok("datetime".to_string()),
                    _ => Ok("string".to_string()),
                },
                _ => Ok("string".to_string()),
            }
        }
        SchemaKind::Type(Type::Integer(_)) => Ok("integer".to_string()),
        SchemaKind::Type(Type::Number(_)) => Ok("number".to_string()),
        SchemaKind::Type(Type::Boolean(_)) => Ok("boolean".to_string()),
        SchemaKind::Type(Type::Array(_)) => Ok("array".to_string()),
        SchemaKind::Type(Type::Object(_)) => Ok("object".to_string()),
        _ => Ok("string".to_string()), // Default fallback
    }
}

/// Check if a field is a primary key candidate
fn is_primary_key_field(field_name: &str, field_def: &FieldDefinition) -> bool {
    // Check common primary key names
    let pk_names = ["id", "uuid", "_id", "pk"];
    if pk_names.contains(&field_name.to_lowercase().as_str()) {
        return true;
    }

    // Check if field is required and has a unique constraint
    if field_def.required {
        // Additional heuristics could be added here
        false
    } else {
        false
    }
}

/// Detect auto-generation rules for a field
fn detect_auto_generation(field_name: &str, schema: &Schema) -> Option<AutoGenerationRule> {
    let name_lower = field_name.to_lowercase();

    // UUID fields
    if name_lower.contains("uuid") || name_lower == "id" {
        if let SchemaKind::Type(Type::String(string_type)) = &schema.schema_kind {
            // Check if format indicates UUID (though StringFormat doesn't have Uuid variant,
            // we check the field name instead)
            if let openapiv3::VariantOrUnknownOrEmpty::Item(_) = &string_type.format {
                // Format exists, but StringFormat doesn't have Uuid variant
                // We'll rely on field name detection instead
            }
        }
        // Default to UUID for id/uuid fields
        return Some(AutoGenerationRule::Uuid);
    }

    // Timestamp fields
    if name_lower.contains("timestamp") || name_lower.contains("created_at") || name_lower.contains("updated_at") {
        return Some(AutoGenerationRule::Timestamp);
    }

    // Date fields
    if name_lower.contains("date") && !name_lower.contains("timestamp") {
        if let SchemaKind::Type(Type::String(string_type)) = &schema.schema_kind {
            if let openapiv3::VariantOrUnknownOrEmpty::Item(
                openapiv3::StringFormat::Date,
            ) = &string_type.format
            {
                return Some(AutoGenerationRule::Date);
            }
        }
    }

    None
}

/// Auto-detect foreign key relationships
fn detect_foreign_keys(
    entity_name: &str,
    vbr_schema: &mut VbrSchemaDefinition,
    entity_names: &[String],
    warnings: &mut Vec<String>,
) {

    for field in &vbr_schema.base.fields {
        // Check if field name suggests a foreign key
        if is_foreign_key_field(&field.name, &entity_names) {
            if let Some(target_entity) = extract_target_entity(&field.name, &entity_names) {
                // Check if foreign key already exists
                if !vbr_schema
                    .foreign_keys
                    .iter()
                    .any(|fk| fk.field == field.name)
                {
                    let fk = ForeignKeyDefinition {
                        field: field.name.clone(),
                        target_entity: target_entity.clone(),
                        target_field: "id".to_string(), // Default to "id"
                        on_delete: CascadeAction::NoAction,
                        on_update: CascadeAction::NoAction,
                    };
                    vbr_schema.foreign_keys.push(fk);
                }
            }
        }
    }
}

/// Check if a field name suggests a foreign key
fn is_foreign_key_field(field_name: &str, entity_names: &[String]) -> bool {
    let name_lower = field_name.to_lowercase();

    // Common foreign key patterns
    if name_lower.ends_with("_id") {
        return true;
    }

    // Check if field name matches an entity name (camelCase or snake_case)
    for entity_name in entity_names {
        let entity_lower = entity_name.to_lowercase();
        // Match patterns like "userId", "user_id", "user"
        if name_lower == entity_lower
            || name_lower == format!("{}_id", entity_lower)
            || name_lower == format!("{}id", entity_lower)
        {
            return true;
        }
    }

    false
}

/// Extract target entity name from a foreign key field name
fn extract_target_entity(field_name: &str, entity_names: &[String]) -> Option<String> {
    let name_lower = field_name.to_lowercase();

    // Remove common suffixes
    let base_name = name_lower
        .trim_end_matches("_id")
        .trim_end_matches("id")
        .to_string();

    // Find matching entity
    for entity_name in entity_names {
        let entity_lower = entity_name.to_lowercase();
        if base_name == entity_lower || name_lower == format!("{}_id", entity_lower) {
            return Some(entity_name.clone());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_core::openapi::OpenApiSpec;

    #[test]
    fn test_extract_schemas() {
        let spec_json = serde_json::json!({
            "openapi": "3.0.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "components": {
                "schemas": {
                    "User": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "format": "uuid"
                            },
                            "name": {
                                "type": "string"
                            },
                            "email": {
                                "type": "string",
                                "format": "email"
                            }
                        },
                        "required": ["id", "name", "email"]
                    }
                }
            },
            "paths": {}
        });

        let spec = OpenApiSpec::from_json(spec_json).unwrap();
        let schemas = extract_schemas_from_openapi(&spec);

        assert_eq!(schemas.len(), 1);
        assert!(schemas.contains_key("User"));
    }

    #[test]
    fn test_convert_schema_to_vbr() {
        let spec_json = serde_json::json!({
            "openapi": "3.0.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "components": {
                "schemas": {
                    "User": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "format": "uuid"
                            },
                            "name": {
                                "type": "string"
                            }
                        },
                        "required": ["id", "name"]
                    }
                }
            },
            "paths": {}
        });

        let spec = OpenApiSpec::from_json(spec_json).unwrap();
        let schemas = extract_schemas_from_openapi(&spec);
        let user_schema = schemas.get("User").unwrap();

        let result = convert_schema_to_vbr("User", user_schema.clone(), &schemas);
        assert!(result.is_ok());

        let vbr_schema = result.unwrap();
        assert_eq!(vbr_schema.primary_key, vec!["id"]);
        assert_eq!(vbr_schema.base.fields.len(), 2);
        assert!(vbr_schema.auto_generation.contains_key("id"));
    }
}
