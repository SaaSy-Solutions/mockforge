//! Entity inference from JSON payloads
//!
//! Analyzes JSON payloads to infer entity structures, relationships, and schemas.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Relationship type between entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    /// One-to-one relationship
    OneToOne,
    /// One-to-many relationship
    OneToMany,
    /// Many-to-many relationship
    ManyToMany,
}

/// Entity definition inferred from JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDefinition {
    /// Entity name (e.g., "User", "Product")
    pub name: String,
    /// Inferred JSON schema
    pub schema: Value,
    /// Primary key field name
    pub primary_key: Option<String>,
    /// Foreign key relationships (field_name -> target_entity)
    pub foreign_keys: HashMap<String, String>,
    /// Relationships to other entities
    pub relationships: Vec<Relationship>,
    /// Example data used for inference
    pub examples: Vec<Value>,
}

/// Relationship between entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Target entity name
    pub target: String,
    /// Relationship type
    pub relationship_type: RelationshipType,
    /// Field name in this entity that references the target
    pub field: Option<String>,
}

/// Entity inference engine
pub struct EntityInference;

impl EntityInference {
    /// Infer entities from a collection of JSON payloads
    pub fn infer_entities(payloads: Vec<Value>) -> Vec<EntityDefinition> {
        let mut entities: HashMap<String, EntityDefinition> = HashMap::new();

        for payload in payloads {
            if let Some(entity_name) = Self::infer_entity_name(&payload) {
                let entity =
                    entities.entry(entity_name.clone()).or_insert_with(|| EntityDefinition {
                        name: entity_name.clone(),
                        schema: json!({}),
                        primary_key: None,
                        foreign_keys: HashMap::new(),
                        relationships: Vec::new(),
                        examples: Vec::new(),
                    });

                // Merge schema
                let merged = Self::merge_schema(&entity.schema, &payload);
                entity.schema = merged;
                entity.examples.push(payload);
            }
        }

        // Infer primary keys (first pass)
        let entity_names: Vec<String> = entities.keys().cloned().collect();
        for entity_name in &entity_names {
            if let Some(entity) = entities.get_mut(entity_name) {
                entity.primary_key = Self::infer_primary_key(&entity.schema);
            }
        }

        // Infer foreign keys (second pass - need immutable access to all entities)
        // Collect entity names and schemas first to avoid borrow conflicts
        let entity_schemas: Vec<(String, Value)> = entities
            .iter()
            .map(|(name, entity)| (name.clone(), entity.schema.clone()))
            .collect();
        let entity_names_set: std::collections::HashSet<String> =
            entities.keys().cloned().collect();

        for (entity_name, schema) in entity_schemas {
            if let Some(entity) = entities.get_mut(&entity_name) {
                entity.foreign_keys = Self::infer_foreign_keys(&schema, &entity_names_set);
            }
        }

        // Build relationships (third pass)
        // Clone foreign keys first to avoid borrow conflicts
        let foreign_keys_map: HashMap<String, HashMap<String, String>> = entities
            .iter()
            .map(|(name, entity)| (name.clone(), entity.foreign_keys.clone()))
            .collect();

        for (entity_name, foreign_keys) in foreign_keys_map {
            if let Some(entity) = entities.get_mut(&entity_name) {
                entity.relationships = foreign_keys
                    .iter()
                    .map(|(field_name, target_entity)| Relationship {
                        target: target_entity.clone(),
                        relationship_type: RelationshipType::OneToMany,
                        field: Some(field_name.clone()),
                    })
                    .collect();
            }
        }

        entities.into_values().collect()
    }

    /// Infer entity name from JSON payload
    fn infer_entity_name(payload: &Value) -> Option<String> {
        if let Some(obj) = payload.as_object() {
            // Check for common ID fields that suggest entity type
            if let Some(id) = obj.get("id") {
                if let Some(id_str) = id.as_str() {
                    // Try to extract entity name from ID pattern (e.g., "user_123" -> "User")
                    if let Some(prefix) = id_str.split('_').next() {
                        return Some(Self::capitalize(prefix));
                    }
                }
            }

            // Check for type field
            if let Some(typ) = obj.get("type") {
                if let Some(typ_str) = typ.as_str() {
                    return Some(Self::capitalize(typ_str));
                }
            }

            // Use first key as hint (e.g., {"user": {...}} -> "User")
            if let Some((first_key, _)) = obj.iter().next() {
                if first_key.len() > 1 {
                    return Some(Self::capitalize(first_key));
                }
            }
        }

        // Default: use "Entity"
        Some("Entity".to_string())
    }

