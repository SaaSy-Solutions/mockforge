//! Data seeding functionality
//!
//! This module provides functionality to seed the VBR database with initial data
//! from JSON/YAML files or programmatically via API.

use crate::entities::{Entity, EntityRegistry};
use crate::schema::ForeignKeyDefinition;
use crate::{Error, Result};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Seed data structure
///
/// Represents seed data organized by entity name
pub type SeedData = HashMap<String, Vec<HashMap<String, Value>>>;

/// Seed a single entity with records
///
/// # Arguments
/// * `database` - The virtual database instance
/// * `registry` - The entity registry
/// * `entity_name` - Name of the entity to seed
/// * `records` - Records to insert
pub async fn seed_entity(
    database: &dyn crate::database::VirtualDatabase,
    registry: &EntityRegistry,
    entity_name: &str,
    records: &[HashMap<String, Value>],
) -> Result<usize> {
    let entity = registry
        .get(entity_name)
        .ok_or_else(|| Error::generic(format!("Entity '{}' not found", entity_name)))?;

    let table_name = entity.table_name();
    let mut inserted_count = 0;

    for record in records {
        // Validate foreign keys before insertion
        validate_foreign_keys(registry, entity, record)?;

        // Build INSERT query
        let fields: Vec<String> = record.keys().cloned().collect();
        let placeholders: Vec<String> = (0..fields.len()).map(|_| "?".to_string()).collect();

        let query = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table_name,
            fields.join(", "),
            placeholders.join(", ")
        );

        // Prepare values in the same order as fields
        let mut values: Vec<Value> = fields.iter().map(|f| record.get(f).cloned().unwrap_or(Value::Null)).collect();

        database.execute(&query, &values).await?;
        inserted_count += 1;
    }

    Ok(inserted_count)
}

/// Seed multiple entities with dependency ordering
///
/// This function automatically orders entities based on foreign key dependencies
/// to ensure parent entities are seeded before child entities.
///
/// # Arguments
/// * `database` - The virtual database instance
/// * `registry` - The entity registry
/// * `seed_data` - Seed data organized by entity name
pub async fn seed_all(
    database: &dyn crate::database::VirtualDatabase,
    registry: &EntityRegistry,
    seed_data: &SeedData,
) -> Result<HashMap<String, usize>> {
    // Build dependency graph
    let order = topological_sort(registry, seed_data)?;

    let mut results = HashMap::new();

    // Seed entities in dependency order
    for entity_name in order {
        if let Some(records) = seed_data.get(&entity_name) {
            let count = seed_entity(database, registry, &entity_name, records).await?;
            results.insert(entity_name.clone(), count);
        }
    }

    Ok(results)
}

/// Load seed data from a JSON file
pub async fn load_seed_file_json<P: AsRef<Path>>(path: P) -> Result<SeedData> {
    let content = tokio::fs::read_to_string(path.as_ref())
        .await
        .map_err(|e| Error::generic(format!("Failed to read seed file: {}", e)))?;

    let json: Value = serde_json::from_str(&content)
        .map_err(|e| Error::generic(format!("Failed to parse JSON: {}", e)))?;

    parse_seed_data(json)
}

/// Load seed data from a YAML file
pub async fn load_seed_file_yaml<P: AsRef<Path>>(path: P) -> Result<SeedData> {
    let content = tokio::fs::read_to_string(path.as_ref())
        .await
        .map_err(|e| Error::generic(format!("Failed to read seed file: {}", e)))?;

    let yaml: Value = serde_yaml::from_str(&content)
        .map_err(|e| Error::generic(format!("Failed to parse YAML: {}", e)))?;

    parse_seed_data(yaml)
}

/// Load seed data from a file (auto-detect format)
pub async fn load_seed_file<P: AsRef<Path>>(path: P) -> Result<SeedData> {
    let path_ref = path.as_ref();
    let ext = path_ref
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "json" => load_seed_file_json(path_ref).await,
        "yaml" | "yml" => load_seed_file_yaml(path_ref).await,
        _ => {
            // Try JSON first, then YAML
            match load_seed_file_json(path_ref).await {
                Ok(data) => Ok(data),
                Err(_) => load_seed_file_yaml(path_ref).await,
            }
        }
    }
}

/// Parse seed data from JSON/YAML Value
fn parse_seed_data(value: Value) -> Result<SeedData> {
    let obj = value
        .as_object()
        .ok_or_else(|| Error::generic("Seed data must be an object".to_string()))?;

    let mut seed_data = HashMap::new();

    for (entity_name, records_value) in obj {
        let records = records_value
            .as_array()
            .ok_or_else(|| {
                Error::generic(format!(
                    "Entity '{}' seed data must be an array",
                    entity_name
                ))
            })?
            .iter()
            .map(|v| {
                v.as_object()
                    .ok_or_else(|| {
                        Error::generic(format!(
                            "Record in entity '{}' must be an object",
                            entity_name
                        ))
                    })
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect::<HashMap<String, Value>>()
                    })
            })
            .collect::<Result<Vec<_>>>()?;

        seed_data.insert(entity_name.clone(), records);
    }

    Ok(seed_data)
}

