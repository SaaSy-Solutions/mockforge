//! Entity management and registry
//!
//! This module provides entity definition, management, and registry functionality
//! for tracking all entities in the VBR engine.

use crate::schema::VbrSchemaDefinition;
use crate::{Error, Result};
use std::collections::HashMap;

/// Entity definition
#[derive(Debug, Clone)]
pub struct Entity {
    /// Entity name
    pub name: String,

    /// Schema definition
    pub schema: VbrSchemaDefinition,

    /// Table name (derived from entity name)
    pub table_name: String,
}

impl Entity {
    /// Create a new entity
    pub fn new(name: String, schema: VbrSchemaDefinition) -> Self {
        let table_name = name.to_lowercase() + "s"; // Simple pluralization
        Self {
            name,
            schema,
            table_name,
        }
    }

    /// Get the entity name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the table name
    pub fn table_name(&self) -> &str {
        &self.table_name
    }
}

/// Entity registry for managing all entities
#[derive(Clone)]
pub struct EntityRegistry {
    /// Registered entities by name
    entities: HashMap<String, Entity>,
}

impl EntityRegistry {
    /// Create a new entity registry
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    /// Register an entity
    pub fn register(&mut self, entity: Entity) -> Result<()> {
        let name = entity.name.clone();
        if self.entities.contains_key(&name) {
            return Err(Error::generic(format!("Entity '{}' already registered", name)));
        }
        self.entities.insert(name, entity);
        Ok(())
    }

    /// Get an entity by name
    pub fn get(&self, name: &str) -> Option<&Entity> {
        self.entities.get(name)
    }

    /// Get all entity names
    pub fn list(&self) -> Vec<String> {
        self.entities.keys().cloned().collect()
    }

    /// Check if an entity exists
    pub fn exists(&self, name: &str) -> bool {
        self.entities.contains_key(name)
    }

    /// Remove an entity
    pub fn remove(&mut self, name: &str) -> Result<()> {
        self.entities
            .remove(name)
            .ok_or_else(|| Error::generic(format!("Entity '{}' not found", name)))?;
        Ok(())
    }
}

impl Default for EntityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_data::SchemaDefinition;

    #[test]
    fn test_entity_creation() {
        let base_schema = SchemaDefinition::new("User".to_string());
        let vbr_schema = VbrSchemaDefinition::new(base_schema);
        let entity = Entity::new("User".to_string(), vbr_schema);

        assert_eq!(entity.name(), "User");
        assert_eq!(entity.table_name(), "users");
    }

    #[test]
    fn test_entity_registry() {
        let mut registry = EntityRegistry::new();

        let base_schema = SchemaDefinition::new("User".to_string());
        let vbr_schema = VbrSchemaDefinition::new(base_schema);
        let entity = Entity::new("User".to_string(), vbr_schema);

        assert!(registry.register(entity).is_ok());
        assert!(registry.exists("User"));
        assert!(registry.get("User").is_some());
    }

    #[test]
    fn test_entity_registry_duplicate() {
        let mut registry = EntityRegistry::new();

        let base_schema1 = SchemaDefinition::new("User".to_string());
        let vbr_schema1 = VbrSchemaDefinition::new(base_schema1);
        let entity1 = Entity::new("User".to_string(), vbr_schema1);

        let base_schema2 = SchemaDefinition::new("User".to_string());
        let vbr_schema2 = VbrSchemaDefinition::new(base_schema2);
        let entity2 = Entity::new("User".to_string(), vbr_schema2);

        assert!(registry.register(entity1).is_ok());
        assert!(registry.register(entity2).is_err());
    }
}