    /// Merge schemas from multiple examples
    fn merge_schema(existing: &Value, new: &Value) -> Value {
        match (existing, new) {
            (Value::Object(existing_obj), Value::Object(new_obj)) => {
                let mut merged = existing_obj.clone();

                for (key, new_val) in new_obj {
                    let key_str = key.as_str();
                    if let Some(existing_val) = merged.get(key_str) {
                        merged.insert(key.clone(), Self::merge_schema(existing_val, new_val));
                    } else {
                        merged.insert(key.clone(), Self::infer_field_schema(new_val));
                    }
                }

                Value::Object(merged)
            }
            (_, new) => Self::infer_field_schema(new),
        }
    }

    /// Infer schema for a single field value
    fn infer_field_schema(value: &Value) -> Value {
        match value {
            Value::Null => json!({"type": "null"}),
            Value::Bool(_) => json!({"type": "boolean"}),
            Value::Number(n) => {
                if n.is_i64() || n.is_u64() {
                    json!({"type": "integer"})
                } else {
                    json!({"type": "number"})
                }
            }
            Value::String(s) => {
                let mut schema = json!({"type": "string"});
                // Detect formats
                if s.contains('@') && s.contains('.') {
                    schema["format"] = json!("email");
                } else if s.len() == 36 && s.contains('-') {
                    schema["format"] = json!("uuid");
                } else if s.starts_with("http://") || s.starts_with("https://") {
                    schema["format"] = json!("uri");
                }
                schema
            }
            Value::Array(arr) => {
                if let Some(first) = arr.first() {
                    json!({
                        "type": "array",
                        "items": Self::infer_field_schema(first)
                    })
                } else {
                    json!({"type": "array"})
                }
            }
            Value::Object(obj) => {
                let mut properties = serde_json::Map::new();
                for (key, val) in obj {
                    properties.insert(key.clone(), Self::infer_field_schema(val));
                }
                json!({
                    "type": "object",
                    "properties": properties
                })
            }
        }
    }

    /// Infer primary key field
    fn infer_primary_key(schema: &Value) -> Option<String> {
        if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
            // Common primary key patterns
            let primary_key_candidates = ["id", "uuid", "_id", "key", "identifier"];

            for candidate in &primary_key_candidates {
                if properties.contains_key(*candidate) {
                    return Some(candidate.to_string());
                }
            }

            // Check for fields ending in "_id" or "Id"
            for key in properties.keys() {
                if key.to_lowercase().ends_with("_id") || key.to_lowercase().ends_with("id") {
                    return Some(key.clone());
                }
            }
        }

        None
    }

    /// Infer foreign key relationships
    fn infer_foreign_keys(
        schema: &Value,
        entity_names: &std::collections::HashSet<String>,
    ) -> HashMap<String, String> {
        let mut foreign_keys = HashMap::new();

        if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
            for (field_name, _field_schema) in properties {
                // Check if field name suggests a foreign key
                if field_name.ends_with("_id")
                    || field_name.ends_with("Id")
                    || field_name.ends_with("_uuid")
                {
                    // Try to infer target entity from field name
                    let base_name = field_name
                        .trim_end_matches("_id")
                        .trim_end_matches("Id")
                        .trim_end_matches("_uuid");
                    let target_entity = Self::capitalize(base_name);

                    // Check if target entity exists
                    if entity_names.contains(&target_entity) {
                        foreign_keys.insert(field_name.clone(), target_entity);
                    }
                }
            }
        }

        foreign_keys
    }

    /// Capitalize first letter
    fn capitalize(s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_entities() {
        let payloads = vec![
            json!({"id": "user_1", "name": "Alice", "email": "alice@example.com"}),
            json!({"id": "user_2", "name": "Bob", "email": "bob@example.com"}),
        ];

        let entities = EntityInference::infer_entities(payloads);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "User");
        assert!(entities[0].primary_key.is_some());
    }
}