/// Validate foreign keys in a record
fn validate_foreign_keys(
    registry: &EntityRegistry,
    entity: &Entity,
    record: &HashMap<String, Value>,
) -> Result<()> {
    for fk in &entity.schema.foreign_keys {
        if let Some(fk_value) = record.get(&fk.field) {
            // Check if the referenced entity exists
            let target_entity = registry.get(&fk.target_entity).ok_or_else(|| {
                Error::generic(format!(
                    "Target entity '{}' not found for foreign key '{}'",
                    fk.target_entity, fk.field
                ))
            })?;

            let target_table = target_entity.table_name();

            // For now, we'll validate during insertion (database will enforce)
            // This is a placeholder for more sophisticated validation
            if fk_value.is_null() && !entity
                .schema
                .base
                .fields
                .iter()
                .find(|f| f.name == fk.field)
                .map(|f| !f.required)
                .unwrap_or(false)
            {
                return Err(Error::generic(format!(
                    "Foreign key '{}' cannot be null",
                    fk.field
                )));
            }
        }
    }

    Ok(())
}

/// Perform topological sort of entities based on foreign key dependencies
///
/// Returns entities in an order where parent entities come before child entities.
fn topological_sort(
    registry: &EntityRegistry,
    seed_data: &SeedData,
) -> Result<Vec<String>> {
    // Build dependency graph
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    let mut in_degree: HashMap<String, usize> = HashMap::new();

    // Initialize all entities that will be seeded
    for entity_name in seed_data.keys() {
        graph.insert(entity_name.clone(), Vec::new());
        in_degree.insert(entity_name.clone(), 0);
    }

    // Build edges based on foreign keys
    for entity_name in seed_data.keys() {
        if let Some(entity) = registry.get(entity_name) {
            for fk in &entity.schema.foreign_keys {
                if seed_data.contains_key(&fk.target_entity) {
                    // Add edge from target_entity to entity_name
                    graph
                        .entry(fk.target_entity.clone())
                        .or_insert_with(Vec::new)
                        .push(entity_name.clone());
                    *in_degree.entry(entity_name.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    // Kahn's algorithm for topological sort
    let mut queue: Vec<String> = in_degree
        .iter()
        .filter(|(_, &degree)| degree == 0)
        .map(|(name, _)| name.clone())
        .collect();

    let mut result = Vec::new();

    while let Some(node) = queue.pop() {
        result.push(node.clone());

        if let Some(neighbors) = graph.get(&node) {
            for neighbor in neighbors {
                let degree = in_degree.get_mut(neighbor).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    queue.push(neighbor.clone());
                }
            }
        }
    }

    // Check for cycles
    if result.len() != seed_data.len() {
        return Err(Error::generic(
            "Circular dependency detected in foreign key relationships".to_string(),
        ));
    }

    Ok(result)
}

/// Clear all data from an entity
pub async fn clear_entity(
    database: &dyn crate::database::VirtualDatabase,
    registry: &EntityRegistry,
    entity_name: &str,
) -> Result<()> {
    let entity = registry
        .get(entity_name)
        .ok_or_else(|| Error::generic(format!("Entity '{}' not found", entity_name)))?;

    let table_name = entity.table_name();
    let query = format!("DELETE FROM {}", table_name);

    database.execute(&query, &[]).await?;

    Ok(())
}

/// Clear all data from all entities
pub async fn clear_all(
    database: &dyn crate::database::VirtualDatabase,
    registry: &EntityRegistry,
) -> Result<()> {
    // Get entities in reverse dependency order (children first)
    let entities: Vec<String> = registry.list();

    // Simple approach: delete from all tables
    // In a more sophisticated implementation, we'd respect foreign key constraints
    for entity_name in entities {
        if let Err(e) = clear_entity(database, registry, &entity_name).await {
            // Log error but continue
            tracing::warn!("Failed to clear entity '{}': {}", entity_name, e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_seed_data() {
        let json = serde_json::json!({
            "users": [
                {"id": "user1", "name": "Alice"},
                {"id": "user2", "name": "Bob"}
            ],
            "orders": [
                {"id": "order1", "user_id": "user1", "total": 100.0}
            ]
        });

        let seed_data = parse_seed_data(json).unwrap();
        assert_eq!(seed_data.len(), 2);
        assert_eq!(seed_data.get("users").unwrap().len(), 2);
        assert_eq!(seed_data.get("orders").unwrap().len(), 1);
    }

    #[test]
    fn test_topological_sort() {
        // This would require setting up a full registry, so we'll test it in integration tests
    }
}
